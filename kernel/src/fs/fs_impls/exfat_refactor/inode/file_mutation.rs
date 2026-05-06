// SPDX-License-Identifier: MPL-2.0

//! Implements regular-file writes, resizes, entry-set republish, and cluster-map growth.
//!
//! Method groups: write/resize entry points, shared growth publication, entry-set publication,
//! growth topology helpers, and cluster-level mutation.

use core::{ops::Range, time::Duration};

use aster_block::BlockDevice;
use ostd::mm::{VmIo, io::util::HasVmReaderWriter};

use super::{
    super::{
        bitmap::ClusterRange, boot::BootRegion, direntry, fat::FatReader,
        inconsistent_bitmap_accounting, invalid_on_disk_layout, invalid_operation_input,
        unpublished_state,
    },
    ExfatFs, ExfatInode, ExfatInodeClusterMap, InodeRewriteTarget, MountedVolumeState,
    page_backend::RegularFilePageCacheState,
    state::InodeStateWriteGuard,
};
use crate::{
    fs::{
        file::{InodeType, StatusFlags},
        vfs::{file_system::FsFlags, inode::Inode, page_cache::PageCache},
    },
    prelude::*,
    time::clocks::RealTimeCoarseClock,
    vm::vmo::{CommitFlags, get_page_idx_range},
};

impl ExfatInode {
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
            let admission = fs.admitted_mutation_state().map_err(Error::from)?;
            if admission.forced_shutdown
                || admission.anomaly.clear_to_zero
                || admission.anomaly.media_failure
            {
                return_errno!(Errno::EIO);
            }
            if admission.options.fs_flags.contains(FsFlags::RDONLY) {
                return_errno!(Errno::EROFS);
            }

            let inode_state_guard = self.inode_state.write();
            let cluster_map_generation =
                self.current_regular_file_cluster_map_generation(&inode_state_guard)?;
            let cluster_map = cluster_map_generation.cluster_map();
            let (data_length, valid_data_length) = cluster_map_generation.validated_lengths()?;

            let effective_offset = if status_flags.contains(StatusFlags::O_APPEND) {
                data_length
            } else {
                offset
            };
            let write_end = effective_offset
                .checked_add(write_len)
                .ok_or_else(|| Error::new(Errno::EINVAL))?;
            if status_flags.contains(StatusFlags::O_DIRECT)
                && (!effective_offset.is_multiple_of(admission.boot_region.sector_size)
                    || !write_len.is_multiple_of(admission.boot_region.sector_size))
            {
                return_errno!(Errno::EINVAL);
            }

            let cache_mutation_range = valid_data_length.min(effective_offset)..write_end;
            let published_data_length = data_length.max(write_end);
            let published_valid_data_length = valid_data_length.max(write_end);
            let timestamp = RealTimeCoarseClock::get().read_time();
            let _page_cache_state = self.install_regular_file_page_cache_state(
                &inode_state_guard,
                RegularFilePageCacheState {
                    anomaly: admission.anomaly,
                    block_device: admission.block_device.clone(),
                    boot_region: admission.boot_region,
                    cluster_map: cluster_map_generation.clone(),
                    data_length,
                    read_only: admission.options.fs_flags.contains(FsFlags::RDONLY),
                    valid_data_length,
                },
            );
            let write_result = (|| {
                if !cache_mutation_range.is_empty()
                    && page_cache.has_dirty_pages(cache_mutation_range.clone())
                {
                    page_cache.evict_range(cache_mutation_range.clone())?;
                }
                if published_data_length > data_length {
                    page_cache.resize(published_data_length)?;
                }
                Self::prepare_regular_file_page_cache_range(
                    page_cache,
                    data_length,
                    cache_mutation_range.clone(),
                )?;

                let mount_state = admission
                    .state_guard
                    .as_ref()
                    .ok_or_else(unpublished_state)?;
                let rollback_cache_mutation_range = cache_mutation_range.clone();
                self.grow_and_republish_regular_file(
                    &inode_state_guard,
                    &fs,
                    mount_state,
                    &admission.block_device,
                    &admission.boot_region,
                    cluster_map,
                    effective_offset,
                    published_data_length,
                    published_valid_data_length,
                    timestamp,
                    |_cluster_map, zero_fill_range| {
                        if !zero_fill_range.is_empty() {
                            page_cache.fill_zeros(zero_fill_range)?;
                        }
                        page_cache.pages().write(effective_offset, reader)?;
                        Ok(())
                    },
                    || {
                        if !rollback_cache_mutation_range.is_empty() {
                            page_cache.discard_range(rollback_cache_mutation_range);
                        }
                        if published_data_length > data_length {
                            let _ = page_cache.resize(data_length);
                        }
                    },
                )
            })();
            if let Err(error) = write_result {
                if published_data_length > data_length {
                    let _ = page_cache.resize(data_length);
                }
                return Err(error);
            }
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
        let admission = fs.admitted_mutation_state().map_err(Error::from)?;
        if admission.forced_shutdown
            || admission.anomaly.clear_to_zero
            || admission.anomaly.media_failure
        {
            return_errno!(Errno::EIO);
        }
        if admission.options.fs_flags.contains(FsFlags::RDONLY) {
            return_errno!(Errno::EROFS);
        }

