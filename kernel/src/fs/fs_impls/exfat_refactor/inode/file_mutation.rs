// SPDX-License-Identifier: MPL-2.0

//! Implements regular-file writes, resizes, shared entry-set rewrite, and cluster-map growth.
//!
//! Method groups: write/resize entry points, shared growth commit, entry-set rewrite,
//! growth topology helpers, and cluster-level mutation.

use core::{ops::Range, time::Duration};

use aster_block::BlockDevice;
use ostd::mm::{VmIo, io::util::HasVmReaderWriter};

use super::{
    super::{
        bitmap::ClusterRange, boot::BootRegion, device_io, direntry, fat::FatReader,
        fs::ClusterAllocGuard, inconsistent_bitmap_accounting, invalid_on_disk_layout,
        invalid_operation_input, not_mounted,
    },
    ClusterMap, ExfatFs, ExfatInode, MountedVolumeState, StreamExtensionDirEntry,
    page_backend::PageCacheContext,
    state::InodeStateWriteGuard,
};
use crate::{
    fs::{
        file::{InodeType, StatusFlags},
        vfs::{file_system::FsFlags, inode::Inode},
    },
    prelude::*,
    time::clocks::RealTimeCoarseClock,
    vm::page_cache::PageCache,
};

impl ExfatInode {
    pub(super) fn read_validated_entry_set(
        &self,
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
    ) -> Result<(direntry::DirEntrySlotRange, Vec<u8>)> {
        let parent = self.parent.read().upgrade().ok_or_else(|| {
            Error::with_message(Errno::EIO, "ordinary exFAT directory parent is not mounted")
        })?;
        let parent_cluster_map = *parent.dir_entry_stream.read();
        let fallback_entry_index =
            usize::try_from(self.metadata.read().ino as u32).map_err(|_| Error::new(Errno::EIO))?;

        if let Some(hinted_slot_range) = self.entry_set_location_hint()? {
            match self.try_read_validated_entry_set_at(
                block_device,
                boot_region,
                parent_cluster_map,
                hinted_slot_range,
            ) {
                Ok(Some((validated_slot_range, entry_set_bytes))) => {
                    self.store_entry_set_location_hint(validated_slot_range)?;
                    return Ok((validated_slot_range, entry_set_bytes));
                }
                Ok(None) => {
                    self.clear_entry_set_location_hint();
                }
                Err(error) if error.error() == Errno::EUCLEAN => {
                    self.clear_entry_set_location_hint();
                }
                Err(error) => return Err(error),
            }
        }

        let primary_slot_range = direntry::DirEntrySlotRange::new(fallback_entry_index, 1)?;
        let primary_entry_bytes = Self::read_entry_set_bytes_for_cluster_map(
            block_device,
            boot_region,
            parent_cluster_map,
            primary_slot_range,
        )?;
        let entry_count = usize::from(primary_entry_bytes[1])
            .checked_add(1)
            .ok_or_else(invalid_on_disk_layout)?;
        let fallback_slot_range =
            direntry::DirEntrySlotRange::new(fallback_entry_index, entry_count)?;
        let (validated_slot_range, entry_set_bytes) = self
            .try_read_validated_entry_set_at(
                block_device,
                boot_region,
                parent_cluster_map,
                fallback_slot_range,
            )?
            .ok_or_else(invalid_on_disk_layout)?;
        self.store_entry_set_location_hint(validated_slot_range)?;
        Ok((validated_slot_range, entry_set_bytes))
    }

    fn read_entry_set_bytes_for_cluster_map(
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        cluster_map: StreamExtensionDirEntry,
        slot_range: direntry::DirEntrySlotRange,
    ) -> Result<Vec<u8>> {
        let entry_set_range = direntry::slot_range_bytes(slot_range)?;
        let entry_set_length = entry_set_range
            .end
            .checked_sub(entry_set_range.start)
            .ok_or_else(invalid_on_disk_layout)?;
        let mut entry_set_bytes = vec![0; entry_set_length];
        Self::visit_directory_byte_range_for_cluster_map(
            block_device,
            boot_region,
            cluster_map,
            entry_set_range,
            |byte_offset, request_range| {
                block_device
                    .read_bytes(byte_offset, &mut entry_set_bytes[request_range])
                    .map_err(|_| device_io())
            },
        )?;
        Ok(entry_set_bytes)
    }

