// SPDX-License-Identifier: MPL-2.0

#![cfg_attr(
    not(ktest),
    expect(
        dead_code,
        reason = "Allocation bitmap ownership is staged before later refactor passes consume it."
    )
)]

use alloc::{boxed::Box, vec, vec::Vec};
use core::convert::TryFrom;

use aster_block::BlockDevice;

use super::{
    boot_sector::EXFAT_RESERVED_CLUSTERS, dentry::ExfatBitmapDentry, fat::ExfatChain,
    io::read_metadata_bytes, super_block::ExfatSuperBlock,
};
use crate::prelude::*;

const BITS_PER_BYTE: usize = 8;

/// Carries one validated, immutable allocation-bitmap snapshot for `ExfatFs`.
#[derive(Debug)]
pub(super) struct AllocationBitmap {
    bitmap_bytes: Box<[u8]>,
    valid_cluster_count: u32,
    used_cluster_count: u32,
    free_cluster_count: u32,
}

impl AllocationBitmap {
    /// Loads and validates one bitmap snapshot from the prepared bitmap file chain.
    pub(super) fn load(
        block_device: &dyn BlockDevice,
        super_block: &ExfatSuperBlock,
        bitmap_dentry: ExfatBitmapDentry,
        bitmap_chain: ExfatChain,
    ) -> Result<Self> {
        let bitmap_size = usize::try_from(bitmap_dentry.size).map_err(|_| {
            Error::with_message(
                Errno::EINVAL,
                "allocation bitmap size does not fit the host",
            )
        })?;
        if bitmap_size == 0 {
            return Err(Error::with_message(
                Errno::EINVAL,
                "allocation bitmap must not be empty",
            ));
        }

        let valid_cluster_count = super_block.data_cluster_count();
        let valid_cluster_count_usize = usize::try_from(valid_cluster_count).map_err(|_| {
            Error::with_message(
                Errno::EINVAL,
                "allocation bitmap geometry does not fit the host",
            )
        })?;
        let required_bitmap_size = valid_cluster_count_usize.div_ceil(BITS_PER_BYTE);
        if bitmap_size != required_bitmap_size {
            return Err(Error::with_message(
                Errno::EINVAL,
                "allocation bitmap size mismatched",
            ));
        }

        let expected_cluster_count = bitmap_size.div_ceil(super_block.cluster_size());
        let expected_cluster_count = u32::try_from(expected_cluster_count).map_err(|_| {
            Error::with_message(
                Errno::EINVAL,
                "allocation bitmap cluster span does not fit the host",
            )
        })?;
        if bitmap_chain.current_cluster() != bitmap_dentry.start_cluster {
            return Err(Error::with_message(
                Errno::EINVAL,
                "allocation bitmap start cluster mismatched",
            ));
        }
        if bitmap_chain.cluster_count() != expected_cluster_count {
            return Err(Error::with_message(
                Errno::EINVAL,
                "allocation bitmap extent mismatched",
            ));
        }

        let bitmap_bytes = load_bitmap_bytes(block_device, super_block, bitmap_chain, bitmap_size)?;
        validate_bitmap_cluster_ownership(&bitmap_bytes, block_device, super_block, bitmap_chain)?;

        let used_cluster_count = count_used_clusters(&bitmap_bytes, valid_cluster_count_usize)?;
        let free_cluster_count = valid_cluster_count
            .checked_sub(used_cluster_count)
            .ok_or_else(|| {
                Error::with_message(Errno::EINVAL, "allocation bitmap accounting mismatched")
            })?;

        Ok(Self {
            bitmap_bytes: bitmap_bytes.into_boxed_slice(),
            valid_cluster_count,
            used_cluster_count,
            free_cluster_count,
        })
    }

