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
        fs::ClusterAllocGuard,
        invalid_on_disk_layout, invalid_operation_input,
    },
    ExfatFs, ExfatInode, MountedVolumeState, StreamExtensionDirEntry, UpcaseTable,
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
        let mut mount_guard = self.mount_access_write_guard(&fs)?;
        if mount_guard.forced_shutdown() {
            return_errno!(Errno::EIO);
        }
        let block_device = mount_guard.block_device();
        let boot_region = mount_guard.boot_region();
        let upcase_table = mount_guard.upcase_table();
        let options = mount_guard.options();
        if options.fs_flags.contains(FsFlags::RDONLY) {
            return_errno!(Errno::EROFS);
        }

        let name = Self::validate_name(name, &options)?;
        let name_hash = upcase_table.name_hash(&name);
        let required_entry_count =
            direntry::file_entry_set_entry_count(name.len()).map_err(Error::from)?;
        let create_result = (|| {
            let parent_directory = self.parent.read().upgrade();
            let mut guarded_directories = vec![self];
            if let Some(parent_directory) = parent_directory.as_ref() {
                guarded_directories.push(parent_directory.as_ref());
            }
            let mut guarded_directory_inos = guarded_directories
                .iter()
                .map(|directory| directory.metadata.read().ino)
                .collect::<Vec<_>>();
            guarded_directory_inos.sort_unstable();
            guarded_directory_inos.dedup();
            let directory_guards = Self::directory_write_guards_by_ino(guarded_directories);
            let self_guard_index = guarded_directory_inos
                .binary_search(&self.metadata.read().ino)
                .map_err(|_| Error::new(Errno::EINVAL))?;
            let self_inode_state_guard = &directory_guards[self_guard_index];
            let parent_inode_state_guard = if let Some(parent_directory) = parent_directory.as_ref()
            {
                let parent_guard_index = guarded_directory_inos
                    .binary_search(&parent_directory.metadata.read().ino)
                    .map_err(|_| Error::new(Errno::EINVAL))?;
                Some(&directory_guards[parent_guard_index])
            } else {
                None
            };
            let cluster_map = *self.dir_entry_stream.read();
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

            let mount_state = mount_guard.mount_state_mut()?;
            fs.publish_dirty_admission(mount_state)?;
            let (cluster_map, mut directory_bytes, slot_range) = self
                .reserve_directory_entry_slots(
                    cluster_map,
                    mount_state,
                    &fs,
                    &block_device,
                    &boot_region,
                    required_entry_count,
                )
                .map_err(Error::from)?;

            let (first_cluster, data_length, no_fat_chain) =
                if type_ == InodeType::Dir && !options.zero_size_dir {
                    let allocated_directory_cluster =
                        ClusterAllocGuard::allocate(&fs, mount_state, 1).map_err(Error::from)?;
                    let allocated_cluster = allocated_directory_cluster
                        .single_cluster()
                        .map_err(Error::from)?;
                    if let Err(error) = Self::initialize_directory_cluster(
                        &block_device,
                        &boot_region,
                        allocated_cluster,
                    ) {
                        return Err(Error::from(error));
                    }
                    let entry_set = direntry::encode_file_entry_set(
                        &name,
                        name_hash,
                        type_,
                        allocated_cluster,
                        boot_region.cluster_size,
                        true,
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
                    allocated_directory_cluster.commit();
                    (allocated_cluster, boot_region.cluster_size, true)
                } else {
                    let entry_set =
                        direntry::encode_file_entry_set(&name, name_hash, type_, 0, 0, false)
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
            let child_inode = Self::new_child(
                &fs,
                self.weak_self(),
                self.entry_location_ino(slot_range.first_entry_index())
                    .map_err(Error::from)?,
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
                    .store_regular_file_entry_set_location_hint(slot_range)
                    .map_err(Error::from)?;
            }
            child_inode.metadata.write().mode = mode;
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
            if let Ok(mount_state) = mount_guard.mount_state_mut() {
                mount_state.volume_flags.volume_dirty = true;
                mount_state.dirty_bracket_opened_by_mount = false;
            }
        }
        create_result
    }

    pub(super) fn unlink_impl(&self, name: &str) -> Result<()> {
        let fs = self
            .fs
            .upgrade()
            .ok_or_else(|| Error::with_message(Errno::EIO, "exFAT filesystem is not mounted"))?;
        let mut mount_guard = self.mount_access_write_guard(&fs)?;
        if mount_guard.forced_shutdown() {
            return_errno!(Errno::EIO);
        }
        let block_device = mount_guard.block_device();
        let boot_region = mount_guard.boot_region();
        let upcase_table = mount_guard.upcase_table();
        let options = mount_guard.options();
        if options.fs_flags.contains(FsFlags::RDONLY) {
            return_errno!(Errno::EROFS);
        }

        let name = Self::validate_name(name, &options)?;
        let lookup_name_hash = upcase_table.name_hash(&name);
        let unlink_result = (|| {
            let parent_directory = self.parent.read().upgrade();
            let mut guarded_directories = vec![self];
            if let Some(parent_directory) = parent_directory.as_ref() {
                guarded_directories.push(parent_directory.as_ref());
            }
            let mut guarded_directory_inos = guarded_directories
                .iter()
                .map(|directory| directory.metadata.read().ino)
                .collect::<Vec<_>>();
            guarded_directory_inos.sort_unstable();
            guarded_directory_inos.dedup();
            let directory_guards = Self::directory_write_guards_by_ino(guarded_directories);
            let self_guard_index = guarded_directory_inos
                .binary_search(&self.metadata.read().ino)
                .map_err(|_| Error::new(Errno::EINVAL))?;
            let self_inode_state_guard = &directory_guards[self_guard_index];
            let parent_inode_state_guard = if let Some(parent_directory) = parent_directory.as_ref()
            {
                let parent_guard_index = guarded_directory_inos
                    .binary_search(&parent_directory.metadata.read().ino)
                    .map_err(|_| Error::new(Errno::EINVAL))?;
                Some(&directory_guards[parent_guard_index])
            } else {
                None
            };
            let cluster_map = *self.dir_entry_stream.read();
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

            let allocated_cluster_ranges = Self::allocated_cluster_ranges(
                &block_device,
                &boot_region,
                first_cluster,
                data_length,
                no_fat_chain,
            )
            .map_err(Error::from)?;

            let mount_state = mount_guard.mount_state_mut()?;
            fs.publish_dirty_admission(mount_state)?;
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
            if !allocated_cluster_ranges.is_empty() {
                let mount_state = mount_guard.mount_state_mut()?;
                let _ = fs.free_clusters(mount_state, &allocated_cluster_ranges);
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
            if let Ok(mount_state) = mount_guard.mount_state_mut() {
                mount_state.volume_flags.volume_dirty = true;
                mount_state.dirty_bracket_opened_by_mount = false;
            }
        }
        unlink_result
    }

    pub(super) fn rmdir_impl(&self, name: &str) -> Result<()> {
        let fs = self
            .fs
            .upgrade()
            .ok_or_else(|| Error::with_message(Errno::EIO, "exFAT filesystem is not mounted"))?;
        let mut mount_guard = self.mount_access_write_guard(&fs)?;
        if mount_guard.forced_shutdown() {
            return_errno!(Errno::EIO);
        }
        let block_device = mount_guard.block_device();
        let boot_region = mount_guard.boot_region();
        let upcase_table = mount_guard.upcase_table();
        let options = mount_guard.options();
        if options.fs_flags.contains(FsFlags::RDONLY) {
            return_errno!(Errno::EROFS);
        }

        let name = Self::validate_name(name, &options)?;
        let lookup_name_hash = upcase_table.name_hash(&name);
        let rmdir_result = (|| {
            let parent_directory = self.parent.read().upgrade();
            let mut guarded_directories = vec![self];
            if let Some(parent_directory) = parent_directory.as_ref() {
                guarded_directories.push(parent_directory.as_ref());
            }
            let mut guarded_directory_inos = guarded_directories
                .iter()
                .map(|directory| directory.metadata.read().ino)
                .collect::<Vec<_>>();
            guarded_directory_inos.sort_unstable();
            guarded_directory_inos.dedup();
            let directory_guards = Self::directory_write_guards_by_ino(guarded_directories);
            let self_guard_index = guarded_directory_inos
                .binary_search(&self.metadata.read().ino)
                .map_err(|_| Error::new(Errno::EINVAL))?;
            let self_inode_state_guard = &directory_guards[self_guard_index];
            let parent_inode_state_guard = if let Some(parent_directory) = parent_directory.as_ref()
            {
                let parent_guard_index = guarded_directory_inos
                    .binary_search(&parent_directory.metadata.read().ino)
                    .map_err(|_| Error::new(Errno::EINVAL))?;
                Some(&directory_guards[parent_guard_index])
            } else {
                None
            };
            let cluster_map = *self.dir_entry_stream.read();
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

            let child_inode = Self::child_inode_from_directory_entry(
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
            .map_err(Error::from)?;
            Self::ensure_directory_entry_is_empty(&child_inode, &block_device, &boot_region)?;

            let allocated_cluster_ranges = Self::allocated_cluster_ranges(
                &block_device,
                &boot_region,
                first_cluster,
                data_length,
                no_fat_chain,
            )
            .map_err(Error::from)?;

            let mount_state = mount_guard.mount_state_mut()?;
            fs.publish_dirty_admission(mount_state)?;
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
            if !allocated_cluster_ranges.is_empty() {
                let mount_state = mount_guard.mount_state_mut()?;
                let _ = fs.free_clusters(mount_state, &allocated_cluster_ranges);
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
            if let Ok(mount_state) = mount_guard.mount_state_mut() {
                mount_state.volume_flags.volume_dirty = true;
                mount_state.dirty_bracket_opened_by_mount = false;
            }
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
        if self.type_() != InodeType::Dir || target_directory.type_() != InodeType::Dir {
            return_errno!(Errno::ENOTDIR);
        }

        let fs = self
            .fs
            .upgrade()
            .ok_or_else(|| Error::with_message(Errno::EIO, "exFAT filesystem is not mounted"))?;
        let mut mount_guard = self.mount_access_write_guard(&fs)?;
        let target_fs = target_directory
            .fs
            .upgrade()
            .ok_or_else(|| Error::with_message(Errno::EIO, "exFAT filesystem is not mounted"))?;
        if !Arc::ptr_eq(&fs, &target_fs) {
            return_errno!(Errno::EXDEV);
        }
        if mount_guard.forced_shutdown() {
            return_errno!(Errno::EIO);
        }
        let block_device = mount_guard.block_device();
        let boot_region = mount_guard.boot_region();
        let upcase_table = mount_guard.upcase_table();
        let options = mount_guard.options();
        if options.fs_flags.contains(FsFlags::RDONLY) {
            return_errno!(Errno::EROFS);
        }

        let old_name = Self::validate_name(old_name, &options)?;
        let new_name = Self::validate_name(new_name, &options)?;
        let old_name_hash = upcase_table.name_hash(&old_name);
        let new_name_hash = upcase_table.name_hash(&new_name);
        let rename_result = (|| {
            if self.metadata.read().ino == target_directory.metadata.read().ino {
                let parent_directory = self.parent.read().upgrade();
                let mut guarded_directories = vec![self];
                if let Some(parent_directory) = parent_directory.as_ref() {
                    guarded_directories.push(parent_directory.as_ref());
                }
                let mut guarded_directory_inos = guarded_directories
                    .iter()
                    .map(|directory| directory.metadata.read().ino)
                    .collect::<Vec<_>>();
                guarded_directory_inos.sort_unstable();
                guarded_directory_inos.dedup();
                let directory_guards = Self::directory_write_guards_by_ino(guarded_directories);
                let self_guard_index = guarded_directory_inos
                    .binary_search(&self.metadata.read().ino)
                    .map_err(|_| Error::new(Errno::EINVAL))?;
                let self_inode_state_guard = &directory_guards[self_guard_index];
                let parent_inode_state_guard = if let Some(parent_directory) = parent_directory.as_ref()
                {
                    let parent_guard_index = guarded_directory_inos
                        .binary_search(&parent_directory.metadata.read().ino)
                        .map_err(|_| Error::new(Errno::EINVAL))?;
                    Some(&directory_guards[parent_guard_index])
                } else {
                    None
                };
                let cluster_map = *self.dir_entry_stream.read();
                let directory_bytes = Self::read_directory_bytes_for_cluster_map(
                    &block_device,
                    &boot_region,
                    cluster_map,
                )
                .map_err(Error::from)?;
                let Some(source_view) = Self::locate_named_child_view(
                    &directory_bytes,
                    cluster_map.data_length.is_none(),
                    &upcase_table,
                    &old_name,
                    old_name_hash,
                )
                .map_err(Error::from)?
                else {
                    return_errno!(Errno::ENOENT);
                };
                let (
                    source_inode_type,
                    source_first_cluster,
                    source_data_length,
                    source_no_fat_chain,
                ) = source_view.child_metadata(&boot_region).map_err(Error::from)?;
                let source_valid_data_length = source_view
                    .cluster_map()
                    .map_err(Error::from)?
                    .valid_data_length
                    .ok_or_else(invalid_on_disk_layout)
                    .map_err(Error::from)?;
                let source_child_inode = Self::child_inode_from_directory_entry(
                    self,
                    &fs,
                    &boot_region,
                    cluster_map.first_cluster,
                    source_view.slot_range(),
                    source_inode_type,
                    source_first_cluster,
                    source_data_length,
                    source_valid_data_length,
                    source_no_fat_chain,
                )
                .map_err(Error::from)?;
                let target_child_inode = Self::locate_named_child_view(
                    &directory_bytes,
                    cluster_map.data_length.is_none(),
                    &upcase_table,
                    &new_name,
                    new_name_hash,
                )
                .map_err(Error::from)?
                .filter(|target_view| target_view.slot_range() != source_view.slot_range())
                .map(|target_view| {
                    let (target_inode_type, first_cluster, data_length, no_fat_chain) =
                        target_view.child_metadata(&boot_region)?;
                    let valid_data_length = target_view
                        .cluster_map()?
                        .valid_data_length
                        .ok_or_else(invalid_on_disk_layout)?;
                    Self::child_inode_from_directory_entry(
                        self,
                        &fs,
                        &boot_region,
                        cluster_map.first_cluster,
                        target_view.slot_range(),
                        target_inode_type,
                        first_cluster,
                        data_length,
                        valid_data_length,
                        no_fat_chain,
                    )
                    .map(Some)
                })
                .transpose()
                .map_err(Error::from)?
                .flatten();
                let mount_state = mount_guard.mount_state_mut()?;
                let cluster_map = *self.dir_entry_stream.read();
                let renamed = self.rename_within_directory(
                    cluster_map,
                    &source_child_inode,
                    target_child_inode.as_ref(),
                    mount_state,
                    &fs,
                    &block_device,
                    &boot_region,
                    &upcase_table,
                    &old_name,
                    old_name_hash,
                    &new_name,
                    new_name_hash,
                )?;
                if renamed {
                    self.refresh_directory_metadata_after_namespace_mutation_with_guards(
                        &block_device,
                        &boot_region,
                        RealTimeCoarseClock::get().read_time(),
                        self_inode_state_guard,
                        parent_inode_state_guard,
                    )?;
                }
                return Ok(());
            }

            let source_parent_directory = self.parent.read().upgrade();
            let target_parent_directory = target_directory.parent.read().upgrade();
            let mut guarded_directories = vec![self, target_directory];
            if let Some(source_parent_directory) = source_parent_directory.as_ref() {
                guarded_directories.push(source_parent_directory.as_ref());
            }
            if let Some(target_parent_directory) = target_parent_directory.as_ref() {
                guarded_directories.push(target_parent_directory.as_ref());
            }
            let mut guarded_directory_inos = guarded_directories
                .iter()
                .map(|directory| directory.metadata.read().ino)
                .collect::<Vec<_>>();
            guarded_directory_inos.sort_unstable();
            guarded_directory_inos.dedup();
            let directory_guards = Self::directory_write_guards_by_ino(guarded_directories);
            let source_guard_index = guarded_directory_inos
                .binary_search(&self.metadata.read().ino)
                .map_err(|_| Error::new(Errno::EINVAL))?;
            let source_inode_state_guard = &directory_guards[source_guard_index];
            let target_guard_index = guarded_directory_inos
                .binary_search(&target_directory.metadata.read().ino)
                .map_err(|_| Error::new(Errno::EINVAL))?;
            let target_inode_state_guard = &directory_guards[target_guard_index];
            let source_parent_inode_state_guard =
                if let Some(source_parent_directory) = source_parent_directory.as_ref() {
                    let parent_guard_index = guarded_directory_inos
                        .binary_search(&source_parent_directory.metadata.read().ino)
                        .map_err(|_| Error::new(Errno::EINVAL))?;
                    Some(&directory_guards[parent_guard_index])
                } else {
                    None
                };
            let target_parent_inode_state_guard =
                if let Some(target_parent_directory) = target_parent_directory.as_ref() {
                    let parent_guard_index = guarded_directory_inos
                        .binary_search(&target_parent_directory.metadata.read().ino)
                        .map_err(|_| Error::new(Errno::EINVAL))?;
                    Some(&directory_guards[parent_guard_index])
                } else {
                    None
                };
            let source_cluster_map = *self.dir_entry_stream.read();
            let source_directory_bytes = Self::read_directory_bytes_for_cluster_map(
                &block_device,
                &boot_region,
                source_cluster_map,
            )
            .map_err(Error::from)?;
            let source_view = Self::locate_named_child_view(
                &source_directory_bytes,
                source_cluster_map.data_length.is_none(),
                &upcase_table,
                &old_name,
                old_name_hash,
            )
            .map_err(Error::from)?
            .ok_or_else(|| Error::new(Errno::ENOENT))?;
            let (
                source_inode_type,
                source_first_cluster,
                source_data_length,
                source_no_fat_chain,
            ) = source_view.child_metadata(&boot_region).map_err(Error::from)?;
            let source_valid_data_length = source_view
                .cluster_map()
                .map_err(Error::from)?
                .valid_data_length
                .ok_or_else(invalid_on_disk_layout)
                .map_err(Error::from)?;
            let source_child_inode = Self::child_inode_from_directory_entry(
                self,
                &fs,
                &boot_region,
                source_cluster_map.first_cluster,
                source_view.slot_range(),
                source_inode_type,
                source_first_cluster,
                source_data_length,
                source_valid_data_length,
                source_no_fat_chain,
            )
            .map_err(Error::from)?;
            let target_cluster_map = *target_directory.dir_entry_stream.read();
            let target_directory_bytes = Self::read_directory_bytes_for_cluster_map(
                &block_device,
                &boot_region,
                target_cluster_map,
            )
            .map_err(Error::from)?;
            let target_child_inode = Self::locate_named_child_view(
                &target_directory_bytes,
                target_cluster_map.data_length.is_none(),
                &upcase_table,
                &new_name,
                new_name_hash,
            )
            .map_err(Error::from)?
            .map(|target_view| {
                let (target_inode_type, first_cluster, data_length, no_fat_chain) =
                    target_view.child_metadata(&boot_region)?;
                let valid_data_length = target_view
                    .cluster_map()?
                    .valid_data_length
                    .ok_or_else(invalid_on_disk_layout)?;
                Self::child_inode_from_directory_entry(
                    target_directory,
                    &fs,
                    &boot_region,
                    target_cluster_map.first_cluster,
                    target_view.slot_range(),
                    target_inode_type,
                    first_cluster,
                    data_length,
                    valid_data_length,
                    no_fat_chain,
                )
                .map(Some)
            })
            .transpose()
            .map_err(Error::from)?
            .flatten();
            let mount_state = mount_guard.mount_state_mut()?;
            let target_cluster_map = *target_directory.dir_entry_stream.read();
            self.rename_across_directories(
                source_cluster_map,
                &source_child_inode,
                target_directory,
                target_cluster_map,
                target_child_inode.as_ref(),
                mount_state,
                &fs,
                &block_device,
                &boot_region,
                &upcase_table,
                &old_name,
                old_name_hash,
                &new_name,
                new_name_hash,
            )?;
            let timestamp = RealTimeCoarseClock::get().read_time();
            self.refresh_directory_metadata_after_namespace_mutation_with_guards(
                &block_device,
                &boot_region,
                timestamp,
                source_inode_state_guard,
                source_parent_inode_state_guard,
            )?;
            target_directory
                .refresh_directory_metadata_after_namespace_mutation_with_guards(
                    &block_device,
                    &boot_region,
                    timestamp,
                    target_inode_state_guard,
                    target_parent_inode_state_guard,
                )
        })();
        if rename_result.is_err() {
            if let Ok(mount_state) = mount_guard.mount_state_mut() {
                mount_state.volume_flags.volume_dirty = true;
                mount_state.dirty_bracket_opened_by_mount = false;
            }
        }
        rename_result
    }

    // Cross-directory rename helpers

    pub(super) fn rename_within_directory(
        &self,
        mut cluster_map: StreamExtensionDirEntry,
        source_child_inode: &Arc<Self>,
        target_child_inode: Option<&Arc<Self>>,
        mount_state: &mut MountedVolumeState,
        fs: &Arc<ExfatFs>,
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        upcase_table: &UpcaseTable,
        old_name: &[u16],
        old_name_hash: u16,
        new_name: &[u16],
        new_name_hash: u16,
    ) -> Result<bool> {
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
        fs.publish_dirty_admission(mount_state)?;
        let current_renamed_entry_set =
            direntry::renamed_entry_set(current_source_view, new_name, new_name_hash)
                .map_err(Error::from)?;
        let required_entry_count = current_renamed_entry_set.len() / DIRECTORY_ENTRY_SIZE;

        let (source_inode_type, _, _, _) = current_source_view
            .child_metadata(boot_region)
            .map_err(Error::from)?;
        let (replaced_target_slot_range, replaced_target_ranges) =
            match Self::collect_replaced_target_ranges(
                current_target_view,
                target_child_inode,
                source_inode_type,
                block_device,
                boot_region,
            )? {
                Some((slot_range, ranges)) => (Some(slot_range), ranges),
                None => (None, Vec::new()),
            };
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
                mount_state,
                fs,
                block_device,
                boot_region,
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
        Self::persist_rename_directory_update(
            block_device,
            boot_region,
            cluster_map,
            renamed_directory_bytes,
            Some((final_slot_range, &renamed_entry_set)),
            replaced_target_slot_range,
            (final_slot_range != source_slot_range).then_some(source_slot_range),
        )?;
        let old_source_ino = source_child_inode.metadata.read().ino;
        let new_source_ino = self
            .entry_location_ino(final_slot_range.first_entry_index())
            .map_err(Error::from)?;
        let replaced_target_ino =
            target_child_inode.map(|target_child_inode| target_child_inode.metadata.read().ino);
        *source_child_inode.parent.write() = self.weak_self();
        source_child_inode.metadata.write().ino = new_source_ino;
        if source_child_inode.metadata.read().type_ == InodeType::File {
            source_child_inode
                .store_regular_file_entry_set_location_hint(final_slot_range)
                .map_err(Error::from)?;
        }
        if let Some(target_child_inode) = target_child_inode {
            Self::finalize_detached_overwritten_target_inode(target_child_inode);
        }
        fs.rebind_rename_inode_cache(
            old_source_ino,
            new_source_ino,
            source_child_inode,
            replaced_target_ino,
        );
        Self::cleanup_replaced_target_ranges(fs, mount_state, &replaced_target_ranges);
        Ok(true)
    }

    pub(super) fn rename_across_directories(
        &self,
        source_cluster_map: StreamExtensionDirEntry,
        source_child_inode: &Arc<Self>,
        target_directory: &ExfatInode,
        mut target_cluster_map: StreamExtensionDirEntry,
        target_child_inode: Option<&Arc<Self>>,
        mount_state: &mut MountedVolumeState,
        fs: &Arc<ExfatFs>,
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        upcase_table: &UpcaseTable,
        old_name: &[u16],
        old_name_hash: u16,
        new_name: &[u16],
        new_name_hash: u16,
    ) -> Result<()> {
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
        let (replaced_target_slot_range, replaced_target_ranges) =
            match Self::collect_replaced_target_ranges(
                target_view,
                target_child_inode,
                source_inode_type,
                block_device,
                boot_region,
            )? {
                Some((slot_range, ranges)) => (Some(slot_range), ranges),
                None => (None, Vec::new()),
            };
        fs.publish_dirty_admission(mount_state)?;
        let (
            updated_target_cluster_map,
            target_directory_bytes,
            target_slot_range,
            _reserved_target_slot,
        ) = target_directory.reserve_rename_destination_slot(
            target_cluster_map,
            target_directory_bytes,
            replaced_target_slot_range,
            mount_state,
            fs,
            block_device,
            boot_region,
            required_entry_count,
        )?;
        target_cluster_map = updated_target_cluster_map;
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
        let old_source_ino = source_child_inode.metadata.read().ino;
        let new_source_ino = target_directory
            .entry_location_ino(target_slot_range.first_entry_index())
            .map_err(Error::from)?;
        let replaced_target_ino =
            target_child_inode.map(|target_child_inode| target_child_inode.metadata.read().ino);
        *source_child_inode.parent.write() = target_directory.weak_self();
        source_child_inode.metadata.write().ino = new_source_ino;
        if source_child_inode.metadata.read().type_ == InodeType::File {
            source_child_inode
                .store_regular_file_entry_set_location_hint(target_slot_range)
                .map_err(Error::from)?;
        }
        if let Some(target_child_inode) = target_child_inode {
            Self::finalize_detached_overwritten_target_inode(target_child_inode);
        }
        fs.rebind_rename_inode_cache(
            old_source_ino,
            new_source_ino,
            source_child_inode,
            replaced_target_ino,
        );
        Self::cleanup_replaced_target_ranges(fs, mount_state, &replaced_target_ranges);
        Ok(())
    }

    // Slot management

    pub(super) fn find_vacant_entry_slots(
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

    pub(super) fn reserve_directory_entry_slots(
        &self,
        mut cluster_map: StreamExtensionDirEntry,
        mount_state: &mut MountedVolumeState,
        fs: &Arc<ExfatFs>,
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
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
                mount_state,
                fs,
                block_device,
                boot_region,
            )?;
        }
    }

    // Directory cluster-map growth

    pub(super) fn grow_directory_cluster_map(
        &self,
        cluster_map: StreamExtensionDirEntry,
        mount_state: &mut MountedVolumeState,
        fs: &Arc<ExfatFs>,
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
    ) -> Result<StreamExtensionDirEntry> {
        let allocated_directory_cluster = ClusterAllocGuard::allocate(fs, mount_state, 1)?;
        let allocated_cluster = allocated_directory_cluster.single_cluster()?;
        Self::initialize_directory_cluster(block_device, boot_region, allocated_cluster)?;
        let updated_cluster_map = self.attach_directory_cluster(
            cluster_map,
            block_device,
            boot_region,
            allocated_cluster,
        )?;
        self.write_back_directory_entry_set(block_device, boot_region, updated_cluster_map)?;
        self.commit_directory_cluster_map(updated_cluster_map, cluster_map, boot_region)?;
        allocated_directory_cluster.commit();
        Ok(updated_cluster_map)
    }

    pub(super) fn attach_directory_cluster(
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

    fn write_back_directory_entry_set(
        &self,
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        updated_cluster_map: StreamExtensionDirEntry,
    ) -> Result<()> {
        if updated_cluster_map.data_length.is_none() {
            return Ok(());
        }
        if updated_cluster_map.valid_data_length.is_none() {
            return Err(invalid_on_disk_layout());
        }

        let parent = self.parent.read().upgrade().ok_or_else(|| {
            Error::with_message(Errno::EIO, "ordinary exFAT directory parent is not mounted")
        })?;
        let parent_cluster_map = *parent.dir_entry_stream.read();
        let mut parent_directory_bytes = Self::read_directory_bytes_for_cluster_map(
            block_device,
            boot_region,
            parent_cluster_map,
        )?;
        let entry_index =
            usize::try_from(self.metadata.read().ino as u32).map_err(|_| Error::new(Errno::EIO))?;
        let entry_view = match direntry::scan_dir_entry(
            parent_cluster_map.data_length.is_none(),
            &parent_directory_bytes,
            entry_index,
        )? {
            ScannedDirEntry::File(entry_view) if entry_view.is_directory() => entry_view,
            _ => return Err(Error::from(invalid_on_disk_layout())),
        };
        let slot_range_bytes = direntry::slot_range_bytes(entry_view.slot_range())?;
        let mut updated_entry_set = entry_view.to_mutable();
        updated_entry_set.set_cluster_map(&updated_cluster_map)?;
        let updated_entry_set_bytes = updated_entry_set.into_bytes();
        let destination_entry_set = parent_directory_bytes
            .get_mut(slot_range_bytes)
            .ok_or(invalid_on_disk_layout())?;
        destination_entry_set.copy_from_slice(&updated_entry_set_bytes);
        Self::write_directory_bytes_for_cluster_map(
            block_device,
            boot_region,
            &parent_directory_bytes,
            parent_cluster_map,
        )?;
        Ok(())
    }

    fn commit_directory_cluster_map(
        &self,
        updated_cluster_map: StreamExtensionDirEntry,
        previous_cluster_map: StreamExtensionDirEntry,
        boot_region: &BootRegion,
    ) -> Result<()> {
        {
            let mut current_cluster_map = self.dir_entry_stream.write();
            if *current_cluster_map != previous_cluster_map {
                return Err(invalid_on_disk_layout());
            }
            *current_cluster_map = updated_cluster_map;
        }
        let mut metadata = self.metadata.write();
        metadata.size = metadata
            .size
            .checked_add(boot_region.cluster_size)
            .ok_or(invalid_on_disk_layout())?;
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

    pub(super) fn ensure_directory_entry_is_empty(
        child_inode: &Arc<Self>,
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
    ) -> Result<()> {
        let (_owner_guard, cluster_map) = child_inode.directory_snapshot().map_err(Error::from)?;
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

    pub(super) fn allocated_cluster_ranges(
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

    fn collect_replaced_target_ranges(
        target_view: Option<FileEntrySetView<'_>>,
        target_child_inode: Option<&Arc<Self>>,
        source_inode_type: InodeType,
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
    ) -> Result<Option<(DirEntrySlotRange, Vec<ClusterRange>)>> {
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
            let Some(child_inode) = target_child_inode else {
                return Err(Error::from(invalid_on_disk_layout()));
            };
            Self::ensure_directory_entry_is_empty(child_inode, block_device, boot_region)?;
        }
        let replaced_target_ranges = Self::allocated_cluster_ranges(
            block_device,
            boot_region,
            first_cluster,
            data_length,
            no_fat_chain,
        )
        .map_err(Error::from)?;
        Ok(Some((target_slot_range, replaced_target_ranges)))
    }

    fn reserve_rename_destination_slot(
        &self,
        cluster_map: StreamExtensionDirEntry,
        current_directory_bytes: Vec<u8>,
        reusable_slot_range: Option<DirEntrySlotRange>,
        mount_state: &mut MountedVolumeState,
        fs: &Arc<ExfatFs>,
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
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
                mount_state,
                fs,
                block_device,
                boot_region,
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
        fs: &Arc<ExfatFs>,
        mount_state: &mut MountedVolumeState,
        replaced_target_ranges: &[ClusterRange],
    ) {
        if replaced_target_ranges.is_empty() {
            return;
        }
        let _ = fs.free_clusters(mount_state, replaced_target_ranges);
    }

    fn finalize_detached_overwritten_target_inode(target_child_inode: &Arc<Self>) {
        *target_child_inode.parent.write() = Weak::new();
        target_child_inode.clear_regular_file_entry_set_location_hint();
        let mut metadata = target_child_inode.metadata.write();
        metadata.nr_hard_links = 0;
        metadata.ino = u64::MAX;
    }
}