    fn try_read_validated_entry_set_at(
        &self,
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        cluster_map: StreamExtensionDirEntry,
        slot_range: direntry::DirEntrySlotRange,
    ) -> Result<Option<(direntry::DirEntrySlotRange, Vec<u8>)>> {
        let current_cluster_map = *self.dir_entry_stream.read();
        let expected_inode_type = self.metadata.read().type_;
        let allow_stale_regular_file_cluster_map = expected_inode_type == InodeType::File
            && self
                .dirty_state
                .read()
                .has_deferred_regular_file_publish();
        let entry_set_bytes = Self::read_entry_set_bytes_for_cluster_map(
            block_device,
            boot_region,
            cluster_map,
            slot_range,
        )?;
        let zero_based_slot_range = direntry::DirEntrySlotRange::new(0, slot_range.entry_count())?;
        let entry_view = match direntry::scan_dir_entry(false, &entry_set_bytes, 0) {
            Ok(direntry::ScannedDirEntry::File(entry_view))
                if entry_view.slot_range() == zero_based_slot_range =>
            {
                entry_view
            }
            Ok(_) => return Ok(None),
            Err(error) if error.error() == Errno::EUCLEAN => return Err(error),
            Err(error) => return Err(error),
        };
        let (inode_type, _first_cluster, _data_length, _no_fat_chain) =
            entry_view.child_metadata(boot_region)?;
        match expected_inode_type {
            InodeType::Dir => {
                if inode_type != InodeType::Dir || !entry_view.is_directory() {
                    return Ok(None);
                }
            }
            InodeType::File => {
                if inode_type != InodeType::File || entry_view.is_directory() {
                    return Ok(None);
                }
            }
            _ => {
                return Err(Error::from(invalid_on_disk_layout()));
            }
        }
        let validated_cluster_map = entry_view.cluster_map()?;
        if !allow_stale_regular_file_cluster_map && validated_cluster_map != current_cluster_map {
            return Ok(None);
        }
        let validated_slot_range = direntry::DirEntrySlotRange::new(
            slot_range.first_entry_index(),
            entry_view.slot_range().entry_count(),
        )?;
        if validated_slot_range != slot_range {
            return Ok(None);
        }
        Ok(Some((validated_slot_range, entry_set_bytes)))
    }

