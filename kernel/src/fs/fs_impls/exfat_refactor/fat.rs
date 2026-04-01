// SPDX-License-Identifier: MPL-2.0
#![cfg_attr(
    not(ktest),
    expect(
        dead_code,
        reason = "FAT helpers are staged before chain integration."
    )
)]

use core::mem::size_of;

use aster_block::BlockDevice;

use super::{io::read_metadata_bytes, super_block::ExfatSuperBlock};
use crate::prelude::*;

pub(super) type ClusterId = u32;

const FAT_ENTRY_SIZE: u64 = size_of::<u32>() as u64;
const FREE_CLUSTER_VALUE: ClusterId = 0;
const BAD_CLUSTER_VALUE: ClusterId = 0xFFFF_FFF7;
const END_OF_CHAIN_VALUE: ClusterId = 0xFFFF_FFFF;

/// Describes a decoded FAT entry.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub(super) enum FatValue {
    Free,
    Next(ClusterId),
    Bad,
    EndOfChain,
}

impl From<ClusterId> for FatValue {
    fn from(raw_value: ClusterId) -> Self {
        match raw_value {
            FREE_CLUSTER_VALUE => Self::Free,
            BAD_CLUSTER_VALUE => Self::Bad,
            END_OF_CHAIN_VALUE => Self::EndOfChain,
            _ => Self::Next(raw_value),
        }
    }
}

impl From<FatValue> for ClusterId {
    fn from(value: FatValue) -> Self {
        match value {
            FatValue::Free => FREE_CLUSTER_VALUE,
            FatValue::Next(cluster) => cluster,
            FatValue::Bad => BAD_CLUSTER_VALUE,
            FatValue::EndOfChain => END_OF_CHAIN_VALUE,
        }
    }
}

/// Reads and decodes the FAT entry for a validated cluster from the first FAT.
pub(super) fn read_next_fat_value(
    block_device: &dyn BlockDevice,
    super_block: &ExfatSuperBlock,
    cluster: ClusterId,
) -> Result<FatValue> {
    validate_source_cluster(super_block, cluster)?;

    let offset = fat_entry_byte_offset(super_block, cluster)?;
    let mut raw_bytes = [0u8; size_of::<u32>()];
    read_metadata_bytes(block_device, offset, &mut raw_bytes)?;

    let value = FatValue::from(u32::from_le_bytes(raw_bytes));
    match value {
        FatValue::Next(next_cluster) => {
            validate_next_cluster(super_block, next_cluster)?;
            Ok(FatValue::Next(next_cluster))
        }
        other => Ok(other),
    }
}

fn fat_entry_byte_offset(super_block: &ExfatSuperBlock, cluster: ClusterId) -> Result<usize> {
    let fat_start = super_block
        .fat1_start_sector
        .checked_mul(u64::from(super_block.sector_size))
        .ok_or_else(|| Error::with_message(Errno::EINVAL, "fat entry offset overflow"))?;
    let cluster_offset = u64::from(cluster)
        .checked_mul(FAT_ENTRY_SIZE)
        .ok_or_else(|| Error::with_message(Errno::EINVAL, "fat entry offset overflow"))?;
    let byte_offset = fat_start
        .checked_add(cluster_offset)
        .ok_or_else(|| Error::with_message(Errno::EINVAL, "fat entry offset overflow"))?;

    usize::try_from(byte_offset)
        .map_err(|_| Error::with_message(Errno::EINVAL, "fat entry offset overflow"))
}

fn validate_source_cluster(super_block: &ExfatSuperBlock, cluster: ClusterId) -> Result<()> {
    if super_block.is_valid_cluster(cluster) {
        Ok(())
    } else {
        Err(Error::with_message(
            Errno::EINVAL,
            "invalid data-region cluster",
        ))
    }
}

fn validate_next_cluster(super_block: &ExfatSuperBlock, cluster: ClusterId) -> Result<()> {
    if super_block.is_valid_cluster(cluster) {
        Ok(())
    } else {
        Err(Error::with_message(
            Errno::EINVAL,
            "invalid decoded FAT next-cluster target",
        ))
    }
}

