// SPDX-License-Identifier: MPL-2.0
#![cfg_attr(
    not(ktest),
    expect(
        dead_code,
        reason = "Boot helpers are staged before mount integration."
    )
)]

use core::mem::size_of;

use aster_block::BlockDevice;
use ostd::mm::VmIo;

use super::{io::read_metadata_bytes, super_block::ExfatSuperBlock};
use crate::prelude::*;

pub(super) const BOOT_SIGNATURE: u16 = 0xAA55;
pub(super) const EXFAT_FIRST_CLUSTER: u32 = 2;
pub(super) const EXFAT_RESERVED_CLUSTERS: u32 = 2;
pub(super) const EXFAT_MAX_SECT_SIZE_BITS: u8 = 12;
pub(super) const EXFAT_MIN_SECT_SIZE_BITS: u8 = 9;
pub(super) const MEDIA_FAILURE: u16 = 0x0004;
pub(super) const VOLUME_DIRTY: u16 = 0x0002;

const EXFAT_BOOT_REGION_SECTORS: usize = 11;
const EXFAT_CHECKSUM_SECTOR_INDEX: usize = 11;
const EXFAT_FAT_OFFSET_MIN: u32 = 24;
const EXFAT_MAX_CLUSTER_SIZE_BITS: u8 = 25;
const EXFAT_NAME: [u8; 8] = *b"EXFAT   ";
const EXFAT_VOLUME_FLAG_MASK: u16 = VOLUME_DIRTY | MEDIA_FAILURE;
const FAT_ENTRY_SIZE: u64 = size_of::<u32>() as u64;

#[repr(C, packed)]
#[derive(Clone, Copy, Debug, Pod)]
pub(super) struct ExfatBootSector {
    pub(super) jmp_boot: [u8; 3],
    pub(super) fs_name: [u8; 8],
    pub(super) must_be_zero: [u8; 53],
    pub(super) partition_offset: u64,
    pub(super) vol_length: u64,
    pub(super) fat_offset: u32,
    pub(super) fat_length: u32,
    pub(super) cluster_offset: u32,
    pub(super) cluster_count: u32,
    pub(super) root_cluster: u32,
    pub(super) vol_serial: u32,
    pub(super) fs_revision: [u8; 2],
    pub(super) vol_flags: u16,
    pub(super) sector_size_bits: u8,
    pub(super) sector_per_cluster_bits: u8,
    pub(super) num_fats: u8,
    pub(super) drv_sel: u8,
    pub(super) percent_in_use: u8,
    pub(super) reserved: [u8; 7],
    pub(super) boot_code: [u8; 390],
    pub(super) signature: u16,
}

pub(super) fn read_primary_super_block(block_device: &dyn BlockDevice) -> Result<ExfatSuperBlock> {
    let boot_sector = read_primary_boot_sector(block_device)?;
    validate_primary_boot_sector(&boot_sector)?;
    verify_primary_boot_region_checksum(block_device, &boot_sector)?;
    Ok(ExfatSuperBlock::from(boot_sector))
}

pub(super) fn read_primary_boot_sector(block_device: &dyn BlockDevice) -> Result<ExfatBootSector> {
    Ok(block_device.read_val(0)?)
}

