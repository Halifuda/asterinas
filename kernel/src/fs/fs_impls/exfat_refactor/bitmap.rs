// SPDX-License-Identifier: MPL-2.0

use alloc::{collections::BTreeSet, vec, vec::Vec};

use aster_block::BlockDevice;
use ostd::mm::VmIo;

use super::{
    boot::BootRegion,
    fat::{ChainVisitControl, FatChainStep, FatReader},
    fs::ExfatFsError,
};

// TODO: `ExfatFsError` is a temporary cross-owner seam for bitmap
// parsing and mutation while mount bootstrap and `FatReader` still expose that
// error type. Remove this import once the boot/FAT owners expose bitmap-local
// error conversion; then `AllocationBitmap` methods should return the
// bitmap-owned error and `fs.rs` should translate it at the free-space boundary.
pub(super) const ALLOCATION_BITMAP_ENTRY_TYPE: u8 = 0x81;

#[derive(Clone, Copy)]
pub(super) struct AllocationBitmap {
    pub(super) data_length: u64,
    pub(super) first_cluster: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct ClusterRange {
    pub(super) start_cluster: u32,
    pub(super) cluster_count: usize,
}

#[derive(Clone, Copy)]
pub(super) enum AllocationBitmapUpdate {
    Allocate,
    Free,
}

impl AllocationBitmap {
    pub(super) fn count_used_clusters(
        self,
        boot_region: &BootRegion,
        fat_reader: &mut FatReader<'_>,
    ) -> core::result::Result<(usize, bool), ExfatFsError> {
        let cluster_count = boot_region.cluster_count_usize()?;
        let (bitmap_bytes, required_bytes) = self.bitmap_lengths(cluster_count, boot_region)?;
        debug_assert!(bitmap_bytes >= required_bytes);

        let mut bits_remaining = cluster_count;
        let mut bitmap_bytes_remaining = bitmap_bytes;
        let mut used_clusters = 0usize;
        let result = fat_reader.walk_cluster_chain(self.first_cluster, |_, cluster_bytes| {
            let bytes_to_visit = bitmap_bytes_remaining.min(cluster_bytes.len());
            for byte in &cluster_bytes[..bytes_to_visit] {
                if bits_remaining == 0 {
                    if *byte != 0 {
                        return Err(ExfatFsError::InconsistentAccounting);
                    }
                    continue;
                }
                let relevant_bits = bits_remaining.min(u8::BITS as usize);
                let mask = Self::relevant_bitmap_mask(relevant_bits)?;
                let masked_byte = *byte & mask;
                if masked_byte != *byte && (*byte & !mask) != 0 {
                    return Err(ExfatFsError::InconsistentAccounting);
                }
                used_clusters = used_clusters
                    .checked_add(masked_byte.count_ones() as usize)
                    .ok_or(ExfatFsError::InconsistentAccounting)?;
                bits_remaining -= relevant_bits;
            }
            bitmap_bytes_remaining -= bytes_to_visit;
            if bitmap_bytes_remaining == 0 {
                return Ok(ChainVisitControl::Stop);
            }
            Ok(ChainVisitControl::Continue)
        });
        match result {
            Ok(()) => (),
            Err(ExfatFsError::InvalidOnDiskLayout) => {
                return Err(ExfatFsError::InconsistentAccounting);
            }
            Err(error) => return Err(error),
        }
        if bits_remaining != 0 || bitmap_bytes_remaining != 0 {
            return Err(ExfatFsError::InconsistentAccounting);
        }

        let counted_percent = if cluster_count == 0 {
            return Err(ExfatFsError::InvalidOnDiskLayout);
        } else {
            (used_clusters.saturating_mul(100) + cluster_count / 2) / cluster_count
        };
        let used_clusters_from_recount = match boot_region.percent_in_use {
            0xFF => true,
            percent_in_use => counted_percent != usize::from(percent_in_use),
        };
        Ok((used_clusters, used_clusters_from_recount))
    }

