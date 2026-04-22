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
    diagnose_decode_mapping(&table_bytes)?;
    Ok(())
}

fn diagnose_decode_mapping(table_bytes: &[u8]) -> core::result::Result<(), &'static str> {
    const TABLE_CODE_UNIT_COUNT: usize = u16::MAX as usize + 1;
    const UNCOMPRESSED_TABLE_BYTE_LEN: usize = TABLE_CODE_UNIT_COUNT * 2;

    let mapping = if table_bytes.len() == UNCOMPRESSED_TABLE_BYTE_LEN {
        let mut mapping = Vec::with_capacity(TABLE_CODE_UNIT_COUNT);
        for word in table_bytes.chunks_exact(2) {
            mapping.push(u16::from_le_bytes([word[0], word[1]]));
        }
        mapping
    } else {
        let mut words = table_bytes.chunks_exact(2);
        if !words.remainder().is_empty() {
            return Err("load_upcase_table:odd_table_length");
        }

        let mut mapping = Vec::with_capacity(TABLE_CODE_UNIT_COUNT);
        while let Some(word) = words.next() {
            let value = u16::from_le_bytes([word[0], word[1]]);
            if value != u16::MAX {
                if mapping.len() == TABLE_CODE_UNIT_COUNT {
                    return Err("load_upcase_table:mapping_too_long");
                }
                mapping.push(value);
                continue;
            }

            let Some(identity_count_word) = words.next() else {
                if mapping.len() == usize::from(u16::MAX) {
                    mapping.push(u16::MAX);
                    break;
                }
                return Err("load_upcase_table:identity_run_missing_count");
            };
            let identity_count =
                u16::from_le_bytes([identity_count_word[0], identity_count_word[1]]);
            if identity_count == 0 {
                return Err("load_upcase_table:empty_identity_run");
            }
            let run_end = match mapping.len().checked_add(usize::from(identity_count)) {
                Some(run_end) => run_end,
                None => return Err("load_upcase_table:mapping_length_overflow"),
            };
            if run_end > TABLE_CODE_UNIT_COUNT {
                return Err("load_upcase_table:mapping_too_long");
            }
            for code_unit in mapping.len()..run_end {
                let code_unit =
                    u16::try_from(code_unit).map_err(|_| "load_upcase_table:mapping_too_long")?;
                mapping.push(code_unit);
            }
        }

        mapping
    };

    if mapping.len() != TABLE_CODE_UNIT_COUNT {
        return Err("load_upcase_table:incomplete_mapping");
    }
    for code_unit in 0u8..128 {
        let expected_mapping = match code_unit {
            b'a'..=b'z' => u16::from(code_unit - b'a' + b'A'),
            _ => u16::from(code_unit),
        };
        if mapping[usize::from(code_unit)] != expected_mapping {
            return Err("load_upcase_table:mandatory_first_128_mismatch");
        }
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
