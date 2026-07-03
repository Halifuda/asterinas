// SPDX-License-Identifier: MPL-2.0

//! Defines the exFAT inode owner and forwards VFS trait methods to focused submodules.
//!
//! Method groups: root-directory byte APIs, inode construction, and VFS trait dispatch.

mod cached_io;
mod dir_mutation;
mod file_mutation;
mod lookup;
mod metadata;
mod page_backend;
mod state;
mod sync;

use alloc::{vec, vec::Vec};
use core::{
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

use aster_block::BlockDevice;
use ostd::{mm::VmReader, sync::RwMutex};
use spin::Once;

pub(in crate::fs::fs_impls::exfat_refactor) use self::state::{
    ClusterMap, StreamExtensionDirEntry,
};
use self::{
    state::{InodeDirtyState, InodeTimestampField},
    sync::InodeSyncScope,
};
use super::{
    boot::BootRegion,
    direntry::DirEntrySlotRange,
    fs::{ExfatFs, MountedVolumeState},
    invalid_on_disk_layout,
    invalid_operation_input,
    upcase::UpcaseTable,
};
use crate::{
    fs::{
        file::{AccessMode, InodeMode, InodeType, PerOpenFileOps, StatusFlags, mkmod},
        utils::DirentVisitor,
        vfs::{
            file_system::FileSystem,
            inode::{
                Extension, FallocMode, FileOps, Inode, Metadata, MknodType, RevalidationPolicy,
                SymbolicLink,
            },
        },
    },
    prelude::*,
    process::{Gid, Uid},
    vm::page_cache::PageCache,
};

pub(super) struct ExfatInode {
    inode_state: RwMutex<()>,
    dirty_state: RwLock<InodeDirtyState>,
    extension: Extension,
    fs: Weak<ExfatFs>,
    metadata: RwLock<Metadata>,
    parent: RwLock<Weak<Self>>,
    regular_file_entry_set_location_hint: AtomicU64,
    page_backend: Arc<page_backend::ExfatFilePageBackend>,
    page_cache: Once<Option<PageCache>>,
    page_cache_context: RwLock<Option<page_backend::PageCacheContext>>,
    cluster_map: RwLock<Option<Arc<ClusterMap>>>,
    dir_entry_stream: RwLock<StreamExtensionDirEntry>,
}

impl ExfatInode {
    pub(super) fn read_root_directory_bytes(
        &self,
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
    ) -> Result<Vec<u8>> {
        let _directory_guard = self.inode_state.read();
        let cluster_map = *self.dir_entry_stream.read();
        if cluster_map.data_length.is_some() {
            return Err(invalid_operation_input());
        }

        Self::read_directory_bytes_for_cluster_map(block_device, boot_region, cluster_map)
    }

    pub(super) fn rewrite_root_directory_bytes(
        &self,
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        directory_bytes: &[u8],
    ) -> Result<()> {
        let _directory_guards = Self::directory_write_guards_by_ino(vec![self]);
        let cluster_map = *self.dir_entry_stream.read();
        if cluster_map.data_length.is_some() {
            return Err(invalid_operation_input());
        }

        Self::write_directory_bytes_for_cluster_map(
            block_device,
            boot_region,
            directory_bytes,
            cluster_map,
        )
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
            inode_state: RwMutex::new(()),
            dirty_state: RwLock::new(InodeDirtyState::default()),
            extension: Extension::new(),
            fs: Arc::downgrade(fs),
            metadata: RwLock::new(metadata),
            parent: RwLock::new(parent),
            regular_file_entry_set_location_hint: AtomicU64::new(0),
            page_backend: Arc::new(page_backend::ExfatFilePageBackend::new(weak_self.clone())),
            page_cache: Once::new(),
            page_cache_context: RwLock::new(None),
            cluster_map: RwLock::new(None),
            dir_entry_stream: RwLock::new(StreamExtensionDirEntry {
                data_length,
                first_cluster,
                valid_data_length,
                no_fat_chain,
            }),
        })
    }

    pub(super) fn new_root(fs: &Arc<ExfatFs>, root_cluster: u32, cluster_size: usize) -> Arc<Self> {
        let root_ino = u64::from(root_cluster);
        let mut metadata = Metadata::new_dir(
            root_ino,
            mkmod!(u+rwx, g+rx, o+rx),
            cluster_size,
            fs.container_device_id(),
        );
        metadata.size = cluster_size;
        fs.get_or_create_cached_inode(root_ino, || {
            Self::new(fs, metadata, root_cluster, None, None, false, Weak::new())
        })
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
        fs.get_or_create_cached_inode(ino, || {
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
        })
    }

    pub(super) fn regular_file_entry_set_location_hint(&self) -> Result<Option<DirEntrySlotRange>> {
        let packed_hint = self.regular_file_entry_set_location_hint.load(Ordering::Relaxed);
        if packed_hint == 0 {
            return Ok(None);
        }

        let encoded_first_entry_index =
            u32::try_from(packed_hint >> 32).map_err(|_| invalid_on_disk_layout())?;
        let entry_count = u32::try_from(packed_hint & u64::from(u32::MAX))
            .map_err(|_| invalid_on_disk_layout())?;
        if encoded_first_entry_index == 0 || entry_count == 0 {
            return Ok(None);
        }

        DirEntrySlotRange::new(
            usize::try_from(encoded_first_entry_index - 1)
                .map_err(|_| invalid_on_disk_layout())?,
            usize::try_from(entry_count).map_err(|_| invalid_on_disk_layout())?,
        )
        .map(Some)
    }

    pub(super) fn store_regular_file_entry_set_location_hint(
        &self,
        slot_range: DirEntrySlotRange,
    ) -> Result<()> {
        let encoded_first_entry_index = u64::from(
            u32::try_from(slot_range.first_entry_index()).map_err(|_| invalid_on_disk_layout())?,
        )
        .checked_add(1)
        .ok_or_else(invalid_on_disk_layout)?;
        let entry_count = u64::from(
            u32::try_from(slot_range.entry_count()).map_err(|_| invalid_on_disk_layout())?,
        );
        let packed_hint = (encoded_first_entry_index << 32) | entry_count;
        self.regular_file_entry_set_location_hint
            .store(packed_hint, Ordering::Relaxed);
        Ok(())
    }

    pub(super) fn clear_regular_file_entry_set_location_hint(&self) {
        self.regular_file_entry_set_location_hint
            .store(0, Ordering::Relaxed);
    }
}