    /// Returns whether the requested data-cluster id is allocated or bad.
    pub(super) fn cluster_is_allocated(&self, cluster: u32) -> Result<bool> {
        if cluster < EXFAT_RESERVED_CLUSTERS {
            return Err(Error::with_message(
                Errno::EINVAL,
                "invalid data-region cluster",
            ));
        }

        let data_cluster_end_exclusive = self.data_cluster_end_exclusive()?;
        if cluster >= data_cluster_end_exclusive {
            return Err(Error::with_message(
                Errno::EINVAL,
                "invalid data-region cluster",
            ));
        }

        Ok(cluster_bit_is_set(&self.bitmap_bytes, cluster))
    }

    /// Returns the number of allocated clusters in the validated bitmap image.
    pub(super) fn used_cluster_count(&self) -> u32 {
        self.used_cluster_count
    }

    /// Returns the number of free clusters in the validated bitmap image.
    pub(super) fn free_cluster_count(&self) -> u32 {
        self.free_cluster_count
    }

    fn data_cluster_end_exclusive(&self) -> Result<u32> {
        self.valid_cluster_count
            .checked_add(EXFAT_RESERVED_CLUSTERS)
            .ok_or_else(|| {
                Error::with_message(
                    Errno::EINVAL,
                    "allocation bitmap geometry does not fit the host",
                )
            })
    }
}

fn load_bitmap_bytes(
    block_device: &dyn BlockDevice,
    super_block: &ExfatSuperBlock,
    bitmap_chain: ExfatChain,
    bitmap_size: usize,
) -> Result<Vec<u8>> {
    let mut bitmap_bytes = vec![0; bitmap_size];
    let mut loaded_chain = bitmap_chain;
    let mut copied_bytes = 0usize;
    let cluster_size = super_block.cluster_size();

    for cluster_index in 0..bitmap_chain.cluster_count() {
        let cluster_offset = loaded_chain.physical_cluster_start_offset(super_block)?;
        let remaining_bytes = bitmap_size - copied_bytes;
        let copy_len = remaining_bytes.min(cluster_size);
        read_metadata_bytes(
            block_device,
            cluster_offset,
            &mut bitmap_bytes[copied_bytes..copied_bytes + copy_len],
        )?;
        copied_bytes += copy_len;

        if cluster_index + 1 < bitmap_chain.cluster_count() {
            loaded_chain = loaded_chain.walk(block_device, super_block, 1)?;
        }
    }

    Ok(bitmap_bytes)
}

fn validate_bitmap_cluster_ownership(
    bitmap_bytes: &[u8],
    block_device: &dyn BlockDevice,
    super_block: &ExfatSuperBlock,
    bitmap_chain: ExfatChain,
) -> Result<()> {
    let mut chain = bitmap_chain;
    for cluster_index in 0..bitmap_chain.cluster_count() {
        let cluster = chain.current_cluster();
        if !super_block.is_data_cluster_id(cluster) {
            return Err(Error::with_message(
                Errno::EINVAL,
                "bitmap file cluster is outside the data region",
            ));
        }
        if !cluster_bit_is_set(bitmap_bytes, cluster) {
            return Err(Error::with_message(
                Errno::EINVAL,
                "bitmap file clusters must be marked allocated",
            ));
        }

        if cluster_index + 1 < bitmap_chain.cluster_count() {
            chain = chain.walk(block_device, super_block, 1)?;
        }
    }

    Ok(())
}

fn count_used_clusters(bitmap_bytes: &[u8], valid_cluster_count: usize) -> Result<u32> {
    let full_byte_count = valid_cluster_count / BITS_PER_BYTE;
    let tail_bit_count = valid_cluster_count % BITS_PER_BYTE;
    let mut used_cluster_count = bitmap_bytes[..full_byte_count]
        .iter()
        .map(|byte| byte.count_ones())
        .sum::<u32>();

    if tail_bit_count != 0 {
        let tail_mask = (1u8 << (tail_bit_count as u32)) - 1;
        used_cluster_count += (bitmap_bytes[full_byte_count] & tail_mask).count_ones();
    }

    Ok(used_cluster_count)
}

fn cluster_bit_is_set(bitmap_bytes: &[u8], cluster: u32) -> bool {
    let bit_index = usize::try_from(cluster - EXFAT_RESERVED_CLUSTERS)
        .expect("validated data cluster ids fit in usize");
    let byte_index = bit_index / BITS_PER_BYTE;
    let bit_offset = (bit_index % BITS_PER_BYTE) as u32;
    bitmap_bytes[byte_index] & (1u8 << bit_offset) != 0
}

