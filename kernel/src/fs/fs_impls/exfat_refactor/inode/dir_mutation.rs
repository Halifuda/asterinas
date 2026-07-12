// SPDX-License-Identifier: MPL-2.0

//! Implements directory namespace mutations and directory cluster-map growth.
//!
//! Method groups: create/unlink/rmdir/rename entry points, slot management, directory growth,
//! emptiness validation, cluster-range collection, and rename-stage helpers.

use aster_block::BlockDevice;

use super::{
    super::{
        bitmap::ClusterRange,
        boot::BootRegion,
        direntry::{
            self, DIRECTORY_ENTRY_SIZE, DirEntrySlotRange, FileEntrySetView,
            MutableDirEntrySlotSpan, ScannedDirEntry,
        },
        fat::{ChainVisitControl, FatReader},
        fs::{AllocGuard, ExfatFs, FsState},
        invalid_on_disk_layout, invalid_operation_input,
    },
    ClusterMap, ExfatInode, StreamExtensionDirEntry, UpcaseTable,
    state::InodeStateWriteGuard,
};
use crate::{
    fs::{
        file::{InodeMode, InodeType},
        vfs::{file_system::FsFlags, inode::Inode},
    },
    prelude::*,
    time::clocks::RealTimeCoarseClock,
};

enum ReplacedTargetCleanup {
    Immediate {
        slot_range: DirEntrySlotRange,
        ranges: Vec<ClusterRange>,
    },
    CachedGeneration {
        slot_range: DirEntrySlotRange,
        cluster_map: Arc<ClusterMap>,
        ranges: Vec<ClusterRange>,
    },
}

struct AdmittedRenameChild<'inode, 'guard> {
    inode: &'inode Arc<ExfatInode>,
    guard: &'inode InodeStateWriteGuard<'guard>,
}

struct RenameNames<'a> {
    source: &'a [u16],
    source_hash: u16,
    destination: &'a [u16],
    destination_hash: u16,
}

struct SameDirectoryRenamePlan {
    cluster_map: StreamExtensionDirEntry,
    directory_bytes: Vec<u8>,
    source_slot_range: DirEntrySlotRange,
    destination_slot_range: DirEntrySlotRange,
    replaced_slot_range: Option<DirEntrySlotRange>,
    renamed_entry_set: Vec<u8>,
    replacement: Option<ReplacedTargetCleanup>,
}

struct CrossDirectoryRenamePlan {
    source_cluster_map: StreamExtensionDirEntry,
    source_directory_bytes: Vec<u8>,
    source_slot_range: DirEntrySlotRange,
    target_cluster_map: StreamExtensionDirEntry,
    target_directory_bytes: Vec<u8>,
    target_slot_range: DirEntrySlotRange,
    renamed_entry_set: Vec<u8>,
    replacement: Option<ReplacedTargetCleanup>,
}

enum RenameDiscoveryRole {
    Source,
    Replacement,
}

enum RenameDiscovery {
    SameDirectory {
        parent_directory: Option<Arc<ExfatInode>>,
        source_child_inode: Arc<ExfatInode>,
        target_child_inode: Option<Arc<ExfatInode>>,
    },
    CrossDirectory {
        source_parent_directory: Option<Arc<ExfatInode>>,
        target_parent_directory: Option<Arc<ExfatInode>>,
        source_child_inode: Arc<ExfatInode>,
        target_child_inode: Option<Arc<ExfatInode>>,
    },
}

enum FinalRenameAdmission<'a, 'guard> {
    SameDirectory {
        directory_guard: &'a InodeStateWriteGuard<'guard>,
        parent_guard: Option<&'a InodeStateWriteGuard<'guard>>,
        source_child: AdmittedRenameChild<'a, 'guard>,
        target_child: Option<AdmittedRenameChild<'a, 'guard>>,
        cluster_map: StreamExtensionDirEntry,
    },
    CrossDirectory {
        source_guard: &'a InodeStateWriteGuard<'guard>,
        source_parent_guard: Option<&'a InodeStateWriteGuard<'guard>>,
        target_guard: &'a InodeStateWriteGuard<'guard>,
        target_parent_guard: Option<&'a InodeStateWriteGuard<'guard>>,
        source_child: AdmittedRenameChild<'a, 'guard>,
        target_child: Option<AdmittedRenameChild<'a, 'guard>>,
        source_cluster_map: StreamExtensionDirEntry,
        target_cluster_map: StreamExtensionDirEntry,
    },
}

impl ExfatInode {
    // VFS entry points

    pub(super) fn create_impl(
        &self,
        name: &str,
        type_: InodeType,
        mode: InodeMode,
    ) -> Result<Arc<dyn Inode>> {
        if !matches!(type_, InodeType::File | InodeType::Dir) {
            return_errno!(Errno::EOPNOTSUPP);
        }

        let fs = self
            .fs
            .upgrade()
            .ok_or_else(|| Error::with_message(Errno::EIO, "exFAT filesystem is not mounted"))?;
        let mut fs_state = fs.fs_state.write();
        let mount_state = fs_state.mount_state.as_ref().ok_or_else(super::super::not_mounted)?;
        if mount_state.forced_shutdown {
            return_errno!(Errno::EIO);
        }
        let block_device = fs.immutable_block_device();
        let boot_region = fs.immutable_boot_region();
        let upcase_table = fs_state.upcase_table.as_ref().ok_or_else(super::super::not_mounted)?.clone();
        let options = mount_state.options.clone();
        if options.fs_flags.contains(FsFlags::RDONLY) {
            return_errno!(Errno::EROFS);
        }

        let name = Self::validate_name(name, &options)?;
        let name_hash = upcase_table.name_hash(&name);
        let required_entry_count =
            direntry::file_entry_set_entry_count(name.len()).map_err(Error::from)?;
        let create_result = (|| {
            let parent_directory = {
                let inode_state_guard = self.inode_state_read_guard();
                inode_state_guard.parent()
            };
            let mut guarded_directories = vec![self];
            if let Some(parent_directory) = parent_directory.as_ref() {
                guarded_directories.push(parent_directory.as_ref());
            }
            let directory_guards = Self::directory_write_guards_by_ino(guarded_directories);
            let guard_for_inode = |inode: &ExfatInode| {
                directory_guards
                    .iter()
                    .find(|guard| guard.guards_inode(inode))
                    .ok_or_else(|| Error::new(Errno::EINVAL))
            };
            let self_inode_state_guard = guard_for_inode(self)?;
            if self_inode_state_guard.metadata().type_ != InodeType::Dir {
                return_errno!(Errno::ENOTDIR);
            }
            let parent_inode_state_guard = if let Some(parent_directory) = parent_directory.as_ref()
            {
                Some(guard_for_inode(parent_directory.as_ref())?)
            } else {
                None
            };
            let mut allocation_guard = fs.allocation_guard()?;
            let cluster_map = self_inode_state_guard.dir_entry_stream();
            let current_directory_bytes = Self::read_directory_bytes_for_cluster_map(
                &block_device,
                &boot_region,
                cluster_map,
            )
            .map_err(Error::from)?;
            if Self::locate_named_child_view(
                &current_directory_bytes,
                cluster_map.data_length.is_none(),
                &upcase_table,
                &name,
                name_hash,
            )
            .map_err(Error::from)?
            .is_some()
            {
                return_errno!(Errno::EEXIST);
            }
            fs.publish_dirty_admission(&mut fs_state)?;
            let now = RealTimeCoarseClock::get().read_time();
            let (timestamp_bytes, ten_ms_increment, encoded_utc_offset_byte) =
                Self::encoded_exfat_timestamp_fields(now, 0).map_err(Error::from)?;
            let normalized_access_timestamp =
                Self::decoded_exfat_timestamp(timestamp_bytes, None, encoded_utc_offset_byte)
                    .map_err(Error::from)?;
            let normalized_modify_timestamp = Self::decoded_exfat_timestamp(
                timestamp_bytes,
                Some(ten_ms_increment),
                encoded_utc_offset_byte,
            )
            .map_err(Error::from)?;
            let create_timestamp = direntry::FileEntryTimestamp::new(
                timestamp_bytes,
                Some(ten_ms_increment),
                encoded_utc_offset_byte,
            );
            let last_accessed_timestamp =
                direntry::FileEntryTimestamp::new(timestamp_bytes, None, encoded_utc_offset_byte);
            let last_modified_timestamp = direntry::FileEntryTimestamp::new(
                timestamp_bytes,
                Some(ten_ms_increment),
                encoded_utc_offset_byte,
            );
            let (cluster_map, mut directory_bytes, slot_range) = self
                .reserve_directory_entry_slots(
                    cluster_map,
                    &mut allocation_guard,
                    &mut fs_state,
                    &block_device,
                    &boot_region,
                    parent_inode_state_guard,
                    self_inode_state_guard,
                    required_entry_count,
                )
                .map_err(Error::from)?;

            let (first_cluster, data_length, no_fat_chain) = if type_ == InodeType::Dir
                && !options.zero_size_dir
            {
                allocation_guard.allocate(1, None).map_err(Error::from)?;
                let allocated_cluster = allocation_guard.single_cluster().map_err(Error::from)?;
                let directory_creation_result = (|| {
                    Self::initialize_directory_cluster(
                        &block_device,
                        &boot_region,
                        allocated_cluster,
                    )?;
                    let entry_set = direntry::encode_file_entry_set(
                        &name,
                        name_hash,
                        type_,
                        allocated_cluster,
                        boot_region.cluster_size,
                        true,
                        create_timestamp,
                        last_accessed_timestamp,
                        last_modified_timestamp,
                    )?;
                    let slot_range_bytes = direntry::slot_range_bytes(slot_range)?;
                    directory_bytes[slot_range_bytes].copy_from_slice(&entry_set);
                    Self::write_directory_bytes_for_cluster_map(
                        &block_device,
                        &boot_region,
                        &directory_bytes,
                        cluster_map,
                    )?;
                    Ok((allocated_cluster, boot_region.cluster_size, true))
                })();
                match directory_creation_result {
                    Ok(created_directory) => {
                        allocation_guard.commit_allocation();
                        created_directory
                    }
                    Err(error) => {
                        if allocation_guard.rollback_allocation()? {
                            ExfatFs::disable_unsupported_discard_after_release(&mut fs_state);
                        }
                        return Err(error);
                    }
                }
            } else {
                let entry_set = direntry::encode_file_entry_set(
                    &name,
                    name_hash,
                    type_,
                    0,
                    0,
                    false,
                    create_timestamp,
                    last_accessed_timestamp,
                    last_modified_timestamp,
                )
                .map_err(Error::from)?;
                let slot_range_bytes =
                    direntry::slot_range_bytes(slot_range).map_err(Error::from)?;
                directory_bytes[slot_range_bytes.clone()].copy_from_slice(&entry_set);
                Self::write_directory_bytes_for_cluster_map(
                    &block_device,
                    &boot_region,
                    &directory_bytes,
                    cluster_map,
                )
                .map_err(Error::from)?;
                (0, 0, false)
            };

            let child_size = if type_ == InodeType::Dir {
                data_length
            } else {
                0
            };
            let child_ino = self
                .entry_location_ino(cluster_map, slot_range.first_entry_index())
                .map_err(Error::from)?;
            let child_inode = Self::new_child(
                &fs,
                self.weak_self(),
                child_ino,
                type_,
                boot_region.cluster_size,
                child_size,
                first_cluster,
                data_length,
                data_length,
                no_fat_chain,
            );
            if type_ == InodeType::File {
                child_inode
                    .store_entry_set_location_hint(slot_range)
                    .map_err(Error::from)?;
            }
            child_inode
                .inode_state_write_guard()
                .with_metadata_mut(|child_metadata| {
                    child_metadata.mode = mode;
                    child_metadata.last_access_at = normalized_access_timestamp;
                    child_metadata.last_modify_at = normalized_modify_timestamp;
                    child_metadata.last_meta_change_at = normalized_modify_timestamp;
                });
            fs_state
                .inode_cache
                .insert(child_ino, Arc::downgrade(&child_inode));
            let child_inode: Arc<dyn Inode> = child_inode;
            self.refresh_directory_metadata_after_namespace_mutation_with_guards(
                &block_device,
                &boot_region,
                RealTimeCoarseClock::get().read_time(),
                self_inode_state_guard,
                parent_inode_state_guard,
            )?;
            Ok(child_inode)
        })();
        if create_result.is_err() {
            ExfatFs::mark_mount_dirty_after_failure(&mut fs_state);
        }
        create_result
    }

