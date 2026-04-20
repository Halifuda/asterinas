// SPDX-License-Identifier: MPL-2.0

use alloc::{
    collections::BTreeSet,
    vec,
    vec::Vec,
};

use aster_block::BlockDevice;
use ostd::mm::VmIo;

use super::super::{
    boot::BootRegion,
    fat::{FatChainStep, FatReader},
    fs::MountVolumeStateError,
    upcase::UpcaseRecord,
};

pub(super) fn diagnose_load_upcase_table(
    block_device: &dyn BlockDevice,
    boot_region: &BootRegion,
    fat_reader: &mut FatReader<'_>,
    upcase: UpcaseRecord,
) -> core::result::Result<(), &'static str> {
    if !boot_region.is_valid_cluster(upcase.first_cluster) {
        return Err("load_upcase_table:first_cluster_out_of_range");
    }
    if upcase.data_length == 0 {
        return Err("load_upcase_table:data_length_zero");
    }
    let data_capacity = match boot_region.data_capacity_bytes() {
        Ok(data_capacity) => data_capacity,
        Err(_) => return Err("load_upcase_table:data_capacity_overflow"),
    };
    if upcase.data_length > data_capacity {
        return Err("load_upcase_table:data_length_exceeds_data_capacity");
    }
    let data_length = match usize::try_from(upcase.data_length) {
        Ok(data_length) => data_length,
        Err(_) => return Err("load_upcase_table:data_length_usize_conversion"),
    };
    let mut remaining = data_length;
    let mut table_bytes = Vec::with_capacity(data_length);
    let mut cluster_buffer = vec![0; boot_region.cluster_size];
    let mut current_cluster = upcase.first_cluster;
    let mut visited_clusters = BTreeSet::new();

    loop {
        if !visited_clusters.insert(current_cluster) {
            return Err("load_upcase_table:cluster_chain_loop");
        }
        let cluster_offset = match boot_region.cluster_offset(current_cluster) {
            Ok(cluster_offset) => cluster_offset,
            Err(_) => return Err("load_upcase_table:cluster_offset_invalid"),
        };
        block_device
            .read_bytes(cluster_offset, &mut cluster_buffer)
            .map_err(|_| "load_upcase_table:device_io")?;

        let bytes_to_copy = remaining.min(cluster_buffer.len());
        table_bytes.extend_from_slice(&cluster_buffer[..bytes_to_copy]);
        remaining -= bytes_to_copy;
        if remaining == 0 {
            break;
        }

        current_cluster = match fat_reader.next_cluster(current_cluster) {
            Ok(FatChainStep::Continue(next_cluster)) => next_cluster,
            Ok(FatChainStep::End) => {
                return Err("load_upcase_table:stream_shorter_than_data_length");
            }
            Err(MountVolumeStateError::DeviceIo) => return Err("load_upcase_table:fat_device_io"),
            Err(_) => return Err("load_upcase_table:fat_chain_invalid"),
        };
    }

    if stream_checksum(&table_bytes) != upcase.checksum {
        return Err("load_upcase_table:stream_checksum_mismatch");
    }
    Ok(())
}

fn stream_checksum(bytes: &[u8]) -> u32 {
    let mut checksum = 0u32;
    for byte in bytes {
        checksum = checksum.rotate_right(1).wrapping_add(u32::from(*byte));
    }
    checksum
}