impl FileOps for ExfatInode {
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

    fn readdir_at(&self, offset: usize, visitor: &mut dyn DirentVisitor) -> Result<usize> {
        self.readdir_at_impl(offset, visitor)
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

    fn page_cache(&self) -> Option<PageCache> {
        self.page_cache_handle().cloned()
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
    ) -> Option<Result<Box<dyn PerOpenFileOps>>> {
        None
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
        match self.type_() {
            InodeType::File => self.sync_regular_file(InodeSyncScope::All),
            // exFAT rewrites directory metadata through the parent entry during mutation, so
            // directories do not retain a separate deferred writeback path for `sync_all()`.
            InodeType::Dir => Ok(()),
            _ => Ok(()),
        }
    }

    fn sync_data(&self) -> Result<()> {
        match self.type_() {
            InodeType::File => self.sync_regular_file(InodeSyncScope::Data),
            // `sync_data()` matches `sync_all()` for directories because namespace mutations
            // already rewrite the owning parent entry eagerly.
            InodeType::Dir => Ok(()),
            _ => Ok(()),
        }
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
            None => unreachable!("mounted exFAT inode must keep its filesystem alive"),
        }
    }

    fn extension(&self) -> &Extension {
        &self.extension
    }

    fn revalidation_policy(&self) -> RevalidationPolicy {
        if self.type_() == InodeType::Dir {
            return RevalidationPolicy::REVALIDATE_EXISTS | RevalidationPolicy::REVALIDATE_ABSENT;
        }
        RevalidationPolicy::empty()
    }

    fn revalidate_exists(&self, _name: &str, _child: &dyn Inode) -> bool {
        true
    }

    fn revalidate_absent(&self, _name: &str) -> bool {
        true
    }
}