pub(super) fn validate_primary_boot_sector(boot_sector: &ExfatBootSector) -> Result<()> {
    if boot_sector.signature != BOOT_SIGNATURE {
        return_errno_with_message!(Errno::EINVAL, "invalid boot record signature");
    }

    if boot_sector.fs_name != EXFAT_NAME {
        return_errno_with_message!(Errno::EINVAL, "invalid fs name");
    }

    if boot_sector.must_be_zero.iter().any(|&byte| byte != 0) {
        return_errno_with_message!(Errno::EINVAL, "must_be_zero field must be filled with zero");
    }

    if boot_sector.num_fats != 1 && boot_sector.num_fats != 2 {
        return_errno_with_message!(Errno::EINVAL, "bogus number of FAT structure");
    }

    if boot_sector.sector_size_bits < EXFAT_MIN_SECT_SIZE_BITS
        || boot_sector.sector_size_bits > EXFAT_MAX_SECT_SIZE_BITS
    {
        return_errno_with_message!(Errno::EINVAL, "bogus sector size bits");
    }

    let cluster_size_bits = boot_sector
        .sector_size_bits
        .checked_add(boot_sector.sector_per_cluster_bits)
        .ok_or_else(|| Error::with_message(Errno::EINVAL, "bogus sector size bits per cluster"))?;
    if cluster_size_bits > EXFAT_MAX_CLUSTER_SIZE_BITS {
        return_errno_with_message!(Errno::EINVAL, "bogus sector size bits per cluster");
    }

    if boot_sector.fat_offset < EXFAT_FAT_OFFSET_MIN {
        return_errno_with_message!(Errno::EINVAL, "bogus fat offset");
    }

    if boot_sector.fat_length == 0 {
        return_errno_with_message!(Errno::EINVAL, "bogus fat length");
    }

    if boot_sector.cluster_count == 0 {
        return_errno_with_message!(Errno::EINVAL, "bogus cluster count");
    }

    let max_cluster_id = boot_sector
        .cluster_count
        .checked_add(EXFAT_RESERVED_CLUSTERS)
        .and_then(|cluster_count| cluster_count.checked_sub(1))
        .ok_or_else(|| Error::with_message(Errno::EINVAL, "bogus cluster count"))?;
    if boot_sector.root_cluster < EXFAT_RESERVED_CLUSTERS
        || boot_sector.root_cluster > max_cluster_id
    {
        return_errno_with_message!(Errno::EINVAL, "bogus root directory cluster");
    }

    let fat_length_bytes = u64::from(boot_sector.fat_length) << boot_sector.sector_size_bits;
    let addressable_clusters = u64::from(max_cluster_id) + 1;
    if fat_length_bytes < addressable_clusters * FAT_ENTRY_SIZE {
        return_errno_with_message!(Errno::EINVAL, "bogus fat length");
    }

    let fat_region_end = u64::from(boot_sector.fat_offset)
        + u64::from(boot_sector.fat_length) * u64::from(boot_sector.num_fats);
    if u64::from(boot_sector.cluster_offset) < fat_region_end {
        return_errno_with_message!(Errno::EINVAL, "bogus data start vector");
    }

    if boot_sector.vol_length <= u64::from(boot_sector.cluster_offset) {
        return_errno_with_message!(Errno::EINVAL, "bogus volume length");
    }

    Ok(())
}

pub(super) fn verify_primary_boot_region_checksum(
    block_device: &dyn BlockDevice,
    boot_sector: &ExfatBootSector,
) -> Result<()> {
    let bytes_per_sector = 1usize << boot_sector.sector_size_bits;
    let boot_region = read_main_boot_region(block_device, bytes_per_sector)?;

    let mut checksum_sector = vec![0; bytes_per_sector];
    read_metadata_bytes(
        block_device,
        EXFAT_CHECKSUM_SECTOR_INDEX * bytes_per_sector,
        &mut checksum_sector,
    )?;

    // exFAT checksum calculation skips the mutable checksum fields so the
    // stored checksum sector can authenticate the rest of the boot region.
    let checksum = boot_region
        .iter()
        .enumerate()
        .filter(|(index, _)| !matches!(*index, 106 | 107 | 112))
        .fold(0u32, |checksum, (_, byte)| {
            checksum.rotate_right(1).wrapping_add(u32::from(*byte))
        });

    for entry in checksum_sector.chunks_exact(size_of::<u32>()) {
        let expected = u32::from_le_bytes(entry.try_into().unwrap());
        if expected != checksum {
            return_errno_with_message!(Errno::EINVAL, "invalid boot region checksum");
        }
    }

    Ok(())
}

fn read_main_boot_region(
    block_device: &dyn BlockDevice,
    bytes_per_sector: usize,
) -> Result<Vec<u8>> {
    let mut boot_region = vec![0; EXFAT_BOOT_REGION_SECTORS * bytes_per_sector];
    read_metadata_bytes(block_device, 0, &mut boot_region)?;

    Ok(boot_region)
}

pub(super) fn persistent_volume_flags(boot_sector: &ExfatBootSector) -> u32 {
    u32::from(boot_sector.vol_flags & EXFAT_VOLUME_FLAG_MASK)
}

#[cfg(ktest)]
mod tests {
    use alloc::vec;

    use ostd::prelude::ktest;
    use zerocopy::IntoBytes;

    use super::{
        read_primary_boot_sector, read_primary_super_block, ExfatBootSector,
        EXFAT_RESERVED_CLUSTERS,
    };
    use crate::fs::fs_impls::exfat_refactor::{
        super_block::ExfatSuperBlock, test_support::load_exfat_disk,
    };

