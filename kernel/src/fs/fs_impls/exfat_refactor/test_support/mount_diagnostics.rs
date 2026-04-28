// SPDX-License-Identifier: MPL-2.0

use aster_block::BlockDevice;

use super::{
    super::fat::FatReader,
    bitmap::diagnose_count_used_clusters,
    boot_region::{diagnose_boot_region, read_anomaly_state},
    root_directory::diagnose_scan_root_directory,
    upcase::diagnose_load_upcase_table,
};

pub(super) fn diagnose_invalid_on_disk_layout_gate(block_device: &dyn BlockDevice) -> &'static str {
    let boot_region = match diagnose_boot_region(block_device) {
        Ok(boot_region) => boot_region,
        Err(gate) => return gate,
    };
    if let Err(gate) = read_anomaly_state(block_device, &boot_region) {
        return gate;
    }
    let mut fat_reader = FatReader::new(block_device, &boot_region);
    let (bitmap, upcase) =
        match diagnose_scan_root_directory(block_device, &boot_region, &mut fat_reader) {
            Ok(records) => records,
            Err(gate) => return gate,
        };
    if let Err(gate) =
        diagnose_load_upcase_table(block_device, &boot_region, &mut fat_reader, upcase)
    {
        return gate;
    }
    match diagnose_count_used_clusters(block_device, &boot_region, &mut fat_reader, bitmap) {
        Ok(()) => "accepted",
        Err(gate) => gate,
    }
}
