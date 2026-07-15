// SPDX-License-Identifier: MPL-2.0

//! Implements regular-file writes, resizes, shared entry-set rewrite, and cluster-map growth.
//!
//! Method groups: write/resize entry points, shared growth commit, entry-set rewrite,
//! growth topology helpers, and cluster-level mutation.

mod cluster_io;
mod cluster_map_growth;
mod page_cache_growth;

use core::{ops::Range, time::Duration};

use align_ext::AlignExt;
use aster_block::BlockDevice;
use ostd::mm::VmIo;

use super::{
    super::{
        bitmap::ClusterRange, boot::BootRegion, fat::FatReader,
        bitmap::AllocGuard,
        fs::{ExfatFs, FsState},
        inconsistent_bitmap_accounting, invalid_on_disk_layout,
    },
    ClusterMap, ExfatInode, StreamExtensionDirEntry, state::InodeStateWriteGuard,
    sync::InodeSyncScope,
};
use crate::{
    fs::{
        file::{InodeType, StatusFlags},
        vfs::file_system::FsFlags,
    },
    prelude::*,
    time::clocks::RealTimeCoarseClock,
};

impl ExfatInode {
    // VFS entry points

    pub(super) fn write_at_impl(
        &self,
        offset: usize,
        reader: &mut VmReader,
        status_flags: StatusFlags,
    ) -> Result<usize> {
        let fs = self
            .fs
            .upgrade()
            .ok_or_else(|| Error::with_message(Errno::EIO, "exFAT filesystem is not mounted"))?;
        let requested_write_len = reader.remain();
        let mut completed_write_len = 0;
        {
            let mut fs_state = fs.fs_state.write();
            let block_device = fs.immutable_block_device();
            let boot_region = fs.immutable_boot_region();
            let mount_state = fs_state.mount_state.as_ref().ok_or_else(super::super::not_mounted)?;
            if mount_state.forced_shutdown
                || mount_state.volume_flags.clear_to_zero
                || mount_state.volume_flags.media_failure
            {
                return_errno!(Errno::EIO);
            }
            if mount_state.options.fs_flags.contains(FsFlags::RDONLY) {
                return_errno!(Errno::EROFS);
            }

            let write_result = (|| {
                let sync_scope = if status_flags.contains(StatusFlags::O_SYNC) {
                    Some(InodeSyncScope::All)
                } else if status_flags.contains(StatusFlags::O_DSYNC) {
                    Some(InodeSyncScope::Data)
                } else {
                    None
                };
                let parent = if sync_scope.is_some() {
                    self.inode_state_read_guard().parent()
                } else {
                    None
                };
                let mut guarded_inodes = vec![self];
                if let Some(parent) = parent.as_ref() {
                    guarded_inodes.push(parent.as_ref());
                }
                let inode_guards = Self::directory_write_guards_by_ino(guarded_inodes);
                let inode_state_guard = inode_guards
                    .iter()
                    .find(|guard| guard.guards_inode(self))
                    .ok_or_else(|| Error::new(Errno::EINVAL))?;
                match inode_state_guard.metadata().type_ {
                    InodeType::Dir => return_errno!(Errno::EISDIR),
                    InodeType::File => {}
                    _ => return_errno!(Errno::EOPNOTSUPP),
                }
                if !reader.has_remain() {
                    return Ok(());
                }
                let parent_inode_state_guard = match parent.as_ref() {
                    Some(parent) => Some(
                        inode_guards
                            .iter()
                            .find(|guard| guard.guards_inode(parent.as_ref()))
                            .ok_or_else(|| Error::new(Errno::EINVAL))?,
                    ),
                    None => None,
                };
                if sync_scope.is_some() {
                    let parent_is_revalidated = match (parent.as_ref(), inode_state_guard.parent()) {
                        (Some(discovered_parent), Some(admitted_parent)) => {
                            Arc::ptr_eq(discovered_parent, &admitted_parent)
                        }
                        (None, None) => true,
                        (Some(_), None) | (None, Some(_)) => false,
                    };
                    if !parent_is_revalidated {
                        return_errno!(Errno::EIO);
                    }
                }
                let page_cache = self
                    .page_cache_handle(inode_state_guard.metadata())
                    .ok_or_else(|| {
                        Error::with_message(Errno::EIO, "regular exFAT file has no page cache")
                    })?;
                let mut allocation_guard = fs.allocation_guard()?;
                let cluster_map_generation =
                    self.current_cluster_map(inode_state_guard, &allocation_guard)?;
                let (data_length, valid_data_length) =
                    cluster_map_generation.validated_lengths()?;

                let effective_offset = if status_flags.contains(StatusFlags::O_APPEND) {
                    data_length
                } else {
                    offset
                };
                effective_offset
                    .checked_add(requested_write_len)
                    .ok_or_else(|| Error::new(Errno::EINVAL))?;
                if status_flags.contains(StatusFlags::O_DIRECT)
                    && (!effective_offset.is_multiple_of(boot_region.sector_size)
                        || !requested_write_len.is_multiple_of(boot_region.sector_size))
                {
                    return_errno!(Errno::EINVAL);
                }

                let mut staged_bytes = vec![0; requested_write_len];
                let mut staged_source = reader.clone();
                let (staged_len, staging_error) = match staged_source
                    .read_fallible(&mut VmWriter::from(staged_bytes.as_mut_slice()))
                {
                    Ok(staged_len) => (staged_len, None),
                    Err((error, staged_len)) if staged_len != 0 => (staged_len, Some(error)),
                    Err((error, _)) => return Err(error.into()),
                };
                let write_len = if staging_error.is_some()
                    && status_flags.contains(StatusFlags::O_DIRECT)
                {
                    staged_len / boot_region.sector_size * boot_region.sector_size
                } else {
                    staged_len
                };
                if write_len == 0 {
                    if let Some(error) = staging_error {
                        return Err(error.into());
                    }
                    return_errno!(Errno::EIO);
                }
                let write_end = effective_offset
                    .checked_add(write_len)
                    .ok_or_else(|| Error::new(Errno::EINVAL))?;
                let mut staged_reader =
                    VmReader::from(&staged_bytes[..write_len]).to_fallible();
                let new_data_length = data_length.max(write_end);
                let new_valid_data_length = valid_data_length.max(write_end);
                let timestamp = RealTimeCoarseClock::get().read_time();
                let write_result = (|| {
                    fs.publish_dirty_admission(&mut fs_state)?;
                    self.grow_and_commit_regular_file(
                        inode_state_guard,
                        &mut fs_state,
                        &mut allocation_guard,
                        &block_device,
                        &boot_region,
                        &cluster_map_generation,
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
                                .write(effective_offset, &mut staged_reader)
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
                write_result?;
                completed_write_len = write_len;
                if let Some(sync_scope) = sync_scope {
                    self.sync_regular_file_with_proofs(
                        fs.as_ref(),
                        &mut fs_state,
                        sync_scope,
                        inode_state_guard,
                        parent_inode_state_guard,
                        &mut allocation_guard,
                    )?;
                }
                Ok(())
            })();
            if write_result.is_err() {
                ExfatFs::mark_mount_dirty_after_failure(&mut fs_state);
            }
            write_result?;
        }
        reader.skip(completed_write_len);
        Ok(completed_write_len)
    }

    pub(super) fn resize_impl(&self, new_size: usize) -> Result<()> {
        let fs = self
            .fs
            .upgrade()
            .ok_or_else(|| Error::with_message(Errno::EIO, "exFAT filesystem is not mounted"))?;
        let mut fs_state = fs.fs_state.write();
        let block_device = fs.immutable_block_device();
        let boot_region = fs.immutable_boot_region();
        let mount_state = fs_state.mount_state.as_ref().ok_or_else(super::super::not_mounted)?;
        if mount_state.forced_shutdown
            || mount_state.volume_flags.clear_to_zero
            || mount_state.volume_flags.media_failure
        {
            return_errno!(Errno::EIO);
        }
        if mount_state.options.fs_flags.contains(FsFlags::RDONLY) {
            return_errno!(Errno::EROFS);
        }

        let inode_state_guard = self.inode_state_write_guard();
        match inode_state_guard.metadata().type_ {
            InodeType::Dir => return_errno!(Errno::EISDIR),
            InodeType::File => {}
            _ => return_errno!(Errno::EOPNOTSUPP),
        }
        let mut allocation_guard = fs.allocation_guard()?;
        let cluster_map_generation =
            self.current_cluster_map(&inode_state_guard, &allocation_guard)?;
        let cluster_map = cluster_map_generation.stream_extension();
        let (data_length, valid_data_length) = cluster_map_generation.validated_lengths()?;
        if new_size == data_length {
            return Ok(());
        }
        let page_cache = self.page_cache_handle(inode_state_guard.metadata());
        let timestamp = RealTimeCoarseClock::get().read_time();
        let resize_result = (|| {
            fs.publish_dirty_admission(&mut fs_state)?;

            if new_size > data_length {
                let page_cache_result = if let Some(page_cache) = page_cache {
                    self.grow_and_commit_regular_file(
                        &inode_state_guard,
                        &mut fs_state,
                        &mut allocation_guard,
                        &block_device,
                        &boot_region,
                        &cluster_map_generation,
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
                        || {
                            let _ = page_cache.resize(data_length, new_size);
                        },
                    )
                } else {
                    self.grow_and_commit_regular_file(
                        &inode_state_guard,
                        &mut fs_state,
                        &mut allocation_guard,
                        &block_device,
                        &boot_region,
                        &cluster_map_generation,
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
                page_cache_result?;
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
            let mut retained_ranges = Vec::new();
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
                    retained_ranges.push(ClusterRange {
                        start_cluster: range.start_cluster,
                        cluster_count: retained_in_range,
                    });
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
            let next_cluster_map_generation = Arc::new(ClusterMap::from_stream_and_ranges(
                &boot_region,
                next_cluster_map,
                retained_ranges,
            )?);
            let page_cache_context = self.page_cache_context_for_mapping(
                inode_state_guard.metadata(),
                next_cluster_map_generation.clone(),
                new_size,
                valid_data_length.min(new_size),
            )?;
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
                    ?;
            }
            let allocated_sectors = retained_clusters
                .checked_mul(boot_region.sectors_per_cluster)
                .ok_or_else(|| Error::new(Errno::EINVAL))?;

            let mut partial_page_rollback = None;
            if let Some(page_cache) = page_cache {
                let partial_page_end = data_length.min(new_size.align_up(PAGE_SIZE));
                if new_size < partial_page_end {
                    let mut old_bytes = vec![0; partial_page_end - new_size];
                    let mut writer = VmWriter::from(old_bytes.as_mut_slice()).to_fallible();
                    page_cache
                        .read(new_size, &mut writer)
                        .map_err(Error::from)?;
                    let page_idx = new_size / PAGE_SIZE;
                    let page_start = page_idx
                        .checked_mul(PAGE_SIZE)
                        .ok_or_else(|| Error::new(Errno::EINVAL))?;
                    let page_end = page_start.saturating_add(PAGE_SIZE).min(page_cache.size());
                    let was_dirty = page_cache.has_dirty_pages(page_start..page_end);
                    partial_page_rollback = Some((page_idx, old_bytes, was_dirty));
                }
                page_cache.resize(new_size, data_length)?;
            }

            if !cluster_map.no_fat_chain && retained_clusters != 0 {
                let retained_last_cluster =
                    previous_retained_cluster.ok_or_else(invalid_on_disk_layout)?;
                if let Err(error) = FatReader::new(block_device.as_ref(), &boot_region)
                    .terminate_cluster_chain(retained_last_cluster)
                {
                    if let Some(page_cache) = page_cache {
                        let rollback_result: Result<()> = (|| {
                            page_cache.resize(data_length, new_size)?;
                            if let Some((page_idx, old_bytes, was_dirty)) =
                                partial_page_rollback.as_ref()
                            {
                                page_cache.restore_prefaulted_pages([(
                                    *page_idx,
                                    (new_size % PAGE_SIZE)
                                        ..(new_size % PAGE_SIZE + old_bytes.len()),
                                    old_bytes.as_slice(),
                                    *was_dirty,
                                )])?;
                            }
                            Ok(())
                        })();
                        if rollback_result.is_err() {
                            fs.latch_forced_shutdown(&mut fs_state);
                        }
                    }
                    return Err(error);
                }
            }

            inode_state_guard.with_metadata_mut(|metadata| {
                metadata.last_meta_change_at = timestamp;
                metadata.last_modify_at = timestamp;
                metadata.nr_sectors_allocated = allocated_sectors;
                metadata.size = new_size;
            });
            let retired_generation = self.replace_cluster_map(
                &inode_state_guard,
                &cluster_map_generation,
                next_cluster_map_generation,
                page_cache_context,
            );
            self.mark_content_dirty(&inode_state_guard);

            if !released_ranges.is_empty() {
                allocation_guard.lazy_reclaim_clusters(retired_generation, released_ranges)?;
            }
            Ok(())
        })();
        if resize_result.is_err() {
            ExfatFs::mark_mount_dirty_after_failure(&mut fs_state);
        }
        resize_result
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "Regular-file growth spans allocation exposure, page-cache publication, and rollback closures, so the live guard/state inputs stay separate instead of being hidden in a carrier."
    )]
    fn grow_and_commit_regular_file(
        &self,
        inode_state_guard: &InodeStateWriteGuard<'_>,
        fs_state: &mut FsState,
        allocation_guard: &mut AllocGuard<'_>,
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        cluster_map_generation: &Arc<ClusterMap>,
        zero_fill_end: usize,
        new_data_length: usize,
        new_valid_data_length: usize,
        timestamp: Duration,
        apply_growth_fn: impl FnOnce(&ClusterMap, Range<usize>) -> Result<()>,
        rollback_growth_fn: impl FnOnce(),
    ) -> Result<()> {
        let mut previous_page_cache_context = None;
        let mut publication_complete = false;
        let result = (|| {
            let cluster_map = cluster_map_generation.stream_extension();
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
            let has_allocation = if additional_clusters == 0 {
                false
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
                allocation_guard.allocate(additional_clusters, preferred_start_cluster)?;
                true
            };
            let allocated_ranges = if has_allocation {
                allocation_guard.ranges()
            } else {
                &[]
            };

            let next_cluster_map_generation = Arc::new(Self::grow_cluster_map(
                boot_region,
                cluster_map_generation,
                new_data_length,
                allocated_ranges,
            )?);
            previous_page_cache_context = inode_state_guard.replace_page_cache_context(
                self.page_cache_context_for_mapping(
                    inode_state_guard.metadata(),
                    next_cluster_map_generation.clone(),
                    new_data_length,
                    current_valid_data_length,
                )?,
            );
            apply_growth_fn(next_cluster_map_generation.as_ref(), zero_fill_range)?;
            let next_cluster_map = StreamExtensionDirEntry {
                valid_data_length: Some(new_valid_data_length),
                ..next_cluster_map_generation.stream_extension()
            };
            let published_cluster_map_generation = Arc::new(ClusterMap::from_stream_and_ranges(
                boot_region,
                next_cluster_map,
                next_cluster_map_generation.cluster_ranges().to_vec(),
            )?);
            let published_page_cache_context = self.page_cache_context_for_mapping(
                inode_state_guard.metadata(),
                published_cluster_map_generation.clone(),
                new_data_length,
                new_valid_data_length,
            )?;
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
                    ?;
            }
            let allocated_clusters = if new_data_length == 0 {
                0
            } else {
                new_data_length.div_ceil(boot_region.cluster_size)
            };
            let allocated_sectors = allocated_clusters
                .checked_mul(boot_region.sectors_per_cluster)
                .ok_or_else(|| Error::new(Errno::EINVAL))?;
            if cluster_map_generation.stream_extension() != inode_state_guard.dir_entry_stream() {
                return Err(invalid_on_disk_layout());
            }

            let mut exposure_error = None;
            if has_allocation {
                let first_new_cluster = allocated_ranges
                    .first()
                    .ok_or_else(inconsistent_bitmap_accounting)?
                    .start_cluster;
                let mut fat_reader = FatReader::new(block_device.as_ref(), boot_region);
                if current_allocated_clusters == 0 {
                    if allocated_ranges.len() != 1 {
                        Self::link_allocated_cluster_ranges(&mut fat_reader, allocated_ranges)
                            ?;
                    }
                } else if !cluster_map.no_fat_chain
                    || allocated_ranges.len() != 1
                    || cluster_map.first_cluster.checked_add(
                        u32::try_from(current_allocated_clusters)
                            .map_err(|_| invalid_on_disk_layout())?,
                    ) != Some(first_new_cluster)
                {
                    Self::link_allocated_cluster_ranges(&mut fat_reader, allocated_ranges)
                        ?;
                    if cluster_map.no_fat_chain {
                        fat_reader
                            .link_contiguous_chain_to_prepared_cluster(
                                cluster_map.first_cluster,
                                current_allocated_clusters,
                                first_new_cluster,
                            )
                            ?;
                    } else {
                        let current_tail_cluster = cluster_map_generation
                            .terminal_cluster(boot_region)?
                            .ok_or_else(|| Error::new(Errno::EINVAL))?;
                        exposure_error = fat_reader
                            .link_prepared_chain_to_tail(
                                current_tail_cluster,
                                first_new_cluster,
                            )
                            ?
                            .err();
                    }
                }
            }

            inode_state_guard.with_metadata_mut(|metadata| {
                metadata.nr_sectors_allocated = allocated_sectors;
                metadata.last_meta_change_at = timestamp;
                metadata.last_modify_at = timestamp;
                metadata.size = new_data_length;
            });
            let _ = self.replace_cluster_map(
                inode_state_guard,
                cluster_map_generation,
                published_cluster_map_generation,
                published_page_cache_context,
            );
            self.mark_content_dirty(inode_state_guard);
            if has_allocation {
                allocation_guard.commit_allocation();
            }
            publication_complete = true;
            exposure_error.map_or(Ok(()), Err)
        })();
        if result.is_err() && !publication_complete {
            inode_state_guard.restore_page_cache_context(previous_page_cache_context);
            rollback_growth_fn();
            if allocation_guard.rollback_allocation()? {
                ExfatFs::disable_unsupported_discard_after_release(fs_state);
            }
        }
        result
    }

}