#[cfg(ktest)]
mod tests {
    use ostd::prelude::ktest;

    use super::{read_next_fat_value, ClusterId, FatValue};
    use crate::fs::fs_impls::exfat_refactor::{
        boot_sector::read_primary_super_block,
        io::read_metadata_bytes,
        super_block::ExfatSuperBlock,
        test_support::load_exfat_disk,
    };

    fn read_raw_fat_entry(
        disk: &dyn aster_block::BlockDevice,
        super_block: &ExfatSuperBlock,
        cluster: ClusterId,
    ) -> ClusterId {
        let offset = super_block.fat1_start_sector as usize * super_block.sector_size()
            + cluster as usize * core::mem::size_of::<ClusterId>();
        let mut raw_bytes = [0u8; core::mem::size_of::<ClusterId>()];
        read_metadata_bytes(disk, offset, &mut raw_bytes).unwrap();
        ClusterId::from_le_bytes(raw_bytes)
    }

    fn write_raw_fat_entry(
        disk: &crate::fs::fs_impls::exfat_refactor::test_support::ExfatMemoryDisk,
        super_block: &ExfatSuperBlock,
        cluster: ClusterId,
        raw_value: ClusterId,
    ) {
        let offset = super_block.fat1_start_sector as usize * super_block.sector_size()
            + cluster as usize * core::mem::size_of::<ClusterId>();
        disk.write_bytes(offset, &raw_value.to_le_bytes());
    }

    #[ktest]
    fn fat_value_preserves_special_markers_and_next_clusters() {
        // Confirms the raw decoder and reverse conversion keep the special FAT
        // markers distinct from ordinary successor cluster values.
        let next_cluster = 7;

        assert_eq!(FatValue::from(0), FatValue::Free);
        assert_eq!(FatValue::from(0xFFFF_FFF7), FatValue::Bad);
        assert_eq!(FatValue::from(0xFFFF_FFFF), FatValue::EndOfChain);
        assert_eq!(FatValue::from(next_cluster), FatValue::Next(next_cluster));
        assert_eq!(u32::from(FatValue::Free), 0);
        assert_eq!(u32::from(FatValue::Bad), 0xFFFF_FFF7);
        assert_eq!(u32::from(FatValue::EndOfChain), 0xFFFF_FFFF);
        assert_eq!(u32::from(FatValue::Next(next_cluster)), next_cluster);
    }

    #[ktest]
    fn read_next_fat_value_decodes_embedded_image_entry() {
        // Confirms the helper reads the on-disk FAT entry from the embedded
        // exFAT image and decodes it the same way as a direct raw-byte read.
        let disk = load_exfat_disk();
        let super_block = read_primary_super_block(&disk).unwrap();
        let cluster = super_block.root_dir;
        let expected = FatValue::from(read_raw_fat_entry(&disk, &super_block, cluster));

        assert_eq!(
            read_next_fat_value(&disk, &super_block, cluster).unwrap(),
            expected
        );
    }

    #[ktest]
    fn read_next_fat_value_rejects_invalid_source_cluster() {
        // Confirms reserved cluster identifiers fail before the helper reaches
        // the block-device read stage.
        let disk = load_exfat_disk();
        let super_block = read_primary_super_block(&disk).unwrap();

        assert!(read_next_fat_value(&disk, &super_block, 1).is_err());
    }

    #[ktest]
    fn read_next_fat_value_rejects_invalid_next_cluster_target() {
        // Confirms the helper rejects a decoded next-cluster value that points
        // outside the valid data-region cluster range.
        let disk = load_exfat_disk();
        let super_block = read_primary_super_block(&disk).unwrap();
        let cluster = super_block.root_dir;

        write_raw_fat_entry(&disk, &super_block, cluster, 1);

        assert!(read_next_fat_value(&disk, &super_block, cluster).is_err());
    }
}
