// SPDX-License-Identifier: MPL-2.0

//! Implements directory namespace mutations and directory cluster-map growth.
//!
//! Method groups: create/unlink/rmdir/rename entry points, slot management, directory growth,
//! emptiness validation, cluster-range collection, and rename-stage helpers.

mod admission;
mod growth;
mod retirement;
mod slots;
mod validation;

use aster_block::BlockDevice;
use ostd::mm::VmIo;

use self::{
    admission::{AdmittedRenameChild, FinalRenameAdmission, RenameNames},
    retirement::ReplacedTargetCleanup,
};
use super::{
    super::{
        bitmap::ClusterRange,
        boot::BootRegion,
        dir_entry_format::{self as direntry, DIRECTORY_ENTRY_SIZE, FileEntrySetView},
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
        let mount_state = fs_state
            .mount_state
            .as_ref()
            .ok_or_else(super::super::not_mounted)?;
        if mount_state.forced_shutdown {
            return_errno!(Errno::EIO);
        }
        let block_device = fs.immutable_block_device();
        let boot_region = fs.immutable_boot_region();
        let upcase_table = fs_state
            .upcase_table
            .as_ref()
            .ok_or_else(super::super::not_mounted)?
            .clone();
        let options = mount_state.options.clone();
        if options.fs_flags.contains(FsFlags::RDONLY) {
            return_errno!(Errno::EROFS);
        }

        let name = Self::validate_name(name, &options)?;
        let name_hash = upcase_table.name_hash(&name);
        let required_entry_count = direntry::file_entry_set_entry_count(name.len())?;
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
            let cluster_map_generation = self.cluster_map_for_write_guard(
                self_inode_state_guard,
                &allocation_guard,
                cluster_map,
            )?;
            let logical_end = match cluster_map.data_length {
                Some(data_length) => data_length,
                None => cluster_map_generation.allocated_byte_length(&boot_region)?,
            };
            let current_directory_bytes = self.read_directory_snapshot_from_page_cache(
                self_inode_state_guard.metadata(),
                cluster_map_generation,
                logical_end,
            )?;
            if Self::locate_named_child_view(
                &current_directory_bytes,
                cluster_map.data_length.is_none(),
                &upcase_table,
                &name,
                name_hash,
            )?
            .is_some()
            {
                return_errno!(Errno::EEXIST);
            }
            fs.publish_dirty_admission(&mut fs_state)?;
            let now = RealTimeCoarseClock::get().read_time();
            let (timestamp_bytes, ten_ms_increment, encoded_utc_offset_byte) =
                Self::encoded_exfat_timestamp_fields(now, 0)?;
            let normalized_access_timestamp =
                Self::decoded_exfat_timestamp(timestamp_bytes, None, encoded_utc_offset_byte)?;
            let normalized_modify_timestamp = Self::decoded_exfat_timestamp(
                timestamp_bytes,
                Some(ten_ms_increment),
                encoded_utc_offset_byte,
            )?;
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
                )?;
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
                self.prepare_directory_metadata_refresh_with_guards(
                    self_inode_state_guard,
                    parent_inode_state_guard,
                    &boot_region,
                    metadata_refresh_timestamp,
                )?
            };
            let slot_range_bytes = direntry::slot_range_bytes(slot_range)?;
            let child_ino = self.entry_location_ino(cluster_map, slot_range.first_entry_index())?;
            let (first_cluster, data_length, no_fat_chain, child_cluster_map) =
                if type_ == InodeType::Dir && !options.zero_size_dir {
                    allocation_guard.allocate(1, None)?;
                    let allocated_cluster = allocation_guard.single_cluster()?;
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
                            .ok_or(invalid_on_disk_layout())?
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
                    (
                        allocated_cluster,
                        boot_region.cluster_size,
                        true,
                        child_cluster_map,
                    )
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
                    )?;
                    let byte_mutations = vec![(
                        slot_range_bytes.clone(),
                        current_directory_bytes
                            .get(slot_range_bytes.clone())
                            .ok_or(invalid_on_disk_layout())?
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
            let child_stream = StreamExtensionDirEntry {
                data_length: Some(data_length),
                first_cluster,
                valid_data_length: Some(data_length),
                no_fat_chain,
            };
            let child_inode = Self::new_child(
                &fs,
                self.weak_self(),
                child_ino,
                type_,
                child_size,
                child_stream,
                child_cluster_map,
            );
            if type_ == InodeType::File {
                child_inode.store_entry_set_location_hint(slot_range)?;
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
            let metadata_refresh_result = self
                .refresh_directory_metadata_after_namespace_mutation_with_guards(
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
        let mount_state = fs_state
            .mount_state
            .as_ref()
            .ok_or_else(super::super::not_mounted)?;
        if mount_state.forced_shutdown {
            return_errno!(Errno::EIO);
        }
        let block_device = fs.immutable_block_device();
        let boot_region = fs.immutable_boot_region();
        let upcase_table = fs_state
            .upcase_table
            .as_ref()
            .ok_or_else(super::super::not_mounted)?
            .clone();
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
            let directory_bytes = self.read_directory_snapshot_from_page_cache(
                provisional_directory_guard.metadata(),
                cluster_map_generation,
                logical_end,
            )?;
            let Some(entry_view) = Self::locate_named_child_view(
                &directory_bytes,
                is_root_directory,
                &upcase_table,
                &name,
                lookup_name_hash,
            )?
            else {
                return_errno!(Errno::ENOENT);
            };
            let slot_range = entry_view.slot_range();
            let child_ino = self.entry_location_ino(cluster_map, slot_range.first_entry_index())?;
            let (inode_type, _first_cluster, _data_length, _no_fat_chain) =
                entry_view.child_metadata(&boot_region)?;
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
            let cluster_map_generation = self.cluster_map_for_write_guard(
                self_inode_state_guard,
                &allocation_guard,
                cluster_map,
            )?;
            let logical_end = match cluster_map.data_length {
                Some(data_length) => data_length,
                None => cluster_map_generation.allocated_byte_length(&boot_region)?,
            };
            let directory_bytes = self.read_directory_snapshot_from_page_cache(
                self_inode_state_guard.metadata(),
                cluster_map_generation,
                logical_end,
            )?;
            let Some(entry_view) = Self::locate_named_child_view(
                &directory_bytes,
                is_root_directory,
                &upcase_table,
                &name,
                lookup_name_hash,
            )?
            else {
                return_errno!(Errno::ENOENT);
            };
            let slot_range = entry_view.slot_range();
            let (inode_type, first_cluster, data_length, no_fat_chain) =
                entry_view.child_metadata(&boot_region)?;
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
                )?
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
                self.prepare_directory_metadata_refresh_with_guards(
                    self_inode_state_guard,
                    parent_inode_state_guard,
                    &boot_region,
                    metadata_refresh_timestamp,
                )?
            };

            fs.publish_dirty_admission(&mut fs_state)?;
            let byte_mutations = vec![Self::prepare_invalidated_slot_mutation(
                &directory_bytes,
                slot_range,
            )?];
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
            let metadata_refresh_result = self
                .refresh_directory_metadata_after_namespace_mutation_with_guards(
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
        let mount_state = fs_state
            .mount_state
            .as_ref()
            .ok_or_else(super::super::not_mounted)?;
        if mount_state.forced_shutdown {
            return_errno!(Errno::EIO);
        }
        let block_device = fs.immutable_block_device();
        let boot_region = fs.immutable_boot_region();
        let upcase_table = fs_state
            .upcase_table
            .as_ref()
            .ok_or_else(super::super::not_mounted)?
            .clone();
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
            let directory_bytes = self.read_directory_snapshot_from_page_cache(
                provisional_directory_guard.metadata(),
                cluster_map_generation,
                logical_end,
            )?;
            let Some(entry_view) = Self::locate_named_child_view(
                &directory_bytes,
                cluster_map.data_length.is_none(),
                &upcase_table,
                &name,
                lookup_name_hash,
            )?
            else {
                return_errno!(Errno::ENOENT);
            };
            let slot_range = entry_view.slot_range();
            let (inode_type, first_cluster, data_length, no_fat_chain) =
                entry_view.child_metadata(&boot_region)?;
            if inode_type != InodeType::Dir {
                return_errno!(Errno::ENOTDIR);
            }

            let child_ino = self.entry_location_ino(cluster_map, slot_range.first_entry_index())?;
            let child_inode =
                if let Some(cached_inode) = ExfatFs::peek_cached_inode(&fs_state, child_ino) {
                    cached_inode
                } else {
                    Self::child_inode_from_directory_entry(
                        self,
                        &fs,
                        &boot_region,
                        cluster_map.first_cluster,
                        slot_range,
                        inode_type,
                        StreamExtensionDirEntry {
                            data_length: Some(data_length),
                            first_cluster,
                            valid_data_length: Some(data_length),
                            no_fat_chain,
                        },
                    )?
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
                self.prepare_directory_metadata_refresh_with_guards(
                    self_inode_state_guard,
                    parent_inode_state_guard,
                    &boot_region,
                    metadata_refresh_timestamp,
                )?
            };
            let cluster_map_generation = self.cluster_map_for_write_guard(
                self_inode_state_guard,
                &allocation_guard,
                cluster_map,
            )?;
            let logical_end = match cluster_map.data_length {
                Some(data_length) => data_length,
                None => cluster_map_generation.allocated_byte_length(&boot_region)?,
            };
            let directory_bytes = self.read_directory_snapshot_from_page_cache(
                self_inode_state_guard.metadata(),
                cluster_map_generation,
                logical_end,
            )?;
            let Some(entry_view) = Self::locate_named_child_view(
                &directory_bytes,
                cluster_map.data_length.is_none(),
                &upcase_table,
                &name,
                lookup_name_hash,
            )?
            else {
                return_errno!(Errno::ENOENT);
            };
            let slot_range = entry_view.slot_range();
            let (inode_type, _first_cluster, _data_length, _no_fat_chain) =
                entry_view.child_metadata(&boot_region)?;
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
                return Err(invalid_on_disk_layout());
            };
            let allocated_cluster_ranges = Self::allocated_cluster_ranges(
                &block_device,
                &boot_region,
                child_cluster_map.first_cluster,
                child_data_length,
                child_cluster_map.no_fat_chain,
            )?;
            fs.publish_dirty_admission(&mut fs_state)?;
            let byte_mutations = vec![Self::prepare_invalidated_slot_mutation(
                &directory_bytes,
                slot_range,
            )?];
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
            let metadata_refresh_result = self
                .refresh_directory_metadata_after_namespace_mutation_with_guards(
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
        let mount_state = fs_state
            .mount_state
            .as_ref()
            .ok_or_else(super::super::not_mounted)?;
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
        let upcase_table = fs_state
            .upcase_table
            .as_ref()
            .ok_or_else(super::super::not_mounted)?
            .clone();
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
                    renamed?;
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
        let cluster_map_generation = self.cluster_map_for_write_guard(
            self_inode_state_guard,
            allocation_guard,
            cluster_map,
        )?;
        let logical_end = match cluster_map.data_length {
            Some(data_length) => data_length,
            None => cluster_map_generation.allocated_byte_length(boot_region)?,
        };
        let current_directory_bytes = self.read_directory_snapshot_from_page_cache(
            self_inode_state_guard.metadata(),
            cluster_map_generation,
            logical_end,
        )?;
        let current_source_view = Self::lookup_rename_source_view(
            &current_directory_bytes,
            cluster_map,
            upcase_table,
            old_name,
            old_name_hash,
        )?;
        let source_name = current_source_view.name()?;
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
            direntry::renamed_entry_set(current_source_view, new_name, new_name_hash)?;
        let required_entry_count = current_renamed_entry_set.len() / DIRECTORY_ENTRY_SIZE;

        let (source_inode_type, _, _, _) = current_source_view.child_metadata(boot_region)?;
        let replacement = Self::collect_replaced_target_cleanup(
            current_target_view,
            target_child_inode,
            target_child_inode_state_guard,
            source_inode_type,
            block_device,
            boot_region,
            allocation_guard,
        )?;
        let replaced_target_slot_range =
            replacement.as_ref().map(|replacement| match replacement {
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
                direntry::renamed_entry_set(latest_source_view, new_name, new_name_hash)?;
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
            self.prepare_directory_metadata_refresh_with_guards(
                self_inode_state_guard,
                parent_inode_state_guard,
                boot_region,
                metadata_refresh_timestamp,
            )?
        };
        let new_source_ino =
            self.entry_location_ino(cluster_map, final_slot_range.first_entry_index())?;
        let mut byte_mutations = vec![Self::prepare_renamed_slot_mutation(
            &renamed_directory_bytes,
            final_slot_range,
            &renamed_entry_set,
        )?];
        if final_slot_range != source_slot_range {
            byte_mutations.push(Self::prepare_invalidated_slot_mutation(
                &renamed_directory_bytes,
                source_slot_range,
            )?);
        }
        if let Some(replaced_slot_range) =
            replaced_target_slot_range.filter(|slot_range| *slot_range != final_slot_range)
        {
            byte_mutations.push(Self::prepare_invalidated_slot_mutation(
                &renamed_directory_bytes,
                replaced_slot_range,
            )?);
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
            final_slot_range,
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
        let metadata_refresh_result = self
            .refresh_directory_metadata_after_namespace_mutation_with_guards(
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
        let source_cluster_map_generation = self.cluster_map_for_write_guard(
            source_inode_state_guard,
            allocation_guard,
            source_cluster_map,
        )?;
        let source_logical_end = match source_cluster_map.data_length {
            Some(data_length) => data_length,
            None => source_cluster_map_generation.allocated_byte_length(boot_region)?,
        };
        let source_directory_bytes = self.read_directory_snapshot_from_page_cache(
            source_inode_state_guard.metadata(),
            source_cluster_map_generation,
            source_logical_end,
        )?;
        let source_view = Self::lookup_rename_source_view(
            &source_directory_bytes,
            source_cluster_map,
            upcase_table,
            old_name,
            old_name_hash,
        )?;
        let source_slot_range = source_view.slot_range();
        let (source_inode_type, _, _, _) = source_view.child_metadata(boot_region)?;
        let renamed_entry_set = direntry::renamed_entry_set(source_view, new_name, new_name_hash)?;
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
        let target_directory_bytes = target_directory.read_directory_snapshot_from_page_cache(
            target_inode_state_guard.metadata(),
            target_cluster_map_generation,
            target_logical_end,
        )?;
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
        let replaced_target_slot_range =
            replacement.as_ref().map(|replacement| match replacement {
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
            let source_parent_inode_state_guard =
                source_parent_inode_state_guard.ok_or_else(|| {
                    Error::with_message(
                        Errno::EINVAL,
                        "ordinary exFAT directory refresh requires parent write-guard proof",
                    )
                })?;
            self.prepare_directory_metadata_refresh_with_guards(
                source_inode_state_guard,
                source_parent_inode_state_guard,
                boot_region,
                metadata_refresh_timestamp,
            )?
        };
        let prepared_target_refresh = if target_cluster_map.data_length.is_none() {
            None
        } else {
            let target_parent_inode_state_guard =
                target_parent_inode_state_guard.ok_or_else(|| {
                    Error::with_message(
                        Errno::EINVAL,
                        "ordinary exFAT directory refresh requires parent write-guard proof",
                    )
                })?;
            target_directory.prepare_directory_metadata_refresh_with_guards(
                target_inode_state_guard,
                target_parent_inode_state_guard,
                boot_region,
                metadata_refresh_timestamp,
            )?
        };
        let new_source_ino = target_directory
            .entry_location_ino(target_cluster_map, target_slot_range.first_entry_index())?;
        let (target_slot_bytes, target_old_bytes, target_new_bytes) =
            Self::prepare_renamed_slot_mutation(
                &target_directory_bytes,
                target_slot_range,
                &renamed_entry_set,
            )?;
        let (source_slot_bytes, source_old_bytes, source_new_bytes) =
            Self::prepare_invalidated_slot_mutation(&source_directory_bytes, source_slot_range)?;
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
                Ok((
                    page_idx,
                    target_page_cache.has_dirty_pages(page_start..page_end),
                ))
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
            if let Err(error) = target_page_cache.flush_range(target_flush_start..target_flush_end)
                && persist_status.is_ok()
            {
                persist_status = Err(error);
            }
        }
        let mut source_image_coherent = true;
        if target_image_coherent
            && let Err(error) = {
                let mut reader = VmReader::from(source_new_bytes.as_slice()).to_fallible();
                source_page_cache
                    .write(source_slot_bytes.start, &mut reader)
                    .map_err(Error::from)
            }
        {
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
                && persist_status.is_ok()
            {
                persist_status = Err(error);
            }
        }
        let finalize_error = Self::finalize_rename_protocol(
            target_directory,
            target_slot_range,
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
        let source_refresh_result = self
            .refresh_directory_metadata_after_namespace_mutation_with_guards(
                fs_state,
                boot_region,
                metadata_refresh_timestamp,
                source_inode_state_guard,
                source_parent_inode_state_guard,
                prepared_source_refresh,
                true,
            );
        let target_refresh_result = target_directory
            .refresh_directory_metadata_after_namespace_mutation_with_guards(
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

    // Slot management
}
