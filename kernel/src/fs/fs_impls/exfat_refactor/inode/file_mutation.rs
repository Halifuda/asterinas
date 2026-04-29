// SPDX-License-Identifier: MPL-2.0

use core::time::Duration;

use aster_block::BlockDevice;
use ostd::mm::VmIo;

use super::{
    super::{
        bitmap::ClusterRange,
        boot::BootRegion,
        direntry,
        fat::FatReader,
        fs::{ExfatFsError, ExfatMountOptions},
    },
    ExfatFs, ExfatInode, ExfatInodeStream, InodeRewriteTarget, MountedVolumeState,
};
use crate::{
    fs::{
        file::{InodeType, StatusFlags},
        vfs::{file_system::FsFlags, inode::Inode},
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
        if !reader.has_remain() {
            return Ok(0);
        }

        let write_len = reader.remain();
        {
            let _owner_guard = self.admission.write();
            let mut stream = self.stream.write();
            let Some(data_length) = stream.data_length else {
                return_errno!(Errno::EINVAL);
            };
            let Some(valid_data_length) = stream.valid_data_length else {
                return_errno!(Errno::EINVAL);
            };
            if valid_data_length > data_length {
                return_errno!(Errno::EINVAL);
            }

            let effective_offset = if status_flags.contains(StatusFlags::O_APPEND) {
                data_length
            } else {
                offset
            };
            if status_flags.contains(StatusFlags::O_DIRECT)
                && (!effective_offset.is_multiple_of(admission.boot_region.sector_size)
                    || !write_len.is_multiple_of(admission.boot_region.sector_size))
            {
                return_errno!(Errno::EINVAL);
            }
            let write_end = effective_offset
                .checked_add(write_len)
                .ok_or_else(|| Error::new(Errno::EINVAL))?;
            let publication = admission
                .state_guard
                .as_ref()
                .ok_or(ExfatFsError::UnpublishedState)
                .map_err(Error::from)?;
            let published_data_length = data_length.max(write_end);
            let mut published_stream = if write_end > data_length {
                Self::grow_regular_file_stream(
                    &fs,
                    publication,
                    &admission.block_device,
                    &admission.boot_region,
                    *stream,
                    published_data_length,
                )?
            } else {
                *stream
            };

            let zero_fill_len = if effective_offset > valid_data_length {
                effective_offset
                    .checked_sub(valid_data_length)
                    .ok_or_else(|| Error::new(Errno::EINVAL))?
            } else {
                0
            };
            if zero_fill_len != 0 {
                Self::mutate_regular_file_range(
                    &admission.block_device,
                    &admission.boot_region,
                    &published_stream,
                    published_data_length,
                    valid_data_length,
                    zero_fill_len,
                    |chunk| {
                        chunk.fill(0);
                        Ok(())
                    },
                )?;
            }
            Self::mutate_regular_file_range(
                &admission.block_device,
                &admission.boot_region,
                &published_stream,
                published_data_length,
                effective_offset,
                write_len,
                |chunk| {
                    let mut writer = VmWriter::from(chunk).to_fallible();
                    writer.write_fallible(reader)?;
                    Ok(())
                },
            )?;

            let published_valid_data_length = valid_data_length.max(write_end);
            let timestamp = RealTimeCoarseClock::get().read_time();
            published_stream.data_length = Some(published_data_length);
            published_stream.valid_data_length = Some(published_valid_data_length);
            let page_cache = self
                .page_cache
                .get()
                .and_then(|maybe_page_cache| maybe_page_cache.as_ref());
            let cache_invalidate_start = valid_data_length.min(effective_offset);
            if let Some(page_cache) = page_cache {
                if published_data_length > data_length {
                    page_cache.resize(published_data_length)?;
                }
                page_cache.discard_range(cache_invalidate_start..write_end);
            }
            if let Err(error) = self.republish_regular_file_entry_set(
                &admission.block_device,
                &admission.boot_region,
                published_stream,
                timestamp,
            ) {
                if published_data_length > data_length {
                    if let Some(page_cache) = page_cache {
                        let _ = page_cache.resize(data_length);
                    }
                }
                return Err(error);
            }

            let allocated_clusters = if published_data_length == 0 {
                0
            } else {
                published_data_length.div_ceil(admission.boot_region.cluster_size)
            };
            let allocated_sectors = allocated_clusters
                .checked_mul(admission.boot_region.sectors_per_cluster)
                .ok_or_else(|| Error::new(Errno::EINVAL))?;

            {
                let mut metadata = self.metadata.write();
                metadata.nr_sectors_allocated = allocated_sectors;
                metadata.last_meta_change_at = timestamp;
                metadata.last_modify_at = timestamp;
                metadata.size = published_data_length;
            }
            *stream = published_stream;
            self.mark_content_publication_dirty();
        }
        drop(admission);
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
        let mut admission = fs.admitted_mutation_state().map_err(Error::from)?;
        if admission.forced_shutdown
            || admission.anomaly.clear_to_zero
            || admission.anomaly.media_failure
        {
            return_errno!(Errno::EIO);
        }
        if admission.options.fs_flags.contains(FsFlags::RDONLY) {
            return_errno!(Errno::EROFS);
        }

        let _owner_guard = self.admission.write();
        let mut stream = self.stream.write();
        let Some(data_length) = stream.data_length else {
            return_errno!(Errno::EINVAL);
        };
        let Some(valid_data_length) = stream.valid_data_length else {
            return_errno!(Errno::EINVAL);
        };
        if valid_data_length > data_length {
            return_errno!(Errno::EINVAL);
        }
        if new_size == data_length {
            return Ok(());
        }
        let page_cache = self
            .page_cache
            .get()
            .and_then(|maybe_page_cache| maybe_page_cache.as_ref());
        let timestamp = RealTimeCoarseClock::get().read_time();

        if new_size > data_length {
            let publication = admission
                .state_guard
                .as_ref()
                .ok_or(ExfatFsError::UnpublishedState)
                .map_err(Error::from)?;
            let mut published_stream = Self::grow_regular_file_stream(
                &fs,
                publication,
                &admission.block_device,
                &admission.boot_region,
                *stream,
                new_size,
            )?;
            if valid_data_length < new_size {
                Self::mutate_regular_file_range(
                    &admission.block_device,
                    &admission.boot_region,
                    &published_stream,
                    new_size,
                    valid_data_length,
                    new_size
                        .checked_sub(valid_data_length)
                        .ok_or_else(|| Error::new(Errno::EINVAL))?,
                    |chunk| {
                        chunk.fill(0);
                        Ok(())
                    },
                )?;
            }
            published_stream.valid_data_length = Some(new_size);
            if let Some(page_cache) = page_cache {
                page_cache.resize(new_size)?;
                page_cache.discard_range(valid_data_length..new_size);
            }
            if let Err(error) = self.republish_regular_file_entry_set(
                &admission.block_device,
                &admission.boot_region,
                published_stream,
                timestamp,
            ) {
                if let Some(page_cache) = page_cache {
                    let _ = page_cache.resize(data_length);
                }
                return Err(error);
            }

            let allocated_clusters = new_size.div_ceil(admission.boot_region.cluster_size);
            let allocated_sectors = allocated_clusters
                .checked_mul(admission.boot_region.sectors_per_cluster)
                .ok_or_else(|| Error::new(Errno::EINVAL))?;
            {
                let mut metadata = self.metadata.write();
                metadata.last_meta_change_at = timestamp;
                metadata.last_modify_at = timestamp;
                metadata.nr_sectors_allocated = allocated_sectors;
                metadata.size = new_size;
            }
            *stream = published_stream;
            self.mark_content_publication_dirty();
            return Ok(());
        }

        let current_ranges = Self::allocated_cluster_ranges(
            &admission.block_device,
            &admission.boot_region,
            stream.first_cluster,
            data_length,
            stream.no_fat_chain,
        )
        .map_err(Error::from)?;
        let retained_clusters = if new_size == 0 {
            0
        } else {
            new_size.div_ceil(admission.boot_region.cluster_size)
        };
        let mut retained_clusters_remaining = retained_clusters;
        let mut retained_is_contiguous = true;
        let mut previous_retained_cluster: Option<u32> = None;
        let mut first_retained_cluster = 0u32;
        let mut last_retained_cluster = 0u32;
        let mut released_ranges = Vec::new();
        for range in &current_ranges {
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
                            .map_err(|_| Error::from(ExfatFsError::InvalidOnDiskLayout))?,
                    )
                    .ok_or_else(|| Error::from(ExfatFsError::InvalidOnDiskLayout))?;
                if let Some(previous_retained_cluster) = previous_retained_cluster {
                    if previous_retained_cluster.checked_add(1) != Some(range.start_cluster) {
                        retained_is_contiguous = false;
                    }
                } else {
                    first_retained_cluster = range.start_cluster;
                }
                previous_retained_cluster = Some(retained_last_cluster);
                last_retained_cluster = retained_last_cluster;
            }
            if retained_in_range < range.cluster_count {
                let released_start_cluster = range
                    .start_cluster
                    .checked_add(
                        u32::try_from(retained_in_range)
                            .map_err(|_| Error::from(ExfatFsError::InvalidOnDiskLayout))?,
                    )
                    .ok_or_else(|| Error::from(ExfatFsError::InvalidOnDiskLayout))?;
                released_ranges.push(ClusterRange {
                    start_cluster: released_start_cluster,
                    cluster_count: range.cluster_count - retained_in_range,
                });
            }
            retained_clusters_remaining -= retained_in_range;
        }
        if retained_clusters_remaining != 0 {
            return Err(Error::from(ExfatFsError::InvalidOnDiskLayout));
        }

        let published_stream = ExfatInodeStream {
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
            published_stream,
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
        *stream = published_stream;
        self.mark_content_publication_dirty();

        if retained_clusters != 0 && !published_stream.no_fat_chain {
            FatReader::new(admission.block_device.as_ref(), &admission.boot_region)
                .terminate_cluster_chain(last_retained_cluster)
                .map_err(Error::from)?;
        }
        if !released_ranges.is_empty() {
            let publication = admission
                .state_guard
                .as_mut()
                .ok_or(ExfatFsError::UnpublishedState)
                .map_err(Error::from)?;
            fs.free_allocated_space_with_publication(publication, &released_ranges)
                .map_err(Error::from)?;
        }
        Ok(())
    }

    // Entry-set publication

    pub(super) fn republish_regular_file_entry_set(
        &self,
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        published_stream: ExfatInodeStream,
        timestamp: Duration,
    ) -> Result<()> {
        let Some(data_length) = published_stream.data_length else {
            return_errno!(Errno::EINVAL);
        };
        let Some(valid_data_length) = published_stream.valid_data_length else {
            return_errno!(Errno::EINVAL);
        };
        if valid_data_length > data_length {
            return_errno!(Errno::EINVAL);
        }
        if data_length == 0 {
            if published_stream.first_cluster != 0 || valid_data_length != 0 {
                return_errno!(Errno::EINVAL);
            }
        } else {
            boot_region
                .validate_stream_data(
                    published_stream.first_cluster,
                    u64::try_from(data_length).map_err(|_| Error::new(Errno::EINVAL))?,
                )
                .map_err(Error::from)?;
        }

        let (timestamp_bytes, hundredths_increment, utc_offset_byte) =
            Self::encoded_exfat_timestamp_fields(timestamp, 0)?;
        let last_modified_fields = (timestamp_bytes, hundredths_increment, utc_offset_byte);
        self.rewrite_inode_entry_set(
            InodeRewriteTarget::RegularFile,
            block_device,
            boot_region,
            |entry_view, _source_entry_set| {
                let stream_flags = if published_stream.no_fat_chain {
                    0x03
                } else {
                    0x01
                };
                let valid_data_length =
                    u64::try_from(valid_data_length).map_err(|_| Error::new(Errno::EINVAL))?;
                let data_length =
                    u64::try_from(data_length).map_err(|_| Error::new(Errno::EINVAL))?;
                direntry::republished_entry_set(
                    entry_view,
                    &direntry::FileEntrySetFieldUpdates {
                        data_length: Some(data_length),
                        first_cluster: Some(published_stream.first_cluster),
                        last_modified_fields: Some(last_modified_fields),
                        stream_flags: Some(stream_flags),
                        valid_data_length: Some(valid_data_length),
                        ..Default::default()
                    },
                )
                .map(Some)
                .map_err(Error::from)
            },
            |_| {},
        )?;
        Ok(())
    }

    // Stream topology

    pub(super) fn grow_regular_file_stream(
        fs: &Arc<ExfatFs>,
        publication: &MountedVolumeState,
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        stream: ExfatInodeStream,
        new_data_length: usize,
    ) -> Result<ExfatInodeStream> {
        let Some(current_data_length) = stream.data_length else {
            return_errno!(Errno::EINVAL);
        };
        let Some(current_valid_data_length) = stream.valid_data_length else {
            return_errno!(Errno::EINVAL);
        };
        if current_valid_data_length > current_data_length || new_data_length < current_data_length
        {
            return_errno!(Errno::EINVAL);
        }
        if new_data_length == current_data_length {
            return Ok(stream);
        }

        let current_allocated_clusters = if current_data_length == 0 {
            0
        } else {
            current_data_length.div_ceil(boot_region.cluster_size)
        };
        let target_allocated_clusters = new_data_length.div_ceil(boot_region.cluster_size);
        if target_allocated_clusters == current_allocated_clusters {
            return Ok(ExfatInodeStream {
                data_length: Some(new_data_length),
                ..stream
            });
        }

        let additional_clusters = target_allocated_clusters
            .checked_sub(current_allocated_clusters)
            .ok_or_else(|| Error::new(Errno::EINVAL))?;
        let (allocated_ranges, _) = fs
            .allocate_free_space_with_publication(publication, additional_clusters)
            .map_err(Error::from)?;
        let allocated_cluster_count =
            allocated_ranges
                .iter()
                .try_fold(0usize, |total_clusters, range| {
                    total_clusters
                        .checked_add(range.cluster_count)
                        .ok_or_else(|| Error::from(ExfatFsError::InconsistentAccounting))
                })?;
        if allocated_cluster_count != additional_clusters {
            return Err(Error::from(ExfatFsError::InconsistentAccounting));
        }
        let first_new_cluster = allocated_ranges
            .first()
            .ok_or_else(|| Error::from(ExfatFsError::InconsistentAccounting))?
            .start_cluster;
        let stays_contiguous = if current_allocated_clusters == 0 {
            allocated_ranges.len() == 1
        } else if stream.no_fat_chain {
            stream.first_cluster.checked_add(
                u32::try_from(current_allocated_clusters)
                    .map_err(|_| Error::from(ExfatFsError::InvalidOnDiskLayout))?,
            ) == Some(first_new_cluster)
                && allocated_ranges.len() == 1
        } else {
            false
        };
        if !stays_contiguous {
            let mut fat_reader = FatReader::new(block_device.as_ref(), boot_region);
            let link_allocated_ranges_fn =
                |fat_reader: &mut FatReader<'_>| -> core::result::Result<(), ExfatFsError> {
                    for (range_index, range) in allocated_ranges.iter().enumerate() {
                        let next_range_start = allocated_ranges
                            .get(range_index + 1)
                            .map(|next_range| next_range.start_cluster);
                        match (range.cluster_count, next_range_start) {
                            (0, _) => return Err(ExfatFsError::InvalidOperationInput),
                            (1, None) => fat_reader.terminate_cluster_chain(range.start_cluster)?,
                            (cluster_count, None) => {
                                let last_cluster = range
                                    .start_cluster
                                    .checked_add(
                                        u32::try_from(cluster_count - 1)
                                            .map_err(|_| ExfatFsError::InvalidOperationInput)?,
                                    )
                                    .ok_or(ExfatFsError::InvalidOperationInput)?;
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
                };
            if current_allocated_clusters == 0 {
                link_allocated_ranges_fn(&mut fat_reader).map_err(Error::from)?;
            } else {
                if stream.no_fat_chain {
                    fat_reader
                        .link_contiguous_chain_to_cluster(
                            stream.first_cluster,
                            current_allocated_clusters,
                            first_new_cluster,
                        )
                        .map_err(Error::from)?;
                } else {
                    fat_reader
                        .append_cluster_to_chain(stream.first_cluster, first_new_cluster)
                        .map_err(Error::from)?;
                }
                link_allocated_ranges_fn(&mut fat_reader).map_err(Error::from)?;
            }
        }

        Ok(ExfatInodeStream {
            data_length: Some(new_data_length),
            first_cluster: if current_allocated_clusters == 0 {
                first_new_cluster
            } else {
                stream.first_cluster
            },
            no_fat_chain: stays_contiguous,
            ..stream
        })
    }

    // Cluster-level I/O helper

    pub(super) fn mutate_regular_file_range(
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        stream: &ExfatInodeStream,
        data_length: usize,
        offset: usize,
        len: usize,
        mut fill_chunk_fn: impl FnMut(&mut [u8]) -> Result<()>,
    ) -> Result<()> {
        if len == 0 {
            return Ok(());
        }

        Self::validate_regular_file_mapping_shape(boot_region, stream, data_length)?;
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
            stream,
            data_length,
            cluster_index,
        )?;
        let mut fat_reader =
            (!stream.no_fat_chain).then(|| FatReader::new(block_device.as_ref(), boot_region));
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
