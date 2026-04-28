// SPDX-License-Identifier: MPL-2.0

use alloc::{collections::BTreeSet, vec};

use aster_block::BlockDevice;
use ostd::mm::VmIo;

use super::super::{
    bitmap::{ALLOCATION_BITMAP_ENTRY_TYPE, AllocationBitmap},
    boot::BootRegion,
    fat::{FatChainStep, FatReader},
    fs::MountVolumeStateError,
    upcase::{UPCASE_TABLE_ENTRY_TYPE, UpcaseRecord},
};

const END_OF_DIRECTORY_ENTRY_TYPE: u8 = 0x00;

pub(super) fn diagnose_scan_root_directory(
    block_device: &dyn BlockDevice,
    boot_region: &BootRegion,
    fat_reader: &mut FatReader<'_>,
) -> core::result::Result<(AllocationBitmap, UpcaseRecord), &'static str> {
    let mut cluster_buffer = vec![0; boot_region.cluster_size];
    let mut current_cluster = boot_region.root_dir_cluster;
    let mut visited_clusters = BTreeSet::new();
    let mut bitmap = None;
    let mut upcase = None;

    loop {
        if !visited_clusters.insert(current_cluster) {
            return Err("scan_root_directory:cluster_chain_loop");
        }
        let cluster_offset = match boot_region.cluster_offset(current_cluster) {
            Ok(cluster_offset) => cluster_offset,
            Err(_) => return Err("scan_root_directory:cluster_offset_invalid"),
        };
        block_device
            .read_bytes(cluster_offset, &mut cluster_buffer)
            .map_err(|_| "scan_root_directory:device_io")?;
        for entry in cluster_buffer.chunks_exact(32) {
            match entry[0] {
                END_OF_DIRECTORY_ENTRY_TYPE => return finalize_root_records(bitmap, upcase),
                ALLOCATION_BITMAP_ENTRY_TYPE => match AllocationBitmap::parse(entry) {
                    Ok(record) => bitmap = Some(record),
                    Err(_) => return Err("scan_root_directory:invalid_allocation_bitmap_record"),
                },
                UPCASE_TABLE_ENTRY_TYPE => match UpcaseRecord::parse(entry) {
                    Ok(record) => upcase = Some(record),
                    Err(_) => return Err("scan_root_directory:invalid_upcase_record"),
                },
                _ => (),
            }
            if bitmap.is_some() && upcase.is_some() {
                return finalize_root_records(bitmap, upcase);
            }
        }

        current_cluster = match fat_reader.next_cluster(current_cluster) {
            Ok(FatChainStep::Continue(next_cluster)) => next_cluster,
            Ok(FatChainStep::End) => return finalize_root_records(bitmap, upcase),
            Err(MountVolumeStateError::DeviceIo) => {
                return Err("scan_root_directory:fat_device_io");
            }
            Err(_) => return Err("scan_root_directory:fat_chain_invalid"),
        };
    }
}

fn finalize_root_records(
    bitmap: Option<AllocationBitmap>,
    upcase: Option<UpcaseRecord>,
) -> core::result::Result<(AllocationBitmap, UpcaseRecord), &'static str> {
    match (bitmap, upcase) {
        (Some(bitmap), Some(upcase)) => Ok((bitmap, upcase)),
        (None, _) => Err("scan_root_directory:missing_allocation_bitmap_record"),
        (_, None) => Err("scan_root_directory:missing_upcase_record"),
    }
}