    pub(super) fn find_free_ranges(
        self,
        boot_region: &BootRegion,
        fat_reader: &mut FatReader<'_>,
        requested_clusters: usize,
    ) -> core::result::Result<Vec<ClusterRange>, ExfatFsError> {
        if requested_clusters == 0 {
            return Err(ExfatFsError::InvalidOperationInput);
        }

        let cluster_count = boot_region.cluster_count_usize()?;
        let (bitmap_bytes, _) = self.bitmap_lengths(cluster_count, boot_region)?;
        let mut bits_remaining = cluster_count;
        let mut bitmap_bytes_remaining = bitmap_bytes;
        let mut next_cluster_index = 0usize;
        let mut requested_clusters_remaining = requested_clusters;
        let mut ranges = Vec::new();
        let mut run_start_index = None;
        let mut run_cluster_count = 0usize;

        let result = fat_reader.walk_cluster_chain(self.first_cluster, |_, cluster_bytes| {
            let bytes_to_visit = bitmap_bytes_remaining.min(cluster_bytes.len());
            for byte in &cluster_bytes[..bytes_to_visit] {
                if bits_remaining == 0 {
                    if *byte != 0 {
                        return Err(ExfatFsError::InconsistentAccounting);
                    }
                    continue;
                }
                let relevant_bits = bits_remaining.min(u8::BITS as usize);
                let mask = Self::relevant_bitmap_mask(relevant_bits)?;
                let masked_byte = *byte & mask;
                if masked_byte != *byte && (*byte & !mask) != 0 {
                    return Err(ExfatFsError::InconsistentAccounting);
                }
                for bit_index in 0..relevant_bits {
                    let bit_mask = 1u8 << bit_index;
                    let cluster_is_used = masked_byte & bit_mask != 0;
                    if cluster_is_used {
                        if Self::flush_cluster_run(
                            boot_region,
                            &mut ranges,
                            &mut run_start_index,
                            &mut run_cluster_count,
                            &mut requested_clusters_remaining,
                        )? {
                            return Ok(ChainVisitControl::Stop);
                        }
                    } else {
                        if run_start_index.is_none() {
                            run_start_index = Some(next_cluster_index);
                        }
                        run_cluster_count = run_cluster_count
                            .checked_add(1)
                            .ok_or(ExfatFsError::InconsistentAccounting)?;
                    }
                    next_cluster_index = next_cluster_index
                        .checked_add(1)
                        .ok_or(ExfatFsError::InconsistentAccounting)?;
                    bits_remaining -= 1;
                    if !cluster_is_used
                        && run_cluster_count >= requested_clusters_remaining
                        && Self::flush_cluster_run(
                            boot_region,
                            &mut ranges,
                            &mut run_start_index,
                            &mut run_cluster_count,
                            &mut requested_clusters_remaining,
                        )?
                    {
                        return Ok(ChainVisitControl::Stop);
                    }
                }
            }
            bitmap_bytes_remaining -= bytes_to_visit;
            if bits_remaining == 0
                && Self::flush_cluster_run(
                    boot_region,
                    &mut ranges,
                    &mut run_start_index,
                    &mut run_cluster_count,
                    &mut requested_clusters_remaining,
                )?
            {
                return Ok(ChainVisitControl::Stop);
            }
            if requested_clusters_remaining == 0 || bitmap_bytes_remaining == 0 {
                return Ok(ChainVisitControl::Stop);
            }
            Ok(ChainVisitControl::Continue)
        });
        match result {
            Ok(()) => (),
            Err(ExfatFsError::InvalidOnDiskLayout) => {
                return Err(ExfatFsError::InconsistentAccounting);
            }
            Err(error) => return Err(error),
        }
        if requested_clusters_remaining == 0 {
            return Ok(ranges);
        }
        if bits_remaining != 0 || bitmap_bytes_remaining != 0 {
            return Err(ExfatFsError::InconsistentAccounting);
        }
        Err(ExfatFsError::NoSpace)
    }