    pub(super) fn unlink_impl(&self, name: &str) -> Result<()> {
        let fs = self
            .fs
            .upgrade()
            .ok_or_else(|| Error::with_message(Errno::EIO, "exFAT filesystem is not mounted"))?;
        let mut fs_state = fs.fs_state.write();
        let mount_state = fs_state.mount_state.as_ref().ok_or_else(super::super::not_mounted)?;
        if mount_state.forced_shutdown {
            return_errno!(Errno::EIO);
        }
        let block_device = fs.immutable_block_device();
        let boot_region = fs.immutable_boot_region();
        let upcase_table = fs_state.upcase_table.as_ref().ok_or_else(super::super::not_mounted)?.clone();
        let options = mount_state.options.clone();
        if options.fs_flags.contains(FsFlags::RDONLY) {
            return_errno!(Errno::EROFS);
        }

        let name = Self::validate_name(name, &options)?;
        let lookup_name_hash = upcase_table.name_hash(&name);
        let unlink_result = (|| {
            let provisional_directory_guard = self.inode_state_read_guard();
            let parent_directory = provisional_directory_guard.parent();
            let cluster_map = provisional_directory_guard.dir_entry_stream();
            let discovery_allocation_guard = fs.allocation_read_guard()?;
            let is_root_directory = cluster_map.data_length.is_none();
            let directory_bytes = Self::read_directory_bytes_for_cluster_map(
                &block_device,
                &boot_region,
                cluster_map,
            )
            .map_err(Error::from)?;
            let Some(entry_view) = Self::locate_named_child_view(
                &directory_bytes,
                is_root_directory,
                &upcase_table,
                &name,
                lookup_name_hash,
            )
            .map_err(Error::from)?
            else {
                return_errno!(Errno::ENOENT);
            };
            let slot_range = entry_view.slot_range();
            let child_ino = self
                .entry_location_ino(cluster_map, slot_range.first_entry_index())
                .map_err(Error::from)?;
            let (inode_type, _first_cluster, _data_length, _no_fat_chain) = entry_view
                .child_metadata(&boot_region)
                .map_err(Error::from)?;
            if inode_type == InodeType::Dir {
                return_errno!(Errno::EISDIR);
            }
            let cached_child_inode = ExfatFs::peek_cached_inode(&fs_state, child_ino);
            drop(discovery_allocation_guard);
            drop(provisional_directory_guard);
            let mut guarded_inodes = vec![self];
            if let Some(parent_directory) = parent_directory.as_ref() {
                guarded_inodes.push(parent_directory.as_ref());
            }
            if let Some(cached_child_inode) = cached_child_inode.as_ref() {
                guarded_inodes.push(cached_child_inode.as_ref());
            }
            let directory_guards = Self::directory_write_guards_by_ino(guarded_inodes);
            let guard_for_inode = |inode: &ExfatInode| {
                directory_guards
                    .iter()
                    .find(|guard| guard.guards_inode(inode))
                    .ok_or_else(|| Error::new(Errno::EINVAL))
            };
            let self_inode_state_guard = guard_for_inode(self)?;
            if self_inode_state_guard.metadata().type_ != InodeType::Dir {
                return_errno!(Errno::ENOTDIR);
            }
            let parent_inode_state_guard = match parent_directory.as_ref() {
                Some(parent_directory) => Some(guard_for_inode(parent_directory.as_ref())?),
                None => None,
            };
            let cached_child_inode_state_guard = match cached_child_inode.as_ref() {
                Some(cached_child_inode) => Some(guard_for_inode(cached_child_inode.as_ref())?),
                None => None,
            };
            let mut allocation_guard = fs.allocation_guard()?;
            let cluster_map = self_inode_state_guard.dir_entry_stream();
            let is_root_directory = cluster_map.data_length.is_none();
            let directory_bytes = Self::read_directory_bytes_for_cluster_map(
                &block_device,
                &boot_region,
                cluster_map,
            )
            .map_err(Error::from)?;
            let Some(entry_view) = Self::locate_named_child_view(
                &directory_bytes,
                is_root_directory,
                &upcase_table,
                &name,
                lookup_name_hash,
            )
            .map_err(Error::from)?
            else {
                return_errno!(Errno::ENOENT);
            };
            let slot_range = entry_view.slot_range();
            let (inode_type, first_cluster, data_length, no_fat_chain) = entry_view
                .child_metadata(&boot_region)
                .map_err(Error::from)?;
            if inode_type == InodeType::Dir {
                return_errno!(Errno::EISDIR);
            }
            let detached_regular_file_reclaim =
                if let (Some(cached_child_inode), Some(cached_child_inode_state_guard)) = (
                    cached_child_inode.as_ref(),
                    cached_child_inode_state_guard.as_ref(),
                ) {
                    Some(Self::capture_cached_regular_file_retirement(
                        cached_child_inode,
                        cached_child_inode_state_guard,
                        &allocation_guard,
                    )?)
                } else {
                    None
                };
            let allocated_cluster_ranges = if cached_child_inode.is_none() {
                Self::allocated_cluster_ranges(
                    &block_device,
                    &boot_region,
                    first_cluster,
                    data_length,
                    no_fat_chain,
                )
                .map_err(Error::from)?
            } else {
                Vec::new()
            };

            fs.publish_dirty_admission(&mut fs_state)?;
            let mut invalidated_directory_bytes = directory_bytes;
            let slot_range_bytes = direntry::slot_range_bytes(slot_range).map_err(Error::from)?;
            let removed_entry_set = invalidated_directory_bytes
                .get_mut(slot_range_bytes)
                .ok_or(invalid_on_disk_layout())
                .map_err(Error::from)?;
            let mut removed_entry_set =
                MutableDirEntrySlotSpan::new(slot_range, removed_entry_set).map_err(Error::from)?;
            direntry::invalidate_entry_set(&mut removed_entry_set).map_err(Error::from)?;
            Self::write_directory_bytes_for_cluster_map(
                &block_device,
                &boot_region,
                &invalidated_directory_bytes,
                cluster_map,
            )
            .map_err(Error::from)?;
            if let (Some(cached_child_inode), Some(cached_child_inode_state_guard)) = (
                cached_child_inode.as_ref(),
                cached_child_inode_state_guard.as_ref(),
            ) {
                Self::detach_namespace_removed_inode(
                    &mut fs_state,
                    &mut allocation_guard,
                    child_ino,
                    cached_child_inode,
                    cached_child_inode_state_guard,
                    detached_regular_file_reclaim,
            )?;
            } else if !allocated_cluster_ranges.is_empty() {
                allocation_guard.free_clusters(&allocated_cluster_ranges)?;
                ExfatFs::disable_unsupported_discard_after_release(&mut fs_state);
            }
            self.refresh_directory_metadata_after_namespace_mutation_with_guards(
                &block_device,
                &boot_region,
                RealTimeCoarseClock::get().read_time(),
                self_inode_state_guard,
                parent_inode_state_guard,
            )?;
            Ok(())
        })();
        if unlink_result.is_err() {
            ExfatFs::mark_mount_dirty_after_failure(&mut fs_state);
        }
        unlink_result
    }

