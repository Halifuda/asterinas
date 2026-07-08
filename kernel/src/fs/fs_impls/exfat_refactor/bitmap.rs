// SPDX-License-Identifier: MPL-2.0

//! Owns allocation-bitmap scanning, range normalization, and cached used-cluster accounting.

use alloc::{collections::BTreeSet, vec, vec::Vec};
use core::ops::Range;

use aster_block::BlockDevice;
use ostd::mm::VmIo;

use super::{
    boot::BootRegion,
    device_io,
    fat::{ChainVisitControl, FatChainStep, FatReader},
    inconsistent_bitmap_accounting,
    inode::ClusterMap,
    invalid_on_disk_layout, invalid_operation_input, no_space,
};
use crate::prelude::*;

pub(super) const ALLOCATION_BITMAP_ENTRY_TYPE: u8 = 0x81;

pub(super) struct AllocationBitmap {
    pub(super) data_length: u64,
    pub(super) first_cluster: u32,
    next_allocation_search_cluster: u32,
    resident_bitmap: Vec<u8>,
    dirty_byte_ranges: Vec<Range<usize>>,
    dirty_byte_generations: Vec<u64>,
    published_dirty_generation: Option<u64>,
    next_dirty_generation: u64,
    used_clusters: usize,
    lazy_reclaimed_clusters: Vec<LazyReclaimedCluster>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct ClusterRange {
    pub(super) start_cluster: u32,
    pub(super) cluster_count: usize,
}

#[derive(Clone, Copy)]
pub(super) enum BitmapOp {
    Allocate,
    Free,
}

struct LazyReclaimedCluster {
    cluster_map: Arc<ClusterMap>,
    ranges: Vec<ClusterRange>,
}

impl AllocationBitmap {
    pub(super) fn load_resident_bitmap(
        &mut self,
        block_device: &dyn BlockDevice,
        boot_region: &BootRegion,
    ) -> Result<()> {
        let cluster_count = boot_region.cluster_count_usize()?;
        let (bitmap_bytes, _) = self.bitmap_lengths(cluster_count, boot_region)?;
        let mut resident_bitmap = vec![0; bitmap_bytes];
        let mut copied_bytes = 0usize;
        let mut fat_reader = FatReader::new(block_device, boot_region);
        fat_reader.walk_cluster_chain(self.first_cluster, |_, cluster_bytes| {
            let bytes_to_copy = resident_bitmap
                .len()
                .saturating_sub(copied_bytes)
                .min(cluster_bytes.len());
            if bytes_to_copy != 0 {
                resident_bitmap[copied_bytes..copied_bytes + bytes_to_copy]
                    .copy_from_slice(&cluster_bytes[..bytes_to_copy]);
                copied_bytes += bytes_to_copy;
            }
            Ok(if copied_bytes == resident_bitmap.len() {
                ChainVisitControl::Stop
            } else {
                ChainVisitControl::Continue
            })
        })?;
        if copied_bytes != resident_bitmap.len() {
            return Err(inconsistent_bitmap_accounting());
        }
        self.resident_bitmap = resident_bitmap;
        self.dirty_byte_ranges.clear();
        self.dirty_byte_generations = vec![0; bitmap_bytes];
        self.published_dirty_generation = None;
        self.next_dirty_generation = 0;
        Ok(())
    }