        let inode_state_guard = self.inode_state.write();
        let cluster_map_generation =
            self.current_regular_file_cluster_map_generation(&inode_state_guard)?;
        let cluster_map = cluster_map_generation.cluster_map();
        let (data_length, valid_data_length) = cluster_map_generation.validated_lengths()?;
        if new_size == data_length {
            return Ok(());
        }
        let page_cache = self.page_cache_handle();
        let timestamp = RealTimeCoarseClock::get().read_time();

        if new_size > data_length {
            let mount_state = admission
                .state_guard
                .as_ref()
                .ok_or_else(unpublished_state)?;
            let page_cache_result = if let Some(page_cache) = page_cache {
                let _page_cache_state = self.install_regular_file_page_cache_state(
                    &inode_state_guard,
                    RegularFilePageCacheState {
                        anomaly: admission.anomaly,
                        block_device: admission.block_device.clone(),
                        boot_region: admission.boot_region,
                        cluster_map: cluster_map_generation.clone(),
                        data_length,
                        read_only: admission.options.fs_flags.contains(FsFlags::RDONLY),
                        valid_data_length,
                    },
                );
                self.grow_and_republish_regular_file(
                    &inode_state_guard,
                    &fs,
                    mount_state,
                    &admission.block_device,
                    &admission.boot_region,
                    cluster_map,
                    new_size,
                    new_size,
                    new_size,
                    timestamp,
                    |_cluster_map, zero_fill_range| {
                        if new_size > data_length {
                            page_cache.resize(new_size)?;
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
                self.grow_and_republish_regular_file(
                    &inode_state_guard,
                    &fs,
                    mount_state,
                    &admission.block_device,
                    &admission.boot_region,
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
                            &admission.block_device,
                            &admission.boot_region,
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
                    if valid_data_length < new_size {
                        page_cache.discard_range(valid_data_length..new_size);
                    }
                    let _ = page_cache.resize(data_length);
                }
                return Err(error);
            }
            return Ok(());
        }

        let current_ranges = cluster_map_generation.cluster_ranges();
        let retained_clusters = if new_size == 0 {
            0
        } else {
            new_size.div_ceil(admission.boot_region.cluster_size)
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
                        u32::try_from(retained_in_range).map_err(|_| invalid_on_disk_layout())?,
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

        let next_cluster_map = ExfatInodeClusterMap {
            data_length: Some(new_size),
            first_cluster: if retained_clusters == 0 {
                0
            } else {
                first_retained_cluster
            },
            valid_data_length: Some(valid_data_length.min(new_size)),
            no_fat_chain: retained_clusters != 0 && retained_is_contiguous,
        };
        if let Some(page_cache) = page_cache {
            page_cache.resize(new_size)?;
        }
        if let Err(error) = self.republish_regular_file_entry_set(
            &admission.block_device,
            &admission.boot_region,
            next_cluster_map,
            timestamp,
        ) {
            if let Some(page_cache) = page_cache {
                let _ = page_cache.resize(data_length);
            }
            return Err(error);
        }

        let allocated_sectors = retained_clusters
            .checked_mul(admission.boot_region.sectors_per_cluster)
            .ok_or_else(|| Error::new(Errno::EINVAL))?;
        {
            let mut metadata = self.metadata.write();
            metadata.last_meta_change_at = timestamp;
            metadata.last_modify_at = timestamp;
            metadata.nr_sectors_allocated = allocated_sectors;
            metadata.size = new_size;
        }
        let retired_generation =
            self.replace_regular_file_cluster_map(&inode_state_guard, next_cluster_map)?;
        self.mark_content_publication_dirty(&inode_state_guard);
        if !cluster_map.no_fat_chain && retained_clusters != 0 {
            let retained_last_cluster =
                previous_retained_cluster.ok_or_else(invalid_on_disk_layout)?;
            FatReader::new(admission.block_device.as_ref(), &admission.boot_region)
                .terminate_cluster_chain(retained_last_cluster)
                .map_err(Error::from)?;
        }

        if !released_ranges.is_empty() {
            fs.retire_regular_file_clusters(retired_generation, released_ranges)?;
        }
        Ok(())
    }

    fn prepare_regular_file_page_cache_range(
        page_cache: &PageCache,
        current_data_length: usize,
        range: Range<usize>,
    ) -> Result<()> {
        for page_idx in get_page_idx_range(&range) {
            let page_offset = page_idx
                .checked_mul(PAGE_SIZE)
                .ok_or_else(|| Error::new(Errno::EINVAL))?;
            let commit_flags = if page_offset >= current_data_length {
                CommitFlags::WILL_OVERWRITE
            } else {
                CommitFlags::empty()
            };
            let frame = page_cache.pages().commit_on(page_idx, commit_flags)?;
            if page_offset >= current_data_length {
                frame.writer().fill_zeros(PAGE_SIZE);
            }
        }
        Ok(())
    }

    fn grow_and_republish_regular_file(
        &self,
        inode_state_guard: &InodeStateWriteGuard<'_>,
        fs: &Arc<ExfatFs>,
        mount_state: &MountedVolumeState,
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        cluster_map: ExfatInodeClusterMap,
        zero_fill_end: usize,
        new_data_length: usize,
        new_valid_data_length: usize,
        timestamp: Duration,
        apply_growth_fn: impl FnOnce(&ExfatInodeClusterMap, Range<usize>) -> Result<()>,
        rollback_growth_fn: impl FnOnce(),
    ) -> Result<()> {
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

            let mut next_cluster_map = Self::grow_regular_file_cluster_map(
                fs,
                mount_state,
                block_device,
                boot_region,
                cluster_map,
                new_data_length,
            )?;
            apply_growth_fn(&next_cluster_map, zero_fill_range)?;

            next_cluster_map.data_length = Some(new_data_length);
            next_cluster_map.valid_data_length = Some(new_valid_data_length);
            self.republish_regular_file_entry_set(
                block_device,
                boot_region,
                next_cluster_map,
                timestamp,
            )?;

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
            let _ = self.replace_regular_file_cluster_map(inode_state_guard, next_cluster_map)?;
            self.mark_content_publication_dirty(&inode_state_guard);
            Ok(())
        })();
        if result.is_err() {
            rollback_growth_fn();
        }
        result
    }

    // Entry-set publication

    pub(super) fn republish_regular_file_entry_set(
        &self,
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        cluster_map: ExfatInodeClusterMap,
        timestamp: Duration,
    ) -> Result<()> {
        let Some(data_length) = cluster_map.data_length else {
            return_errno!(Errno::EINVAL);
        };
        let Some(valid_data_length) = cluster_map.valid_data_length else {
            return_errno!(Errno::EINVAL);
        };
        if valid_data_length > data_length {
            return_errno!(Errno::EINVAL);
        }
        if data_length == 0 {
            if cluster_map.first_cluster != 0 || valid_data_length != 0 {
                return_errno!(Errno::EINVAL);
            }
        } else {
            boot_region
                .validate_stream_data(
                    cluster_map.first_cluster,
                    u64::try_from(data_length).map_err(|_| Error::new(Errno::EINVAL))?,
                )
                .map_err(Error::from)?;
        }

        self.rewrite_inode_entry_set(
            InodeRewriteTarget::RegularFile,
            block_device,
            boot_region,
            |entry_view| {
                let valid_data_length =
                    u64::try_from(valid_data_length).map_err(|_| Error::new(Errno::EINVAL))?;
                let data_length =
                    u64::try_from(data_length).map_err(|_| Error::new(Errno::EINVAL))?;
                let (timestamp_bytes, hundredths_increment, encoded_utc_offset_byte) =
                    Self::encoded_exfat_timestamp_fields(
                        timestamp,
                        entry_view.last_modified_timestamp().utc_offset_byte(),
                    )?;
                let entry_cluster_map = direntry::FileEntryClusterMap::new(
                    cluster_map.first_cluster,
                    data_length,
                    valid_data_length,
                    cluster_map.no_fat_chain,
                )
                .map_err(Error::from)?;
                let mut republished_entry_set = entry_view.republished();
                republished_entry_set.set_cluster_map(entry_cluster_map);
                republished_entry_set.set_last_modified_timestamp(
                    direntry::FileEntryTimestamp::new(
                        timestamp_bytes,
                        Some(hundredths_increment),
                        encoded_utc_offset_byte,
                    ),
                );
                Ok(Some(republished_entry_set.into_bytes()))
            },
            |_| {},
        )?;
        Ok(())
    }

    // Cluster-map topology

    pub(super) fn grow_regular_file_cluster_map(
        fs: &Arc<ExfatFs>,
        mount_state: &MountedVolumeState,
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        cluster_map: ExfatInodeClusterMap,
        new_data_length: usize,
    ) -> Result<ExfatInodeClusterMap> {
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
            return Ok(ExfatInodeClusterMap {
                data_length: Some(new_data_length),
                ..cluster_map
            });
        }

        let additional_clusters = target_allocated_clusters
            .checked_sub(current_allocated_clusters)
            .ok_or_else(|| Error::new(Errno::EINVAL))?;
        let allocated_ranges =
            fs.allocate_free_space_with_publication(mount_state, additional_clusters)?;
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
        cluster_map: ExfatInodeClusterMap,
        new_data_length: usize,
        allocated_ranges: &[ClusterRange],
    ) -> Result<ExfatInodeClusterMap> {
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
        Ok(ExfatInodeClusterMap {
            data_length: Some(new_data_length),
            first_cluster: first_new_cluster,
            no_fat_chain: is_single_contiguous_allocation,
            ..cluster_map
        })
    }

    fn extend_contiguous_regular_file_clusters(
        cluster_map: ExfatInodeClusterMap,
        new_data_length: usize,
    ) -> ExfatInodeClusterMap {
        ExfatInodeClusterMap {
            data_length: Some(new_data_length),
            ..cluster_map
        }
    }

    fn extend_fragmented_regular_file_clusters(
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        cluster_map: ExfatInodeClusterMap,
        current_allocated_clusters: usize,
        new_data_length: usize,
        allocated_ranges: &[ClusterRange],
    ) -> Result<ExfatInodeClusterMap> {
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
        Ok(ExfatInodeClusterMap {
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
        cluster_map: &ExfatInodeClusterMap,
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
