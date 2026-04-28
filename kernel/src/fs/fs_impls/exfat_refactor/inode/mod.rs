// SPDX-License-Identifier: MPL-2.0

mod cached_io;
mod content_mutation;
mod dir_metadata;
mod file_metadata;
mod lookup;
mod mutation;
mod page_backend;
mod shared;
mod sync;

use alloc::{string::String, vec, vec::Vec};
use core::time::Duration;

use aster_block::{
    BlockDevice,
    bio::{Bio, BioDirection, BioSegment, BioStatus, BioType, BioWaiter},
    id::Sid,
};
use ostd::{
    mm::{FallibleVmWrite, Segment, VmIo, VmReader, io::util::HasVmReaderWriter},
    sync::{RwMutex, RwMutexReadGuard, RwMutexWriteGuard},
};
use spin::Once;
use time::{Date, Month, OffsetDateTime, PrimitiveDateTime, Time, UtcOffset};

use super::{
    bitmap::ClusterRange,
    boot::BootRegion,
    direntry::{
        self, DIRECTORY_ENTRY_SIZE, DirectoryEntryAnomalyKind, DirectoryEntrySlotRange,
        FileEntrySetView, ScannedDirectoryEntry, WritableDirectoryEntrySlotSpan,
    },
    fat::{ChainVisitControl, FatChainStep, FatReader},
    fs::{ExfatFs, ExfatMountOptions, MountVolumeStateError, MountedVolumeState},
    upcase::UpcaseTable,
};
use crate::{
    fs::{
        file::{AccessMode, FileIo, InodeMode, InodeType, StatusFlags, chmod, mkmod},
        utils::DirentVisitor,
        vfs::{
            file_system::{FileSystem, FsFlags},
            inode::{Extension, FallocMode, Inode, Metadata, MknodType, SymbolicLink},
            page_cache::{CachePage, PageCache, PageCacheBackend},
        },
    },
    prelude::*,
    process::{Gid, Uid},
    time::clocks::RealTimeCoarseClock,
    vm::vmo::Vmo,
};

use self::shared::{ExfatInodeDirtyState, ExfatInodeStream, FileSyncScope};

#[derive(Clone, Copy)]
enum RewriteTarget {
    Directory,
    RegularFile,
}

#[derive(Clone, Copy)]
enum TimestampFieldKind {
    Accessed,
    Modified,
}

pub(super) struct ExfatInode {
    admission: RwMutex<()>,
    dirty_state: RwLock<ExfatInodeDirtyState>,
    extension: Extension,
    fs: Weak<ExfatFs>,
    metadata: RwLock<Metadata>,
    parent: Weak<Self>,
    page_cache: Once<Option<PageCache>>,
    stream: RwLock<ExfatInodeStream>,
    this: Weak<Self>,
}

impl ExfatInode {
    pub(super) fn read_root_directory<T>(
        &self,
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        read_root_directory_fn: impl FnOnce(&[u8]) -> core::result::Result<T, MountVolumeStateError>,
    ) -> core::result::Result<T, MountVolumeStateError> {
        let _directory_guard = self.admission.read();
        let stream = *self.stream.read();
        if stream.data_length.is_some() {
            return Err(MountVolumeStateError::InvalidOperationInput);
        }

        let directory_bytes =
            Self::read_directory_bytes_for_stream(block_device, boot_region, stream)?;
        read_root_directory_fn(&directory_bytes)
    }

    pub(super) fn rewrite_root_directory<T>(
        &self,
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        rewrite_root_directory_fn: impl FnOnce(
            &mut Vec<u8>,
        )
            -> core::result::Result<T, MountVolumeStateError>,
    ) -> core::result::Result<T, MountVolumeStateError> {
        let _directory_guards = Self::ordered_directory_write_guards(vec![self]);
        let stream = *self.stream.read();
        if stream.data_length.is_some() {
            return Err(MountVolumeStateError::InvalidOperationInput);
        }

        let mut directory_bytes =
            Self::read_directory_bytes_for_stream(block_device, boot_region, stream)?;
        let rewrite_result = rewrite_root_directory_fn(&mut directory_bytes)?;
        Self::write_directory_bytes_for_stream(
            block_device,
            boot_region,
            &directory_bytes,
            stream,
        )?;
        Ok(rewrite_result)
    }

