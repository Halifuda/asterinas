// SPDX-License-Identifier: MPL-2.0

use alloc::{
    collections::BTreeSet,
    vec,
};

use aster_block::BlockDevice;
use ostd::mm::VmIo;

use super::super::{
    bitmap::AllocationBitmap,
    boot::BootRegion,
    fat::{FatChainStep, FatReader},
    fs::MountVolumeStateError,
};

pub(super) fn diagnose_count_used_clusters(
    block_device: &dyn BlockDevice,
    boot_region: &BootRegion,
    fat_reader: &mut FatReader<'_>,
    bitmap: AllocationBitmap,
) -> core::result::Result<(), &'static str> {
    if !boot_region.is_valid_cluster(bitmap.first_cluster) {
        return Err("count_used_clusters:first_cluster_out_of_range");
    }
    if bitmap.data_length == 0 {
        return Err("count_used_clusters:data_length_zero");
    }
    let data_capacity = match boot_region.data_capacity_bytes() {
        Ok(data_capacity) => data_capacity,
        Err(_) => return Err("count_used_clusters:data_capacity_overflow"),
    };
    if bitmap.data_length > data_capacity {
        return Err("count_used_clusters:data_length_exceeds_data_capacity");
    }
    let cluster_count = match boot_region.cluster_count_usize() {
        Ok(cluster_count) => cluster_count,
        Err(_) => return Err("count_used_clusters:cluster_count_usize_conversion"),
    };
    let required_bytes = cluster_count.div_ceil(8);
    let bitmap_bytes = match usize::try_from(bitmap.data_length) {
        Ok(bitmap_bytes) => bitmap_bytes,
        Err(_) => return Err("count_used_clusters:data_length_usize_conversion"),
    };
    if bitmap_bytes < required_bytes {
        return Err("count_used_clusters:bitmap_shorter_than_cluster_map");
    }

    let mut bits_remaining = cluster_count;
    let mut bitmap_bytes_remaining = bitmap_bytes;
    let mut cluster_buffer = vec![0; boot_region.cluster_size];
    let mut current_cluster = bitmap.first_cluster;
    let mut visited_clusters = BTreeSet::new();

    loop {
        if !visited_clusters.insert(current_cluster) {
            return Err("count_used_clusters:cluster_chain_loop");
        }
        let cluster_offset = match boot_region.cluster_offset(current_cluster) {
            Ok(cluster_offset) => cluster_offset,
            Err(_) => return Err("count_used_clusters:cluster_offset_invalid"),
        };
        block_device
            .read_bytes(cluster_offset, &mut cluster_buffer)
            .map_err(|_| "count_used_clusters:device_io")?;

        let bytes_to_visit = bitmap_bytes_remaining.min(cluster_buffer.len());
        for byte in &cluster_buffer[..bytes_to_visit] {
            if bits_remaining == 0 {
                if *byte != 0 {
                    return Err("count_used_clusters:nonzero_trailing_stream_padding");
                }
                continue;
            }
            let relevant_bits = bits_remaining.min(u8::BITS as usize);
            let mask = if relevant_bits == u8::BITS as usize {
                u8::MAX
            } else {
                let shift = match u32::try_from(relevant_bits) {
                    Ok(shift) => shift,
                    Err(_) => return Err("count_used_clusters:mask_shift_overflow"),
                };
                match 1u16.checked_shl(shift) {
                    Some(shifted) => (shifted - 1) as u8,
                    None => return Err("count_used_clusters:mask_shift_overflow"),
                }
            };
            let masked_byte = *byte & mask;
            if masked_byte != *byte && (*byte & !mask) != 0 {
                return Err("count_used_clusters:nonzero_unused_bits");
            }
            bits_remaining -= relevant_bits;
        }
        bitmap_bytes_remaining -= bytes_to_visit;
        if bitmap_bytes_remaining == 0 {
            return if bits_remaining == 0 {
                Ok(())
            } else {
                Err("count_used_clusters:stream_shorter_than_bitmap")
            };
        }

        current_cluster = match fat_reader.next_cluster(current_cluster) {
            Ok(FatChainStep::Continue(next_cluster)) => next_cluster,
            Ok(FatChainStep::End) => {
                return Err("count_used_clusters:stream_shorter_than_bitmap");
            }
            Err(MountVolumeStateError::DeviceIo) => return Err("count_used_clusters:fat_device_io"),
            Err(_) => return Err("count_used_clusters:fat_chain_invalid"),
        };
    }
}
