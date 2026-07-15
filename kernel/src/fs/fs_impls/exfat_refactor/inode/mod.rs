// SPDX-License-Identifier: MPL-2.0

//! Defines the exFAT inode owner and forwards VFS trait methods to focused submodules.
//!
//! Method groups: inode construction and VFS trait dispatch.

mod dir_mutation;
mod file_mutation;
mod file_read;
mod lookup;
mod metadata;
mod page_backend;
mod parent_entry_set;
mod state;
pub(super) mod sync;

use core::{
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

use spin::Once;

pub(in crate::fs::fs_impls::exfat_refactor) use self::state::{
    ClusterMap, StreamExtensionDirEntry,
};
use self::{
    state::InodeState,
    sync::InodeSyncScope,
};
use super::{
    dir_entry_format::DirEntrySlotRange,
    fs::ExfatFs,
    invalid_on_disk_layout,
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
                RenameMode, SymbolicLink,
            },
        },
    },
    prelude::*,
    process::{Gid, Uid},
    vm::page_cache::PageCache,
};

pub(super) struct ExfatInode {
    inode_state: RwMutex<InodeState>,
    extension: Extension,
    fs: Weak<ExfatFs>,
    entry_set_location_hint: AtomicU64,
    page_backend: Arc<page_backend::ExfatFilePageBackend>,
    page_cache: Once<Option<PageCache>>,
    weak_self: Weak<ExfatInode>,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(super) enum PersistenceRecovery {
    RollbackAllowed,
    RewriteRequired,
}

impl ExfatInode {
    fn new(
        fs: &Arc<ExfatFs>,
        metadata: Metadata,
        dir_entry_stream: StreamExtensionDirEntry,
        cluster_map: Option<Arc<ClusterMap>>,
        parent: Weak<Self>,
    ) -> Arc<Self> {
        Arc::new_cyclic(|weak_self| Self {
            inode_state: RwMutex::new(InodeState {
                dirty_state: Default::default(),
                dirty_file_retention: None,
                metadata,
                parent,
                cluster_map,
                dir_entry_stream,
            }),
            extension: Extension::new(),
            fs: Arc::downgrade(fs),
            entry_set_location_hint: AtomicU64::new(0),
            page_backend: Arc::new(page_backend::ExfatFilePageBackend::new(
                fs.immutable_block_device(),
                fs.immutable_boot_region(),
            )),
            page_cache: Once::new(),
            weak_self: weak_self.clone(),
        })
    }

    pub(super) fn new_root(
        fs: &Arc<ExfatFs>,
        root_cluster_map: Arc<ClusterMap>,
        cluster_size: usize,
    ) -> Result<Arc<Self>> {
        let root_stream = root_cluster_map.stream_extension();
        let root_cluster = root_stream.first_cluster;
        let allocated_size = root_cluster_map.allocated_byte_length(&fs.immutable_boot_region())?;
        let root_ino = u64::from(root_cluster);
        let mut metadata = Metadata::new_dir(
            root_ino,
            mkmod!(u+rwx, g+rx, o+rx),
            cluster_size,
            fs.container_device_id(),
        );
        metadata.size = allocated_size;
        let root_inode = Self::new(
            fs,
            metadata,
            root_stream,
            Some(root_cluster_map),
            Weak::new(),
        );
        ExfatFs::publish_cached_inode(&mut fs.fs_state.write(), root_ino, &root_inode);
        Ok(root_inode)
    }

    fn new_child(
        fs: &Arc<ExfatFs>,
        parent: Weak<Self>,
        ino: u64,
        inode_type: InodeType,
        size: usize,
        child_stream: StreamExtensionDirEntry,
        cluster_map: Option<Arc<ClusterMap>>,
    ) -> Arc<Self> {
        let cluster_size = fs.immutable_boot_region().cluster_size;
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
            child_stream,
            cluster_map,
            parent,
        )
    }

    pub(super) fn entry_set_location_hint(&self) -> Result<Option<DirEntrySlotRange>> {
        let packed_hint = self.entry_set_location_hint.load(Ordering::Relaxed);
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
            usize::try_from(encoded_first_entry_index - 1).map_err(|_| invalid_on_disk_layout())?,
            usize::try_from(entry_count).map_err(|_| invalid_on_disk_layout())?,
        )
        .map(Some)
    }

    pub(super) fn store_entry_set_location_hint(
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
        self.entry_set_location_hint
            .store(packed_hint, Ordering::Relaxed);
        Ok(())
    }

    pub(super) fn clear_entry_set_location_hint(&self) {
        self.entry_set_location_hint.store(0, Ordering::Relaxed);
    }

    pub(super) fn stable_lock_identity(&self) -> usize {
        core::ptr::from_ref(self).addr()
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
        self.inode_state_read_guard().metadata().size
    }

    fn resize(&self, new_size: usize) -> Result<()> {
        self.resize_impl(new_size)
    }

    fn metadata(&self) -> Metadata {
        self.inode_state_read_guard().metadata()
    }

    fn ino(&self) -> u64 {
        self.inode_state_read_guard().metadata().ino
    }

    fn type_(&self) -> InodeType {
        self.inode_state_read_guard().metadata().type_
    }

    fn mode(&self) -> Result<InodeMode> {
        Ok(self.inode_state_read_guard().metadata().mode)
    }

    fn set_mode(&self, mode: InodeMode) -> Result<()> {
        self.set_mode_impl(mode)
    }

    fn owner(&self) -> Result<Uid> {
        Ok(self.inode_state_read_guard().metadata().uid)
    }

    fn set_owner(&self, uid: Uid) -> Result<()> {
        self.set_owner_impl(uid)
    }

    fn group(&self) -> Result<Gid> {
        Ok(self.inode_state_read_guard().metadata().gid)
    }

    fn set_group(&self, gid: Gid) -> Result<()> {
        self.set_group_impl(gid)
    }

    fn atime(&self) -> Duration {
        self.inode_state_read_guard().metadata().last_access_at
    }

    fn set_atime(&self, time: Duration) {
        self.set_atime_impl(time);
    }

    fn mtime(&self) -> Duration {
        self.inode_state_read_guard().metadata().last_modify_at
    }

    fn set_mtime(&self, time: Duration) {
        self.set_mtime_impl(time);
    }

    fn ctime(&self) -> Duration {
        self.inode_state_read_guard().metadata().last_meta_change_at
    }

    fn set_ctime(&self, time: Duration) {
        self.set_ctime_impl(time);
    }

    fn page_cache(&self) -> Option<PageCache> {
        let metadata = self.inode_state_read_guard().metadata();
        if metadata.type_ != InodeType::File {
            return None;
        }
        self.page_cache_handle(metadata).cloned()
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

    fn rename(
        &self,
        old_name: &str,
        target: &Arc<dyn Inode>,
        new_name: &str,
        mode: RenameMode,
    ) -> Result<()> {
        match mode {
            RenameMode::Replace | RenameMode::NoReplace => {
                self.rename_impl(old_name, target, new_name)
            }
            RenameMode::Exchange => return_errno_with_message!(
                Errno::EINVAL,
                "RENAME_EXCHANGE is not supported on exfat_refactor"
            ),
        }
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
        self.sync_regular_file(InodeSyncScope::All)
    }

    fn sync_data(&self) -> Result<()> {
        self.sync_regular_file(InodeSyncScope::Data)
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