    pub(super) fn apply_update(
        self,
        block_device: &dyn BlockDevice,
        boot_region: &BootRegion,
        cluster_ranges: &[ClusterRange],
        update: AllocationBitmapUpdate,
    ) -> core::result::Result<usize, ExfatFsError> {
        if cluster_ranges.is_empty() {
            return Err(ExfatFsError::InvalidOperationInput);
        }

        let cluster_count = boot_region.cluster_count_usize()?;
        let mut normalized_ranges = Vec::with_capacity(cluster_ranges.len());
        for cluster_range in cluster_ranges {
            if cluster_range.cluster_count == 0 {
                return Err(ExfatFsError::InvalidOperationInput);
            }

            let start_index = boot_region
                .cluster_index(cluster_range.start_cluster)
                .map_err(|_| ExfatFsError::InvalidOperationInput)?;
            let end_index = start_index
                .checked_add(cluster_range.cluster_count)
                .ok_or(ExfatFsError::InvalidOperationInput)?;
            if end_index > cluster_count {
                return Err(ExfatFsError::InvalidOperationInput);
            }
            normalized_ranges.push(start_index..end_index);
        }
        normalized_ranges.sort_by_key(|range| range.start);
        for window in normalized_ranges.windows(2) {
            if window[0].end > window[1].start {
                return Err(ExfatFsError::InvalidOperationInput);
            }
        }
        let mut expected_cluster_count = 0usize;
        for normalized_range in &normalized_ranges {
            expected_cluster_count = expected_cluster_count
                .checked_add(normalized_range.end - normalized_range.start)
                .ok_or(ExfatFsError::InvalidOperationInput)?;
        }
        let (bitmap_bytes, _) = self.bitmap_lengths(cluster_count, boot_region)?;
        let mut cluster_buffer = vec![0; boot_region.cluster_size];
        let mut current_cluster = self.first_cluster;
        let mut fat_reader = FatReader::new(block_device, boot_region);
        let mut visited_clusters = BTreeSet::new();
        let mut bitmap_bytes_remaining = bitmap_bytes;
        let mut current_cluster_index = 0usize;
        let mut normalized_range_index = 0usize;
        let mut updated_clusters = 0usize;

        while bitmap_bytes_remaining != 0 {
            if !visited_clusters.insert(current_cluster) {
                return Err(ExfatFsError::InconsistentAccounting);
            }
            let cluster_offset = boot_region.cluster_offset(current_cluster)?;
            block_device
                .read_bytes(cluster_offset, &mut cluster_buffer)
                .map_err(|_| ExfatFsError::DeviceIo)?;

            let bytes_to_visit = bitmap_bytes_remaining.min(cluster_buffer.len());
            let mut cluster_dirty = false;
            for byte in &mut cluster_buffer[..bytes_to_visit] {
                if current_cluster_index >= cluster_count {
                    if *byte != 0 {
                        return Err(ExfatFsError::InconsistentAccounting);
                    }
                    continue;
                }
                let relevant_bits = (cluster_count - current_cluster_index).min(u8::BITS as usize);
                let mask = Self::relevant_bitmap_mask(relevant_bits)?;
                if *byte & !mask != 0 {
                    return Err(ExfatFsError::InconsistentAccounting);
                }

                let mut next_byte = *byte;
                for bit_index in 0..relevant_bits {
                    let bitmap_index = current_cluster_index + bit_index;
                    while normalized_range_index < normalized_ranges.len()
                        && normalized_ranges[normalized_range_index].end <= bitmap_index
                    {
                        normalized_range_index += 1;
                    }
                    if normalized_range_index == normalized_ranges.len()
                        || bitmap_index < normalized_ranges[normalized_range_index].start
                    {
                        continue;
                    }

                    let bit_mask = 1u8 << bit_index;
                    match update {
                        AllocationBitmapUpdate::Allocate => {
                            if next_byte & bit_mask != 0 {
                                return Err(ExfatFsError::InconsistentAccounting);
                            }
                            next_byte |= bit_mask;
                        }
                        AllocationBitmapUpdate::Free => {
                            if next_byte & bit_mask == 0 {
                                return Err(ExfatFsError::InconsistentAccounting);
                            }
                            next_byte &= !bit_mask;
                        }
                    }
                    updated_clusters = updated_clusters
                        .checked_add(1)
                        .ok_or(ExfatFsError::InconsistentAccounting)?;
                }

                if next_byte != *byte {
                    *byte = next_byte;
                    cluster_dirty = true;
                }
                current_cluster_index += relevant_bits;
            }

            if cluster_dirty {
                block_device
                    .write_bytes(cluster_offset, &cluster_buffer)
                    .map_err(|_| ExfatFsError::DeviceIo)?;
            }
            bitmap_bytes_remaining -= bytes_to_visit;
            if bitmap_bytes_remaining == 0 {
                break;
            }
            current_cluster = match fat_reader.next_cluster(current_cluster)? {
                FatChainStep::Continue(next_cluster) => next_cluster,
                FatChainStep::End => return Err(ExfatFsError::InconsistentAccounting),
            };
        }

        if updated_clusters != expected_cluster_count {
            return Err(ExfatFsError::InconsistentAccounting);
        }
        Ok(updated_clusters)
    }

