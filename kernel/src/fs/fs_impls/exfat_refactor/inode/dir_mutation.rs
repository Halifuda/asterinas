// SPDX-License-Identifier: MPL-2.0

//! Implements directory namespace mutations and directory cluster-map growth.
//!
//! Method groups: create/unlink/rmdir/rename entry points, slot management, directory growth,
//! emptiness validation, cluster-range collection, and rename-stage helpers.

use core::sync::atomic::Ordering;

use aster_block::BlockDevice;
use ostd::mm::VmIo;

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
            let cluster_map_generation =
                self.cluster_map_for_write_guard(self_inode_state_guard, &allocation_guard, cluster_map)?;
            let logical_end = match cluster_map.data_length {
                Some(data_length) => data_length,
                None => cluster_map_generation.allocated_byte_length(&boot_region)?,
            };
            let current_directory_bytes = self
                .read_directory_snapshot_from_page_cache(
                    self_inode_state_guard.metadata(),
                    cluster_map_generation,
                    logical_end,
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
            let (cluster_map, current_directory_bytes, slot_range) = self
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
            let mut create_primary_error = None;
            let metadata_refresh_timestamp = RealTimeCoarseClock::get().read_time();
            let prepared_directory_refresh = if cluster_map.data_length.is_none() {
                None
            } else {
                let parent_inode_state_guard = parent_inode_state_guard.ok_or_else(|| {
                    Error::with_message(
                        Errno::EINVAL,
                        "ordinary exFAT directory refresh requires parent write-guard proof",
                    )
                })?;
                self.prepare_rewritten_entry_set_write_with_guard(
                    self_inode_state_guard,
                    parent_inode_state_guard,
                    &boot_region,
                    |entry_view| {
                        let (timestamp_bytes, ten_ms_increment, encoded_utc_offset_byte) =
                            Self::encoded_exfat_timestamp_fields(
                                metadata_refresh_timestamp,
                                entry_view.last_modified_timestamp().utc_offset_byte(),
                            )?;
                        let mut mutable_entry_set = entry_view.to_mutable();
                        mutable_entry_set.set_last_modified_timestamp(
                            direntry::FileEntryTimestamp::new(
                                timestamp_bytes,
                                Some(ten_ms_increment),
                                encoded_utc_offset_byte,
                            ),
                        );
                        Ok(Some(mutable_entry_set.into_bytes()))
                    },
                )?
            };
            let slot_range_bytes = direntry::slot_range_bytes(slot_range).map_err(Error::from)?;
            let child_ino = self
                .entry_location_ino(cluster_map, slot_range.first_entry_index())
                .map_err(Error::from)?;
            let child_entry_set_location_hint = if type_ == InodeType::File {
                let encoded_first_entry_index = u64::from(
                    u32::try_from(slot_range.first_entry_index())
                        .map_err(|_| Error::from(invalid_on_disk_layout()))?,
                )
                .checked_add(1)
                .ok_or_else(|| Error::from(invalid_on_disk_layout()))?;
                let entry_count = u64::from(
                    u32::try_from(slot_range.entry_count())
                        .map_err(|_| Error::from(invalid_on_disk_layout()))?,
                );
                Some((encoded_first_entry_index << 32) | entry_count)
            } else {
                None
            };
            let (first_cluster, data_length, no_fat_chain, child_cluster_map) = if type_ == InodeType::Dir
                && !options.zero_size_dir
            {
                allocation_guard.allocate(1, None).map_err(Error::from)?;
                let allocated_cluster = allocation_guard.single_cluster().map_err(Error::from)?;
                if let Err(error) = Self::initialize_directory_cluster(
                    &block_device,
                    &boot_region,
                    allocated_cluster,
                ) {
                    if allocation_guard.rollback_allocation()? {
                        ExfatFs::disable_unsupported_discard_after_release(&mut fs_state);
                    }
                    return Err(error);
                }
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
                let byte_mutations = vec![(
                    slot_range_bytes.clone(),
                    current_directory_bytes
                        .get(slot_range_bytes.clone())
                        .ok_or(invalid_on_disk_layout())
                        .map_err(Error::from)?
                        .to_vec(),
                    entry_set,
                )];
                let child_cluster_map = Some(Arc::new(ClusterMap::from_stream_and_ranges(
                    &boot_region,
                    StreamExtensionDirEntry {
                        data_length: Some(boot_region.cluster_size),
                        first_cluster: allocated_cluster,
                        valid_data_length: Some(boot_region.cluster_size),
                        no_fat_chain: true,
                    },
                    vec![ClusterRange {
                        start_cluster: allocated_cluster,
                        cluster_count: 1,
                    }],
                )?));
                match self.persist_directory_page_cache_mutation_classified(
                    &mut fs_state,
                    self_inode_state_guard.metadata(),
                    &byte_mutations,
                    true,
                ) {
                    Ok(Ok(())) => {
                        allocation_guard.commit_allocation();
                    }
                    Ok(Err(error)) => {
                        allocation_guard.commit_allocation();
                        create_primary_error = Some(error);
                    }
                    Err(error) => {
                        if allocation_guard.rollback_allocation()? {
                            ExfatFs::disable_unsupported_discard_after_release(&mut fs_state);
                        }
                        return Err(error);
                    }
                }
                (allocated_cluster, boot_region.cluster_size, true, child_cluster_map)
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
                let byte_mutations = vec![(
                    slot_range_bytes.clone(),
                    current_directory_bytes
                        .get(slot_range_bytes.clone())
                        .ok_or(invalid_on_disk_layout())
                        .map_err(Error::from)?
                        .to_vec(),
                    entry_set,
                )];
                let child_cluster_map = if type_ == InodeType::Dir {
                    Some(Arc::new(ClusterMap::from_stream_and_ranges(
                        &boot_region,
                        StreamExtensionDirEntry {
                            data_length: Some(0),
                            first_cluster: 0,
                            valid_data_length: Some(0),
                            no_fat_chain: false,
                        },
                        Vec::new(),
                    )?))
                } else {
                    None
                };
                match self.persist_directory_page_cache_mutation_classified(
                    &mut fs_state,
                    self_inode_state_guard.metadata(),
                    &byte_mutations,
                    true,
                ) {
                    Ok(Ok(())) => {}
                    Ok(Err(error)) => create_primary_error = Some(error),
                    Err(error) => return Err(error),
                }
                (0, 0, false, child_cluster_map)
            };

            let child_size = if type_ == InodeType::Dir {
                data_length
            } else {
                0
            };
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
                child_cluster_map,
            );
            if let Some(child_entry_set_location_hint) = child_entry_set_location_hint {
                child_inode
                    .entry_set_location_hint
                    .store(child_entry_set_location_hint, Ordering::Relaxed);
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
            let metadata_refresh_result = self.refresh_directory_metadata_after_namespace_mutation_with_guards(
                &mut fs_state,
                &boot_region,
                metadata_refresh_timestamp,
                self_inode_state_guard,
                parent_inode_state_guard,
                prepared_directory_refresh,
                true,
            );
            match (create_primary_error, metadata_refresh_result) {
                (None, Ok(())) => Ok(child_inode),
                (Some(error), Ok(())) => Err(error),
                (None, Err(error)) => Err(error),
                (Some(primary_error), Err(_refresh_error)) => Err(primary_error),
            }
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
            let cluster_map_generation = self.cluster_map_for_read_guard(
                &provisional_directory_guard,
                &discovery_allocation_guard,
                cluster_map,
            )?;
            let logical_end = match cluster_map.data_length {
                Some(data_length) => data_length,
                None => cluster_map_generation.allocated_byte_length(&boot_region)?,
            };
            let directory_bytes = self
                .read_directory_snapshot_from_page_cache(
                    provisional_directory_guard.metadata(),
                    cluster_map_generation,
                    logical_end,
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
            let cluster_map_generation =
                self.cluster_map_for_write_guard(self_inode_state_guard, &allocation_guard, cluster_map)?;
            let logical_end = match cluster_map.data_length {
                Some(data_length) => data_length,
                None => cluster_map_generation.allocated_byte_length(&boot_region)?,
            };
            let directory_bytes = self
                .read_directory_snapshot_from_page_cache(
                    self_inode_state_guard.metadata(),
                    cluster_map_generation,
                    logical_end,
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
            let metadata_refresh_timestamp = RealTimeCoarseClock::get().read_time();
            let prepared_directory_refresh = if cluster_map.data_length.is_none() {
                None
            } else {
                let parent_inode_state_guard = parent_inode_state_guard.ok_or_else(|| {
                    Error::with_message(
                        Errno::EINVAL,
                        "ordinary exFAT directory refresh requires parent write-guard proof",
                    )
                })?;
                self.prepare_rewritten_entry_set_write_with_guard(
                    self_inode_state_guard,
                    parent_inode_state_guard,
                    &boot_region,
                    |entry_view| {
                        let (timestamp_bytes, ten_ms_increment, encoded_utc_offset_byte) =
                            Self::encoded_exfat_timestamp_fields(
                                metadata_refresh_timestamp,
                                entry_view.last_modified_timestamp().utc_offset_byte(),
                            )?;
                        let mut mutable_entry_set = entry_view.to_mutable();
                        mutable_entry_set.set_last_modified_timestamp(
                            direntry::FileEntryTimestamp::new(
                                timestamp_bytes,
                                Some(ten_ms_increment),
                                encoded_utc_offset_byte,
                            ),
                        );
                        Ok(Some(mutable_entry_set.into_bytes()))
                    },
                )?
            };

            fs.publish_dirty_admission(&mut fs_state)?;
            let slot_range_bytes = direntry::slot_range_bytes(slot_range).map_err(Error::from)?;
            let old_entry_set_bytes = directory_bytes
                .get(slot_range_bytes.clone())
                .ok_or(invalid_on_disk_layout())
                .map_err(Error::from)?
                .to_vec();
            let mut new_entry_set_bytes = old_entry_set_bytes.clone();
            let mut removed_entry_set =
                MutableDirEntrySlotSpan::new(slot_range, new_entry_set_bytes.as_mut_slice())
                    .map_err(Error::from)?;
            direntry::invalidate_entry_set(&mut removed_entry_set).map_err(Error::from)?;
            let byte_mutations = vec![(slot_range_bytes, old_entry_set_bytes, new_entry_set_bytes)];
            let unlink_primary_error = match self.persist_directory_page_cache_mutation_classified(
                &mut fs_state,
                self_inode_state_guard.metadata(),
                &byte_mutations,
                true,
            ) {
                Ok(Ok(())) => None,
                Ok(Err(error)) => Some(error),
                Err(error) => return Err(error),
            };
            let mut unlink_followup_error = None;
            if let (Some(cached_child_inode), Some(cached_child_inode_state_guard)) = (
                cached_child_inode.as_ref(),
                cached_child_inode_state_guard.as_ref(),
            ) {
                if let Err(error) = Self::detach_namespace_removed_inode(
                    &mut fs_state,
                    &mut allocation_guard,
                    child_ino,
                    cached_child_inode,
                    cached_child_inode_state_guard,
                    unlink_primary_error
                        .is_none()
                        .then_some(detached_regular_file_reclaim)
                        .flatten(),
                ) {
                    unlink_followup_error = Some(error);
                }
            } else if unlink_primary_error.is_none() && !allocated_cluster_ranges.is_empty() {
                if let Err(error) = allocation_guard.free_clusters(&allocated_cluster_ranges) {
                    unlink_followup_error = Some(error);
                } else {
                    ExfatFs::disable_unsupported_discard_after_release(&mut fs_state);
                }
            }
            let metadata_refresh_result = self.refresh_directory_metadata_after_namespace_mutation_with_guards(
                &mut fs_state,
                &boot_region,
                metadata_refresh_timestamp,
                self_inode_state_guard,
                parent_inode_state_guard,
                prepared_directory_refresh,
                true,
            );
            let unlink_primary_error = unlink_primary_error.or(unlink_followup_error);
            match (unlink_primary_error, metadata_refresh_result) {
                (None, Ok(())) => Ok(()),
                (Some(error), Ok(())) => Err(error),
                (None, Err(error)) => Err(error),
                (Some(primary_error), Err(_refresh_error)) => Err(primary_error),
            }
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
            let cluster_map_generation = self.cluster_map_for_read_guard(
                &provisional_directory_guard,
                &discovery_allocation_guard,
                cluster_map,
            )?;
            let logical_end = match cluster_map.data_length {
                Some(data_length) => data_length,
                None => cluster_map_generation.allocated_byte_length(&boot_region)?,
            };
            let directory_bytes = self
                .read_directory_snapshot_from_page_cache(
                    provisional_directory_guard.metadata(),
                    cluster_map_generation,
                    logical_end,
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
            let metadata_refresh_timestamp = RealTimeCoarseClock::get().read_time();
            let prepared_directory_refresh = if cluster_map.data_length.is_none() {
                None
            } else {
                let parent_inode_state_guard = parent_inode_state_guard.ok_or_else(|| {
                    Error::with_message(
                        Errno::EINVAL,
                        "ordinary exFAT directory refresh requires parent write-guard proof",
                    )
                })?;
                self.prepare_rewritten_entry_set_write_with_guard(
                    self_inode_state_guard,
                    parent_inode_state_guard,
                    &boot_region,
                    |entry_view| {
                        let (timestamp_bytes, ten_ms_increment, encoded_utc_offset_byte) =
                            Self::encoded_exfat_timestamp_fields(
                                metadata_refresh_timestamp,
                                entry_view.last_modified_timestamp().utc_offset_byte(),
                            )?;
                        let mut mutable_entry_set = entry_view.to_mutable();
                        mutable_entry_set.set_last_modified_timestamp(
                            direntry::FileEntryTimestamp::new(
                                timestamp_bytes,
                                Some(ten_ms_increment),
                                encoded_utc_offset_byte,
                            ),
                        );
                        Ok(Some(mutable_entry_set.into_bytes()))
                    },
                )?
            };
            let cluster_map_generation =
                self.cluster_map_for_write_guard(self_inode_state_guard, &allocation_guard, cluster_map)?;
            let logical_end = match cluster_map.data_length {
                Some(data_length) => data_length,
                None => cluster_map_generation.allocated_byte_length(&boot_region)?,
            };
            let directory_bytes = self
                .read_directory_snapshot_from_page_cache(
                    self_inode_state_guard.metadata(),
                    cluster_map_generation,
                    logical_end,
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
                child_inode.as_ref(),
                child_inode_state_guard,
                &allocation_guard,
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
            let slot_range_bytes = direntry::slot_range_bytes(slot_range).map_err(Error::from)?;
            let old_entry_set_bytes = directory_bytes
                .get(slot_range_bytes.clone())
                .ok_or(invalid_on_disk_layout())
                .map_err(Error::from)?
                .to_vec();
            let mut new_entry_set_bytes = old_entry_set_bytes.clone();
            let mut removed_entry_set =
                MutableDirEntrySlotSpan::new(slot_range, new_entry_set_bytes.as_mut_slice())
                    .map_err(Error::from)?;
            direntry::invalidate_entry_set(&mut removed_entry_set).map_err(Error::from)?;
            let byte_mutations = vec![(slot_range_bytes, old_entry_set_bytes, new_entry_set_bytes)];
            let rmdir_primary_error = match self.persist_directory_page_cache_mutation_classified(
                &mut fs_state,
                self_inode_state_guard.metadata(),
                &byte_mutations,
                true,
            ) {
                Ok(Ok(())) => None,
                Ok(Err(error)) => Some(error),
                Err(error) => return Err(error),
            };
            let mut rmdir_followup_error = None;
            if let Err(error) = Self::detach_namespace_removed_inode(
                &mut fs_state,
                &mut allocation_guard,
                child_ino,
                &child_inode,
                child_inode_state_guard,
                None,
            ) {
                rmdir_followup_error = Some(error);
            }
            if rmdir_primary_error.is_none() && !allocated_cluster_ranges.is_empty() {
                if let Err(error) = allocation_guard.free_clusters(&allocated_cluster_ranges) {
                    if rmdir_followup_error.is_none() {
                        rmdir_followup_error = Some(error);
                    }
                } else {
                    ExfatFs::disable_unsupported_discard_after_release(&mut fs_state);
                }
            }
            let metadata_refresh_result = self.refresh_directory_metadata_after_namespace_mutation_with_guards(
                &mut fs_state,
                &boot_region,
                metadata_refresh_timestamp,
                self_inode_state_guard,
                parent_inode_state_guard,
                prepared_directory_refresh,
                true,
            );
            let rmdir_primary_error = rmdir_primary_error.or(rmdir_followup_error);
            match (rmdir_primary_error, metadata_refresh_result) {
                (None, Ok(())) => Ok(()),
                (Some(error), Ok(())) => Err(error),
                (None, Err(error)) => Err(error),
                (Some(primary_error), Err(_refresh_error)) => Err(primary_error),
            }
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
        boot_region: &BootRegion,
        upcase_table: &UpcaseTable,
        names: RenameNames<'_>,
    ) -> Result<RenameDiscovery> {
        let provisional_directory_guards =
            Self::directory_read_guards_by_stable_identity(vec![self, target_directory]);
        let provisional_guard_for_inode = |inode: &ExfatInode| {
            provisional_directory_guards
                .iter()
                .find(|guard| guard.guards_inode(inode))
                .ok_or_else(|| Error::new(Errno::EINVAL))
        };
        let (
            self_ino,
            source_parent_directory,
            source_cluster_map,
            target_directory_ino,
            target_parent_directory,
            target_cluster_map,
        ) = {
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
                let source_guard = provisional_guard_for_inode(self)?;
                let source_cluster_map_generation = self.cluster_map_for_read_guard(
                    source_guard,
                    &discovery_allocation_guard,
                    source_cluster_map,
                )?;
                let source_logical_end = match source_cluster_map.data_length {
                    Some(data_length) => data_length,
                    None => source_cluster_map_generation.allocated_byte_length(boot_region)?,
                };
                let directory_bytes = self
                    .read_directory_snapshot_from_page_cache(
                        source_guard.metadata(),
                        source_cluster_map_generation,
                        source_logical_end,
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
                let source_guard = provisional_guard_for_inode(self)?;
                let source_cluster_map_generation = self.cluster_map_for_read_guard(
                    source_guard,
                    &discovery_allocation_guard,
                    source_cluster_map,
                )?;
                let source_logical_end = match source_cluster_map.data_length {
                    Some(data_length) => data_length,
                    None => source_cluster_map_generation.allocated_byte_length(boot_region)?,
                };
                let source_directory_bytes = self
                    .read_directory_snapshot_from_page_cache(
                        source_guard.metadata(),
                        source_cluster_map_generation,
                        source_logical_end,
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
                let target_guard = provisional_guard_for_inode(target_directory)?;
                let target_cluster_map_generation = target_directory.cluster_map_for_read_guard(
                    target_guard,
                    &discovery_allocation_guard,
                    target_cluster_map,
                )?;
                let target_logical_end = match target_cluster_map.data_length {
                    Some(data_length) => data_length,
                    None => target_cluster_map_generation.allocated_byte_length(boot_region)?,
                };
                let target_directory_bytes = target_directory
                    .read_directory_snapshot_from_page_cache(
                        target_guard.metadata(),
                        target_cluster_map_generation,
                        target_logical_end,
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
                    if let Err(error) = renamed {
                        return Err(error);
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
                    let rename_result = self.rename_across_directories(
                        source_cluster_map,
                        source_guard,
                        source_parent_guard,
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
                    match rename_result {
                        Ok(()) => Ok(()),
                        Err(error) => Err(error),
                    }
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
    ) -> Result<Result<bool>> {
        let (target_child_inode, target_child_inode_state_guard) = match target_child.as_ref() {
            Some(child) => (Some(child.inode), Some(child.guard)),
            None => (None, None),
        };
        let old_name = names.source;
        let old_name_hash = names.source_hash;
        let new_name = names.destination;
        let new_name_hash = names.destination_hash;
        let cluster_map_generation =
            self.cluster_map_for_write_guard(self_inode_state_guard, allocation_guard, cluster_map)?;
        let logical_end = match cluster_map.data_length {
            Some(data_length) => data_length,
            None => cluster_map_generation.allocated_byte_length(boot_region)?,
        };
        let current_directory_bytes = self
            .read_directory_snapshot_from_page_cache(
                self_inode_state_guard.metadata(),
                cluster_map_generation,
                logical_end,
            )
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
            return Ok(Ok(false));
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
        let metadata_refresh_timestamp = RealTimeCoarseClock::get().read_time();
        let prepared_directory_refresh = if cluster_map.data_length.is_none() {
            None
        } else {
            let parent_inode_state_guard = parent_inode_state_guard.ok_or_else(|| {
                Error::with_message(
                    Errno::EINVAL,
                    "ordinary exFAT directory refresh requires parent write-guard proof",
                )
            })?;
            self.prepare_rewritten_entry_set_write_with_guard(
                self_inode_state_guard,
                parent_inode_state_guard,
                boot_region,
                |entry_view| {
                    let (timestamp_bytes, ten_ms_increment, encoded_utc_offset_byte) =
                        Self::encoded_exfat_timestamp_fields(
                            metadata_refresh_timestamp,
                            entry_view.last_modified_timestamp().utc_offset_byte(),
                        )?;
                    let mut mutable_entry_set = entry_view.to_mutable();
                    mutable_entry_set.set_last_modified_timestamp(direntry::FileEntryTimestamp::new(
                        timestamp_bytes,
                        Some(ten_ms_increment),
                        encoded_utc_offset_byte,
                    ));
                    Ok(Some(mutable_entry_set.into_bytes()))
                },
            )?
        };
        let new_source_ino = self
            .entry_location_ino(cluster_map, final_slot_range.first_entry_index())
            .map_err(Error::from)?;
        let source_entry_set_location_hint = if source_child.guard.metadata().type_ == InodeType::File
        {
            let encoded_first_entry_index = u64::from(
                u32::try_from(final_slot_range.first_entry_index())
                    .map_err(|_| Error::from(invalid_on_disk_layout()))?,
            )
            .checked_add(1)
            .ok_or_else(|| Error::from(invalid_on_disk_layout()))?;
            let entry_count = u64::from(
                u32::try_from(final_slot_range.entry_count())
                    .map_err(|_| Error::from(invalid_on_disk_layout()))?,
            );
            Some((encoded_first_entry_index << 32) | entry_count)
        } else {
            None
        };
        let destination_slot_bytes = direntry::slot_range_bytes(final_slot_range).map_err(Error::from)?;
        let destination_old_bytes = renamed_directory_bytes
            .get(destination_slot_bytes.clone())
            .ok_or(invalid_on_disk_layout())
            .map_err(Error::from)?
            .to_vec();
        let mut destination_new_bytes = destination_old_bytes.clone();
        {
            let mut destination_entry_set =
                MutableDirEntrySlotSpan::new(final_slot_range, destination_new_bytes.as_mut_slice())
                    .map_err(Error::from)?;
            direntry::invalidate_entry_set(&mut destination_entry_set).map_err(Error::from)?;
        }
        destination_new_bytes
            .get_mut(..renamed_entry_set.len())
            .ok_or(invalid_on_disk_layout())
            .map_err(Error::from)?
            .copy_from_slice(&renamed_entry_set);
        let mut byte_mutations =
            vec![(destination_slot_bytes, destination_old_bytes, destination_new_bytes)];
        if final_slot_range != source_slot_range {
            let source_slot_bytes = direntry::slot_range_bytes(source_slot_range).map_err(Error::from)?;
            let source_old_bytes = renamed_directory_bytes
                .get(source_slot_bytes.clone())
                .ok_or(invalid_on_disk_layout())
                .map_err(Error::from)?
                .to_vec();
            let mut source_new_bytes = source_old_bytes.clone();
            let mut source_entry_set =
                MutableDirEntrySlotSpan::new(source_slot_range, source_new_bytes.as_mut_slice())
                    .map_err(Error::from)?;
            direntry::invalidate_entry_set(&mut source_entry_set).map_err(Error::from)?;
            byte_mutations.push((source_slot_bytes, source_old_bytes, source_new_bytes));
        }
        if let Some(replaced_slot_range) =
            replaced_target_slot_range.filter(|slot_range| *slot_range != final_slot_range)
        {
            let replaced_slot_bytes =
                direntry::slot_range_bytes(replaced_slot_range).map_err(Error::from)?;
            let replaced_old_bytes = renamed_directory_bytes
                .get(replaced_slot_bytes.clone())
                .ok_or(invalid_on_disk_layout())
                .map_err(Error::from)?
                .to_vec();
            let mut replaced_new_bytes = replaced_old_bytes.clone();
            let mut replaced_entry_set = MutableDirEntrySlotSpan::new(
                replaced_slot_range,
                replaced_new_bytes.as_mut_slice(),
            )
            .map_err(Error::from)?;
            direntry::invalidate_entry_set(&mut replaced_entry_set).map_err(Error::from)?;
            byte_mutations.push((replaced_slot_bytes, replaced_old_bytes, replaced_new_bytes));
        }
        byte_mutations.sort_by_key(|(byte_range, _, _)| byte_range.start);
        let persist_status = self.persist_directory_page_cache_mutation_classified(
            fs_state,
            self_inode_state_guard.metadata(),
            &byte_mutations,
            true,
        )?;
        let finalize_error = Self::finalize_rename_protocol(
            self,
            cluster_map,
            final_slot_range,
            source_entry_set_location_hint,
            new_source_ino,
            source_child,
            target_child,
            replacement,
            fs_state,
            allocation_guard,
            persist_status.is_ok(),
        )
        .err();
        let persist_status = match (persist_status, finalize_error) {
            (Ok(()), None) => Ok(()),
            (Ok(()), Some(error)) => Err(error),
            (Err(error), None) => Err(error),
            (Err(primary_error), Some(_finalize_error)) => Err(primary_error),
        };
        let metadata_refresh_result = self.refresh_directory_metadata_after_namespace_mutation_with_guards(
            fs_state,
            boot_region,
            metadata_refresh_timestamp,
            self_inode_state_guard,
            parent_inode_state_guard,
            prepared_directory_refresh,
            true,
        );
        match (persist_status, metadata_refresh_result) {
            (Ok(()), Ok(())) => Ok(Ok(true)),
            (Err(error), Ok(())) => Ok(Err(error)),
            (Ok(()), Err(error)) => Ok(Err(error)),
            (Err(primary_error), Err(_refresh_error)) => Ok(Err(primary_error)),
        }
    }

    fn rename_across_directories(
        &self,
        source_cluster_map: StreamExtensionDirEntry,
        source_inode_state_guard: &InodeStateWriteGuard<'_>,
        source_parent_inode_state_guard: Option<&InodeStateWriteGuard<'_>>,
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
    ) -> Result<Result<()>> {
        let (target_child_inode, target_child_inode_state_guard) = match target_child.as_ref() {
            Some(child) => (Some(child.inode), Some(child.guard)),
            None => (None, None),
        };
        let old_name = names.source;
        let old_name_hash = names.source_hash;
        let new_name = names.destination;
        let new_name_hash = names.destination_hash;
        let source_cluster_map_generation =
            self.cluster_map_for_write_guard(source_inode_state_guard, allocation_guard, source_cluster_map)?;
        let source_logical_end = match source_cluster_map.data_length {
            Some(data_length) => data_length,
            None => source_cluster_map_generation.allocated_byte_length(boot_region)?,
        };
        let source_directory_bytes = self
            .read_directory_snapshot_from_page_cache(
                source_inode_state_guard.metadata(),
                source_cluster_map_generation,
                source_logical_end,
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
        let target_cluster_map_generation = target_directory.cluster_map_for_write_guard(
            target_inode_state_guard,
            allocation_guard,
            target_cluster_map,
        )?;
        let target_logical_end = match target_cluster_map.data_length {
            Some(data_length) => data_length,
            None => target_cluster_map_generation.allocated_byte_length(boot_region)?,
        };
        let target_directory_bytes = target_directory
            .read_directory_snapshot_from_page_cache(
                target_inode_state_guard.metadata(),
                target_cluster_map_generation,
                target_logical_end,
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
        let metadata_refresh_timestamp = RealTimeCoarseClock::get().read_time();
        let prepared_source_refresh = if source_cluster_map.data_length.is_none() {
            None
        } else {
            let source_parent_inode_state_guard = source_parent_inode_state_guard.ok_or_else(|| {
                Error::with_message(
                    Errno::EINVAL,
                    "ordinary exFAT directory refresh requires parent write-guard proof",
                )
            })?;
            self.prepare_rewritten_entry_set_write_with_guard(
                source_inode_state_guard,
                source_parent_inode_state_guard,
                boot_region,
                |entry_view| {
                    let (timestamp_bytes, ten_ms_increment, encoded_utc_offset_byte) =
                        Self::encoded_exfat_timestamp_fields(
                            metadata_refresh_timestamp,
                            entry_view.last_modified_timestamp().utc_offset_byte(),
                        )?;
                    let mut mutable_entry_set = entry_view.to_mutable();
                    mutable_entry_set.set_last_modified_timestamp(direntry::FileEntryTimestamp::new(
                        timestamp_bytes,
                        Some(ten_ms_increment),
                        encoded_utc_offset_byte,
                    ));
                    Ok(Some(mutable_entry_set.into_bytes()))
                },
            )?
        };
        let prepared_target_refresh = if target_cluster_map.data_length.is_none() {
            None
        } else {
            let target_parent_inode_state_guard = target_parent_inode_state_guard.ok_or_else(|| {
                Error::with_message(
                    Errno::EINVAL,
                    "ordinary exFAT directory refresh requires parent write-guard proof",
                )
            })?;
            target_directory.prepare_rewritten_entry_set_write_with_guard(
                target_inode_state_guard,
                target_parent_inode_state_guard,
                boot_region,
                |entry_view| {
                    let (timestamp_bytes, ten_ms_increment, encoded_utc_offset_byte) =
                        Self::encoded_exfat_timestamp_fields(
                            metadata_refresh_timestamp,
                            entry_view.last_modified_timestamp().utc_offset_byte(),
                        )?;
                    let mut mutable_entry_set = entry_view.to_mutable();
                    mutable_entry_set.set_last_modified_timestamp(direntry::FileEntryTimestamp::new(
                        timestamp_bytes,
                        Some(ten_ms_increment),
                        encoded_utc_offset_byte,
                    ));
                    Ok(Some(mutable_entry_set.into_bytes()))
                },
            )?
        };
        let new_source_ino = target_directory
            .entry_location_ino(target_cluster_map, target_slot_range.first_entry_index())
            .map_err(Error::from)?;
        let source_entry_set_location_hint = if source_child.guard.metadata().type_ == InodeType::File
        {
            let encoded_first_entry_index = u64::from(
                u32::try_from(target_slot_range.first_entry_index())
                    .map_err(|_| Error::from(invalid_on_disk_layout()))?,
            )
            .checked_add(1)
            .ok_or_else(|| Error::from(invalid_on_disk_layout()))?;
            let entry_count = u64::from(
                u32::try_from(target_slot_range.entry_count())
                    .map_err(|_| Error::from(invalid_on_disk_layout()))?,
            );
            Some((encoded_first_entry_index << 32) | entry_count)
        } else {
            None
        };
        let target_slot_bytes = direntry::slot_range_bytes(target_slot_range).map_err(Error::from)?;
        let target_old_bytes = target_directory_bytes
            .get(target_slot_bytes.clone())
            .ok_or(invalid_on_disk_layout())
            .map_err(Error::from)?
            .to_vec();
        let mut target_new_bytes = target_old_bytes.clone();
        {
            let mut target_entry_set =
                MutableDirEntrySlotSpan::new(target_slot_range, target_new_bytes.as_mut_slice())
                    .map_err(Error::from)?;
            direntry::invalidate_entry_set(&mut target_entry_set).map_err(Error::from)?;
        }
        target_new_bytes
            .get_mut(..renamed_entry_set.len())
            .ok_or(invalid_on_disk_layout())
            .map_err(Error::from)?
            .copy_from_slice(&renamed_entry_set);
        let source_slot_bytes = direntry::slot_range_bytes(source_slot_range).map_err(Error::from)?;
        let source_old_bytes = source_directory_bytes
            .get(source_slot_bytes.clone())
            .ok_or(invalid_on_disk_layout())
            .map_err(Error::from)?
            .to_vec();
        let mut source_new_bytes = source_old_bytes.clone();
        {
            let mut source_entry_set =
                MutableDirEntrySlotSpan::new(source_slot_range, source_new_bytes.as_mut_slice())
                    .map_err(Error::from)?;
            direntry::invalidate_entry_set(&mut source_entry_set).map_err(Error::from)?;
        }
        let target_page_cache = target_directory
            .page_cache_handle(target_inode_state_guard.metadata())
            .cloned()
            .ok_or_else(|| {
                Error::with_message(Errno::EIO, "directory exFAT inode has no page cache")
            })?;
        let source_page_cache = self
            .page_cache_handle(source_inode_state_guard.metadata())
            .cloned()
            .ok_or_else(|| {
                Error::with_message(Errno::EIO, "directory exFAT inode has no page cache")
            })?;
        let target_start_page = target_slot_bytes.start / PAGE_SIZE;
        let target_end_page = (target_slot_bytes.end - 1) / PAGE_SIZE;
        let source_start_page = source_slot_bytes.start / PAGE_SIZE;
        let source_end_page = (source_slot_bytes.end - 1) / PAGE_SIZE;
        let mut prefaulted_target_old_bytes = vec![0; target_slot_bytes.len()];
        let mut target_writer =
            VmWriter::from(prefaulted_target_old_bytes.as_mut_slice()).to_fallible();
        target_page_cache
            .read(target_slot_bytes.start, &mut target_writer)
            .map_err(Error::from)?;
        if prefaulted_target_old_bytes.as_slice() != target_old_bytes.as_slice() {
            return Err(invalid_operation_input());
        }
        let mut prefaulted_source_old_bytes = vec![0; source_slot_bytes.len()];
        let mut source_writer =
            VmWriter::from(prefaulted_source_old_bytes.as_mut_slice()).to_fallible();
        source_page_cache
            .read(source_slot_bytes.start, &mut source_writer)
            .map_err(Error::from)?;
        if prefaulted_source_old_bytes.as_slice() != source_old_bytes.as_slice() {
            return Err(invalid_operation_input());
        }
        let target_page_dirty_states = (target_start_page..=target_end_page)
            .map(|page_idx| {
                let page_start = page_idx
                    .checked_mul(PAGE_SIZE)
                    .ok_or_else(invalid_operation_input)?;
                let page_end = page_start
                    .saturating_add(PAGE_SIZE)
                    .min(target_page_cache.size());
                Ok((page_idx, target_page_cache.has_dirty_pages(page_start..page_end)))
            })
            .collect::<Result<Vec<_>>>()?;
        let mut persist_status = Ok(());
        let mut target_image_coherent = true;
        if let Err(error) = {
            let mut reader = VmReader::from(target_new_bytes.as_slice()).to_fallible();
            target_page_cache
                .write(target_slot_bytes.start, &mut reader)
                .map_err(Error::from)
        } {
            let mut target_page_restores = Vec::new();
            let mut old_byte_offset = 0usize;
            for page_idx in target_start_page..=target_end_page {
                let page_start = page_idx
                    .checked_mul(PAGE_SIZE)
                    .ok_or_else(invalid_operation_input)?;
                let page_end = page_start.saturating_add(PAGE_SIZE);
                let segment_start = target_slot_bytes.start.max(page_start);
                let segment_end = target_slot_bytes.end.min(page_end);
                let segment_len = segment_end
                    .checked_sub(segment_start)
                    .ok_or_else(invalid_operation_input)?;
                let was_dirty = target_page_dirty_states
                    .iter()
                    .find_map(|(captured_page_idx, was_dirty)| {
                        (*captured_page_idx == page_idx).then_some(*was_dirty)
                    })
                    .ok_or_else(invalid_operation_input)?;
                let old_byte_end = old_byte_offset
                    .checked_add(segment_len)
                    .ok_or_else(invalid_operation_input)?;
                target_page_restores.push((
                    page_idx,
                    (segment_start - page_start)..(segment_end - page_start),
                    &target_old_bytes[old_byte_offset..old_byte_end],
                    was_dirty,
                ));
                old_byte_offset = old_byte_end;
            }
            match target_page_cache.restore_prefaulted_pages(target_page_restores) {
                Ok(()) => return Err(error),
                Err(_restore_error) => {
                    let rewrite_result = {
                        let mut reader = VmReader::from(target_new_bytes.as_slice()).to_fallible();
                        target_page_cache
                            .write(target_slot_bytes.start, &mut reader)
                            .map_err(Error::from)
                    };
                    match rewrite_result {
                        Ok(()) => persist_status = Err(error),
                        Err(_) => {
                            if let Some(fs) = target_directory.fs.upgrade() {
                                fs.latch_forced_shutdown(fs_state);
                            }
                            target_image_coherent = false;
                            persist_status = Err(error);
                        }
                    }
                }
            }
        }
        if target_image_coherent {
            let target_flush_start = target_start_page
                .checked_mul(PAGE_SIZE)
                .ok_or_else(invalid_operation_input)?;
            let target_flush_end = target_end_page
                .checked_add(1)
                .and_then(|page_idx| page_idx.checked_mul(PAGE_SIZE))
                .ok_or_else(invalid_operation_input)?
                .min(target_page_cache.size());
            if let Err(error) = target_page_cache.flush_range(target_flush_start..target_flush_end) {
                if persist_status.is_ok() {
                    persist_status = Err(error);
                }
            }
        }
        let mut source_image_coherent = true;
        if target_image_coherent {
            if let Err(error) = {
                let mut reader = VmReader::from(source_new_bytes.as_slice()).to_fallible();
                source_page_cache
                    .write(source_slot_bytes.start, &mut reader)
                    .map_err(Error::from)
            } {
                let rewrite_result = {
                    let mut reader = VmReader::from(source_new_bytes.as_slice()).to_fallible();
                    source_page_cache
                        .write(source_slot_bytes.start, &mut reader)
                        .map_err(Error::from)
                };
                match rewrite_result {
                    Ok(()) => {
                        if persist_status.is_ok() {
                            persist_status = Err(error);
                        }
                    }
                    Err(_) => {
                        if let Some(fs) = self.fs.upgrade() {
                            fs.latch_forced_shutdown(fs_state);
                        }
                        source_image_coherent = false;
                        if persist_status.is_ok() {
                            persist_status = Err(error);
                        }
                    }
                }
            }
        }
        if target_image_coherent && source_image_coherent {
            let source_flush_start = source_start_page
                .checked_mul(PAGE_SIZE)
                .ok_or_else(invalid_operation_input)?;
            let source_flush_end = source_end_page
                .checked_add(1)
                .and_then(|page_idx| page_idx.checked_mul(PAGE_SIZE))
                .ok_or_else(invalid_operation_input)?
                .min(source_page_cache.size());
            if let Err(error) = source_page_cache.flush_range(source_flush_start..source_flush_end)
            {
                if persist_status.is_ok() {
                    persist_status = Err(error);
                }
            }
        }
        let finalize_error = Self::finalize_rename_protocol(
            target_directory,
            target_cluster_map,
            target_slot_range,
            source_entry_set_location_hint,
            new_source_ino,
            source_child,
            target_child,
            replacement,
            fs_state,
            allocation_guard,
            persist_status.is_ok(),
        )
        .err();
        let persist_status = match (persist_status, finalize_error) {
            (Ok(()), None) => Ok(()),
            (Ok(()), Some(error)) => Err(error),
            (Err(error), None) => Err(error),
            (Err(primary_error), Some(_finalize_error)) => Err(primary_error),
        };
        let source_refresh_result = self.refresh_directory_metadata_after_namespace_mutation_with_guards(
            fs_state,
            boot_region,
            metadata_refresh_timestamp,
            source_inode_state_guard,
            source_parent_inode_state_guard,
            prepared_source_refresh,
            true,
        );
        let target_refresh_result = target_directory.refresh_directory_metadata_after_namespace_mutation_with_guards(
            fs_state,
            boot_region,
            metadata_refresh_timestamp,
            target_inode_state_guard,
            target_parent_inode_state_guard,
            prepared_target_refresh,
            true,
        );
        match (persist_status, source_refresh_result, target_refresh_result) {
            (Ok(()), Ok(()), Ok(())) => Ok(Ok(())),
            (Err(error), _, _) => Ok(Err(error)),
            (Ok(()), Err(error), _) | (Ok(()), _, Err(error)) => Ok(Err(error)),
        }
    }

    fn finalize_rename_protocol(
        destination_directory: &ExfatInode,
        _destination_cluster_map: StreamExtensionDirEntry,
        _destination_slot_range: DirEntrySlotRange,
        source_entry_set_location_hint: Option<u64>,
        new_source_ino: u64,
        source_child: AdmittedRenameChild<'_, '_>,
        target_child: Option<AdmittedRenameChild<'_, '_>>,
        replacement: Option<ReplacedTargetCleanup>,
        fs_state: &mut FsState,
        allocation_guard: &mut AllocGuard<'_>,
        finalize_cleanup: bool,
    ) -> Result<()> {
        let old_source_ino = source_child.guard.metadata().ino;
        let replaced_target_ino = target_child.as_ref().map(|child| child.guard.metadata().ino);
        source_child
            .guard
            .set_parent(destination_directory.weak_self());
        source_child
            .guard
            .with_metadata_mut(|metadata| metadata.ino = new_source_ino);
        if let Some(source_entry_set_location_hint) = source_entry_set_location_hint {
            source_child
                .inode
                .entry_set_location_hint
                .store(source_entry_set_location_hint, Ordering::Relaxed);
        }
        let mut finalization_error = None;
        let (replaced_target_ranges, detached_regular_file_reclaim) = match replacement {
            Some(ReplacedTargetCleanup::Immediate { ranges, .. }) => (ranges, None),
            Some(ReplacedTargetCleanup::CachedGeneration {
                cluster_map, ranges, ..
            }) => (Vec::new(), Some((cluster_map, ranges))),
            None => (Vec::new(), None),
        };
        if let Some(target_child) = target_child {
            if let Err(error) = Self::detach_namespace_removed_inode(
                fs_state,
                allocation_guard,
                target_child.guard.metadata().ino,
                target_child.inode,
                target_child.guard,
                finalize_cleanup.then_some(detached_regular_file_reclaim).flatten(),
            ) {
                if finalization_error.is_none() {
                    finalization_error = Some(error);
                }
            }
        }
        ExfatFs::rebind_rename_inode_cache(
            fs_state,
            old_source_ino,
            new_source_ino,
            source_child.inode,
            replaced_target_ino,
        );
        if finalize_cleanup && finalization_error.is_none() {
            if let Err(error) = Self::cleanup_replaced_target_ranges(
                fs_state,
                allocation_guard,
                &replaced_target_ranges,
            ) {
                finalization_error = Some(error);
            }
        }
        if let Some(error) = finalization_error {
            return Err(error);
        }
        Ok(())
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
            let cluster_map_generation =
                self.cluster_map_for_write_guard(self_inode_state_guard, allocation_guard, cluster_map)?;
            let logical_end = match cluster_map.data_length {
                Some(data_length) => data_length,
                None => cluster_map_generation.allocated_byte_length(boot_region)?,
            };
            let directory_bytes = self.read_directory_snapshot_from_page_cache(
                self_inode_state_guard.metadata(),
                cluster_map_generation,
                logical_end,
            )?;
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
        let mut publication_complete = false;
        let update_result = (|| {
            Self::initialize_directory_cluster(block_device, boot_region, allocated_cluster)?;
            let (
                updated_cluster_map,
                updated_cluster_map_generation,
                updated_allocated_size,
                exposed_old_topology,
                exposure_error,
                prepared_parent_entry_set_write,
            ) = self.attach_directory_cluster(
                cluster_map,
                allocation_guard,
                self_inode_state_guard,
                block_device,
                boot_region,
                allocated_cluster,
                |updated_cluster_map| {
                    if updated_cluster_map.data_length.is_none() {
                        return Ok(None);
                    }
                    let parent_inode_state_guard = parent_inode_state_guard.ok_or_else(|| {
                        Error::with_message(
                            Errno::EINVAL,
                            "ordinary exFAT directory growth requires parent write-guard proof",
                        )
                    })?;
                    self.prepare_rewritten_entry_set_write_with_guard(
                        self_inode_state_guard,
                        parent_inode_state_guard,
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
                    )
                },
            )?;
            if exposed_old_topology {
                self.commit_directory_cluster_map(
                    self_inode_state_guard,
                    updated_cluster_map_generation.clone(),
                    updated_allocated_size,
                )?;
                allocation_guard.commit_allocation();
                publication_complete = true;
                if let Some(error) = exposure_error {
                    return Err(error);
                }
            }
            let parent_entry_set_write_result = if let Some(prepared_parent_entry_set_write) =
                prepared_parent_entry_set_write
            {
                let parent_inode_state_guard = parent_inode_state_guard.ok_or_else(|| {
                    Error::with_message(
                        Errno::EINVAL,
                        "ordinary exFAT directory growth requires parent write-guard proof",
                    )
                })?;
                let parent_inode = self_inode_state_guard.parent().ok_or_else(|| {
                    Error::with_message(
                        Errno::EIO,
                        "ordinary exFAT directory parent is not mounted",
                    )
                })?;
                if !parent_inode_state_guard.guards_inode(parent_inode.as_ref()) {
                    return Err(Error::new(Errno::EINVAL));
                }
                let entry_set_write_result = self.persist_prepared_entry_set_write_classified(
                    fs_state,
                    prepared_parent_entry_set_write,
                    parent_inode.as_ref(),
                    parent_inode_state_guard.metadata(),
                    true,
                )?;
                Some(entry_set_write_result)
            } else {
                None
            };
            if !exposed_old_topology
                && matches!(parent_entry_set_write_result.as_ref(), Some(Ok(false)))
            {
                return Err(invalid_on_disk_layout());
            }
            if !exposed_old_topology {
                self.commit_directory_cluster_map(
                    self_inode_state_guard,
                    updated_cluster_map_generation,
                    updated_allocated_size,
                )?;
                allocation_guard.commit_allocation();
                publication_complete = true;
            }
            if let Some(entry_set_write_result) = parent_entry_set_write_result {
                if !entry_set_write_result? {
                    return Err(invalid_on_disk_layout());
                }
            }
            Ok(updated_cluster_map)
        })();
        match update_result {
            Ok(updated_cluster_map) => {
                allocation_guard.commit_allocation();
                Ok(updated_cluster_map)
            }
            Err(error) => {
                if !publication_complete && allocation_guard.rollback_allocation()? {
                    ExfatFs::disable_unsupported_discard_after_release(fs_state);
                }
                Err(error)
            }
        }
    }

    fn attach_directory_cluster(
        &self,
        cluster_map: StreamExtensionDirEntry,
        allocation_guard: &AllocGuard<'_>,
        self_inode_state_guard: &InodeStateWriteGuard<'_>,
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        allocated_cluster: u32,
        prepare_parent_entry_set_write_fn: impl FnOnce(
            StreamExtensionDirEntry,
        ) -> Result<Option<(
            DirEntrySlotRange,
            Vec<u8>,
            Vec<u8>,
            Vec<(usize, bool)>,
        )>>,
    ) -> Result<(
        StreamExtensionDirEntry,
        Arc<ClusterMap>,
        usize,
        bool,
        Option<Error>,
        Option<(
            DirEntrySlotRange,
            Vec<u8>,
            Vec<u8>,
            Vec<(usize, bool)>,
        )>,
    )> {
        let next_data_length = match cluster_map.data_length {
            Some(data_length) => data_length
                .checked_add(boot_region.cluster_size)
                .ok_or(invalid_on_disk_layout())?,
            None => boot_region.cluster_size,
        };

        let admitted_cluster_map = match cluster_map.data_length {
            Some(_) => {
                self.cluster_map_for_write_guard(
                    self_inode_state_guard,
                    allocation_guard,
                    cluster_map,
                )
            }
            None => self_inode_state_guard
                .cached_cluster_map()
                .filter(|generation| generation.stream_extension() == cluster_map)
                .ok_or_else(invalid_on_disk_layout),
        }?;
        if self_inode_state_guard.dir_entry_stream() != cluster_map {
            return Err(invalid_on_disk_layout());
        }
        if cluster_map.data_length.is_none()
            && !self_inode_state_guard
                .cached_cluster_map()
                .as_ref()
                .is_some_and(|current| Arc::ptr_eq(current, &admitted_cluster_map))
        {
            return Err(invalid_on_disk_layout());
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
            None => cluster_map,
        };
        let updated_generation = Arc::new(admitted_cluster_map.appended(
            boot_region,
            updated_cluster_map,
            &[ClusterRange {
                start_cluster: allocated_cluster,
                cluster_count: 1,
            }],
        )?);
        let updated_allocated_size = updated_generation.allocated_byte_length(boot_region)?;
        let exposed_old_topology = match cluster_map.data_length {
            None => true,
            Some(data_length) => data_length != 0 && !cluster_map.no_fat_chain,
        };
        let prepared_parent_entry_set_write =
            prepare_parent_entry_set_write_fn(updated_cluster_map)?;

        let mut fat_reader = FatReader::new(block_device.as_ref(), boot_region);
        let exposure_error = match cluster_map.data_length {
            Some(0) => {
                fat_reader.terminate_cluster_chain(allocated_cluster)?;
                None
            }
            Some(data_length) if cluster_map.no_fat_chain => {
                let cluster_count = data_length.div_ceil(boot_region.cluster_size);
                fat_reader.link_contiguous_chain_to_cluster(
                    cluster_map.first_cluster,
                    cluster_count,
                    allocated_cluster,
                )?;
                None
            }
            Some(_) | None => {
                fat_reader.terminate_cluster_chain(allocated_cluster)?;
                let tail_cluster = admitted_cluster_map
                    .terminal_cluster(boot_region)?
                    .ok_or_else(invalid_on_disk_layout)?;
                fat_reader
                    .link_prepared_chain_to_tail(tail_cluster, allocated_cluster)?
                    .err()
            }
        };
        Ok((
            updated_cluster_map,
            updated_generation,
            updated_allocated_size,
            exposed_old_topology,
            exposure_error,
            prepared_parent_entry_set_write,
        ))
    }

    fn commit_directory_cluster_map(
        &self,
        self_inode_state_guard: &InodeStateWriteGuard<'_>,
        updated_cluster_map_generation: Arc<ClusterMap>,
        updated_allocated_size: usize,
    ) -> Result<()> {
        let metadata = self_inode_state_guard.metadata();
        let previous_size = metadata.size;
        let page_cache_context = self.page_cache_context_for_mapping(
            metadata,
            updated_cluster_map_generation.clone(),
            updated_allocated_size,
            updated_allocated_size,
        )?;
        let _ = self_inode_state_guard
            .replace_dir_entry_stream(updated_cluster_map_generation.stream_extension());
        self_inode_state_guard.set_cached_cluster_map(updated_cluster_map_generation);
        let _ = self_inode_state_guard.replace_page_cache_context(page_cache_context);
        self_inode_state_guard.with_metadata_mut(|metadata| {
            metadata.size = updated_allocated_size;
        });
        if let Some(page_cache) = self.page_cache.get().and_then(|page_cache| page_cache.as_ref())
        {
            page_cache.resize(updated_allocated_size, previous_size)?;
        }
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
        child_inode: &ExfatInode,
        child_inode_state_guard: &InodeStateWriteGuard<'_>,
        allocation_guard: &AllocGuard<'_>,
        boot_region: &BootRegion,
    ) -> Result<()> {
        let cluster_map = child_inode_state_guard.dir_entry_stream();
        let cluster_map_generation = child_inode.cluster_map_for_write_guard(
            child_inode_state_guard,
            allocation_guard,
            cluster_map,
        )?;
        let logical_end = match cluster_map.data_length {
            Some(data_length) => data_length,
            None => cluster_map_generation.allocated_byte_length(boot_region)?,
        };
        let child_directory_bytes = child_inode
            .read_directory_snapshot_from_page_cache(
                child_inode_state_guard.metadata(),
                cluster_map_generation,
                logical_end,
            )
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
            let (Some(child_inode), Some(child_inode_state_guard)) =
                (target_child_inode, target_child_inode_state_guard)
            else {
                return Err(Error::from(invalid_on_disk_layout()));
            };
            Self::ensure_directory_snapshot_is_empty(
                child_inode.as_ref(),
                child_inode_state_guard,
                allocation_guard,
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