    pub(super) fn rmdir_impl(&self, name: &str) -> Result<()> {
        let fs = self
            .fs
            .upgrade()
            .ok_or_else(|| Error::with_message(Errno::EIO, "exFAT filesystem is not mounted"))?;
        let mut fs_state = fs.fs_state.write();
        let mount_state = fs_state.mount_state.as_ref().ok_or_else(super::super::not_mounted)?;
        if mount_state.forced_shutdown {
            return_errno!(Errno::EIO);
        }
        let block_device = fs.immutable_block_device();
        let boot_region = fs.immutable_boot_region();
        let upcase_table = fs_state.upcase_table.as_ref().ok_or_else(super::super::not_mounted)?.clone();
        let options = mount_state.options.clone();
        if options.fs_flags.contains(FsFlags::RDONLY) {
            return_errno!(Errno::EROFS);
        }

        let name = Self::validate_name(name, &options)?;
        let lookup_name_hash = upcase_table.name_hash(&name);
        let rmdir_result = (|| {
            let provisional_directory_guard = self.inode_state_read_guard();
            let parent_directory = provisional_directory_guard.parent();
            let cluster_map = provisional_directory_guard.dir_entry_stream();
            let discovery_allocation_guard = fs.allocation_read_guard()?;
            let directory_bytes = Self::read_directory_bytes_for_cluster_map(
                &block_device,
                &boot_region,
                cluster_map,
            )
            .map_err(Error::from)?;
            let Some(entry_view) = Self::locate_named_child_view(
                &directory_bytes,
                cluster_map.data_length.is_none(),
                &upcase_table,
                &name,
                lookup_name_hash,
            )
            .map_err(Error::from)?
            else {
                return_errno!(Errno::ENOENT);
            };
            let slot_range = entry_view.slot_range();
            let (inode_type, first_cluster, data_length, no_fat_chain) = entry_view
                .child_metadata(&boot_region)
                .map_err(Error::from)?;
            if inode_type != InodeType::Dir {
                return_errno!(Errno::ENOTDIR);
            }

            let child_ino = self.entry_location_ino(cluster_map, slot_range.first_entry_index())?;
            let child_inode = if let Some(cached_inode) =
                ExfatFs::peek_cached_inode(&fs_state, child_ino)
            {
                cached_inode
            } else {
                Self::child_inode_from_directory_entry(
                    self,
                    &fs,
                    &boot_region,
                    cluster_map.first_cluster,
                    slot_range,
                    inode_type,
                    first_cluster,
                    data_length,
                    data_length,
                    no_fat_chain,
                )
                .map_err(Error::from)?
            };
            drop(discovery_allocation_guard);
            drop(provisional_directory_guard);
            let mut guarded_inodes = vec![self, child_inode.as_ref()];
            if let Some(parent_directory) = parent_directory.as_ref() {
                guarded_inodes.push(parent_directory.as_ref());
            }
            let directory_guards = Self::directory_write_guards_by_ino(guarded_inodes);
            let guard_for_inode = |inode: &ExfatInode| {
                directory_guards
                    .iter()
                    .find(|guard| guard.guards_inode(inode))
                    .ok_or_else(|| Error::new(Errno::EINVAL))
            };
            let self_inode_state_guard = guard_for_inode(self)?;
            if self_inode_state_guard.metadata().type_ != InodeType::Dir {
                return_errno!(Errno::ENOTDIR);
            }
            let parent_inode_state_guard = match parent_directory.as_ref() {
                Some(parent_directory) => Some(guard_for_inode(parent_directory.as_ref())?),
                None => None,
            };
            let child_inode_state_guard = guard_for_inode(child_inode.as_ref())?;
            let mut allocation_guard = fs.allocation_guard()?;
            let cluster_map = self_inode_state_guard.dir_entry_stream();
            let directory_bytes = Self::read_directory_bytes_for_cluster_map(
                &block_device,
                &boot_region,
                cluster_map,
            )
            .map_err(Error::from)?;
            let Some(entry_view) = Self::locate_named_child_view(
                &directory_bytes,
                cluster_map.data_length.is_none(),
                &upcase_table,
                &name,
                lookup_name_hash,
            )
            .map_err(Error::from)?
            else {
                return_errno!(Errno::ENOENT);
            };
            let slot_range = entry_view.slot_range();
            let (inode_type, _first_cluster, _data_length, _no_fat_chain) = entry_view
                .child_metadata(&boot_region)
                .map_err(Error::from)?;
            if inode_type != InodeType::Dir {
                return_errno!(Errno::ENOTDIR);
            }
            let child_cluster_map = child_inode_state_guard.dir_entry_stream();
            Self::ensure_directory_snapshot_is_empty(
                child_cluster_map,
                &block_device,
                &boot_region,
            )?;

            let Some(child_data_length) = child_cluster_map.data_length else {
                return Err(Error::from(invalid_on_disk_layout()));
            };
            let allocated_cluster_ranges = Self::allocated_cluster_ranges(
                &block_device,
                &boot_region,
                child_cluster_map.first_cluster,
                child_data_length,
                child_cluster_map.no_fat_chain,
            )
            .map_err(Error::from)?;
            fs.publish_dirty_admission(&mut fs_state)?;
            let mut invalidated_directory_bytes = directory_bytes;
            let slot_range_bytes = direntry::slot_range_bytes(slot_range).map_err(Error::from)?;
            let removed_entry_set = invalidated_directory_bytes
                .get_mut(slot_range_bytes)
                .ok_or(invalid_on_disk_layout())
                .map_err(Error::from)?;
            let mut removed_entry_set =
                MutableDirEntrySlotSpan::new(slot_range, removed_entry_set).map_err(Error::from)?;
            direntry::invalidate_entry_set(&mut removed_entry_set).map_err(Error::from)?;
            Self::write_directory_bytes_for_cluster_map(
                &block_device,
                &boot_region,
                &invalidated_directory_bytes,
                cluster_map,
            )
            .map_err(Error::from)?;
            Self::detach_namespace_removed_inode(
                &mut fs_state,
                &mut allocation_guard,
                child_ino,
                &child_inode,
                child_inode_state_guard,
                None,
            )?;
            if !allocated_cluster_ranges.is_empty() {
                allocation_guard.free_clusters(&allocated_cluster_ranges)?;
                ExfatFs::disable_unsupported_discard_after_release(&mut fs_state);
            }
            self.refresh_directory_metadata_after_namespace_mutation_with_guards(
                &block_device,
                &boot_region,
                RealTimeCoarseClock::get().read_time(),
                self_inode_state_guard,
                parent_inode_state_guard,
            )?;
            Ok(())
        })();
        if rmdir_result.is_err() {
            ExfatFs::mark_mount_dirty_after_failure(&mut fs_state);
        }
        rmdir_result
    }

    fn discover_rename_child(
        directory: &ExfatInode,
        fs: &Arc<ExfatFs>,
        fs_state: &FsState,
        boot_region: &BootRegion,
        directory_cluster_map: StreamExtensionDirEntry,
        entry_view: FileEntrySetView<'_>,
        role: RenameDiscoveryRole,
    ) -> Result<Option<Arc<ExfatInode>>> {
        let (inode_type, first_cluster, data_length, no_fat_chain) = entry_view
            .child_metadata(boot_region)
            .map_err(Error::from)?;
        let ino = directory.entry_location_ino(
            directory_cluster_map,
            entry_view.slot_range().first_entry_index(),
        )?;
        if let Some(cached_inode) = ExfatFs::peek_cached_inode(fs_state, ino) {
            return Ok(Some(cached_inode));
        }
        if matches!(role, RenameDiscoveryRole::Replacement) && inode_type != InodeType::Dir {
            return Ok(None);
        }
        let valid_data_length = entry_view
            .cluster_map()
            .map_err(Error::from)?
            .valid_data_length
            .ok_or_else(invalid_on_disk_layout)
            .map_err(Error::from)?;
        Self::child_inode_from_directory_entry(
            directory,
            fs,
            boot_region,
            directory_cluster_map.first_cluster,
            entry_view.slot_range(),
            inode_type,
            first_cluster,
            data_length,
            valid_data_length,
            no_fat_chain,
        )
        .map(Some)
        .map_err(Error::from)
    }