#[cfg(ktest)]
mod tests {
    use alloc::{sync::Arc, vec, vec::Vec};

    use aster_block::BlockDevice;
    use ostd::prelude::ktest;

    use super::*;
    use crate::{
        fs::fs_impls::exfat_refactor::{
            boot_sector::EXFAT_RESERVED_CLUSTERS,
            dentry::{DENTRY_SIZE, ExfatBitmapDentry, ExfatDentry, RawExfatDentry},
            fat::{ChainMode, ExfatChain},
            fs::ExfatFs,
            io::read_metadata_bytes,
            super_block::ExfatSuperBlock,
            test_support::{ExfatMemoryDisk, load_exfat_disk},
        },
        prelude::Errno,
    };

    fn new_exfat_fs() -> (Arc<ExfatMemoryDisk>, ExfatSuperBlock, ExfatFs) {
        let disk = Arc::new(load_exfat_disk());
        let super_block =
            crate::fs::fs_impls::exfat_refactor::boot_sector::read_primary_super_block(
                disk.as_ref(),
            )
            .unwrap();
        let block_device: Arc<dyn BlockDevice> = disk.clone();
        let fs = ExfatFs::new(block_device, super_block).unwrap();

        (disk, super_block, fs)
    }

    fn root_dir_chain(disk: &dyn BlockDevice, super_block: &ExfatSuperBlock) -> ExfatChain {
        ExfatChain::new(
            disk,
            super_block,
            super_block.root_dir,
            None,
            ChainMode::FatBacked,
        )
        .unwrap()
    }

    fn read_chain_bytes(
        disk: &dyn BlockDevice,
        super_block: &ExfatSuperBlock,
        chain: ExfatChain,
        byte_len: usize,
    ) -> Vec<u8> {
        let mut bytes = vec![0; byte_len];
        let mut loaded_chain = chain;
        let mut copied_bytes = 0usize;
        let cluster_size = super_block.cluster_size();

        for cluster_index in 0..chain.cluster_count() {
            let cluster_offset = loaded_chain
                .physical_cluster_start_offset(super_block)
                .unwrap();
            let remaining_bytes = byte_len - copied_bytes;
            let copy_len = remaining_bytes.min(cluster_size);
            read_metadata_bytes(
                disk,
                cluster_offset,
                &mut bytes[copied_bytes..copied_bytes + copy_len],
            )
            .unwrap();
            copied_bytes += copy_len;

            if cluster_index + 1 < chain.cluster_count() {
                loaded_chain = loaded_chain.walk(disk, super_block, 1).unwrap();
            }
        }

        bytes
    }

    fn bitmap_fixture(
        disk: &dyn BlockDevice,
        super_block: &ExfatSuperBlock,
    ) -> (ExfatBitmapDentry, ExfatChain, Vec<u8>) {
        let root_chain = root_dir_chain(disk, super_block);
        let root_chain_cluster_count = root_chain.cluster_count();
        let root_bytes = read_chain_bytes(
            disk,
            super_block,
            root_chain,
            usize::try_from(root_chain_cluster_count).unwrap() * super_block.cluster_size(),
        );

        for chunk in root_bytes.chunks_exact(DENTRY_SIZE) {
            if let ExfatDentry::Bitmap(bitmap_dentry) =
                ExfatDentry::from(RawExfatDentry::from_bytes(chunk))
            {
                let bitmap_chain = ExfatChain::new(
                    disk,
                    super_block,
                    bitmap_dentry.start_cluster,
                    None,
                    ChainMode::FatBacked,
                )
                .unwrap();
                let bitmap_bytes = read_chain_bytes(
                    disk,
                    super_block,
                    bitmap_chain,
                    usize::try_from(bitmap_dentry.size).unwrap(),
                );
                return (bitmap_dentry, bitmap_chain, bitmap_bytes);
            }
        }

        panic!("expected bitmap singleton in root directory")
    }

