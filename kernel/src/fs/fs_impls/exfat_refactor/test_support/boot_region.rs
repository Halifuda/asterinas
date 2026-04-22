// SPDX-License-Identifier: MPL-2.0

use alloc::vec;
use core::mem;

use aster_block::BlockDevice;
use ostd::mm::VmIo;

use super::super::boot::BootRegion;

const MAX_CLUSTER_SIZE: usize = 32 * 1024 * 1024;

pub(super) fn diagnose_boot_region(
    block_device: &dyn BlockDevice,
) -> core::result::Result<BootRegion, &'static str> {
    let mut sector_header = [0u8; 512];
    block_device
        .read_bytes(0, &mut sector_header)
        .map_err(|_| "read_boot_region:device_io")?;
    if &sector_header[3..11] != b"EXFAT   " {
        return Err("read_boot_region:oem_name");
    }
    if u16::from_le_bytes([sector_header[510], sector_header[511]]) != 0xAA55 {
        return Err("read_boot_region:boot_signature");
    }

    let bytes_per_sector_shift = sector_header[108];
    if !(9..=12).contains(&bytes_per_sector_shift) {
        return Err("read_boot_region:bytes_per_sector_shift");
    }
    let sector_size = match 1usize.checked_shl(u32::from(bytes_per_sector_shift)) {
        Some(sector_size) => sector_size,
        None => return Err("read_boot_region:sector_size_shift_overflow"),
    };
    let sectors_per_cluster_shift = sector_header[109];
    let sectors_per_cluster = match 1usize.checked_shl(u32::from(sectors_per_cluster_shift)) {
        Some(sectors_per_cluster) => sectors_per_cluster,
        None => return Err("read_boot_region:sectors_per_cluster_shift_overflow"),
    };
    let cluster_size = match sector_size.checked_mul(sectors_per_cluster) {
        Some(cluster_size) if cluster_size != 0 => cluster_size,
        _ => return Err("read_boot_region:cluster_size_overflow"),
    };
    if cluster_size > MAX_CLUSTER_SIZE {
        return Err("read_boot_region:cluster_size_too_large");
    }

    let fat_offset_sectors = u32::from_le_bytes([
        sector_header[80],
        sector_header[81],
        sector_header[82],
        sector_header[83],
    ]);
    let fat_length_sectors = u32::from_le_bytes([
        sector_header[84],
        sector_header[85],
        sector_header[86],
        sector_header[87],
    ]);
    let cluster_count = u32::from_le_bytes([
        sector_header[92],
        sector_header[93],
        sector_header[94],
        sector_header[95],
    ]);
    let number_of_fats = sector_header[110];
    if number_of_fats != 1 {
        return Err("read_boot_region:number_of_fats");
    }
    if fat_offset_sectors == 0 {
        return Err("read_boot_region:fat_offset_zero");
    }
    if fat_length_sectors == 0 {
        return Err("read_boot_region:fat_length_zero");
    }
    if cluster_count == 0 {
        return Err("read_boot_region:cluster_count_zero");
    }

    let boot_region = BootRegion {
        cluster_count,
        cluster_heap_offset_sectors: u32::from_le_bytes([
            sector_header[88],
            sector_header[89],
            sector_header[90],
            sector_header[91],
        ]),
        cluster_size,
        fat_length_sectors,
        fat_offset_sectors,
        percent_in_use: sector_header[112],
        root_dir_cluster: u32::from_le_bytes([
            sector_header[96],
            sector_header[97],
            sector_header[98],
            sector_header[99],
        ]),
        sector_size,
        sectors_per_cluster,
        volume_length_sectors: u64::from_le_bytes([
            sector_header[72],
            sector_header[73],
            sector_header[74],
            sector_header[75],
            sector_header[76],
            sector_header[77],
            sector_header[78],
            sector_header[79],
        ]),
        volume_serial_number: u32::from_le_bytes([
            sector_header[100],
            sector_header[101],
            sector_header[102],
            sector_header[103],
        ]),
    };
    diagnose_validate_boot_geometry(&boot_region)?;
    diagnose_validate_boot_checksum(block_device, &boot_region)?;
    Ok(boot_region)
}

pub(super) fn read_anomaly_state(
    block_device: &dyn BlockDevice,
    boot_region: &BootRegion,
) -> core::result::Result<(), &'static str> {
    let mut boot_sector = vec![0; boot_region.sector_size];
    block_device
        .read_bytes(0, &mut boot_sector)
        .map_err(|_| "read_anomaly_state:device_io")
}

fn diagnose_validate_boot_checksum(
    block_device: &dyn BlockDevice,
    boot_region: &BootRegion,
) -> core::result::Result<(), &'static str> {
    let checksum_region_len = match boot_region.sector_size.checked_mul(11) {
        Some(checksum_region_len) => checksum_region_len,
        None => return Err("validate_boot_checksum:checksum_region_len_overflow"),
    };
    let mut checksum_region = vec![0; checksum_region_len];
    block_device
        .read_bytes(0, &mut checksum_region)
        .map_err(|_| "validate_boot_checksum:checksum_region_device_io")?;
    let expected_checksum = boot_region_checksum(&checksum_region);

    let mut checksum_sector = vec![0; boot_region.sector_size];
    block_device
        .read_bytes(checksum_region_len, &mut checksum_sector)
        .map_err(|_| "validate_boot_checksum:checksum_sector_device_io")?;
    for chunk in checksum_sector.chunks_exact(mem::size_of::<u32>()) {
        if u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]) != expected_checksum {
            return Err("validate_boot_checksum:mismatched_checksum_sector");
        }
    }
    Ok(())
}

fn diagnose_validate_boot_geometry(
    boot_region: &BootRegion,
) -> core::result::Result<(), &'static str> {
    if !boot_region.is_valid_cluster(boot_region.root_dir_cluster) {
        return Err("validate_boot_geometry:root_dir_cluster_out_of_range");
    }
    let sectors_per_cluster = match u64::try_from(boot_region.sectors_per_cluster) {
        Ok(sectors_per_cluster) => sectors_per_cluster,
        Err(_) => return Err("validate_boot_geometry:sectors_per_cluster_conversion"),
    };
    let data_sectors = match u64::from(boot_region.cluster_count).checked_mul(sectors_per_cluster) {
        Some(data_sectors) => data_sectors,
        None => return Err("validate_boot_geometry:data_sectors_overflow"),
    };
    let heap_end =
        match u64::from(boot_region.cluster_heap_offset_sectors).checked_add(data_sectors) {
            Some(heap_end) => heap_end,
            None => return Err("validate_boot_geometry:heap_end_overflow"),
        };
    if heap_end > boot_region.volume_length_sectors {
        return Err("validate_boot_geometry:heap_end_past_volume");
    }
    let fat_end = match u64::from(boot_region.fat_offset_sectors)
        .checked_add(u64::from(boot_region.fat_length_sectors))
    {
        Some(fat_end) => fat_end,
        None => return Err("validate_boot_geometry:fat_end_overflow"),
    };
    if fat_end > u64::from(boot_region.cluster_heap_offset_sectors) {
        return Err("validate_boot_geometry:fat_overlaps_cluster_heap");
    }
    Ok(())
}

fn boot_region_checksum(bytes: &[u8]) -> u32 {
    let mut checksum = 0u32;
    for (offset, byte) in bytes.iter().enumerate() {
        if offset == 106 || offset == 107 || offset == 112 {
            continue;
        }
        checksum = checksum.rotate_right(1).wrapping_add(u32::from(*byte));
    }
    checksum
}