    fn discover_rename_participants(
        &self,
        target_directory: &ExfatInode,
        fs: &Arc<ExfatFs>,
        fs_state: &FsState,
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        upcase_table: &UpcaseTable,
        names: RenameNames<'_>,
    ) -> Result<RenameDiscovery> {
        let provisional_directory_guards =
            Self::directory_read_guards_by_stable_identity(vec![self, target_directory]);
        let (
            self_ino,
            source_parent_directory,
            source_cluster_map,
            target_directory_ino,
            target_parent_directory,
            target_cluster_map,
        ) = {
            let provisional_guard_for_inode = |inode: &ExfatInode| {
                provisional_directory_guards
                    .iter()
                    .find(|guard| guard.guards_inode(inode))
                    .ok_or_else(|| Error::new(Errno::EINVAL))
            };
            let source_guard = provisional_guard_for_inode(self)?;
            let target_guard = provisional_guard_for_inode(target_directory)?;
            if source_guard.metadata().type_ != InodeType::Dir
                || target_guard.metadata().type_ != InodeType::Dir
            {
                return_errno!(Errno::ENOTDIR);
            }
            (
                source_guard.metadata().ino,
                source_guard.parent(),
                source_guard.dir_entry_stream(),
                target_guard.metadata().ino,
                target_guard.parent(),
                target_guard.dir_entry_stream(),
            )
        };
        let discovery_allocation_guard = fs.allocation_read_guard()?;
        let discovery_result = (|| {
            let discovery = if self_ino == target_directory_ino {
            let directory_bytes = Self::read_directory_bytes_for_cluster_map(
                block_device,
                boot_region,
                source_cluster_map,
            )
            .map_err(Error::from)?;
            let source_view = Self::locate_named_child_view(
                &directory_bytes,
                source_cluster_map.data_length.is_none(),
                upcase_table,
                names.source,
                names.source_hash,
            )
            .map_err(Error::from)?
            .ok_or_else(|| Error::new(Errno::ENOENT))?;
            let source_child_inode = Self::discover_rename_child(
                self,
                fs,
                fs_state,
                boot_region,
                source_cluster_map,
                source_view,
                RenameDiscoveryRole::Source,
            )?
            .ok_or_else(invalid_on_disk_layout)?;
            let target_child_inode = Self::locate_named_child_view(
                &directory_bytes,
                source_cluster_map.data_length.is_none(),
                upcase_table,
                names.destination,
                names.destination_hash,
            )
            .map_err(Error::from)?
            .filter(|target_view| target_view.slot_range() != source_view.slot_range())
            .map(|target_view| {
                Self::discover_rename_child(
                    self,
                    fs,
                    fs_state,
                    boot_region,
                    source_cluster_map,
                    target_view,
                    RenameDiscoveryRole::Replacement,
                )
            })
            .transpose()
            .map_err(Error::from)?
            .flatten();
            RenameDiscovery::SameDirectory {
                parent_directory: source_parent_directory,
                source_child_inode,
                target_child_inode,
            }
        } else {
            let source_directory_bytes = Self::read_directory_bytes_for_cluster_map(
                block_device,
                boot_region,
                source_cluster_map,
            )
            .map_err(Error::from)?;
            let source_view = Self::locate_named_child_view(
                &source_directory_bytes,
                source_cluster_map.data_length.is_none(),
                upcase_table,
                names.source,
                names.source_hash,
            )
            .map_err(Error::from)?
            .ok_or_else(|| Error::new(Errno::ENOENT))?;
            let source_child_inode = Self::discover_rename_child(
                self,
                fs,
                fs_state,
                boot_region,
                source_cluster_map,
                source_view,
                RenameDiscoveryRole::Source,
            )?
            .ok_or_else(invalid_on_disk_layout)?;
            let target_directory_bytes = Self::read_directory_bytes_for_cluster_map(
                block_device,
                boot_region,
                target_cluster_map,
            )
            .map_err(Error::from)?;
            let target_child_inode = Self::locate_named_child_view(
                &target_directory_bytes,
                target_cluster_map.data_length.is_none(),
                upcase_table,
                names.destination,
                names.destination_hash,
            )
            .map_err(Error::from)?
            .map(|target_view| {
                Self::discover_rename_child(
                    target_directory,
                    fs,
                    fs_state,
                    boot_region,
                    target_cluster_map,
                    target_view,
                    RenameDiscoveryRole::Replacement,
                )
            })
            .transpose()
            .map_err(Error::from)?
            .flatten();
            RenameDiscovery::CrossDirectory {
                source_parent_directory,
                target_parent_directory,
                source_child_inode,
                target_child_inode,
            }
            };
            Ok(discovery)
        })();
        drop(discovery_allocation_guard);
        drop(provisional_directory_guards);
        discovery_result
    }