    fn manual_bitmap_snapshot(bitmap_bytes: Vec<u8>, valid_cluster_count: u32) -> AllocationBitmap {
        let used_cluster_count =
            count_used_clusters(&bitmap_bytes, usize::try_from(valid_cluster_count).unwrap())
                .unwrap();
        AllocationBitmap {
            bitmap_bytes: bitmap_bytes.into_boxed_slice(),
            valid_cluster_count,
            used_cluster_count,
            free_cluster_count: valid_cluster_count - used_cluster_count,
        }
    }

    fn bit_is_set(bitmap_bytes: &[u8], cluster: u32) -> bool {
        let bit_index = usize::try_from(cluster - EXFAT_RESERVED_CLUSTERS).unwrap();
        bitmap_bytes[bit_index / BITS_PER_BYTE] & (1u8 << (bit_index % BITS_PER_BYTE)) != 0
    }

    #[ktest]
    fn invalid_bitmap_load_is_rejected_before_publication() {
        let (disk, super_block, fs) = new_exfat_fs();
        let (bitmap_dentry, bitmap_chain, _) = bitmap_fixture(disk.as_ref(), &super_block);
        let invalid_bitmap_dentry = ExfatBitmapDentry {
            size: bitmap_dentry.size - 1,
            ..bitmap_dentry
        };

        let error = fs
            .load_allocation_bitmap(invalid_bitmap_dentry, bitmap_chain)
            .unwrap_err();
        assert_eq!(error.error(), Errno::EINVAL);
        assert!(fs.cluster_is_allocated(EXFAT_RESERVED_CLUSTERS).is_err());
    }

    #[ktest]
    fn loaded_bitmap_reports_first_middle_and_tail_cluster_occupancy() {
        let (disk, super_block, fs) = new_exfat_fs();
        let (bitmap_dentry, bitmap_chain, bitmap_bytes) =
            bitmap_fixture(disk.as_ref(), &super_block);
        fs.load_allocation_bitmap(bitmap_dentry, bitmap_chain)
            .unwrap();

        let first_cluster = EXFAT_RESERVED_CLUSTERS;
        let middle_cluster = EXFAT_RESERVED_CLUSTERS + super_block.data_cluster_count() / 2;
        let tail_cluster = super_block.data_cluster_end_exclusive() - 1;

        assert!(super_block.data_cluster_count() > 2);
        assert_eq!(
            fs.cluster_is_allocated(first_cluster).unwrap(),
            bit_is_set(&bitmap_bytes, first_cluster)
        );
        assert_eq!(
            fs.cluster_is_allocated(middle_cluster).unwrap(),
            bit_is_set(&bitmap_bytes, middle_cluster)
        );
        assert_eq!(
            fs.cluster_is_allocated(tail_cluster).unwrap(),
            bit_is_set(&bitmap_bytes, tail_cluster)
        );
        assert!(
            fs.cluster_is_allocated(super_block.data_cluster_end_exclusive())
                .is_err()
        );
    }

    #[ktest]
    fn bitmap_accounting_ignores_padding_bits_beyond_valid_range() {
        let snapshot = manual_bitmap_snapshot(vec![0xAD, 0xFF], 10);
        let first_cluster = EXFAT_RESERVED_CLUSTERS;
        let middle_cluster = EXFAT_RESERVED_CLUSTERS + 4;
        let tail_cluster = EXFAT_RESERVED_CLUSTERS + 9;

        assert!(bit_is_set(&snapshot.bitmap_bytes, first_cluster));
        assert!(!bit_is_set(&snapshot.bitmap_bytes, middle_cluster));
        assert!(bit_is_set(&snapshot.bitmap_bytes, tail_cluster));
        assert_eq!(snapshot.used_cluster_count(), 8);
        assert_eq!(snapshot.free_cluster_count(), 2);
        assert!(
            snapshot
                .cluster_is_allocated(super::EXFAT_RESERVED_CLUSTERS + 10)
                .is_err()
        );
    }
}