    fn new(
        fs: &Arc<ExfatFs>,
        metadata: Metadata,
        first_cluster: u32,
        data_length: Option<usize>,
        valid_data_length: Option<usize>,
        no_fat_chain: bool,
        parent: Weak<Self>,
    ) -> Arc<Self> {
        Arc::new_cyclic(|weak_self| Self {
            admission: RwMutex::new(()),
            dirty_state: RwLock::new(ExfatInodeDirtyState::default()),
            extension: Extension::new(),
            fs: Arc::downgrade(fs),
            metadata: RwLock::new(metadata),
            parent,
            page_cache: Once::new(),
            stream: RwLock::new(ExfatInodeStream {
                data_length,
                first_cluster,
                valid_data_length,
                no_fat_chain,
            }),
            this: weak_self.clone(),
        })
    }

    pub(super) fn new_root(fs: &Arc<ExfatFs>, root_cluster: u32, cluster_size: usize) -> Arc<Self> {
        let mut metadata = Metadata::new_dir(
            u64::from(root_cluster),
            mkmod!(u+rwx, g+rx, o+rx),
            cluster_size,
            fs.container_device_id(),
        );
        metadata.size = cluster_size;
        Self::new(fs, metadata, root_cluster, None, None, false, Weak::new())
    }

    fn new_child(
        fs: &Arc<ExfatFs>,
        parent: Weak<Self>,
        ino: u64,
        inode_type: InodeType,
        cluster_size: usize,
        size: usize,
        first_cluster: u32,
        data_length: usize,
        valid_data_length: usize,
        no_fat_chain: bool,
    ) -> Arc<Self> {
        let mut metadata = match inode_type {
            InodeType::Dir => Metadata::new_dir(
                ino,
                mkmod!(u+rwx, g+rx, o+rx),
                cluster_size,
                fs.container_device_id(),
            ),
            _ => Metadata::new_file(
                ino,
                mkmod!(u+rw, g+r, o+r),
                cluster_size,
                fs.container_device_id(),
            ),
        };
        metadata.size = size;
        Self::new(
            fs,
            metadata,
            first_cluster,
            Some(data_length),
            Some(valid_data_length),
            no_fat_chain,
            parent,
        )
    }
}

impl crate::fs::vfs::inode::InodeIo for ExfatInode {
    fn read_at(
        &self,
        offset: usize,
        writer: &mut VmWriter,
        status_flags: StatusFlags,
    ) -> Result<usize> {
        self.read_at_impl(offset, writer, status_flags)
    }

    fn write_at(
        &self,
        offset: usize,
        reader: &mut VmReader,
        status_flags: StatusFlags,
    ) -> Result<usize> {
        self.write_at_impl(offset, reader, status_flags)
    }
}

impl Inode for ExfatInode {
    fn size(&self) -> usize {
        self.metadata_projection().size
    }

    fn resize(&self, new_size: usize) -> Result<()> {
        self.resize_impl(new_size)
    }

    fn metadata(&self) -> Metadata {
        self.metadata_impl()
    }

    fn ino(&self) -> u64 {
        self.ino_impl()
    }

    fn type_(&self) -> InodeType {
        self.type_impl()
    }

    fn mode(&self) -> Result<InodeMode> {
        self.mode_impl()
    }

    fn set_mode(&self, mode: InodeMode) -> Result<()> {
        self.set_mode_impl(mode)
    }

    fn owner(&self) -> Result<Uid> {
        self.owner_impl()
    }