    fn assert_super_block_matches_boot_sector(
        super_block: ExfatSuperBlock,
        boot_sector: ExfatBootSector,
    ) {
        let sector_size_bits = boot_sector.sector_size_bits;
        let sector_per_cluster_bits = boot_sector.sector_per_cluster_bits;
        let fat_offset = boot_sector.fat_offset;
        let fat_length = boot_sector.fat_length;
        let cluster_offset = boot_sector.cluster_offset;
        let vol_length = boot_sector.vol_length;
        let root_cluster = boot_sector.root_cluster;

        assert_eq!(super_block.sector_size, 1u32 << sector_size_bits);
        assert_eq!(
            super_block.sect_per_cluster,
            1u32 << sector_per_cluster_bits
        );
        assert_eq!(
            super_block.cluster_size_bits,
            u32::from(sector_size_bits + sector_per_cluster_bits)
        );
        assert_eq!(
            super_block.cluster_size,
            1u32 << super_block.cluster_size_bits
        );
        assert_eq!(super_block.fat1_start_sector, u64::from(fat_offset));
        assert_eq!(super_block.num_fat_sectors, fat_length);
        assert_eq!(super_block.data_start_sector, u64::from(cluster_offset));
        assert_eq!(super_block.num_sectors, vol_length);
        assert_eq!(super_block.root_dir, root_cluster);
        assert_eq!(super_block.dentries_per_clu, super_block.cluster_size / 32);
        assert_eq!(super_block.cluster_search_ptr, EXFAT_RESERVED_CLUSTERS);
    }

    #[ktest]
    fn boot_region_loads_super_block() {
        // Confirms a valid primary boot region produces normalized runtime
        // geometry that still matches the serialized boot sector fields.
        let disk = load_exfat_disk();
        let boot_sector = read_primary_boot_sector(&disk).unwrap();
        let super_block = read_primary_super_block(&disk).unwrap();

        assert_super_block_matches_boot_sector(super_block, boot_sector);
    }

    #[ktest]
    fn boot_region_rejects_invalid_signature() {
        // Confirms primary-boot parsing rejects a sector whose trailer no longer
        // carries the mandatory exFAT signature.
        let disk = load_exfat_disk();
        let mut boot_sector = read_primary_boot_sector(&disk).unwrap();
        boot_sector.signature = 0;
        disk.write_bytes(0, boot_sector.as_bytes());

        assert!(read_primary_super_block(&disk).is_err());
    }

    #[ktest]
    fn boot_region_rejects_invalid_fs_name() {
        // Confirms the boot record is rejected when the filesystem name marker
        // is no longer the required `EXFAT   ` identifier.
        let disk = load_exfat_disk();
        let mut boot_sector = read_primary_boot_sector(&disk).unwrap();
        boot_sector.fs_name = *b"INVALID!";
        disk.write_bytes(0, boot_sector.as_bytes());

        assert!(read_primary_super_block(&disk).is_err());
    }

    #[ktest]
    fn boot_region_rejects_nonzero_reserved_bytes() {
        // Confirms validation fails if the reserved bytes stop being all-zero,
        // because later components rely on that serialized invariant.
        let disk = load_exfat_disk();
        let mut boot_sector = read_primary_boot_sector(&disk).unwrap();
        boot_sector.must_be_zero[0] = 1;
        disk.write_bytes(0, boot_sector.as_bytes());

        assert!(read_primary_super_block(&disk).is_err());
    }

    #[ktest]
    fn boot_region_rejects_corrupted_checksum_region() {
        // Confirms checksum verification catches corruption in the protected
        // boot-region payload before the checksum sector itself is read.
        let disk = load_exfat_disk();
        let boot_sector = read_primary_boot_sector(&disk).unwrap();
        let sector_size = 1usize << boot_sector.sector_size_bits;
        let boot_region = 11 * sector_size;
        let mut corrupted = vec![0; boot_region];
        disk.read_bytes(0, &mut corrupted);
        corrupted[128] ^= 0x5A;
        disk.write_bytes(0, &corrupted);

        assert!(read_primary_super_block(&disk).is_err());
    }

    #[ktest]
    fn boot_region_rejects_corrupted_checksum_sector() {
        // Confirms checksum verification also rejects tampering in the replicated
        // checksum sector that authenticates the primary boot region.
        let disk = load_exfat_disk();
        let boot_sector = read_primary_boot_sector(&disk).unwrap();
        let sector_size = 1usize << boot_sector.sector_size_bits;
        let checksum_sector_offset = 11 * sector_size;
        let mut corrupted = vec![0; sector_size];
        disk.read_bytes(checksum_sector_offset, &mut corrupted);
        corrupted[0] ^= 0xA5;
        disk.write_bytes(checksum_sector_offset, &corrupted);

        assert!(read_primary_super_block(&disk).is_err());
    }

    #[ktest]
    fn boot_region_rejects_invalid_region_layout() {
        // Confirms mount geometry is rejected when the data region overlaps the
        // FAT region instead of starting after it.
        let disk = load_exfat_disk();
        let mut boot_sector = read_primary_boot_sector(&disk).unwrap();
        let fat_offset = boot_sector.fat_offset;
        boot_sector.cluster_offset = fat_offset;
        disk.write_bytes(0, boot_sector.as_bytes());

        assert!(read_primary_super_block(&disk).is_err());
    }
}