    fn collect_rename_final_participants<'a>(
        &'a self,
        target_directory: &'a ExfatInode,
        discovery: &'a RenameDiscovery,
    ) -> Vec<&'a ExfatInode> {
        match discovery {
            RenameDiscovery::SameDirectory {
                parent_directory,
                source_child_inode,
                target_child_inode,
            } => {
                let mut participants = vec![self, source_child_inode.as_ref()];
                if let Some(parent_directory) = parent_directory.as_ref() {
                    participants.push(parent_directory.as_ref());
                }
                if let Some(target_child_inode) = target_child_inode.as_ref() {
                    participants.push(target_child_inode.as_ref());
                }
                participants
            }
            RenameDiscovery::CrossDirectory {
                source_parent_directory,
                target_parent_directory,
                source_child_inode,
                target_child_inode,
            } => {
                let mut participants = vec![self, target_directory, source_child_inode.as_ref()];
                if let Some(source_parent_directory) = source_parent_directory.as_ref() {
                    participants.push(source_parent_directory.as_ref());
                }
                if let Some(target_parent_directory) = target_parent_directory.as_ref() {
                    participants.push(target_parent_directory.as_ref());
                }
                if let Some(target_child_inode) = target_child_inode.as_ref() {
                    participants.push(target_child_inode.as_ref());
                }
                participants
            }
        }
    }

    fn project_final_rename_admission<'a, 'guard>(
        &'a self,
        target_directory: &'a ExfatInode,
        discovery: &'a RenameDiscovery,
        inode_guards: &'a [InodeStateWriteGuard<'guard>],
    ) -> Result<FinalRenameAdmission<'a, 'guard>> {
        let guard_for_inode = |inode: &ExfatInode| {
            inode_guards
                .iter()
                .find(|guard| guard.guards_inode(inode))
                .ok_or_else(|| Error::new(Errno::EINVAL))
        };
        match discovery {
            RenameDiscovery::SameDirectory {
                parent_directory,
                source_child_inode,
                target_child_inode,
            } => {
                let directory_guard = guard_for_inode(self)?;
                if directory_guard.metadata().type_ != InodeType::Dir {
                    return_errno!(Errno::ENOTDIR);
                }
                let parent_guard = parent_directory
                    .as_ref()
                    .map(|parent| guard_for_inode(parent.as_ref()))
                    .transpose()?;
                let source_child = AdmittedRenameChild {
                    inode: source_child_inode,
                    guard: guard_for_inode(source_child_inode.as_ref())?,
                };
                let target_child = target_child_inode
                    .as_ref()
                    .map(|target| -> Result<AdmittedRenameChild<'a, 'guard>> {
                        Ok(AdmittedRenameChild {
                            inode: target,
                            guard: guard_for_inode(target.as_ref())?,
                        })
                    })
                    .transpose()?;
                Ok(FinalRenameAdmission::SameDirectory {
                    directory_guard,
                    parent_guard,
                    source_child,
                    target_child,
                    cluster_map: directory_guard.dir_entry_stream(),
                })
            }
            RenameDiscovery::CrossDirectory {
                source_parent_directory,
                target_parent_directory,
                source_child_inode,
                target_child_inode,
            } => {
                let source_guard = guard_for_inode(self)?;
                let target_guard = guard_for_inode(target_directory)?;
                if source_guard.metadata().type_ != InodeType::Dir
                    || target_guard.metadata().type_ != InodeType::Dir
                {
                    return_errno!(Errno::ENOTDIR);
                }
                let source_parent_guard = source_parent_directory
                    .as_ref()
                    .map(|parent| guard_for_inode(parent.as_ref()))
                    .transpose()?;
                let target_parent_guard = target_parent_directory
                    .as_ref()
                    .map(|parent| guard_for_inode(parent.as_ref()))
                    .transpose()?;
                let source_child = AdmittedRenameChild {
                    inode: source_child_inode,
                    guard: guard_for_inode(source_child_inode.as_ref())?,
                };
                let target_child = target_child_inode
                    .as_ref()
                    .map(|target| -> Result<AdmittedRenameChild<'a, 'guard>> {
                        Ok(AdmittedRenameChild {
                            inode: target,
                            guard: guard_for_inode(target.as_ref())?,
                        })
                    })
                    .transpose()?;
                Ok(FinalRenameAdmission::CrossDirectory {
                    source_guard,
                    source_parent_guard,
                    target_guard,
                    target_parent_guard,
                    source_child,
                    target_child,
                    source_cluster_map: source_guard.dir_entry_stream(),
                    target_cluster_map: target_guard.dir_entry_stream(),
                })
            }
        }
    }

    pub(super) fn rename_impl(
        &self,
        old_name: &str,
        target: &Arc<dyn Inode>,
        new_name: &str,
    ) -> Result<()> {
        let Some(target_directory) = target.downcast_ref::<Self>() else {
            return_errno!(Errno::EXDEV);
        };
        let fs = self
            .fs
            .upgrade()
            .ok_or_else(|| Error::with_message(Errno::EIO, "exFAT filesystem is not mounted"))?;
        let mut fs_state = fs.fs_state.write();
        let mount_state = fs_state.mount_state.as_ref().ok_or_else(super::super::not_mounted)?;
        let target_fs = target_directory
            .fs
            .upgrade()
            .ok_or_else(|| Error::with_message(Errno::EIO, "exFAT filesystem is not mounted"))?;
        if !Arc::ptr_eq(&fs, &target_fs) {
            return_errno!(Errno::EXDEV);
        }
        if mount_state.forced_shutdown {
            return_errno!(Errno::EIO);
        }
        let block_device = fs.immutable_block_device();
        let boot_region = fs.immutable_boot_region();
        let upcase_table = fs_state.upcase_table.as_ref().ok_or_else(super::super::not_mounted)?.clone();
        let options = mount_state.options.clone();
        if options.fs_flags.contains(FsFlags::RDONLY) {
            return_errno!(Errno::EROFS);
        }

        let old_name = Self::validate_name(old_name, &options)?;
        let new_name = Self::validate_name(new_name, &options)?;
        let old_name_hash = upcase_table.name_hash(&old_name);
        let new_name_hash = upcase_table.name_hash(&new_name);
        let rename_result = (|| {
            let names = RenameNames {
                source: &old_name,
                source_hash: old_name_hash,
                destination: &new_name,
                destination_hash: new_name_hash,
            };
            let discovery = self.discover_rename_participants(
                target_directory,
                &fs,
                &fs_state,
                &block_device,
                &boot_region,
                &upcase_table,
                names,
            )?;
            let final_participants =
                self.collect_rename_final_participants(target_directory, &discovery);
            let inode_guards = Self::directory_write_guards_by_ino(final_participants);
            let admission = Self::project_final_rename_admission(
                self,
                target_directory,
                &discovery,
                &inode_guards,
            )?;
            match admission {
                FinalRenameAdmission::SameDirectory {
                    directory_guard,
                    parent_guard,
                    source_child,
                    target_child,
                    cluster_map,
                } => {
                    let mut allocation_guard = fs.allocation_guard()?;
                    let renamed = self.rename_within_directory(
                        cluster_map,
                        directory_guard,
                        parent_guard,
                        source_child,
                        target_child,
                        fs.as_ref(),
                        &mut fs_state,
                        &mut allocation_guard,
                        &block_device,
                        &boot_region,
                        &upcase_table,
                        RenameNames {
                            source: &old_name,
                            source_hash: old_name_hash,
                            destination: &new_name,
                            destination_hash: new_name_hash,
                        },
                    )?;
                    if renamed {
                        self.refresh_directory_metadata_after_namespace_mutation_with_guards(
                            &block_device,
                            &boot_region,
                            RealTimeCoarseClock::get().read_time(),
                            directory_guard,
                            parent_guard,
                        )?;
                    }
                    Ok(())
                }
                FinalRenameAdmission::CrossDirectory {
                    source_guard,
                    source_parent_guard,
                    target_guard,
                    target_parent_guard,
                    source_child,
                    target_child,
                    source_cluster_map,
                    target_cluster_map,
                } => {
                    let mut allocation_guard = fs.allocation_guard()?;
                    self.rename_across_directories(
                        source_cluster_map,
                        source_guard,
                        target_directory,
                        target_cluster_map,
                        target_guard,
                        target_parent_guard,
                        source_child,
                        target_child,
                        fs.as_ref(),
                        &mut fs_state,
                        &mut allocation_guard,
                        &block_device,
                        &boot_region,
                        &upcase_table,
                        RenameNames {
                            source: &old_name,
                            source_hash: old_name_hash,
                            destination: &new_name,
                            destination_hash: new_name_hash,
                        },
                    )?;
                    let timestamp = RealTimeCoarseClock::get().read_time();
                    self.refresh_directory_metadata_after_namespace_mutation_with_guards(
                        &block_device,
                        &boot_region,
                        timestamp,
                        source_guard,
                        source_parent_guard,
                    )?;
                    target_directory
                        .refresh_directory_metadata_after_namespace_mutation_with_guards(
                            &block_device,
                            &boot_region,
                            timestamp,
                            target_guard,
                            target_parent_guard,
                        )
                }
            }
        })();
        if rename_result.is_err() {
            ExfatFs::mark_mount_dirty_after_failure(&mut fs_state);
        }
        rename_result
    }

    // Cross-directory rename helpers

    fn rename_within_directory(
        &self,
        mut cluster_map: StreamExtensionDirEntry,
        self_inode_state_guard: &InodeStateWriteGuard<'_>,
        parent_inode_state_guard: Option<&InodeStateWriteGuard<'_>>,
        source_child: AdmittedRenameChild<'_, '_>,
        target_child: Option<AdmittedRenameChild<'_, '_>>,
        fs: &ExfatFs,
        fs_state: &mut FsState,
        allocation_guard: &mut AllocGuard<'_>,
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        upcase_table: &UpcaseTable,
        names: RenameNames<'_>,
    ) -> Result<bool> {
        let source_child_inode = source_child.inode;
        let source_child_inode_state_guard = source_child.guard;
        let (target_child_inode, target_child_inode_state_guard) = match target_child.as_ref() {
            Some(child) => (Some(child.inode), Some(child.guard)),
            None => (None, None),
        };
        let old_name = names.source;
        let old_name_hash = names.source_hash;
        let new_name = names.destination;
        let new_name_hash = names.destination_hash;
        let current_directory_bytes =
            Self::read_directory_bytes_for_cluster_map(block_device, boot_region, cluster_map)
                .map_err(Error::from)?;
        let current_source_view = Self::lookup_rename_source_view(
            &current_directory_bytes,
            cluster_map,
            upcase_table,
            old_name,
            old_name_hash,
        )?;
        let source_name = current_source_view.name().map_err(Error::from)?;
        let current_source_slot_range = current_source_view.slot_range();
        let current_target_view = Self::lookup_rename_target_view(
            &current_directory_bytes,
            cluster_map,
            upcase_table,
            new_name,
            new_name_hash,
            Some(current_source_slot_range),
        )?;
        if current_target_view.is_none() && source_name == new_name {
            return Ok(false);
        }
        fs.publish_dirty_admission(fs_state)?;
        let current_renamed_entry_set =
            direntry::renamed_entry_set(current_source_view, new_name, new_name_hash)
                .map_err(Error::from)?;
        let required_entry_count = current_renamed_entry_set.len() / DIRECTORY_ENTRY_SIZE;

        let (source_inode_type, _, _, _) = current_source_view
            .child_metadata(boot_region)
            .map_err(Error::from)?;
        let replacement = Self::collect_replaced_target_cleanup(
            current_target_view,
            target_child_inode,
            target_child_inode_state_guard,
            source_inode_type,
            block_device,
            boot_region,
            allocation_guard,
        )?;
        let replaced_target_slot_range = replacement.as_ref().map(|replacement| match replacement {
            ReplacedTargetCleanup::Immediate { slot_range, .. }
            | ReplacedTargetCleanup::CachedGeneration { slot_range, .. } => *slot_range,
        });
        let reusable_slot_range = if current_source_slot_range.entry_count() >= required_entry_count
        {
            Some(current_source_slot_range)
        } else {
            replaced_target_slot_range
        };

        let (updated_cluster_map, renamed_directory_bytes, final_slot_range, reserved_new_slot) =
            self.reserve_rename_destination_slot(
                cluster_map,
                current_directory_bytes,
                reusable_slot_range,
                fs_state,
                allocation_guard,
                block_device,
                boot_region,
                parent_inode_state_guard,
                self_inode_state_guard,
                required_entry_count,
            )?;
        cluster_map = updated_cluster_map;
        let (source_slot_range, renamed_entry_set) = if reserved_new_slot {
            let latest_source_view = Self::lookup_rename_source_view(
                &renamed_directory_bytes,
                cluster_map,
                upcase_table,
                old_name,
                old_name_hash,
            )?;
            let source_slot_range = latest_source_view.slot_range();
            let renamed_entry_set =
                direntry::renamed_entry_set(latest_source_view, new_name, new_name_hash)
                    .map_err(Error::from)?;
            (source_slot_range, renamed_entry_set)
        } else {
            (current_source_slot_range, current_renamed_entry_set)
        };
        let replaced_target_slot_range = Self::lookup_rename_target_view(
            &renamed_directory_bytes,
            cluster_map,
            upcase_table,
            new_name,
            new_name_hash,
            Some(source_slot_range),
        )?
        .map(FileEntrySetView::slot_range)
        .filter(|slot_range| *slot_range != final_slot_range);
        let plan = SameDirectoryRenamePlan {
            cluster_map,
            directory_bytes: renamed_directory_bytes,
            source_slot_range,
            destination_slot_range: final_slot_range,
            replaced_slot_range: replaced_target_slot_range,
            renamed_entry_set,
            replacement,
        };
        let (cluster_map, destination_slot_range, replacement) =
            Self::persist_same_directory_rename(plan, block_device, boot_region)?;
        Self::finalize_rename_protocol(
            self,
            cluster_map,
            destination_slot_range,
            source_child,
            target_child,
            replacement,
            fs_state,
            allocation_guard,
        )?;
        Ok(true)
    }

    fn rename_across_directories(
        &self,
        source_cluster_map: StreamExtensionDirEntry,
        _source_inode_state_guard: &InodeStateWriteGuard<'_>,
        target_directory: &ExfatInode,
        mut target_cluster_map: StreamExtensionDirEntry,
        target_inode_state_guard: &InodeStateWriteGuard<'_>,
        target_parent_inode_state_guard: Option<&InodeStateWriteGuard<'_>>,
        source_child: AdmittedRenameChild<'_, '_>,
        target_child: Option<AdmittedRenameChild<'_, '_>>,
        fs: &ExfatFs,
        fs_state: &mut FsState,
        allocation_guard: &mut AllocGuard<'_>,
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        upcase_table: &UpcaseTable,
        names: RenameNames<'_>,
    ) -> Result<()> {
        let source_child_inode = source_child.inode;
        let source_child_inode_state_guard = source_child.guard;
        let (target_child_inode, target_child_inode_state_guard) = match target_child.as_ref() {
            Some(child) => (Some(child.inode), Some(child.guard)),
            None => (None, None),
        };
        let old_name = names.source;
        let old_name_hash = names.source_hash;
        let new_name = names.destination;
        let new_name_hash = names.destination_hash;
        let source_directory_bytes = Self::read_directory_bytes_for_cluster_map(
            block_device,
            boot_region,
            source_cluster_map,
        )
        .map_err(Error::from)?;
        let source_view = Self::lookup_rename_source_view(
            &source_directory_bytes,
            source_cluster_map,
            upcase_table,
            old_name,
            old_name_hash,
        )?;
        let source_slot_range = source_view.slot_range();
        let (source_inode_type, _, _, _) = source_view
            .child_metadata(boot_region)
            .map_err(Error::from)?;
        let renamed_entry_set = direntry::renamed_entry_set(source_view, new_name, new_name_hash)
            .map_err(Error::from)?;
        let required_entry_count = renamed_entry_set.len() / DIRECTORY_ENTRY_SIZE;
        let target_directory_bytes = Self::read_directory_bytes_for_cluster_map(
            block_device,
            boot_region,
            target_cluster_map,
        )
        .map_err(Error::from)?;
        let target_view = Self::lookup_rename_target_view(
            &target_directory_bytes,
            target_cluster_map,
            upcase_table,
            new_name,
            new_name_hash,
            None,
        )?;
        let replacement = Self::collect_replaced_target_cleanup(
            target_view,
            target_child_inode,
            target_child_inode_state_guard,
            source_inode_type,
            block_device,
            boot_region,
            allocation_guard,
        )?;
        let replaced_target_slot_range = replacement.as_ref().map(|replacement| match replacement {
            ReplacedTargetCleanup::Immediate { slot_range, .. }
            | ReplacedTargetCleanup::CachedGeneration { slot_range, .. } => *slot_range,
        });
        fs.publish_dirty_admission(fs_state)?;
        let (
            updated_target_cluster_map,
            target_directory_bytes,
            target_slot_range,
            _reserved_target_slot,
        ) = target_directory.reserve_rename_destination_slot(
            target_cluster_map,
            target_directory_bytes,
            replaced_target_slot_range,
            fs_state,
            allocation_guard,
            block_device,
            boot_region,
            target_parent_inode_state_guard,
            target_inode_state_guard,
            required_entry_count,
        )?;
        target_cluster_map = updated_target_cluster_map;
        let plan = CrossDirectoryRenamePlan {
            source_cluster_map,
            source_directory_bytes,
            source_slot_range,
            target_cluster_map,
            target_directory_bytes,
            target_slot_range,
            renamed_entry_set,
            replacement,
        };
        let (target_cluster_map, target_slot_range, replacement) =
            Self::persist_cross_directory_rename(plan, block_device, boot_region)?;
        Self::finalize_rename_protocol(
            target_directory,
            target_cluster_map,
            target_slot_range,
            source_child,
            target_child,
            replacement,
            fs_state,
            allocation_guard,
        )?;
        Ok(())
    }

    fn finalize_rename_protocol(
        destination_directory: &ExfatInode,
        destination_cluster_map: StreamExtensionDirEntry,
        destination_slot_range: DirEntrySlotRange,
        source_child: AdmittedRenameChild<'_, '_>,
        target_child: Option<AdmittedRenameChild<'_, '_>>,
        replacement: Option<ReplacedTargetCleanup>,
        fs_state: &mut FsState,
        allocation_guard: &mut AllocGuard<'_>,
    ) -> Result<()> {
        let old_source_ino = source_child.guard.metadata().ino;
        let new_source_ino = destination_directory
            .entry_location_ino(
                destination_cluster_map,
                destination_slot_range.first_entry_index(),
            )
            .map_err(Error::from)?;
        let replaced_target_ino = target_child.as_ref().map(|child| child.guard.metadata().ino);
        source_child
            .guard
            .set_parent(destination_directory.weak_self());
        source_child
            .guard
            .with_metadata_mut(|metadata| metadata.ino = new_source_ino);
        if source_child.guard.metadata().type_ == InodeType::File {
            source_child
                .inode
                .store_entry_set_location_hint(destination_slot_range)
                .map_err(Error::from)?;
        }
        let (replaced_target_ranges, detached_regular_file_reclaim) = match replacement {
            Some(ReplacedTargetCleanup::Immediate { ranges, .. }) => (ranges, None),
            Some(ReplacedTargetCleanup::CachedGeneration {
                cluster_map, ranges, ..
            }) => (Vec::new(), Some((cluster_map, ranges))),
            None => (Vec::new(), None),
        };
        if let Some(target_child) = target_child {
            Self::detach_namespace_removed_inode(
                fs_state,
                allocation_guard,
                target_child.guard.metadata().ino,
                target_child.inode,
                target_child.guard,
                detached_regular_file_reclaim,
            )?;
        }
        ExfatFs::rebind_rename_inode_cache(
            fs_state,
            old_source_ino,
            new_source_ino,
            source_child.inode,
            replaced_target_ino,
        );
        Self::cleanup_replaced_target_ranges(
            fs_state,
            allocation_guard,
            &replaced_target_ranges,
        )
    }

    fn persist_same_directory_rename(
        plan: SameDirectoryRenamePlan,
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
    ) -> Result<(
        StreamExtensionDirEntry,
        DirEntrySlotRange,
        Option<ReplacedTargetCleanup>,
    )> {
        let SameDirectoryRenamePlan {
            cluster_map,
            directory_bytes,
            source_slot_range,
            destination_slot_range,
            replaced_slot_range,
            renamed_entry_set,
            replacement,
        } = plan;
        Self::persist_rename_directory_update(
            block_device,
            boot_region,
            cluster_map,
            directory_bytes,
            Some((destination_slot_range, &renamed_entry_set)),
            replaced_slot_range,
            (destination_slot_range != source_slot_range).then_some(source_slot_range),
        )?;
        Ok((cluster_map, destination_slot_range, replacement))
    }

    fn persist_cross_directory_rename(
        plan: CrossDirectoryRenamePlan,
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
    ) -> Result<(
        StreamExtensionDirEntry,
        DirEntrySlotRange,
        Option<ReplacedTargetCleanup>,
    )> {
        let CrossDirectoryRenamePlan {
            source_cluster_map,
            source_directory_bytes,
            source_slot_range,
            target_cluster_map,
            target_directory_bytes,
            target_slot_range,
            renamed_entry_set,
            replacement,
        } = plan;
        Self::persist_rename_directory_update(
            block_device,
            boot_region,
            target_cluster_map,
            target_directory_bytes,
            Some((target_slot_range, &renamed_entry_set)),
            None,
            None,
        )?;
        Self::persist_rename_directory_update(
            block_device,
            boot_region,
            source_cluster_map,
            source_directory_bytes,
            None,
            Some(source_slot_range),
            None,
        )?;
        Ok((target_cluster_map, target_slot_range, replacement))
    }

    // Slot management

    fn find_vacant_entry_slots(
        is_root_directory: bool,
        directory_bytes: &[u8],
        required_entry_count: usize,
    ) -> Result<Option<DirEntrySlotRange>> {
        if required_entry_count == 0 {
            return Err(invalid_operation_input());
        }
        if directory_bytes.len() % DIRECTORY_ENTRY_SIZE != 0 {
            return Err(invalid_on_disk_layout());
        }

        let total_entries = directory_bytes.len() / DIRECTORY_ENTRY_SIZE;
        let mut run_length = 0usize;
        let mut run_start_index = 0usize;
        let mut entry_index = 0usize;
        loop {
            let scan_start_index = entry_index;
            match direntry::scan_dir_entry(is_root_directory, directory_bytes, entry_index)? {
                ScannedDirEntry::EndOfDirectory { entry_index } => {
                    if entry_index != scan_start_index {
                        run_length = 0;
                        run_start_index = entry_index;
                    }
                    let available_entries = total_entries
                        .checked_sub(entry_index)
                        .ok_or(invalid_on_disk_layout())?;
                    if run_length == 0 {
                        run_start_index = entry_index;
                    }
                    run_length = run_length
                        .checked_add(available_entries)
                        .ok_or(invalid_on_disk_layout())?;
                    if run_length >= required_entry_count {
                        return Ok(Some(DirEntrySlotRange::new(
                            run_start_index,
                            required_entry_count,
                        )?));
                    }
                    return Ok(None);
                }
                ScannedDirEntry::Vacant(slot_range) => {
                    if run_length == 0 || slot_range.first_entry_index() != scan_start_index {
                        run_start_index = slot_range.first_entry_index();
                        run_length = 0;
                    }
                    run_length = run_length
                        .checked_add(slot_range.entry_count())
                        .ok_or(invalid_on_disk_layout())?;
                    if run_length >= required_entry_count {
                        return Ok(Some(DirEntrySlotRange::new(
                            run_start_index,
                            required_entry_count,
                        )?));
                    }
                    entry_index = slot_range.next_entry_index()?;
                }
                ScannedDirEntry::File(entry_view) => {
                    run_length = 0;
                    entry_index = entry_view.slot_range().next_entry_index()?;
                }
                ScannedDirEntry::Issue { .. } => {
                    return Err(invalid_on_disk_layout());
                }
            }
        }
    }

    fn reserve_directory_entry_slots(
        &self,
        mut cluster_map: StreamExtensionDirEntry,
        allocation_guard: &mut AllocGuard<'_>,
        fs_state: &mut FsState,
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        parent_inode_state_guard: Option<&InodeStateWriteGuard<'_>>,
        self_inode_state_guard: &InodeStateWriteGuard<'_>,
        required_entry_count: usize,
    ) -> Result<(StreamExtensionDirEntry, Vec<u8>, DirEntrySlotRange)> {
        loop {
            let directory_bytes =
                Self::read_directory_bytes_for_cluster_map(block_device, boot_region, cluster_map)?;
            if let Some(slot_range) = Self::find_vacant_entry_slots(
                cluster_map.data_length.is_none(),
                &directory_bytes,
                required_entry_count,
            )? {
                return Ok((cluster_map, directory_bytes, slot_range));
            }
            cluster_map = self.grow_directory_cluster_map(
                cluster_map,
                allocation_guard,
                fs_state,
                block_device,
                boot_region,
                parent_inode_state_guard,
                self_inode_state_guard,
            )?;
        }
    }

    // Directory cluster-map growth

    fn grow_directory_cluster_map(
        &self,
        cluster_map: StreamExtensionDirEntry,
        allocation_guard: &mut AllocGuard<'_>,
        fs_state: &mut FsState,
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        parent_inode_state_guard: Option<&InodeStateWriteGuard<'_>>,
        self_inode_state_guard: &InodeStateWriteGuard<'_>,
    ) -> Result<StreamExtensionDirEntry> {
        allocation_guard.allocate(1, None)?;
        let allocated_cluster = allocation_guard.single_cluster()?;
        let update_result = (|| {
            Self::initialize_directory_cluster(block_device, boot_region, allocated_cluster)?;
            let updated_cluster_map = self.attach_directory_cluster(
                cluster_map,
                block_device,
                boot_region,
                allocated_cluster,
            )?;
        if updated_cluster_map.data_length.is_some() {
            let parent_inode_state_guard = parent_inode_state_guard.ok_or_else(|| {
                Error::with_message(
                    Errno::EINVAL,
                    "ordinary exFAT directory growth requires parent write-guard proof",
                )
            })?;
            self.rewrite_validated_entry_set_with_guard(
                self_inode_state_guard,
                parent_inode_state_guard,
                block_device,
                boot_region,
                |entry_view| {
                    let (inode_type, _first_cluster, _data_length, _no_fat_chain) =
                        entry_view.child_metadata(boot_region)?;
                    if inode_type != InodeType::Dir || !entry_view.is_directory() {
                        return Err(Error::from(invalid_on_disk_layout()));
                    }

                    let mut updated_entry_set = entry_view.to_mutable();
                    updated_entry_set.set_cluster_map(&updated_cluster_map)?;
                    Ok(Some(updated_entry_set.into_bytes()))
                },
            )?;
        }
            self.commit_directory_cluster_map(
                self_inode_state_guard,
                updated_cluster_map,
                cluster_map,
                boot_region,
            )?;
            Ok(updated_cluster_map)
        })();
        match update_result {
            Ok(updated_cluster_map) => {
                allocation_guard.commit_allocation();
                Ok(updated_cluster_map)
            }
            Err(error) => {
                if allocation_guard.rollback_allocation()? {
                    ExfatFs::disable_unsupported_discard_after_release(fs_state);
                }
                Err(error)
            }
        }
    }

    fn attach_directory_cluster(
        &self,
        cluster_map: StreamExtensionDirEntry,
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        allocated_cluster: u32,
    ) -> Result<StreamExtensionDirEntry> {
        let next_data_length = match cluster_map.data_length {
            Some(data_length) => data_length
                .checked_add(boot_region.cluster_size)
                .ok_or(invalid_on_disk_layout())?,
            None => boot_region.cluster_size,
        };

        let mut fat_reader = FatReader::new(block_device.as_ref(), boot_region);
        match cluster_map.data_length {
            Some(0) => {
                fat_reader.terminate_cluster_chain(allocated_cluster)?;
            }
            Some(data_length) if cluster_map.no_fat_chain => {
                let cluster_count = data_length.div_ceil(boot_region.cluster_size);
                fat_reader.link_contiguous_chain_to_cluster(
                    cluster_map.first_cluster,
                    cluster_count,
                    allocated_cluster,
                )?;
            }
            Some(_) => {
                fat_reader.append_cluster_to_chain(cluster_map.first_cluster, allocated_cluster)?;
            }
            None => {
                fat_reader.append_cluster_to_chain(cluster_map.first_cluster, allocated_cluster)?;
            }
        }

        let updated_cluster_map = match cluster_map.data_length {
            Some(0) => StreamExtensionDirEntry {
                first_cluster: allocated_cluster,
                data_length: Some(next_data_length),
                valid_data_length: Some(next_data_length),
                no_fat_chain: false,
                ..cluster_map
            },
            Some(_) if cluster_map.no_fat_chain => StreamExtensionDirEntry {
                data_length: Some(next_data_length),
                valid_data_length: Some(next_data_length),
                no_fat_chain: false,
                ..cluster_map
            },
            Some(_) => StreamExtensionDirEntry {
                data_length: Some(next_data_length),
                valid_data_length: Some(next_data_length),
                ..cluster_map
            },
            None => StreamExtensionDirEntry {
                data_length: None,
                ..cluster_map
            },
        };
        Ok(updated_cluster_map)
    }

    fn commit_directory_cluster_map(
        &self,
        self_inode_state_guard: &InodeStateWriteGuard<'_>,
        updated_cluster_map: StreamExtensionDirEntry,
        previous_cluster_map: StreamExtensionDirEntry,
        boot_region: &BootRegion,
    ) -> Result<()> {
        if self_inode_state_guard.dir_entry_stream() != previous_cluster_map {
            return Err(invalid_on_disk_layout());
        }
        let _ = self_inode_state_guard.replace_dir_entry_stream(updated_cluster_map);
        self_inode_state_guard.with_metadata_mut(|metadata| {
            metadata.size = metadata
                .size
                .checked_add(boot_region.cluster_size)
                .ok_or_else(invalid_on_disk_layout)?;
            Ok::<(), Error>(())
        })?;
        Ok(())
    }

    // Validation helpers

    fn first_directory_child_scan<'a>(
        cluster_map: StreamExtensionDirEntry,
        directory_bytes: &'a [u8],
    ) -> Result<Option<ScannedDirEntry<'a>>> {
        let is_root_directory = cluster_map.data_length.is_none();
        let mut entry_index = 0usize;
        loop {
            let entry_scan =
                direntry::scan_dir_entry(is_root_directory, directory_bytes, entry_index)?;
            match entry_scan {
                ScannedDirEntry::EndOfDirectory { .. } => return Ok(None),
                ScannedDirEntry::Vacant(slot_range) => {
                    entry_index = slot_range.next_entry_index()?;
                }
                ScannedDirEntry::Issue { .. } | ScannedDirEntry::File(_) => {
                    return Ok(Some(entry_scan));
                }
            }
        }
    }

    fn ensure_directory_snapshot_is_empty(
        cluster_map: StreamExtensionDirEntry,
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
    ) -> Result<()> {
        let child_directory_bytes =
            Self::read_directory_bytes_for_cluster_map(block_device, boot_region, cluster_map)
                .map_err(Error::from)?;
        if let Some(first_child_scan) =
            Self::first_directory_child_scan(cluster_map, &child_directory_bytes)
                .map_err(Error::from)?
        {
            match first_child_scan {
                ScannedDirEntry::Issue { .. } => {
                    return Err(Error::from(invalid_on_disk_layout()));
                }
                ScannedDirEntry::File(_) => return_errno!(Errno::ENOTEMPTY),
                ScannedDirEntry::EndOfDirectory { .. } | ScannedDirEntry::Vacant(_) => {
                    unreachable!()
                }
            }
        }
        Ok(())
    }

    fn allocated_cluster_ranges(
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        first_cluster: u32,
        data_length: usize,
        no_fat_chain: bool,
    ) -> Result<Vec<ClusterRange>> {
        if data_length == 0 {
            if first_cluster != 0 {
                return Err(invalid_on_disk_layout());
            }
            return Ok(Vec::new());
        }

        boot_region.validate_stream_data(
            first_cluster,
            u64::try_from(data_length).map_err(|_| invalid_on_disk_layout())?,
        )?;
        let expected_cluster_count = data_length.div_ceil(boot_region.cluster_size);
        if no_fat_chain {
            return Ok(vec![ClusterRange {
                start_cluster: first_cluster,
                cluster_count: expected_cluster_count,
            }]);
        }

        let mut cluster_ranges = Vec::new();
        let mut current_range_start = 0u32;
        let mut current_range_count = 0usize;
        let mut previous_cluster: Option<u32> = None;
        let mut total_cluster_count = 0usize;
        let mut fat_reader = FatReader::new(block_device.as_ref(), boot_region);
        fat_reader.walk_cluster_chain(first_cluster, |cluster, _| {
            total_cluster_count = total_cluster_count
                .checked_add(1)
                .ok_or(invalid_on_disk_layout())?;
            match previous_cluster {
                Some(previous_cluster) if previous_cluster.checked_add(1) == Some(cluster) => {
                    current_range_count = current_range_count
                        .checked_add(1)
                        .ok_or(invalid_on_disk_layout())?;
                }
                Some(_) => {
                    cluster_ranges.push(ClusterRange {
                        start_cluster: current_range_start,
                        cluster_count: current_range_count,
                    });
                    current_range_start = cluster;
                    current_range_count = 1;
                }
                None => {
                    current_range_start = cluster;
                    current_range_count = 1;
                }
            }
            previous_cluster = Some(cluster);
            Ok(ChainVisitControl::Continue)
        })?;
        if current_range_count == 0 || total_cluster_count != expected_cluster_count {
            return Err(invalid_on_disk_layout());
        }
        cluster_ranges.push(ClusterRange {
            start_cluster: current_range_start,
            cluster_count: current_range_count,
        });
        Ok(cluster_ranges)
    }

    fn lookup_rename_source_view<'a>(
        directory_bytes: &'a [u8],
        cluster_map: StreamExtensionDirEntry,
        upcase_table: &UpcaseTable,
        old_name: &[u16],
        old_name_hash: u16,
    ) -> Result<FileEntrySetView<'a>> {
        Self::locate_named_child_view(
            directory_bytes,
            cluster_map.data_length.is_none(),
            upcase_table,
            old_name,
            old_name_hash,
        )
        .map_err(Error::from)?
        .ok_or_else(|| Error::new(Errno::ENOENT))
    }

    fn lookup_rename_target_view<'a>(
        directory_bytes: &'a [u8],
        cluster_map: StreamExtensionDirEntry,
        upcase_table: &UpcaseTable,
        new_name: &[u16],
        new_name_hash: u16,
        excluded_slot_range: Option<DirEntrySlotRange>,
    ) -> Result<Option<FileEntrySetView<'a>>> {
        Ok(Self::locate_named_child_view(
            directory_bytes,
            cluster_map.data_length.is_none(),
            upcase_table,
            new_name,
            new_name_hash,
        )
        .map_err(Error::from)?
        .filter(|entry_view| Some(entry_view.slot_range()) != excluded_slot_range))
    }

    fn collect_replaced_target_cleanup(
        target_view: Option<FileEntrySetView<'_>>,
        target_child_inode: Option<&Arc<Self>>,
        target_child_inode_state_guard: Option<&InodeStateWriteGuard<'_>>,
        source_inode_type: InodeType,
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        allocation_guard: &AllocGuard<'_>,
    ) -> Result<Option<ReplacedTargetCleanup>> {
        let Some(target_view) = target_view else {
            return Ok(None);
        };
        let target_slot_range = target_view.slot_range();
        let (target_inode_type, first_cluster, data_length, no_fat_chain) = target_view
            .child_metadata(boot_region)
            .map_err(Error::from)?;
        if source_inode_type == InodeType::Dir && target_inode_type != InodeType::Dir {
            return_errno!(Errno::ENOTDIR);
        }
        if source_inode_type != InodeType::Dir && target_inode_type == InodeType::Dir {
            return_errno!(Errno::EISDIR);
        }
        if target_inode_type == InodeType::Dir {
            let (Some(_child_inode), Some(child_inode_state_guard)) =
                (target_child_inode, target_child_inode_state_guard)
            else {
                return Err(Error::from(invalid_on_disk_layout()));
            };
            // The rename caller already owns child inode admission here, so only consume a
            // caller-captured snapshot and avoid the wrapper-owned reentry path.
            let child_directory_snapshot = {
                let _rename_owned_child_guard = child_inode_state_guard;
                child_inode_state_guard.dir_entry_stream()
            };
            Self::ensure_directory_snapshot_is_empty(
                child_directory_snapshot,
                block_device,
                boot_region,
            )?;
        }
        if target_inode_type == InodeType::File {
            if let (Some(child_inode), Some(child_inode_state_guard)) =
                (target_child_inode, target_child_inode_state_guard)
            {
                let detached_regular_file_reclaim = Self::capture_cached_regular_file_retirement(
                    child_inode,
                    child_inode_state_guard,
                    allocation_guard,
                )?;
                return Ok(Some(ReplacedTargetCleanup::CachedGeneration {
                    slot_range: target_slot_range,
                    cluster_map: detached_regular_file_reclaim.0,
                    ranges: detached_regular_file_reclaim.1,
                }));
            }
        }
        let replaced_target_ranges = Self::allocated_cluster_ranges(
            block_device,
            boot_region,
            first_cluster,
            data_length,
            no_fat_chain,
        )
        .map_err(Error::from)?;
        Ok(Some(ReplacedTargetCleanup::Immediate {
            slot_range: target_slot_range,
            ranges: replaced_target_ranges,
        }))
    }

    fn reserve_rename_destination_slot(
        &self,
        cluster_map: StreamExtensionDirEntry,
        current_directory_bytes: Vec<u8>,
        reusable_slot_range: Option<DirEntrySlotRange>,
        fs_state: &mut FsState,
        allocation_guard: &mut AllocGuard<'_>,
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        parent_inode_state_guard: Option<&InodeStateWriteGuard<'_>>,
        self_inode_state_guard: &InodeStateWriteGuard<'_>,
        required_entry_count: usize,
    ) -> Result<(StreamExtensionDirEntry, Vec<u8>, DirEntrySlotRange, bool)> {
        if let Some(slot_range) = reusable_slot_range
            .filter(|slot_range| slot_range.entry_count() >= required_entry_count)
        {
            return Ok((cluster_map, current_directory_bytes, slot_range, false));
        }
        let (updated_cluster_map, directory_bytes, slot_range) = self
            .reserve_directory_entry_slots(
                cluster_map,
                allocation_guard,
                fs_state,
                block_device,
                boot_region,
                parent_inode_state_guard,
                self_inode_state_guard,
                required_entry_count,
            )
            .map_err(Error::from)?;
        Ok((updated_cluster_map, directory_bytes, slot_range, true))
    }

    fn persist_rename_directory_update(
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        cluster_map: StreamExtensionDirEntry,
        mut directory_bytes: Vec<u8>,
        written_entry: Option<(DirEntrySlotRange, &[u8])>,
        first_invalidated_slot: Option<DirEntrySlotRange>,
        second_invalidated_slot: Option<DirEntrySlotRange>,
    ) -> Result<()> {
        let mut invalidate_slot_range = |slot_range: DirEntrySlotRange| -> Result<()> {
            let slot_range_bytes = direntry::slot_range_bytes(slot_range).map_err(Error::from)?;
            let removed_entry_set = directory_bytes
                .get_mut(slot_range_bytes)
                .ok_or(invalid_on_disk_layout())
                .map_err(Error::from)?;
            let mut removed_entry_set =
                MutableDirEntrySlotSpan::new(slot_range, removed_entry_set).map_err(Error::from)?;
            direntry::invalidate_entry_set(&mut removed_entry_set).map_err(Error::from)
        };
        if let Some(slot_range) = first_invalidated_slot {
            invalidate_slot_range(slot_range)?;
        }
        if let Some(slot_range) =
            second_invalidated_slot.filter(|slot_range| first_invalidated_slot != Some(*slot_range))
        {
            invalidate_slot_range(slot_range)?;
        }
        if let Some((destination_slot_range, renamed_entry_set)) = written_entry {
            if first_invalidated_slot != Some(destination_slot_range)
                && second_invalidated_slot != Some(destination_slot_range)
            {
                invalidate_slot_range(destination_slot_range)?;
            }
            let destination_slot_bytes =
                direntry::slot_range_bytes(destination_slot_range).map_err(Error::from)?;
            let destination_entry_set = directory_bytes
                .get_mut(destination_slot_bytes)
                .ok_or(invalid_on_disk_layout())
                .map_err(Error::from)?;
            let mut destination_entry_set =
                MutableDirEntrySlotSpan::new(destination_slot_range, destination_entry_set)
                    .map_err(Error::from)?;
            destination_entry_set
                .bytes_mut()
                .get_mut(..renamed_entry_set.len())
                .ok_or(invalid_on_disk_layout())
                .map_err(Error::from)?
                .copy_from_slice(renamed_entry_set);
        }
        Self::write_directory_bytes_for_cluster_map(
            block_device,
            boot_region,
            &directory_bytes,
            cluster_map,
        )
        .map_err(Error::from)
    }

    fn cleanup_replaced_target_ranges(
        fs_state: &mut FsState,
        allocation_guard: &mut AllocGuard<'_>,
        replaced_target_ranges: &[ClusterRange],
    ) -> Result<()> {
        if replaced_target_ranges.is_empty() {
            return Ok(());
        }
        allocation_guard.free_clusters(replaced_target_ranges)?;
        ExfatFs::disable_unsupported_discard_after_release(fs_state);
        Ok(())
    }

    fn capture_cached_regular_file_retirement(
        child_inode: &Arc<Self>,
        child_inode_state_guard: &InodeStateWriteGuard<'_>,
        allocation_guard: &AllocGuard<'_>,
    ) -> Result<(Arc<ClusterMap>, Vec<ClusterRange>)> {
        let retired_generation =
            child_inode.current_cluster_map(child_inode_state_guard, allocation_guard)?;
        let retired_ranges = retired_generation.cluster_ranges().to_vec();
        Ok((retired_generation, retired_ranges))
    }

    fn detach_namespace_removed_inode(
        fs_state: &mut FsState,
        allocation_guard: &mut AllocGuard<'_>,
        child_ino: u64,
        child_inode: &Arc<Self>,
        child_inode_state_guard: &InodeStateWriteGuard<'_>,
        detached_regular_file_reclaim: Option<(Arc<ClusterMap>, Vec<ClusterRange>)>,
    ) -> Result<()> {
        child_inode_state_guard.set_parent(Weak::new());
        child_inode.clear_entry_set_location_hint();
        child_inode_state_guard.with_metadata_mut(|metadata| metadata.nr_hard_links = 0);
        if let Some((retired_generation, retired_ranges)) = detached_regular_file_reclaim {
            child_inode
                .clear_detached_regular_file_publish_debt_with_guard(child_inode_state_guard);
            ExfatFs::remove_cached_inode(fs_state, child_ino);
            allocation_guard.lazy_reclaim_clusters(retired_generation, retired_ranges)?;
        }
        Ok(())
    }
}