    pub(super) fn rewrite_validated_entry_set_with_guard(
        &self,
        _parent_inode_state_guard: &InodeStateWriteGuard<'_>,
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        rewrite_entry_set_fn: impl FnOnce(direntry::FileEntrySetView<'_>) -> Result<Option<Vec<u8>>>,
    ) -> Result<bool> {
        let parent = self.parent.read().upgrade().ok_or_else(|| {
            Error::with_message(Errno::EIO, "ordinary exFAT directory parent is not mounted")
        })?;
        let parent_cluster_map = *parent.dir_entry_stream.read();
        let (slot_range, mut entry_set_bytes) =
            self.read_validated_entry_set(block_device, boot_region)?;
        let entry_view = match direntry::scan_dir_entry(false, &entry_set_bytes, 0)? {
            direntry::ScannedDirEntry::File(entry_view) => entry_view,
            _ => return Err(Error::from(invalid_on_disk_layout())),
        };
        if entry_view.slot_range().entry_count() != slot_range.entry_count() {
            return Err(Error::from(invalid_on_disk_layout()));
        }

        let Some(updated_entry_set_bytes) = rewrite_entry_set_fn(entry_view)? else {
            return Ok(false);
        };
        if updated_entry_set_bytes.len() != entry_set_bytes.len() {
            return Err(Error::from(invalid_on_disk_layout()));
        }
        entry_set_bytes.copy_from_slice(&updated_entry_set_bytes);
        Self::visit_directory_byte_range_for_cluster_map(
            block_device,
            boot_region,
            parent_cluster_map,
            direntry::slot_range_bytes(slot_range)?,
            |byte_offset, request_range| {
                block_device
                    .write_bytes(byte_offset, &entry_set_bytes[request_range])
                    .map_err(|_| device_io())
            },
        )?;
        self.store_entry_set_location_hint(slot_range)?;
        Ok(true)
    }

    // VFS entry points

    pub(super) fn write_at_impl(
        &self,
        offset: usize,
        reader: &mut VmReader,
        status_flags: StatusFlags,
    ) -> Result<usize> {
        match self.type_() {
            InodeType::Dir => return_errno!(Errno::EISDIR),
            InodeType::File => {}
            _ => return_errno!(Errno::EOPNOTSUPP),
        }

        let fs = self
            .fs
            .upgrade()
            .ok_or_else(|| Error::with_message(Errno::EIO, "exFAT filesystem is not mounted"))?;
        if !reader.has_remain() {
            return Ok(0);
        }

        let write_len = reader.remain();
        let page_cache = self.page_cache_handle().ok_or_else(|| {
            Error::with_message(Errno::EIO, "regular exFAT file has no page cache")
        })?;
        {
            let mut admission = fs.mount_state_write_guard().map_err(Error::from)?;
            let block_device = fs.immutable_block_device();
            let boot_region = fs.immutable_boot_region();
            if admission.forced_shutdown
                || admission.flags.clear_to_zero
                || admission.flags.media_failure
            {
                return_errno!(Errno::EIO);
            }
            if admission.options.fs_flags.contains(FsFlags::RDONLY) {
                return_errno!(Errno::EROFS);
            }

            let write_result = (|| {
                let inode_state_guard = self.inode_state.write();
                let cluster_map_generation = self.current_cluster_map(&inode_state_guard)?;
                let cluster_map = cluster_map_generation.stream_extension();
                let (data_length, valid_data_length) =
                    cluster_map_generation.validated_lengths()?;

                let effective_offset = if status_flags.contains(StatusFlags::O_APPEND) {
                    data_length
                } else {
                    offset
                };
                let write_end = effective_offset
                    .checked_add(write_len)
                    .ok_or_else(|| Error::new(Errno::EINVAL))?;
                if status_flags.contains(StatusFlags::O_DIRECT)
                    && (!effective_offset.is_multiple_of(boot_region.sector_size)
                        || !write_len.is_multiple_of(boot_region.sector_size))
                {
                    return_errno!(Errno::EINVAL);
                }

                let cache_mutation_range = valid_data_length.min(effective_offset)..write_end;
                let new_data_length = data_length.max(write_end);
                let new_valid_data_length = valid_data_length.max(write_end);
                let timestamp = RealTimeCoarseClock::get().read_time();
                let write_result = (|| {
                    let mount_state = admission.state_guard.as_mut().ok_or_else(not_mounted)?;
                    fs.publish_dirty_admission(mount_state)?;
                    if !cache_mutation_range.is_empty()
                        && page_cache.has_dirty_pages(cache_mutation_range.clone())
                    {
                        page_cache.flush_range(cache_mutation_range.clone())?;
                    }
                    let mount_state = admission.state_guard.as_mut().ok_or_else(not_mounted)?;
                    self.grow_and_commit_regular_file(
                        &inode_state_guard,
                        &fs,
                        mount_state,
                        &block_device,
                        &boot_region,
                        &cluster_map_generation,
                        cluster_map,
                        effective_offset,
                        new_data_length,
                        new_valid_data_length,
                        timestamp,
                        |_cluster_map, zero_fill_range| {
                            if new_data_length > data_length {
                                page_cache.resize(new_data_length, data_length)?;
                            }
                            Self::prepare_regular_file_page_cache_range(
                                page_cache,
                                data_length,
                                zero_fill_range.clone(),
                            )?;
                            if !zero_fill_range.is_empty() {
                                page_cache.fill_zeros(zero_fill_range.clone())?;
                            }
                            Self::prepare_regular_file_page_cache_range(
                                page_cache,
                                data_length,
                                effective_offset..write_end,
                            )?;
                            page_cache
                                .write(effective_offset, reader)
                                .map_err(Error::from)?;
                            Ok(())
                        },
                        || {
                            if new_data_length > data_length {
                                let _ = page_cache.resize(data_length, new_data_length);
                            }
                        },
                    )
                })();
                if let Err(error) = write_result {
                    if new_data_length > data_length {
                        let _ = page_cache.resize(data_length, new_data_length);
                    }
                    return Err(error);
                }
                Ok(())
            })();
            if write_result.is_err() {
                if let Some(mount_state) = admission.state_guard.as_mut() {
                    mount_state.volume_flags.volume_dirty = true;
                    mount_state.dirty_bracket_opened_by_mount = false;
                }
            }
            write_result?;
        }
        if status_flags.contains(StatusFlags::O_SYNC) {
            self.sync_all()?;
        } else if status_flags.contains(StatusFlags::O_DSYNC) {
            self.sync_data()?;
        }

        Ok(write_len)
    }

    pub(super) fn resize_impl(&self, new_size: usize) -> Result<()> {
        match self.type_() {
            InodeType::Dir => return_errno!(Errno::EISDIR),
            InodeType::File => {}
            _ => return_errno!(Errno::EOPNOTSUPP),
        }

        let fs = self
            .fs
            .upgrade()
            .ok_or_else(|| Error::with_message(Errno::EIO, "exFAT filesystem is not mounted"))?;
        let mut admission = fs.mount_state_write_guard().map_err(Error::from)?;
        let block_device = fs.immutable_block_device();
        let boot_region = fs.immutable_boot_region();
        if admission.forced_shutdown
            || admission.flags.clear_to_zero
            || admission.flags.media_failure
        {
            return_errno!(Errno::EIO);
        }
        if admission.options.fs_flags.contains(FsFlags::RDONLY) {
            return_errno!(Errno::EROFS);
        }

        let inode_state_guard = self.inode_state.write();
        let cluster_map_generation = self.current_cluster_map(&inode_state_guard)?;
        let cluster_map = cluster_map_generation.stream_extension();
        let (data_length, valid_data_length) = cluster_map_generation.validated_lengths()?;
        if new_size == data_length {
            return Ok(());
        }
        let page_cache = self.page_cache_handle();
        let timestamp = RealTimeCoarseClock::get().read_time();
        let resize_result = (|| {
            let mount_state = admission.state_guard.as_mut().ok_or_else(not_mounted)?;
            fs.publish_dirty_admission(mount_state)?;

            if new_size > data_length {
                let page_cache_result = if let Some(page_cache) = page_cache {
                    let mount_state = admission.state_guard.as_mut().ok_or_else(not_mounted)?;
                    self.grow_and_commit_regular_file(
                        &inode_state_guard,
                        &fs,
                        mount_state,
                        &block_device,
                        &boot_region,
                        &cluster_map_generation,
                        cluster_map,
                        new_size,
                        new_size,
                        new_size,
                        timestamp,
                        |_cluster_map, zero_fill_range| {
                            if new_size > data_length {
                                page_cache.resize(new_size, data_length)?;
                            }
                            Self::prepare_regular_file_page_cache_range(
                                page_cache,
                                data_length,
                                zero_fill_range.clone(),
                            )?;
                            if !zero_fill_range.is_empty() {
                                page_cache.fill_zeros(zero_fill_range)?;
                            }
                            Ok(())
                        },
                        || {},
                    )
                } else {
                    let mount_state = admission.state_guard.as_mut().ok_or_else(not_mounted)?;
                    self.grow_and_commit_regular_file(
                        &inode_state_guard,
                        &fs,
                        mount_state,
                        &block_device,
                        &boot_region,
                        &cluster_map_generation,
                        cluster_map,
                        new_size,
                        new_size,
                        new_size,
                        timestamp,
                        |cluster_map, zero_fill_range| {
                            if zero_fill_range.is_empty() {
                                return Ok(());
                            }

                            Self::mutate_regular_file_range(
                                &block_device,
                                &boot_region,
                                cluster_map,
                                new_size,
                                zero_fill_range.start,
                                zero_fill_range
                                    .end
                                    .checked_sub(zero_fill_range.start)
                                    .ok_or_else(|| Error::new(Errno::EINVAL))?,
                                |chunk| {
                                    chunk.fill(0);
                                    Ok(())
                                },
                            )
                        },
                        || {},
                    )
                };
                if let Err(error) = page_cache_result {
                    if let Some(page_cache) = page_cache {
                        let _ = page_cache.resize(data_length, new_size);
                    }
                    return Err(error);
                }
                return Ok(());
            }

            let current_ranges = cluster_map_generation.cluster_ranges();
            let retained_clusters = if new_size == 0 {
                0
            } else {
                new_size.div_ceil(boot_region.cluster_size)
            };
            let mut retained_clusters_remaining = retained_clusters;
            let mut retained_is_contiguous = true;
            let mut previous_retained_cluster: Option<u32> = None;
            let mut first_retained_cluster = 0u32;
            let mut released_ranges = Vec::new();
            for range in current_ranges {
                if retained_clusters_remaining == 0 {
                    released_ranges.push(*range);
                    continue;
                }

                let retained_in_range = retained_clusters_remaining.min(range.cluster_count);
                if retained_in_range != 0 {
                    let retained_last_cluster = range
                        .start_cluster
                        .checked_add(
                            u32::try_from(retained_in_range - 1)
                                .map_err(|_| invalid_on_disk_layout())?,
                        )
                        .ok_or_else(invalid_on_disk_layout)?;
                    if let Some(previous_retained_cluster) = previous_retained_cluster {
                        if previous_retained_cluster.checked_add(1) != Some(range.start_cluster) {
                            retained_is_contiguous = false;
                        }
                    } else {
                        first_retained_cluster = range.start_cluster;
                    }
                    previous_retained_cluster = Some(retained_last_cluster);
                }
                if retained_in_range < range.cluster_count {
                    let released_start_cluster = range
                        .start_cluster
                        .checked_add(
                            u32::try_from(retained_in_range)
                                .map_err(|_| invalid_on_disk_layout())?,
                        )
                        .ok_or_else(invalid_on_disk_layout)?;
                    released_ranges.push(ClusterRange {
                        start_cluster: released_start_cluster,
                        cluster_count: range.cluster_count - retained_in_range,
                    });
                }
                retained_clusters_remaining -= retained_in_range;
            }
            if retained_clusters_remaining != 0 {
                return Err(invalid_on_disk_layout());
            }

            let next_cluster_map = StreamExtensionDirEntry {
                data_length: Some(new_size),
                first_cluster: if retained_clusters == 0 {
                    0
                } else {
                    first_retained_cluster
                },
                valid_data_length: Some(valid_data_length.min(new_size)),
                no_fat_chain: retained_clusters != 0 && retained_is_contiguous,
            };
            let next_cluster_map_generation = self.cluster_map_for(next_cluster_map)?;
            let previous_page_cache_context = self.install_page_cache_context(
                &inode_state_guard,
                PageCacheContext {
                    cluster_map: next_cluster_map_generation,
                    data_length: new_size,
                    valid_data_length: valid_data_length.min(new_size),
                },
            );
            if let Some(page_cache) = page_cache {
                page_cache.resize(new_size, data_length)?;
            }
            let Some(next_valid_data_length) = next_cluster_map.valid_data_length else {
                return_errno!(Errno::EINVAL);
            };
            if next_valid_data_length > new_size {
                return_errno!(Errno::EINVAL);
            }
            if new_size == 0 {
                if next_cluster_map.first_cluster != 0 || next_valid_data_length != 0 {
                    return_errno!(Errno::EINVAL);
                }
            } else {
                boot_region
                    .validate_stream_data(
                        next_cluster_map.first_cluster,
                        u64::try_from(new_size).map_err(|_| Error::new(Errno::EINVAL))?,
                    )
                    .map_err(Error::from)?;
            }
            let allocated_sectors = retained_clusters
                .checked_mul(boot_region.sectors_per_cluster)
                .ok_or_else(|| Error::new(Errno::EINVAL))?;
            {
                let mut metadata = self.metadata.write();
                metadata.last_meta_change_at = timestamp;
                metadata.last_modify_at = timestamp;
                metadata.nr_sectors_allocated = allocated_sectors;
                metadata.size = new_size;
            }
            let retired_generation =
                self.replace_cluster_map(&inode_state_guard, next_cluster_map)?;
            self.mark_content_dirty(&inode_state_guard);
            if !cluster_map.no_fat_chain && retained_clusters != 0 {
                let retained_last_cluster =
                    previous_retained_cluster.ok_or_else(invalid_on_disk_layout)?;
                FatReader::new(block_device.as_ref(), &boot_region)
                    .terminate_cluster_chain(retained_last_cluster)
                    .map_err(Error::from)?;
            }

            if !released_ranges.is_empty() {
                fs.lazy_reclaim_clusters(retired_generation, released_ranges)?;
            }
            Ok(())
        })();
        if resize_result.is_err() {
            if let Some(mount_state) = admission.state_guard.as_mut() {
                mount_state.volume_flags.volume_dirty = true;
                mount_state.dirty_bracket_opened_by_mount = false;
            }
        }
        resize_result
    }

    fn prepare_regular_file_page_cache_range(
        page_cache: &PageCache,
        current_data_length: usize,
        range: Range<usize>,
    ) -> Result<()> {
        if range.is_empty() {
            return Ok(());
        }

        let vmo = page_cache.as_vmo().clone();
        let prepare_page = |page_idx: usize| -> Result<()> {
            let frame = vmo.commit_on(page_idx)?;
            frame.writer().fill_zeros(PAGE_SIZE);
            Ok(())
        };

        let start_page_idx = range.start / PAGE_SIZE;
        let start_page_offset = start_page_idx
            .checked_mul(PAGE_SIZE)
            .ok_or_else(|| Error::new(Errno::EINVAL))?;
        if !range.start.is_multiple_of(PAGE_SIZE) && start_page_offset >= current_data_length {
            prepare_page(start_page_idx)?;
        }

        if !range.end.is_multiple_of(PAGE_SIZE) {
            let end_page_idx = range.end / PAGE_SIZE;
            let end_page_offset = end_page_idx
                .checked_mul(PAGE_SIZE)
                .ok_or_else(|| Error::new(Errno::EINVAL))?;
            if end_page_offset >= current_data_length
                && (end_page_idx != start_page_idx || range.start.is_multiple_of(PAGE_SIZE))
            {
                prepare_page(end_page_idx)?;
            }
        }
        Ok(())
    }

    fn grow_and_commit_regular_file(
        &self,
        inode_state_guard: &InodeStateWriteGuard<'_>,
        fs: &Arc<ExfatFs>,
        mount_state: &mut MountedVolumeState,
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        cluster_map_generation: &Arc<ClusterMap>,
        cluster_map: StreamExtensionDirEntry,
        zero_fill_end: usize,
        new_data_length: usize,
        new_valid_data_length: usize,
        timestamp: Duration,
        apply_growth_fn: impl FnOnce(&StreamExtensionDirEntry, Range<usize>) -> Result<()>,
        rollback_growth_fn: impl FnOnce(),
    ) -> Result<()> {
        let mut previous_page_cache_context = None;
        let result = (|| {
            let Some(current_data_length) = cluster_map.data_length else {
                return_errno!(Errno::EINVAL);
            };
            let Some(current_valid_data_length) = cluster_map.valid_data_length else {
                return_errno!(Errno::EINVAL);
            };
            if current_valid_data_length > current_data_length
                || zero_fill_end > new_valid_data_length
                || new_valid_data_length > new_data_length
                || new_data_length < current_data_length
            {
                return_errno!(Errno::EINVAL);
            }
            let zero_fill_range =
                current_valid_data_length..current_valid_data_length.max(zero_fill_end);
            let current_allocated_clusters = if current_data_length == 0 {
                0
            } else {
                current_data_length.div_ceil(boot_region.cluster_size)
            };
            let target_allocated_clusters = new_data_length.div_ceil(boot_region.cluster_size);
            let additional_clusters = target_allocated_clusters
                .checked_sub(current_allocated_clusters)
                .ok_or_else(|| Error::new(Errno::EINVAL))?;
            let cluster_alloc_guard = if additional_clusters == 0 {
                None
            } else {
                let preferred_start_cluster = if current_allocated_clusters == 0 {
                    None
                } else {
                    Some(
                        cluster_map_generation
                            .mapped_cluster(boot_region, current_allocated_clusters - 1)?,
                    )
                            .and_then(|last_cluster| last_cluster.checked_add(1))
                            .filter(|cluster| boot_region.is_valid_cluster(*cluster))
                };
                let allocation_guard = ClusterAllocGuard::allocate(
                    fs,
                    mount_state,
                    additional_clusters,
                    preferred_start_cluster,
                )?;
                Some(allocation_guard)
            };
            let allocated_ranges = cluster_alloc_guard
                .as_ref()
                .map_or(&[][..], ClusterAllocGuard::ranges);

            let next_cluster_map = Self::grow_cluster_map(
                block_device,
                boot_region,
                cluster_map,
                new_data_length,
                allocated_ranges,
            )?;
            let next_cluster_map_generation = self.cluster_map_for(next_cluster_map)?;
            previous_page_cache_context = self.install_page_cache_context(
                inode_state_guard,
                PageCacheContext {
                    cluster_map: next_cluster_map_generation.clone(),
                    data_length: new_data_length,
                    valid_data_length: current_valid_data_length,
                },
            );
            apply_growth_fn(&next_cluster_map, zero_fill_range)?;
            let next_cluster_map = StreamExtensionDirEntry {
                valid_data_length: Some(new_valid_data_length),
                ..next_cluster_map
            };
            let _ = self.install_page_cache_context(
                inode_state_guard,
                PageCacheContext {
                    cluster_map: next_cluster_map_generation,
                    data_length: new_data_length,
                    valid_data_length: new_valid_data_length,
                },
            );
            if new_valid_data_length > new_data_length {
                return_errno!(Errno::EINVAL);
            }
            if new_data_length == 0 {
                if next_cluster_map.first_cluster != 0 || new_valid_data_length != 0 {
                    return_errno!(Errno::EINVAL);
                }
            } else {
                boot_region
                    .validate_stream_data(
                        next_cluster_map.first_cluster,
                        u64::try_from(new_data_length).map_err(|_| Error::new(Errno::EINVAL))?,
                    )
                    .map_err(Error::from)?;
            }
            let allocated_clusters = if new_data_length == 0 {
                0
            } else {
                new_data_length.div_ceil(boot_region.cluster_size)
            };
            let allocated_sectors = allocated_clusters
                .checked_mul(boot_region.sectors_per_cluster)
                .ok_or_else(|| Error::new(Errno::EINVAL))?;
            {
                let mut metadata = self.metadata.write();
                metadata.nr_sectors_allocated = allocated_sectors;
                metadata.last_meta_change_at = timestamp;
                metadata.last_modify_at = timestamp;
                metadata.size = new_data_length;
            }
            let _ = self.replace_cluster_map(inode_state_guard, next_cluster_map)?;
            self.mark_content_dirty(&inode_state_guard);
            if let Some(cluster_alloc_guard) = cluster_alloc_guard {
                cluster_alloc_guard.commit();
            }
            Ok(())
        })();
        if result.is_err() {
            *self.page_cache_context.write() = previous_page_cache_context;
            rollback_growth_fn();
        }
        result
    }

    // Cluster-map topology

    pub(super) fn grow_cluster_map(
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        cluster_map: StreamExtensionDirEntry,
        new_data_length: usize,
        allocated_ranges: &[ClusterRange],
    ) -> Result<StreamExtensionDirEntry> {
        let Some(current_data_length) = cluster_map.data_length else {
            return_errno!(Errno::EINVAL);
        };
        let Some(current_valid_data_length) = cluster_map.valid_data_length else {
            return_errno!(Errno::EINVAL);
        };
        if current_valid_data_length > current_data_length || new_data_length < current_data_length
        {
            return_errno!(Errno::EINVAL);
        }
        if new_data_length == current_data_length {
            return Ok(cluster_map);
        }

        let current_allocated_clusters = if current_data_length == 0 {
            0
        } else {
            current_data_length.div_ceil(boot_region.cluster_size)
        };
        let target_allocated_clusters = new_data_length.div_ceil(boot_region.cluster_size);
        if target_allocated_clusters == current_allocated_clusters {
            return Ok(StreamExtensionDirEntry {
                data_length: Some(new_data_length),
                ..cluster_map
            });
        }

        let additional_clusters = target_allocated_clusters
            .checked_sub(current_allocated_clusters)
            .ok_or_else(|| Error::new(Errno::EINVAL))?;
        let allocated_cluster_count =
            allocated_ranges
                .iter()
                .try_fold(0usize, |total_clusters, range| {
                    total_clusters
                        .checked_add(range.cluster_count)
                        .ok_or_else(inconsistent_bitmap_accounting)
                })?;
        if allocated_cluster_count != additional_clusters {
            return Err(inconsistent_bitmap_accounting());
        }
        if current_allocated_clusters == 0 {
            return Self::allocate_initial_regular_file_clusters(
                block_device,
                boot_region,
                cluster_map,
                new_data_length,
                &allocated_ranges,
            );
        }

        let first_new_cluster = allocated_ranges
            .first()
            .ok_or_else(inconsistent_bitmap_accounting)?
            .start_cluster;
        if cluster_map.no_fat_chain
            && allocated_ranges.len() == 1
            && cluster_map.first_cluster.checked_add(
                u32::try_from(current_allocated_clusters).map_err(|_| invalid_on_disk_layout())?,
            ) == Some(first_new_cluster)
        {
            return Ok(Self::extend_contiguous_regular_file_clusters(
                cluster_map,
                new_data_length,
            ));
        }

        Self::extend_fragmented_regular_file_clusters(
            block_device,
            boot_region,
            cluster_map,
            current_allocated_clusters,
            new_data_length,
            &allocated_ranges,
        )
    }

    fn allocate_initial_regular_file_clusters(
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        cluster_map: StreamExtensionDirEntry,
        new_data_length: usize,
        allocated_ranges: &[ClusterRange],
    ) -> Result<StreamExtensionDirEntry> {
        let first_new_cluster = allocated_ranges
            .first()
            .ok_or_else(inconsistent_bitmap_accounting)?
            .start_cluster;
        let is_single_contiguous_allocation = allocated_ranges.len() == 1;
        if !is_single_contiguous_allocation {
            let mut fat_reader = FatReader::new(block_device.as_ref(), boot_region);
            Self::link_allocated_cluster_ranges(&mut fat_reader, allocated_ranges)
                .map_err(Error::from)?;
        }
        Ok(StreamExtensionDirEntry {
            data_length: Some(new_data_length),
            first_cluster: first_new_cluster,
            no_fat_chain: is_single_contiguous_allocation,
            ..cluster_map
        })
    }

    fn extend_contiguous_regular_file_clusters(
        cluster_map: StreamExtensionDirEntry,
        new_data_length: usize,
    ) -> StreamExtensionDirEntry {
        StreamExtensionDirEntry {
            data_length: Some(new_data_length),
            ..cluster_map
        }
    }

    fn extend_fragmented_regular_file_clusters(
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        cluster_map: StreamExtensionDirEntry,
        current_allocated_clusters: usize,
        new_data_length: usize,
        allocated_ranges: &[ClusterRange],
    ) -> Result<StreamExtensionDirEntry> {
        let first_new_cluster = allocated_ranges
            .first()
            .ok_or_else(inconsistent_bitmap_accounting)?
            .start_cluster;
        let mut fat_reader = FatReader::new(block_device.as_ref(), boot_region);
        if cluster_map.no_fat_chain {
            fat_reader
                .link_contiguous_chain_to_cluster(
                    cluster_map.first_cluster,
                    current_allocated_clusters,
                    first_new_cluster,
                )
                .map_err(Error::from)?;
        } else {
            fat_reader
                .append_cluster_to_chain(cluster_map.first_cluster, first_new_cluster)
                .map_err(Error::from)?;
        }
        Self::link_allocated_cluster_ranges(&mut fat_reader, allocated_ranges)
            .map_err(Error::from)?;
        Ok(StreamExtensionDirEntry {
            data_length: Some(new_data_length),
            no_fat_chain: false,
            ..cluster_map
        })
    }

    fn link_allocated_cluster_ranges(
        fat_reader: &mut FatReader<'_>,
        allocated_ranges: &[ClusterRange],
    ) -> Result<()> {
        for (range_index, range) in allocated_ranges.iter().enumerate() {
            let next_range_start = allocated_ranges
                .get(range_index + 1)
                .map(|next_range| next_range.start_cluster);
            match (range.cluster_count, next_range_start) {
                (0, _) => return Err(invalid_operation_input()),
                (1, None) => fat_reader.terminate_cluster_chain(range.start_cluster)?,
                (cluster_count, None) => {
                    let last_cluster = range
                        .start_cluster
                        .checked_add(
                            u32::try_from(cluster_count - 1)
                                .map_err(|_| invalid_operation_input())?,
                        )
                        .ok_or_else(invalid_operation_input)?;
                    fat_reader.link_contiguous_chain_to_cluster(
                        range.start_cluster,
                        cluster_count - 1,
                        last_cluster,
                    )?;
                }
                (cluster_count, Some(next_cluster)) => {
                    fat_reader.link_contiguous_chain_to_cluster(
                        range.start_cluster,
                        cluster_count,
                        next_cluster,
                    )?;
                }
            }
        }
        Ok(())
    }

    // Cluster-level I/O helper

    pub(super) fn mutate_regular_file_range(
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        cluster_map: &StreamExtensionDirEntry,
        data_length: usize,
        offset: usize,
        len: usize,
        mut fill_chunk_fn: impl FnMut(&mut [u8]) -> Result<()>,
    ) -> Result<()> {
        if len == 0 {
            return Ok(());
        }

        Self::validate_regular_file_mapping_shape(boot_region, cluster_map, data_length)?;
        let write_end = offset
            .checked_add(len)
            .ok_or_else(|| Error::new(Errno::EINVAL))?;
        if write_end > data_length {
            return_errno!(Errno::EOPNOTSUPP);
        }

        let cluster_size = boot_region.cluster_size;
        let cluster_index = offset / cluster_size;
        let mut cluster_offset = offset % cluster_size;
        let mut remaining = len;
        let mut current_cluster = Self::mapped_regular_file_cluster(
            block_device,
            boot_region,
            cluster_map,
            data_length,
            cluster_index,
        )?;
        let mut fat_reader =
            (!cluster_map.no_fat_chain).then(|| FatReader::new(block_device.as_ref(), boot_region));
        let mut cluster_buffer = vec![0; cluster_size];
        while remaining != 0 {
            let chunk_len = remaining.min(cluster_size - cluster_offset);
            let chunk_end = cluster_offset
                .checked_add(chunk_len)
                .ok_or_else(|| Error::new(Errno::EINVAL))?;
            let cluster_start = boot_region
                .cluster_offset(current_cluster)
                .map_err(Error::from)?;
            block_device
                .read_bytes(cluster_start, &mut cluster_buffer)
                .map_err(|_| Error::new(Errno::EIO))?;
            fill_chunk_fn(&mut cluster_buffer[cluster_offset..chunk_end])?;
            block_device
                .write_bytes(cluster_start, &cluster_buffer)
                .map_err(|_| Error::new(Errno::EIO))?;
            remaining -= chunk_len;
            cluster_offset = 0;
            if remaining == 0 {
                break;
            }
            let using_fat_chain = fat_reader.is_some();
            current_cluster = match Self::advance_cluster(current_cluster, fat_reader.as_mut()) {
                Ok(Some(next_cluster)) => next_cluster,
                Ok(None) | Err(_) if using_fat_chain => return_errno!(Errno::EIO),
                Ok(None) | Err(_) => return_errno!(Errno::EINVAL),
            };
        }
        Ok(())
    }
}