    pub(super) fn recount_used_clusters(
        self,
        boot_region: &BootRegion,
        fat_reader: &mut FatReader<'_>,
    ) -> core::result::Result<usize, ExfatFsError> {
        let (used_clusters, _) = self.count_used_clusters(boot_region, fat_reader)?;
        Ok(used_clusters)
    }

    pub(super) fn parse(entry: &[u8]) -> core::result::Result<Self, ExfatFsError> {
        if entry.len() != 32 {
            return Err(ExfatFsError::InvalidOnDiskLayout);
        }
        Ok(Self {
            first_cluster: u32::from_le_bytes([entry[20], entry[21], entry[22], entry[23]]),
            data_length: u64::from_le_bytes([
                entry[24], entry[25], entry[26], entry[27], entry[28], entry[29], entry[30],
                entry[31],
            ]),
        })
    }

    fn bitmap_lengths(
        self,
        cluster_count: usize,
        boot_region: &BootRegion,
    ) -> core::result::Result<(usize, usize), ExfatFsError> {
        boot_region.validate_stream_data(self.first_cluster, self.data_length)?;
        let required_bytes = cluster_count.div_ceil(8);
        let bitmap_bytes =
            usize::try_from(self.data_length).map_err(|_| ExfatFsError::InvalidOnDiskLayout)?;
        if bitmap_bytes < required_bytes {
            return Err(ExfatFsError::InconsistentAccounting);
        }
        Ok((bitmap_bytes, required_bytes))
    }

    fn flush_cluster_run(
        boot_region: &BootRegion,
        ranges: &mut Vec<ClusterRange>,
        run_start_index: &mut Option<usize>,
        run_cluster_count: &mut usize,
        requested_clusters_remaining: &mut usize,
    ) -> core::result::Result<bool, ExfatFsError> {
        let Some(start_cluster_index) = run_start_index.take() else {
            *run_cluster_count = 0;
            return Ok(*requested_clusters_remaining == 0);
        };
        let admitted_clusters = (*run_cluster_count).min(*requested_clusters_remaining);
        *run_cluster_count = 0;
        if admitted_clusters == 0 {
            return Ok(*requested_clusters_remaining == 0);
        }

        ranges.push(ClusterRange {
            start_cluster: boot_region.cluster_from_index(start_cluster_index)?,
            cluster_count: admitted_clusters,
        });
        *requested_clusters_remaining -= admitted_clusters;
        Ok(*requested_clusters_remaining == 0)
    }

    fn relevant_bitmap_mask(relevant_bits: usize) -> core::result::Result<u8, ExfatFsError> {
        if relevant_bits > u8::BITS as usize {
            return Err(ExfatFsError::InconsistentAccounting);
        }
        if relevant_bits == u8::BITS as usize {
            return Ok(u8::MAX);
        }
        let shift =
            u32::try_from(relevant_bits).map_err(|_| ExfatFsError::InconsistentAccounting)?;
        let shifted = 1u16
            .checked_shl(shift)
            .ok_or(ExfatFsError::InconsistentAccounting)?;
        Ok((shifted - 1) as u8)
    }
}