    fn set_owner(&self, uid: Uid) -> Result<()> {
        self.set_owner_impl(uid)
    }

    fn group(&self) -> Result<Gid> {
        self.group_impl()
    }

    fn set_group(&self, gid: Gid) -> Result<()> {
        self.set_group_impl(gid)
    }

    fn atime(&self) -> Duration {
        self.atime_impl()
    }

    fn set_atime(&self, time: Duration) {
        self.set_atime_impl(time);
    }

    fn mtime(&self) -> Duration {
        self.mtime_impl()
    }

    fn set_mtime(&self, time: Duration) {
        self.set_mtime_impl(time);
    }

    fn ctime(&self) -> Duration {
        self.ctime_impl()
    }

    fn set_ctime(&self, time: Duration) {
        self.set_ctime_impl(time);
    }

    fn page_cache(&self) -> Option<Arc<Vmo>> {
        self.page_cache_vmo()
    }

    fn create(&self, name: &str, type_: InodeType, mode: InodeMode) -> Result<Arc<dyn Inode>> {
        self.create_impl(name, type_, mode)
    }

    fn mknod(&self, _name: &str, _mode: InodeMode, _type_: MknodType) -> Result<Arc<dyn Inode>> {
        return_errno!(Errno::EOPNOTSUPP);
    }

    fn open(
        &self,
        _access_mode: AccessMode,
        _status_flags: StatusFlags,
    ) -> Option<Result<Box<dyn FileIo>>> {
        None
    }

    fn readdir_at(&self, offset: usize, visitor: &mut dyn DirentVisitor) -> Result<usize> {
        self.readdir_at_impl(offset, visitor)
    }

    fn link(&self, _old: &Arc<dyn Inode>, _name: &str) -> Result<()> {
        return_errno!(Errno::EOPNOTSUPP);
    }

    fn unlink(&self, name: &str) -> Result<()> {
        self.unlink_impl(name)
    }

    fn rmdir(&self, name: &str) -> Result<()> {
        self.rmdir_impl(name)
    }

    fn lookup(&self, name: &str) -> Result<Arc<dyn Inode>> {
        self.lookup_impl(name)
    }

    fn rename(&self, old_name: &str, target: &Arc<dyn Inode>, new_name: &str) -> Result<()> {
        self.rename_impl(old_name, target, new_name)
    }

    fn read_link(&self) -> Result<SymbolicLink> {
        if self.type_() == InodeType::Dir {
            return_errno!(Errno::EISDIR);
        }
        return_errno!(Errno::EOPNOTSUPP);
    }

    fn write_link(&self, _target: &str) -> Result<()> {
        if self.type_() == InodeType::Dir {
            return_errno!(Errno::EISDIR);
        }
        return_errno!(Errno::EOPNOTSUPP);
    }

    fn sync_all(&self) -> Result<()> {
        if self.type_() != InodeType::File {
            return Ok(());
        }

        self.sync_regular_file(FileSyncScope::All)
    }

    fn sync_data(&self) -> Result<()> {
        if self.type_() != InodeType::File {
            return Ok(());
        }

        self.sync_regular_file(FileSyncScope::Data)
    }

    fn fallocate(&self, _mode: FallocMode, _offset: usize, _len: usize) -> Result<()> {
        if self.type_() == InodeType::Dir {
            return_errno!(Errno::EISDIR);
        }
        return_errno!(Errno::EOPNOTSUPP);
    }

    fn fs(&self) -> Arc<dyn FileSystem> {
        match Weak::upgrade(&self.fs) {
            Some(fs) => fs,
            None => unreachable!("published exFAT inode must keep its filesystem alive"),
        }
    }

    fn extension(&self) -> &Extension {
        &self.extension
    }

    fn is_dentry_cacheable(&self) -> bool {
        false
    }
}

#[cfg(ktest)]
#[path = "../test_support/inode_ktests.rs"]
mod tests;
