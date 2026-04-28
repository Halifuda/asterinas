// SPDX-License-Identifier: MPL-2.0

use alloc::string::String;

use aster_block::BlockDevice;

use super::{
    super::{
        bitmap::ClusterRange,
        boot::BootRegion,
        direntry::{
            self, DIRECTORY_ENTRY_SIZE, DirectoryEntrySlotRange, FileEntrySetFieldUpdates,
            FileEntrySetView, ScannedDirectoryEntry, WritableDirectoryEntrySlotSpan,
        },
        fat::{ChainVisitControl, FatReader},
        fs::{ExfatFsError, ExfatMountOptions},
    },
    ExfatFs, ExfatInode, ExfatInodeStream, MountedVolumeState, UpcaseTable,
};
use crate::{
    fs::{
        file::{InodeMode, InodeType, chmod},
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
        if self.type_() != InodeType::Dir {
            return_errno!(Errno::ENOTDIR);
        }
        if !matches!(type_, InodeType::File | InodeType::Dir) {
            return_errno!(Errno::EOPNOTSUPP);
        }

        let fs = self
            .fs
            .upgrade()
            .ok_or_else(|| Error::with_message(Errno::EIO, "exFAT filesystem is not mounted"))?;
        let (mut state_guard, block_device, boot_region, _anomaly, upcase_table, options) =
            fs.admitted_mutation_state().map_err(Error::from)?;
        if options.fs_flags.contains(FsFlags::RDONLY) {
            return_errno!(Errno::EROFS);
        }

        let admitted_name = Self::admitted_name(name, &options)?;
        let name_hash = upcase_table.name_hash(&admitted_name);
        let required_entry_count =
            direntry::file_entry_set_entry_count(admitted_name.len()).map_err(Error::from)?;
        let child_inode = {
            let _directory_guards = Self::ordered_directory_write_guards(vec![self]);
            let stream = *self.stream.read();
            let current_directory_bytes =
                Self::read_directory_bytes_for_stream(&block_device, &boot_region, stream)
                    .map_err(Error::from)?;
            if Self::locate_named_child_view(
                &current_directory_bytes,
                stream.data_length.is_none(),
                &upcase_table,
                &admitted_name,
                name_hash,
            )
            .map_err(Error::from)?
            .is_some()
            {
                return_errno!(Errno::EEXIST);
            }

            let publication = state_guard
                .as_mut()
                .ok_or(ExfatFsError::UnpublishedState)
                .map_err(Error::from)?;
            let (stream, mut published_directory_bytes, slot_range) = self
                .reserve_directory_entry_slots(
                    stream,
                    publication,
                    &fs,
                    &block_device,
                    &boot_region,
                    required_entry_count,
                )
                .map_err(Error::from)?;

            let mut allocated_directory_ranges = None;
            let (first_cluster, data_length, no_fat_chain) = if type_ == InodeType::Dir
                && !options.zero_size_dir
            {
                let (allocated_ranges, _) = fs
                    .allocate_free_space_with_publication(publication, 1)
                    .map_err(Error::from)?;
                let allocated_cluster = match allocated_ranges.as_slice() {
                    [allocated_range] if allocated_range.cluster_count == 1 => {
                        allocated_range.start_cluster
                    }
                    _ => {
                        let _ = fs
                            .free_allocated_space_with_publication(publication, &allocated_ranges);
                        return Err(Error::from(ExfatFsError::InconsistentAccounting));
                    }
                };
                if let Err(error) = Self::initialize_directory_cluster(
                    &block_device,
                    &boot_region,
                    allocated_cluster,
                ) {
                    let _ =
                        fs.free_allocated_space_with_publication(publication, &allocated_ranges);
                    return Err(Error::from(error));
                }
                allocated_directory_ranges = Some(allocated_ranges);
                (allocated_cluster, boot_region.cluster_size, true)
            } else {
                (0, 0, false)
            };

            let entry_set = direntry::encode_file_entry_set(
                &admitted_name,
                name_hash,
                type_,
                first_cluster,
                data_length,
                no_fat_chain,
            )
            .map_err(|error| {
                if let Some(allocated_ranges) = allocated_directory_ranges.as_ref() {
                    let _ = fs.free_allocated_space_with_publication(publication, allocated_ranges);
                }
                Error::from(error)
            })?;
            let slot_range_bytes = direntry::slot_range_bytes(slot_range).map_err(|error| {
                if let Some(allocated_ranges) = allocated_directory_ranges.as_ref() {
                    let _ = fs.free_allocated_space_with_publication(publication, allocated_ranges);
                }
                Error::from(error)
            })?;
            published_directory_bytes[slot_range_bytes.clone()].copy_from_slice(&entry_set);
            Self::write_directory_bytes_for_stream(
                &block_device,
                &boot_region,
                &published_directory_bytes,
                stream,
            )
            .map_err(|error| {
                if let Some(allocated_ranges) = allocated_directory_ranges.as_ref() {
                    let _ = fs.free_allocated_space_with_publication(publication, allocated_ranges);
                }
                Error::from(error)
            })?;

            let child_size = if type_ == InodeType::Dir {
                data_length
            } else {
                0
            };
            let child_inode = Self::new_child(
                &fs,
                self.this.clone(),
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
            child_inode.metadata.write().mode = mode;
            let child_inode: Arc<dyn Inode> = child_inode;
            child_inode
        };
        self.refresh_directory_metadata_after_namespace_mutation(
            &block_device,
            &boot_region,
            RealTimeCoarseClock::get().read_time(),
        )?;
        Ok(child_inode)
    }

    pub(super) fn unlink_impl(&self, name: &str) -> Result<()> {
        if self.type_() != InodeType::Dir {
            return_errno!(Errno::ENOTDIR);
        }

        let fs = self
            .fs
            .upgrade()
            .ok_or_else(|| Error::with_message(Errno::EIO, "exFAT filesystem is not mounted"))?;
        let (mut state_guard, block_device, boot_region, _anomaly, upcase_table, options) =
            fs.admitted_mutation_state().map_err(Error::from)?;
        if options.fs_flags.contains(FsFlags::RDONLY) {
            return_errno!(Errno::EROFS);
        }

        let admitted_name = Self::admitted_name(name, &options)?;
        let lookup_name_hash = upcase_table.name_hash(&admitted_name);

        let allocated_cluster_ranges = {
            let _directory_guards = Self::ordered_directory_write_guards(vec![self]);
            let stream = *self.stream.read();
            let is_root_directory = stream.data_length.is_none();
            let directory_bytes =
                Self::read_directory_bytes_for_stream(&block_device, &boot_region, stream)
                    .map_err(Error::from)?;
            let Some((slot_range, inode_type, first_cluster, data_length, _, no_fat_chain)) =
                Self::locate_named_child(
                    &directory_bytes,
                    is_root_directory,
                    &boot_region,
                    &upcase_table,
                    &admitted_name,
                    lookup_name_hash,
                )
                .map_err(Error::from)?
            else {
                return_errno!(Errno::ENOENT);
            };
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

            let mut invalidated_directory_bytes = directory_bytes;
            let slot_range_bytes = direntry::slot_range_bytes(slot_range).map_err(Error::from)?;
            let removed_entry_set = invalidated_directory_bytes
                .get_mut(slot_range_bytes)
                .ok_or(ExfatFsError::InvalidOnDiskLayout)
                .map_err(Error::from)?;
            let mut removed_entry_set =
                WritableDirectoryEntrySlotSpan::new(slot_range, removed_entry_set)
                    .map_err(Error::from)?;
            direntry::invalidate_entry_set(&mut removed_entry_set).map_err(Error::from)?;
            Self::write_directory_bytes_for_stream(
                &block_device,
                &boot_region,
                &invalidated_directory_bytes,
                stream,
            )
            .map_err(Error::from)?;
            allocated_cluster_ranges
        };

        if !allocated_cluster_ranges.is_empty() {
            let publication = state_guard
                .as_mut()
                .ok_or(ExfatFsError::UnpublishedState)
                .map_err(Error::from)?;
            let _ =
                fs.free_allocated_space_with_publication(publication, &allocated_cluster_ranges);
        }
        self.refresh_directory_metadata_after_namespace_mutation(
            &block_device,
            &boot_region,
            RealTimeCoarseClock::get().read_time(),
        )?;
        Ok(())
    }

    pub(super) fn rmdir_impl(&self, name: &str) -> Result<()> {
        if self.type_() != InodeType::Dir {
            return_errno!(Errno::ENOTDIR);
        }

        let fs = self
            .fs
            .upgrade()
            .ok_or_else(|| Error::with_message(Errno::EIO, "exFAT filesystem is not mounted"))?;
        let (mut state_guard, block_device, boot_region, _anomaly, upcase_table, options) =
            fs.admitted_mutation_state().map_err(Error::from)?;
        if options.fs_flags.contains(FsFlags::RDONLY) {
            return_errno!(Errno::EROFS);
        }

        let admitted_name = Self::admitted_name(name, &options)?;
        let lookup_name_hash = upcase_table.name_hash(&admitted_name);

        let allocated_cluster_ranges = {
            let _directory_guards = Self::ordered_directory_write_guards(vec![self]);
            let stream = *self.stream.read();
            let directory_bytes =
                Self::read_directory_bytes_for_stream(&block_device, &boot_region, stream)
                    .map_err(Error::from)?;
            let Some((slot_range, inode_type, first_cluster, data_length, _, no_fat_chain)) =
                Self::locate_named_child(
                    &directory_bytes,
                    stream.data_length.is_none(),
                    &boot_region,
                    &upcase_table,
                    &admitted_name,
                    lookup_name_hash,
                )
                .map_err(Error::from)?
            else {
                return_errno!(Errno::ENOENT);
            };
            if inode_type != InodeType::Dir {
                return_errno!(Errno::ENOTDIR);
            }

            let child_inode = Self::child_inode_from_directory_entry(
                self,
                &fs,
                &boot_region,
                stream.first_cluster,
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

            let mut invalidated_directory_bytes = directory_bytes;
            let slot_range_bytes = direntry::slot_range_bytes(slot_range).map_err(Error::from)?;
            let removed_entry_set = invalidated_directory_bytes
                .get_mut(slot_range_bytes)
                .ok_or(ExfatFsError::InvalidOnDiskLayout)
                .map_err(Error::from)?;
            let mut removed_entry_set =
                WritableDirectoryEntrySlotSpan::new(slot_range, removed_entry_set)
                    .map_err(Error::from)?;
            direntry::invalidate_entry_set(&mut removed_entry_set).map_err(Error::from)?;
            Self::write_directory_bytes_for_stream(
                &block_device,
                &boot_region,
                &invalidated_directory_bytes,
                stream,
            )
            .map_err(Error::from)?;
            allocated_cluster_ranges
        };

        if !allocated_cluster_ranges.is_empty() {
            let publication = state_guard
                .as_mut()
                .ok_or(ExfatFsError::UnpublishedState)
                .map_err(Error::from)?;
            let _ =
                fs.free_allocated_space_with_publication(publication, &allocated_cluster_ranges);
        }
        self.refresh_directory_metadata_after_namespace_mutation(
            &block_device,
            &boot_region,
            RealTimeCoarseClock::get().read_time(),
        )?;
        Ok(())
    }

    pub(super) fn rename_impl(
        &self,
        old_name: &str,
        target: &Arc<dyn Inode>,
        new_name: &str,
    ) -> Result<()> {
        if self.type_() != InodeType::Dir {
            return_errno!(Errno::ENOTDIR);
        }

        let Some(target_directory) = target.downcast_ref::<Self>() else {
            return_errno!(Errno::EXDEV);
        };
        if target_directory.type_() != InodeType::Dir {
            return_errno!(Errno::ENOTDIR);
        }

        let fs = self
            .fs
            .upgrade()
            .ok_or_else(|| Error::with_message(Errno::EIO, "exFAT filesystem is not mounted"))?;
        let target_fs = target_directory
            .fs
            .upgrade()
            .ok_or_else(|| Error::with_message(Errno::EIO, "exFAT filesystem is not mounted"))?;
        if !Arc::ptr_eq(&fs, &target_fs) {
            return_errno!(Errno::EXDEV);
        }

        let (mut state_guard, block_device, boot_region, _anomaly, upcase_table, options) =
            fs.admitted_mutation_state().map_err(Error::from)?;
        if options.fs_flags.contains(FsFlags::RDONLY) {
            return_errno!(Errno::EROFS);
        }

        let admitted_old_name = Self::admitted_name(old_name, &options)?;
        let admitted_new_name = Self::admitted_name(new_name, &options)?;
        let old_name_hash = upcase_table.name_hash(&admitted_old_name);
        let new_name_hash = upcase_table.name_hash(&admitted_new_name);

        if self.metadata.read().ino == target_directory.metadata.read().ino {
            let renamed = {
                let _directory_guards = Self::ordered_directory_write_guards(vec![self]);
                let stream = *self.stream.read();
                let directory_bytes =
                    Self::read_directory_bytes_for_stream(&block_device, &boot_region, stream)
                        .map_err(Error::from)?;
                let Some(source_view) = Self::locate_named_child_view(
                    &directory_bytes,
                    stream.data_length.is_none(),
                    &upcase_table,
                    &admitted_old_name,
                    old_name_hash,
                )
                .map_err(Error::from)?
                else {
                    return_errno!(Errno::ENOENT);
                };
                let target_child_inode = Self::locate_named_child_view(
                    &directory_bytes,
                    stream.data_length.is_none(),
                    &upcase_table,
                    &admitted_new_name,
                    new_name_hash,
                )
                .map_err(Error::from)?
                .filter(|target_view| target_view.slot_range() != source_view.slot_range())
                .map(|target_view| {
                    let (target_inode_type, first_cluster, data_length, no_fat_chain) =
                        target_view.child_metadata(&boot_region)?;
                    if target_inode_type != InodeType::Dir {
                        return Ok(None);
                    }
                    Self::child_inode_from_directory_entry(
                        self,
                        &fs,
                        &boot_region,
                        stream.first_cluster,
                        target_view.slot_range(),
                        target_inode_type,
                        first_cluster,
                        data_length,
                        data_length,
                        no_fat_chain,
                    )
                    .map(Some)
                })
                .transpose()
                .map_err(Error::from)?
                .flatten();
                let publication = state_guard
                    .as_mut()
                    .ok_or(ExfatFsError::UnpublishedState)
                    .map_err(Error::from)?;
                let stream = *self.stream.read();
                self.rename_within_directory(
                    stream,
                    target_child_inode.as_ref(),
                    publication,
                    &fs,
                    &block_device,
                    &boot_region,
                    &upcase_table,
                    &admitted_old_name,
                    old_name_hash,
                    &admitted_new_name,
                    new_name_hash,
                )?
            };
            if renamed {
                self.refresh_directory_metadata_after_namespace_mutation(
                    &block_device,
                    &boot_region,
                    RealTimeCoarseClock::get().read_time(),
                )?;
            }
            return Ok(());
        }

        {
            let _directory_guards =
                Self::ordered_directory_write_guards(vec![self, target_directory]);
            let target_stream = *target_directory.stream.read();
            let target_directory_bytes =
                Self::read_directory_bytes_for_stream(&block_device, &boot_region, target_stream)
                    .map_err(Error::from)?;
            let target_child_inode = Self::locate_named_child_view(
                &target_directory_bytes,
                target_stream.data_length.is_none(),
                &upcase_table,
                &admitted_new_name,
                new_name_hash,
            )
            .map_err(Error::from)?
            .map(|target_view| {
                let (target_inode_type, first_cluster, data_length, no_fat_chain) =
                    target_view.child_metadata(&boot_region)?;
                if target_inode_type != InodeType::Dir {
                    return Ok(None);
                }
                Self::child_inode_from_directory_entry(
                    target_directory,
                    &fs,
                    &boot_region,
                    target_stream.first_cluster,
                    target_view.slot_range(),
                    target_inode_type,
                    first_cluster,
                    data_length,
                    data_length,
                    no_fat_chain,
                )
                .map(Some)
            })
            .transpose()
            .map_err(Error::from)?
            .flatten();
            let publication = state_guard
                .as_mut()
                .ok_or(ExfatFsError::UnpublishedState)
                .map_err(Error::from)?;
            let source_stream = *self.stream.read();
            let target_stream = *target_directory.stream.read();
            self.rename_across_directories(
                source_stream,
                target_directory,
                target_stream,
                target_child_inode.as_ref(),
                publication,
                &fs,
                &block_device,
                &boot_region,
                &upcase_table,
                &admitted_old_name,
                old_name_hash,
                &admitted_new_name,
                new_name_hash,
            )?;
        }
        let timestamp = RealTimeCoarseClock::get().read_time();
        self.refresh_directory_metadata_after_namespace_mutation(
            &block_device,
            &boot_region,
            timestamp,
        )?;
        target_directory.refresh_directory_metadata_after_namespace_mutation(
            &block_device,
            &boot_region,
            timestamp,
        )
    }

    // Cross-directory rename helpers

    pub(super) fn rename_within_directory(
        &self,
        mut stream: ExfatInodeStream,
        target_child_inode: Option<&Arc<Self>>,
        publication: &mut MountedVolumeState,
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
            Self::read_directory_bytes_for_stream(block_device, boot_region, stream)
                .map_err(Error::from)?;
        let Some(current_source_view) = Self::locate_named_child_view(
            &current_directory_bytes,
            stream.data_length.is_none(),
            upcase_table,
            old_name,
            old_name_hash,
        )
        .map_err(Error::from)?
        else {
            return_errno!(Errno::ENOENT);
        };
        let source_name = current_source_view.name().map_err(Error::from)?;
        let current_source_slot_range = current_source_view.slot_range();
        let current_target_view = Self::locate_named_child_view(
            &current_directory_bytes,
            stream.data_length.is_none(),
            upcase_table,
            new_name,
            new_name_hash,
        )
        .map_err(Error::from)?;
        if current_target_view
            .map(FileEntrySetView::slot_range)
            .is_some_and(|slot_range| slot_range == current_source_slot_range)
            && source_name == new_name
        {
            return Ok(false);
        }
        let current_renamed_entry_set =
            direntry::renamed_entry_set(current_source_view, new_name, new_name_hash)
                .map_err(Error::from)?;
        let required_entry_count = current_renamed_entry_set.len() / DIRECTORY_ENTRY_SIZE;

        let (source_inode_type, _, _, _) = current_source_view
            .child_metadata(boot_region)
            .map_err(Error::from)?;
        let mut replaced_target_ranges = Vec::new();
        let mut final_slot_range = current_source_slot_range;
        if let Some(target_view) = current_target_view
            .filter(|entry_view| entry_view.slot_range() != current_source_slot_range)
        {
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
                    return Err(Error::from(ExfatFsError::InvalidOnDiskLayout));
                };
                Self::ensure_directory_entry_is_empty(child_inode, block_device, boot_region)?;
            }
            replaced_target_ranges = Self::allocated_cluster_ranges(
                block_device,
                boot_region,
                first_cluster,
                data_length,
                no_fat_chain,
            )
            .map_err(Error::from)?;
            if current_source_slot_range.entry_count() < required_entry_count {
                final_slot_range = target_view.slot_range();
            }
        }

        let (mut renamed_directory_bytes, source_slot_range, renamed_entry_set) =
            if final_slot_range == current_source_slot_range
                && current_source_slot_range.entry_count() < required_entry_count
            {
                let (updated_stream, latest_directory_bytes, reserved_slot_range) = self
                    .reserve_directory_entry_slots(
                        stream,
                        publication,
                        fs,
                        block_device,
                        boot_region,
                        required_entry_count,
                    )
                    .map_err(Error::from)?;
                stream = updated_stream;
                final_slot_range = reserved_slot_range;
                let Some(latest_source_view) = Self::locate_named_child_view(
                    &latest_directory_bytes,
                    stream.data_length.is_none(),
                    upcase_table,
                    old_name,
                    old_name_hash,
                )
                .map_err(Error::from)?
                else {
                    return_errno!(Errno::ENOENT);
                };
                let source_slot_range = latest_source_view.slot_range();
                let renamed_entry_set =
                    direntry::renamed_entry_set(latest_source_view, new_name, new_name_hash)
                        .map_err(Error::from)?;
                (latest_directory_bytes, source_slot_range, renamed_entry_set)
            } else {
                (
                    current_directory_bytes,
                    current_source_slot_range,
                    current_renamed_entry_set,
                )
            };

        let target_slot_range = Self::locate_named_child_view(
            &renamed_directory_bytes,
            stream.data_length.is_none(),
            upcase_table,
            new_name,
            new_name_hash,
        )
        .map_err(Error::from)?
        .filter(|entry_view| {
            entry_view.slot_range() != source_slot_range
                && entry_view.slot_range() != final_slot_range
        })
        .map(FileEntrySetView::slot_range);
        if let Some(target_slot_range) = target_slot_range {
            let slot_range_bytes =
                direntry::slot_range_bytes(target_slot_range).map_err(Error::from)?;
            let overwritten_entry_set = renamed_directory_bytes
                .get_mut(slot_range_bytes)
                .ok_or(ExfatFsError::InvalidOnDiskLayout)
                .map_err(Error::from)?;
            let mut overwritten_entry_set =
                WritableDirectoryEntrySlotSpan::new(target_slot_range, overwritten_entry_set)
                    .map_err(Error::from)?;
            direntry::invalidate_entry_set(&mut overwritten_entry_set).map_err(Error::from)?;
        }
        if final_slot_range != source_slot_range {
            let slot_range_bytes =
                direntry::slot_range_bytes(source_slot_range).map_err(Error::from)?;
            let removed_entry_set = renamed_directory_bytes
                .get_mut(slot_range_bytes)
                .ok_or(ExfatFsError::InvalidOnDiskLayout)
                .map_err(Error::from)?;
            let mut removed_entry_set =
                WritableDirectoryEntrySlotSpan::new(source_slot_range, removed_entry_set)
                    .map_err(Error::from)?;
            direntry::invalidate_entry_set(&mut removed_entry_set).map_err(Error::from)?;
        }

        let final_slot_bytes = direntry::slot_range_bytes(final_slot_range).map_err(Error::from)?;
        let destination_entry_set = renamed_directory_bytes
            .get_mut(final_slot_bytes)
            .ok_or(ExfatFsError::InvalidOnDiskLayout)
            .map_err(Error::from)?;
        let mut destination_entry_set =
            WritableDirectoryEntrySlotSpan::new(final_slot_range, destination_entry_set)
                .map_err(Error::from)?;
        direntry::invalidate_entry_set(&mut destination_entry_set).map_err(Error::from)?;
        destination_entry_set
            .bytes_mut()
            .get_mut(..renamed_entry_set.len())
            .ok_or(ExfatFsError::InvalidOnDiskLayout)
            .map_err(Error::from)?
            .copy_from_slice(&renamed_entry_set);
        Self::write_directory_bytes_for_stream(
            block_device,
            boot_region,
            &renamed_directory_bytes,
            stream,
        )
        .map_err(Error::from)?;

        if !replaced_target_ranges.is_empty() {
            let _ = fs.free_allocated_space_with_publication(publication, &replaced_target_ranges);
        }
        Ok(true)
    }

    pub(super) fn rename_across_directories(
        &self,
        source_stream: ExfatInodeStream,
        target_directory: &ExfatInode,
        mut target_stream: ExfatInodeStream,
        target_child_inode: Option<&Arc<Self>>,
        publication: &mut MountedVolumeState,
        fs: &Arc<ExfatFs>,
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        upcase_table: &UpcaseTable,
        old_name: &[u16],
        old_name_hash: u16,
        new_name: &[u16],
        new_name_hash: u16,
    ) -> Result<()> {
        let source_directory_bytes =
            Self::read_directory_bytes_for_stream(block_device, boot_region, source_stream)
                .map_err(Error::from)?;
        let Some(source_view) = Self::locate_named_child_view(
            &source_directory_bytes,
            source_stream.data_length.is_none(),
            upcase_table,
            old_name,
            old_name_hash,
        )
        .map_err(Error::from)?
        else {
            return_errno!(Errno::ENOENT);
        };
        let source_slot_range = source_view.slot_range();
        let (source_inode_type, _, _, _) = source_view
            .child_metadata(boot_region)
            .map_err(Error::from)?;
        let renamed_entry_set = direntry::renamed_entry_set(source_view, new_name, new_name_hash)
            .map_err(Error::from)?;
        let required_entry_count = renamed_entry_set.len() / DIRECTORY_ENTRY_SIZE;

        let target_directory_bytes =
            Self::read_directory_bytes_for_stream(block_device, boot_region, target_stream)
                .map_err(Error::from)?;
        let target_view = Self::locate_named_child_view(
            &target_directory_bytes,
            target_stream.data_length.is_none(),
            upcase_table,
            new_name,
            new_name_hash,
        )
        .map_err(Error::from)?;
        let (mut published_target_directory_bytes, target_slot_range, replaced_target_ranges) =
            if let Some(target_view) = target_view {
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
                        return Err(Error::from(ExfatFsError::InvalidOnDiskLayout));
                    };
                    Self::ensure_directory_entry_is_empty(child_inode, block_device, boot_region)?;
                }
                let target_ranges = Self::allocated_cluster_ranges(
                    block_device,
                    boot_region,
                    first_cluster,
                    data_length,
                    no_fat_chain,
                )
                .map_err(Error::from)?;
                (target_directory_bytes, target_slot_range, target_ranges)
            } else {
                let (updated_target_stream, latest_target_directory_bytes, reserved_slot_range) =
                    target_directory
                        .reserve_directory_entry_slots(
                            target_stream,
                            publication,
                            fs,
                            block_device,
                            boot_region,
                            required_entry_count,
                        )
                        .map_err(Error::from)?;
                target_stream = updated_target_stream;
                (
                    latest_target_directory_bytes,
                    reserved_slot_range,
                    Vec::new(),
                )
            };

        let target_slot_bytes =
            direntry::slot_range_bytes(target_slot_range).map_err(Error::from)?;
        let destination_entry_set = published_target_directory_bytes
            .get_mut(target_slot_bytes)
            .ok_or(ExfatFsError::InvalidOnDiskLayout)
            .map_err(Error::from)?;
        let mut destination_entry_set =
            WritableDirectoryEntrySlotSpan::new(target_slot_range, destination_entry_set)
                .map_err(Error::from)?;
        direntry::invalidate_entry_set(&mut destination_entry_set).map_err(Error::from)?;
        destination_entry_set
            .bytes_mut()
            .get_mut(..renamed_entry_set.len())
            .ok_or(ExfatFsError::InvalidOnDiskLayout)
            .map_err(Error::from)?
            .copy_from_slice(&renamed_entry_set);
        Self::write_directory_bytes_for_stream(
            block_device,
            boot_region,
            &published_target_directory_bytes,
            target_stream,
        )
        .map_err(Error::from)?;

        let mut invalidated_source_directory_bytes = source_directory_bytes;
        let source_slot_bytes =
            direntry::slot_range_bytes(source_slot_range).map_err(Error::from)?;
        let removed_entry_set = invalidated_source_directory_bytes
            .get_mut(source_slot_bytes)
            .ok_or(ExfatFsError::InvalidOnDiskLayout)
            .map_err(Error::from)?;
        let mut removed_entry_set =
            WritableDirectoryEntrySlotSpan::new(source_slot_range, removed_entry_set)
                .map_err(Error::from)?;
        direntry::invalidate_entry_set(&mut removed_entry_set).map_err(Error::from)?;
        Self::write_directory_bytes_for_stream(
            block_device,
            boot_region,
            &invalidated_source_directory_bytes,
            source_stream,
        )
        .map_err(Error::from)?;

        if !replaced_target_ranges.is_empty() {
            let _ = fs.free_allocated_space_with_publication(publication, &replaced_target_ranges);
        }
        Ok(())
    }

    // Slot management

    pub(super) fn find_vacant_entry_slots(
        is_root_directory: bool,
        directory_bytes: &[u8],
        required_entry_count: usize,
    ) -> core::result::Result<Option<DirectoryEntrySlotRange>, ExfatFsError> {
        if required_entry_count == 0 {
            return Err(ExfatFsError::InvalidOperationInput);
        }
        if directory_bytes.len() % DIRECTORY_ENTRY_SIZE != 0 {
            return Err(ExfatFsError::InvalidOnDiskLayout);
        }

        let total_entries = directory_bytes.len() / DIRECTORY_ENTRY_SIZE;
        let mut run_length = 0usize;
        let mut run_start_index = 0usize;
        let mut entry_index = 0usize;
        loop {
            match direntry::scan_directory_entry(is_root_directory, directory_bytes, entry_index)? {
                ScannedDirectoryEntry::EndOfDirectory { entry_index } => {
                    let available_entries = total_entries
                        .checked_sub(entry_index)
                        .ok_or(ExfatFsError::InvalidOnDiskLayout)?;
                    if run_length == 0 {
                        run_start_index = entry_index;
                    }
                    run_length = run_length
                        .checked_add(available_entries)
                        .ok_or(ExfatFsError::InvalidOnDiskLayout)?;
                    if run_length >= required_entry_count {
                        return Ok(Some(DirectoryEntrySlotRange::new(
                            run_start_index,
                            required_entry_count,
                        )?));
                    }
                    return Ok(None);
                }
                ScannedDirectoryEntry::Vacant(slot_range) => {
                    if run_length == 0 {
                        run_start_index = slot_range.first_entry_index();
                    }
                    run_length = run_length
                        .checked_add(slot_range.entry_count())
                        .ok_or(ExfatFsError::InvalidOnDiskLayout)?;
                    if run_length >= required_entry_count {
                        return Ok(Some(DirectoryEntrySlotRange::new(
                            run_start_index,
                            required_entry_count,
                        )?));
                    }
                    entry_index = slot_range.next_entry_index()?;
                }
                ScannedDirectoryEntry::File(entry_view) => {
                    run_length = 0;
                    entry_index = entry_view.slot_range().next_entry_index()?;
                }
                ScannedDirectoryEntry::Anomaly { .. } => {
                    return Err(ExfatFsError::InvalidOnDiskLayout);
                }
            }
        }
    }

    pub(super) fn reserve_directory_entry_slots(
        &self,
        mut stream: ExfatInodeStream,
        publication: &mut MountedVolumeState,
        fs: &Arc<ExfatFs>,
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        required_entry_count: usize,
    ) -> core::result::Result<(ExfatInodeStream, Vec<u8>, DirectoryEntrySlotRange), ExfatFsError>
    {
        loop {
            let directory_bytes =
                Self::read_directory_bytes_for_stream(block_device, boot_region, stream)?;
            if let Some(slot_range) = Self::find_vacant_entry_slots(
                stream.data_length.is_none(),
                &directory_bytes,
                required_entry_count,
            )? {
                return Ok((stream, directory_bytes, slot_range));
            }
            stream =
                self.grow_directory_stream(stream, publication, fs, block_device, boot_region)?;
        }
    }

    // Directory stream growth

    pub(super) fn grow_directory_stream(
        &self,
        stream: ExfatInodeStream,
        publication: &mut MountedVolumeState,
        fs: &Arc<ExfatFs>,
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
    ) -> core::result::Result<ExfatInodeStream, ExfatFsError> {
        let (allocated_ranges, _) = fs.allocate_free_space_with_publication(publication, 1)?;
        let allocated_cluster = match allocated_ranges.as_slice() {
            [allocated_range] if allocated_range.cluster_count == 1 => {
                allocated_range.start_cluster
            }
            _ => {
                let _ = fs.free_allocated_space_with_publication(publication, &allocated_ranges);
                return Err(ExfatFsError::InconsistentAccounting);
            }
        };

        if let Err(error) =
            Self::initialize_directory_cluster(block_device, boot_region, allocated_cluster)
        {
            let _ = fs.free_allocated_space_with_publication(publication, &allocated_ranges);
            return Err(error);
        }

        let updated_stream = match self.attach_directory_cluster(
            stream,
            block_device,
            boot_region,
            allocated_cluster,
        ) {
            Ok(updated_stream) => updated_stream,
            Err(error) => {
                let _ = fs.free_allocated_space_with_publication(publication, &allocated_ranges);
                return Err(error);
            }
        };
        Ok(updated_stream)
    }

    pub(super) fn attach_directory_cluster(
        &self,
        stream: ExfatInodeStream,
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        allocated_cluster: u32,
    ) -> core::result::Result<ExfatInodeStream, ExfatFsError> {
        let next_data_length = match stream.data_length {
            Some(data_length) => data_length
                .checked_add(boot_region.cluster_size)
                .ok_or(ExfatFsError::InvalidOnDiskLayout)?,
            None => boot_region.cluster_size,
        };

        let mut fat_reader = FatReader::new(block_device.as_ref(), boot_region);
        match stream.data_length {
            Some(0) => {
                fat_reader.terminate_cluster_chain(allocated_cluster)?;
            }
            Some(data_length) if stream.no_fat_chain => {
                let cluster_count = data_length.div_ceil(boot_region.cluster_size);
                fat_reader.link_contiguous_chain_to_cluster(
                    stream.first_cluster,
                    cluster_count,
                    allocated_cluster,
                )?;
            }
            Some(_) => {
                fat_reader.append_cluster_to_chain(stream.first_cluster, allocated_cluster)?;
            }
            None => {
                fat_reader.append_cluster_to_chain(stream.first_cluster, allocated_cluster)?;
            }
        }

        let updated_stream = match stream.data_length {
            Some(0) => ExfatInodeStream {
                first_cluster: allocated_cluster,
                data_length: Some(next_data_length),
                no_fat_chain: false,
                ..stream
            },
            Some(_) if stream.no_fat_chain => ExfatInodeStream {
                data_length: Some(next_data_length),
                no_fat_chain: false,
                ..stream
            },
            Some(_) => ExfatInodeStream {
                data_length: Some(next_data_length),
                ..stream
            },
            None => ExfatInodeStream {
                data_length: None,
                ..stream
            },
        };
        {
            let mut published_stream = self.stream.write();
            if *published_stream != stream {
                return Err(ExfatFsError::InvalidOnDiskLayout);
            }
            *published_stream = updated_stream;
        }
        let mut metadata = self.metadata.write();
        metadata.size = metadata
            .size
            .checked_add(boot_region.cluster_size)
            .ok_or(ExfatFsError::InvalidOnDiskLayout)?;
        Ok(updated_stream)
    }

    // Validation helpers

    pub(super) fn ensure_directory_entry_is_empty(
        child_inode: &Arc<Self>,
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
    ) -> Result<()> {
        let (_owner_guard, stream, child_directory_bytes) = child_inode
            .admitted_directory_snapshot(block_device, boot_region)
            .map_err(Error::from)?;
        if let Some(first_child_scan) = child_inode
            .first_directory_child_scan(stream, &child_directory_bytes)
            .map_err(Error::from)?
        {
            match first_child_scan {
                ScannedDirectoryEntry::Anomaly { .. } => {
                    return Err(Error::from(ExfatFsError::InvalidOnDiskLayout));
                }
                ScannedDirectoryEntry::File(_) => return_errno!(Errno::ENOTEMPTY),
                ScannedDirectoryEntry::EndOfDirectory { .. } | ScannedDirectoryEntry::Vacant(_) => {
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
    ) -> core::result::Result<Vec<ClusterRange>, ExfatFsError> {
        if data_length == 0 {
            if first_cluster != 0 {
                return Err(ExfatFsError::InvalidOnDiskLayout);
            }
            return Ok(Vec::new());
        }

        boot_region.validate_stream_data(
            first_cluster,
            u64::try_from(data_length).map_err(|_| ExfatFsError::InvalidOnDiskLayout)?,
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
                .ok_or(ExfatFsError::InvalidOnDiskLayout)?;
            match previous_cluster {
                Some(previous_cluster) if previous_cluster.checked_add(1) == Some(cluster) => {
                    current_range_count = current_range_count
                        .checked_add(1)
                        .ok_or(ExfatFsError::InvalidOnDiskLayout)?;
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
            return Err(ExfatFsError::InvalidOnDiskLayout);
        }
        cluster_ranges.push(ClusterRange {
            start_cluster: current_range_start,
            cluster_count: current_range_count,
        });
        Ok(cluster_ranges)
    }
}