    pub(super) fn count_used_clusters(
        &self,
        boot_region: &BootRegion,
        fat_reader: &mut FatReader<'_>,
    ) -> Result<usize> {
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
                        return Err(inconsistent_bitmap_accounting());
                    }
                    continue;
                }
                let relevant_bits = bits_remaining.min(u8::BITS as usize);
                let mask = Self::relevant_bitmap_mask(relevant_bits)?;
                let masked_byte = *byte & mask;
                if masked_byte != *byte && (*byte & !mask) != 0 {
                    return Err(inconsistent_bitmap_accounting());
                }
                used_clusters = used_clusters
                    .checked_add(masked_byte.count_ones() as usize)
                    .ok_or_else(inconsistent_bitmap_accounting)?;
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
            Err(error) if error.error() == Errno::EUCLEAN => {
                return Err(inconsistent_bitmap_accounting());
            }
            Err(error) => return Err(error),
        }
        if bits_remaining != 0 || bitmap_bytes_remaining != 0 {
            return Err(inconsistent_bitmap_accounting());
        }
        if cluster_count == 0 {
            return Err(invalid_on_disk_layout());
        }
        Ok(used_clusters)
    }

    pub(super) fn find_free_ranges(
        &self,
        _block_device: &dyn BlockDevice,
        boot_region: &BootRegion,
        _fat_reader: &mut FatReader<'_>,
        requested_clusters: usize,
        preferred_start_cluster: Option<u32>,
    ) -> Result<Vec<ClusterRange>> {
        if requested_clusters == 0 {
            return Err(invalid_operation_input());
        }

        let cluster_count = boot_region.cluster_count_usize()?;
        let (bitmap_bytes, _) = self.bitmap_lengths(cluster_count, boot_region)?;
        if self.resident_bitmap.len() != bitmap_bytes {
            return Err(inconsistent_bitmap_accounting());
        }
        let mut requested_clusters_remaining = requested_clusters;
        let mut ranges = Vec::new();
        let effective_start_cluster = preferred_start_cluster
            .filter(|cluster| boot_region.is_valid_cluster(*cluster))
            .or_else(|| {
                boot_region
                    .is_valid_cluster(self.next_allocation_search_cluster)
                    .then_some(self.next_allocation_search_cluster)
            })
            .unwrap_or_else(|| {
                boot_region
                    .cluster_from_index(0)
                    .unwrap_or_else(|_| unreachable!("non-empty volumes must have cluster 2"))
            });
        let effective_start_index = boot_region.cluster_index(effective_start_cluster)?;

        let mut scan_window = |scan_start_index: usize,
                               scan_end_index: usize,
                               requested_clusters_remaining: &mut usize,
                               ranges: &mut Vec<ClusterRange>|
         -> Result<()> {
            if *requested_clusters_remaining == 0 || scan_start_index >= scan_end_index {
                return Ok(());
            }

            let mut run_start_index = None;
            let mut run_cluster_count = 0usize;
            let start_byte = scan_start_index / 8;
            let end_byte = scan_end_index.div_ceil(8);
            for byte_index in start_byte..end_byte {
                let byte = *self
                    .resident_bitmap
                    .get(byte_index)
                    .ok_or_else(inconsistent_bitmap_accounting)?;
                let cluster_index_base = byte_index
                    .checked_mul(u8::BITS as usize)
                    .ok_or_else(inconsistent_bitmap_accounting)?;
                let relevant_bits = cluster_count
                    .saturating_sub(cluster_index_base)
                    .min(u8::BITS as usize);
                let mask = Self::relevant_bitmap_mask(relevant_bits)?;
                let masked_byte = byte & mask;
                if masked_byte != byte && (byte & !mask) != 0 {
                    return Err(inconsistent_bitmap_accounting());
                }

                for bit_index in 0..relevant_bits {
                    let cluster_index = cluster_index_base
                        .checked_add(bit_index)
                        .ok_or_else(inconsistent_bitmap_accounting)?;
                    if cluster_index < scan_start_index {
                        continue;
                    }
                    if cluster_index >= scan_end_index {
                        let _ = Self::commit_run_to_ranges(
                            boot_region,
                            ranges,
                            &mut run_start_index,
                            &mut run_cluster_count,
                            requested_clusters_remaining,
                        )?;
                        return Ok(());
                    }

                    let bit_mask = 1u8 << bit_index;
                    if masked_byte & bit_mask != 0 {
                        if Self::commit_run_to_ranges(
                            boot_region,
                            ranges,
                            &mut run_start_index,
                            &mut run_cluster_count,
                            requested_clusters_remaining,
                        )? {
                            return Ok(());
                        }
                        continue;
                    }

                    if run_start_index.is_none() {
                        run_start_index = Some(cluster_index);
                    }
                    run_cluster_count = run_cluster_count
                        .checked_add(1)
                        .ok_or_else(inconsistent_bitmap_accounting)?;
                    if run_cluster_count >= *requested_clusters_remaining
                        && Self::commit_run_to_ranges(
                            boot_region,
                            ranges,
                            &mut run_start_index,
                            &mut run_cluster_count,
                            requested_clusters_remaining,
                        )?
                    {
                        return Ok(());
                    }
                }
            }
            let _ = Self::commit_run_to_ranges(
                boot_region,
                ranges,
                &mut run_start_index,
                &mut run_cluster_count,
                requested_clusters_remaining,
            )?;
            Ok(())
        };

        scan_window(
            effective_start_index,
            cluster_count,
            &mut requested_clusters_remaining,
            &mut ranges,
        )?;
        if requested_clusters_remaining != 0 && effective_start_index != 0 {
            scan_window(
                0,
                effective_start_index,
                &mut requested_clusters_remaining,
                &mut ranges,
            )?;
        }
        if requested_clusters_remaining == 0 {
            return Ok(ranges);
        }
        Err(no_space())
    }

    pub(super) fn validate_and_normalize_ranges(
        &self,
        boot_region: &BootRegion,
        cluster_ranges: &[ClusterRange],
    ) -> Result<Vec<Range<usize>>> {
        if cluster_ranges.is_empty() {
            return Err(invalid_operation_input());
        }

        let cluster_count = boot_region.cluster_count_usize()?;
        let mut normalized_ranges = Vec::with_capacity(cluster_ranges.len());
        for cluster_range in cluster_ranges {
            if cluster_range.cluster_count == 0 {
                return Err(invalid_operation_input());
            }

            let start_index = boot_region
                .cluster_index(cluster_range.start_cluster)
                .map_err(|_| invalid_operation_input())?;
            let end_index = start_index
                .checked_add(cluster_range.cluster_count)
                .ok_or_else(invalid_operation_input)?;
            if end_index > cluster_count {
                return Err(invalid_operation_input());
            }
            normalized_ranges.push(start_index..end_index);
        }
        normalized_ranges.sort_by_key(|range| range.start);
        for window in normalized_ranges.windows(2) {
            if window[0].end > window[1].start {
                return Err(invalid_operation_input());
            }
        }
        if normalized_ranges.iter().all(Range::is_empty) {
            return Err(invalid_operation_input());
        }
        Ok(normalized_ranges)
    }

    pub(super) fn apply_cluster_ranges(
        &mut self,
        _block_device: &dyn BlockDevice,
        boot_region: &BootRegion,
        cluster_ranges: &[ClusterRange],
        update: BitmapOp,
    ) -> Result<usize> {
        if cluster_ranges.is_empty() {
            return Err(invalid_operation_input());
        }

        let cluster_count = boot_region.cluster_count_usize()?;
        let normalized_ranges = self.validate_and_normalize_ranges(boot_region, cluster_ranges)?;
        let expected_cluster_count =
            normalized_ranges.iter().try_fold(0usize, |total, range| {
                total
                    .checked_add(range.end - range.start)
                    .ok_or_else(invalid_operation_input)
            })?;
        let (bitmap_bytes, _) = self.bitmap_lengths(cluster_count, boot_region)?;
        if self.resident_bitmap.len() != bitmap_bytes {
            return Err(inconsistent_bitmap_accounting());
        }
        let mut updated_clusters = 0usize;
        let mut dirty_byte_ranges = Vec::with_capacity(normalized_ranges.len());
        for normalized_range in &normalized_ranges {
            let dirty_byte_start = normalized_range.start / 8;
            let dirty_byte_end = normalized_range.end.saturating_sub(1) / 8 + 1;
            for bitmap_index in normalized_range.clone() {
                let byte_index = bitmap_index / 8;
                let bit_index = bitmap_index % 8;
                let byte = self
                    .resident_bitmap
                    .get_mut(byte_index)
                    .ok_or_else(inconsistent_bitmap_accounting)?;
                let relevant_bits = cluster_count
                    .saturating_sub(byte_index * 8)
                    .min(u8::BITS as usize);
                let mask = Self::relevant_bitmap_mask(relevant_bits)?;
                if *byte & !mask != 0 {
                    return Err(inconsistent_bitmap_accounting());
                }

                let bit_mask = 1u8 << bit_index;
                match update {
                    BitmapOp::Allocate => {
                        if *byte & bit_mask != 0 {
                            return Err(inconsistent_bitmap_accounting());
                        }
                        *byte |= bit_mask;
                    }
                    BitmapOp::Free => {
                        if *byte & bit_mask == 0 {
                            return Err(inconsistent_bitmap_accounting());
                        }
                        *byte &= !bit_mask;
                    }
                }
                updated_clusters = updated_clusters
                    .checked_add(1)
                    .ok_or_else(inconsistent_bitmap_accounting)?;
            }
            dirty_byte_ranges.push(dirty_byte_start..dirty_byte_end);
        }

        if updated_clusters != expected_cluster_count {
            return Err(inconsistent_bitmap_accounting());
        }
        for dirty_byte_range in dirty_byte_ranges {
            self.record_dirty_byte_range(dirty_byte_range)?;
        }
        if matches!(update, BitmapOp::Allocate) {
            let next_allocation_search_cluster = cluster_ranges
                .last()
                .and_then(|cluster_range| {
                    cluster_range
                        .start_cluster
                        .checked_add(u32::try_from(cluster_range.cluster_count).ok()?)
                })
                .filter(|cluster| boot_region.is_valid_cluster(*cluster))
                .unwrap_or_else(|| {
                    boot_region
                        .cluster_from_index(0)
                        .unwrap_or_else(|_| unreachable!("non-empty volumes must have cluster 2"))
                });
            self.next_allocation_search_cluster = next_allocation_search_cluster;
        }
        Ok(updated_clusters)
    }

    pub(super) fn record_allocated_clusters(&mut self, allocated_clusters: usize) -> Result<()> {
        self.used_clusters = self
            .used_clusters
            .checked_add(allocated_clusters)
            .ok_or_else(inconsistent_bitmap_accounting)?;
        Ok(())
    }

    pub(super) fn record_released_clusters(&mut self, released_clusters: usize) -> Result<()> {
        self.used_clusters = self
            .used_clusters
            .checked_sub(released_clusters)
            .ok_or_else(inconsistent_bitmap_accounting)?;
        Ok(())
    }

    pub(super) fn set_used_clusters(&mut self, used_clusters: usize) {
        self.used_clusters = used_clusters;
    }

    pub(super) fn used_clusters(&self) -> usize {
        self.used_clusters
    }

    pub(super) fn release_lazy_reclaimed_clusters(
        &mut self,
        block_device: &dyn BlockDevice,
        boot_region: &BootRegion,
    ) -> Result<()> {
        let mut pending_lazy_reclaims = Vec::new();
        let mut lazy_reclaimed_clusters =
            core::mem::take(&mut self.lazy_reclaimed_clusters).into_iter();
        while let Some(lazy_reclaimed_cluster) = lazy_reclaimed_clusters.next() {
            if Arc::strong_count(&lazy_reclaimed_cluster.cluster_map) != 1 {
                pending_lazy_reclaims.push(lazy_reclaimed_cluster);
                continue;
            }

            let release_result = (|| {
                let released_clusters = self.apply_cluster_ranges(
                    block_device,
                    boot_region,
                    &lazy_reclaimed_cluster.ranges,
                    BitmapOp::Free,
                )?;
                self.record_released_clusters(released_clusters)?;
                Ok(())
            })();
            if let Err(error) = release_result {
                pending_lazy_reclaims.push(lazy_reclaimed_cluster);
                pending_lazy_reclaims.extend(lazy_reclaimed_clusters);
                self.lazy_reclaimed_clusters = pending_lazy_reclaims;
                return Err(error);
            }
        }
        self.lazy_reclaimed_clusters = pending_lazy_reclaims;
        Ok(())
    }

    pub(super) fn publish_dirty_ranges(
        &mut self,
        block_device: &dyn BlockDevice,
        boot_region: &BootRegion,
    ) -> Result<()> {
        if self.dirty_byte_ranges.is_empty() {
            return Ok(());
        }

        let cluster_count = boot_region.cluster_count_usize()?;
        let (bitmap_bytes, _) = self.bitmap_lengths(cluster_count, boot_region)?;
        if self.resident_bitmap.len() != bitmap_bytes {
            return Err(inconsistent_bitmap_accounting());
        }
        let dirty_byte_ranges = self.dirty_byte_ranges.clone();
        let publish_generation = self.next_dirty_generation;
        let publish_result = (|| {
            let mut current_cluster = self.first_cluster;
            let mut current_bitmap_cluster_index = 0usize;
            let mut visited_clusters = BTreeSet::new();
            let mut fat_reader = FatReader::new(block_device, boot_region);
            if !visited_clusters.insert(current_cluster) {
                return Err(inconsistent_bitmap_accounting());
            }

            for dirty_byte_range in &dirty_byte_ranges {
                let mut dirty_byte_start = dirty_byte_range.start;
                while dirty_byte_start < dirty_byte_range.end {
                    let target_bitmap_cluster_index = dirty_byte_start / boot_region.cluster_size;
                    while current_bitmap_cluster_index < target_bitmap_cluster_index {
                        current_cluster = match fat_reader.next_cluster(current_cluster)? {
                            FatChainStep::Continue(next_cluster) => next_cluster,
                            FatChainStep::End => return Err(inconsistent_bitmap_accounting()),
                        };
                        if !visited_clusters.insert(current_cluster) {
                            return Err(inconsistent_bitmap_accounting());
                        }
                        current_bitmap_cluster_index += 1;
                    }

                    let cluster_byte_start = current_bitmap_cluster_index
                        .checked_mul(boot_region.cluster_size)
                        .ok_or_else(inconsistent_bitmap_accounting)?;
                    if cluster_byte_start >= bitmap_bytes {
                        return Err(inconsistent_bitmap_accounting());
                    }
                    let intra_cluster_offset = dirty_byte_start
                        .checked_sub(cluster_byte_start)
                        .ok_or_else(inconsistent_bitmap_accounting)?;
                    let bytes_to_write = boot_region
                        .cluster_size
                        .checked_sub(intra_cluster_offset)
                        .ok_or_else(inconsistent_bitmap_accounting)?
                        .min(dirty_byte_range.end - dirty_byte_start);
                    let cluster_offset = boot_region.cluster_offset(current_cluster)?;
                    block_device
                        .write_bytes(
                            cluster_offset
                                .checked_add(intra_cluster_offset)
                                .ok_or_else(inconsistent_bitmap_accounting)?,
                            &self.resident_bitmap[dirty_byte_start..dirty_byte_start + bytes_to_write],
                        )
                        .map_err(|_| device_io())?;
                    dirty_byte_start += bytes_to_write;
                    if dirty_byte_start < dirty_byte_range.end {
                        current_cluster = match fat_reader.next_cluster(current_cluster)? {
                            FatChainStep::Continue(next_cluster) => next_cluster,
                            FatChainStep::End => return Err(inconsistent_bitmap_accounting()),
                        };
                        if !visited_clusters.insert(current_cluster) {
                            return Err(inconsistent_bitmap_accounting());
                        }
                        current_bitmap_cluster_index += 1;
                    }
                }
            }
            Ok(())
        })();
        if let Err(error) = publish_result {
            return Err(error);
        }
        self.published_dirty_generation = Some(publish_generation);
        Ok(())
    }

    pub(super) fn commit_published_ranges(&mut self) -> Result<()> {
        let Some(published_dirty_generation) = self.published_dirty_generation else {
            return Ok(());
        };
        if self.dirty_byte_generations.len() != self.resident_bitmap.len() {
            return Err(inconsistent_bitmap_accounting());
        }

        for dirty_generation in &mut self.dirty_byte_generations {
            if *dirty_generation != 0 && *dirty_generation <= published_dirty_generation {
                *dirty_generation = 0;
            }
        }
        self.rebuild_dirty_byte_ranges();
        self.published_dirty_generation = None;
        Ok(())
    }

    pub(super) fn lazy_reclaim_clusters(
        &mut self,
        cluster_map: Arc<ClusterMap>,
        ranges: Vec<ClusterRange>,
    ) {
        if ranges.is_empty() {
            return;
        }
        self.lazy_reclaimed_clusters.push(LazyReclaimedCluster {
            cluster_map,
            ranges,
        });
    }

    pub(super) fn parse(entry: &[u8]) -> Result<Self> {
        if entry.len() != 32 {
            return Err(invalid_on_disk_layout());
        }
        Ok(Self {
            first_cluster: u32::from_le_bytes([entry[20], entry[21], entry[22], entry[23]]),
            data_length: u64::from_le_bytes([
                entry[24], entry[25], entry[26], entry[27], entry[28], entry[29], entry[30],
                entry[31],
            ]),
            next_allocation_search_cluster: 0,
            resident_bitmap: Vec::new(),
            dirty_byte_ranges: Vec::new(),
            dirty_byte_generations: Vec::new(),
            published_dirty_generation: None,
            next_dirty_generation: 0,
            used_clusters: 0,
            lazy_reclaimed_clusters: Vec::new(),
        })
    }

    fn bitmap_lengths(
        &self,
        cluster_count: usize,
        boot_region: &BootRegion,
    ) -> Result<(usize, usize)> {
        boot_region.validate_stream_data(self.first_cluster, self.data_length)?;
        let required_bytes = cluster_count.div_ceil(8);
        let bitmap_bytes =
            usize::try_from(self.data_length).map_err(|_| invalid_on_disk_layout())?;
        if bitmap_bytes < required_bytes {
            return Err(inconsistent_bitmap_accounting());
        }
        Ok((bitmap_bytes, required_bytes))
    }

    fn commit_run_to_ranges(
        boot_region: &BootRegion,
        ranges: &mut Vec<ClusterRange>,
        run_start_index: &mut Option<usize>,
        run_cluster_count: &mut usize,
        requested_clusters_remaining: &mut usize,
    ) -> Result<bool> {
        let Some(start_cluster_index) = run_start_index.take() else {
            *run_cluster_count = 0;
            return Ok(*requested_clusters_remaining == 0);
        };
        let allocated_clusters = (*run_cluster_count).min(*requested_clusters_remaining);
        *run_cluster_count = 0;
        if allocated_clusters == 0 {
            return Ok(*requested_clusters_remaining == 0);
        }

        ranges.push(ClusterRange {
            start_cluster: boot_region.cluster_from_index(start_cluster_index)?,
            cluster_count: allocated_clusters,
        });
        *requested_clusters_remaining -= allocated_clusters;
        Ok(*requested_clusters_remaining == 0)
    }

    fn record_dirty_byte_range(&mut self, mut dirty_byte_range: Range<usize>) -> Result<()> {
        if dirty_byte_range.is_empty() {
            return Ok(());
        }
        if dirty_byte_range.end > self.dirty_byte_generations.len() {
            return Err(inconsistent_bitmap_accounting());
        }

        self.next_dirty_generation = self.next_dirty_generation.saturating_add(1);
        for dirty_generation in &mut self.dirty_byte_generations[dirty_byte_range.clone()] {
            *dirty_generation = self.next_dirty_generation;
        }

        let mut merged_dirty_ranges = Vec::with_capacity(self.dirty_byte_ranges.len() + 1);
        let mut inserted = false;
        for existing_range in self.dirty_byte_ranges.drain(..) {
            if existing_range.end < dirty_byte_range.start {
                merged_dirty_ranges.push(existing_range);
                continue;
            }
            if dirty_byte_range.end < existing_range.start {
                if !inserted {
                    merged_dirty_ranges.push(dirty_byte_range.clone());
                    inserted = true;
                }
                merged_dirty_ranges.push(existing_range);
                continue;
            }
            dirty_byte_range.start = dirty_byte_range.start.min(existing_range.start);
            dirty_byte_range.end = dirty_byte_range.end.max(existing_range.end);
        }
        if !inserted {
            merged_dirty_ranges.push(dirty_byte_range);
        }
        self.dirty_byte_ranges = merged_dirty_ranges;
        Ok(())
    }

    fn rebuild_dirty_byte_ranges(&mut self) {
        let mut rebuilt_dirty_ranges = Vec::new();
        let mut current_range_start = None;
        for (byte_index, dirty_generation) in self.dirty_byte_generations.iter().enumerate() {
            if *dirty_generation != 0 {
                if current_range_start.is_none() {
                    current_range_start = Some(byte_index);
                }
                continue;
            }

            if let Some(range_start) = current_range_start.take() {
                rebuilt_dirty_ranges.push(range_start..byte_index);
            }
        }
        if let Some(range_start) = current_range_start {
            rebuilt_dirty_ranges.push(range_start..self.dirty_byte_generations.len());
        }
        self.dirty_byte_ranges = rebuilt_dirty_ranges;
    }

    fn relevant_bitmap_mask(relevant_bits: usize) -> Result<u8> {
        if relevant_bits > u8::BITS as usize {
            return Err(inconsistent_bitmap_accounting());
        }
        if relevant_bits == u8::BITS as usize {
            return Ok(u8::MAX);
        }
        let shift = u32::try_from(relevant_bits).map_err(|_| inconsistent_bitmap_accounting())?;
        let shifted = 1u16
            .checked_shl(shift)
            .ok_or_else(inconsistent_bitmap_accounting)?;
        Ok((shifted - 1) as u8)
    }
}
