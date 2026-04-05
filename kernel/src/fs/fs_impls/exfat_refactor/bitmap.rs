// SPDX-License-Identifier: MPL-2.0

#![cfg_attr(
    not(ktest),
    expect(
        dead_code,
        reason = "Allocation bitmap loading is staged before mount integration."
    )
)]

use alloc::vec::Vec;
use core::{convert::TryFrom, ops::Range};

use aster_block::BlockDevice;

use super::{
    boot_sector::EXFAT_RESERVED_CLUSTERS,
    fat::{ChainMode, ExfatChain},
    io::read_metadata_bytes,
    super_block::ExfatSuperBlock,
    sysroot::ExfatSysRootBitmapDiscovery,
};
use crate::prelude::*;

/// Stores the validated allocation bitmap as a read-only occupancy surface.
#[derive(Debug, Eq, PartialEq)]
pub(super) struct ExfatAllocationBitmap {
    bytes: Vec<u8>,
    data_cluster_end_exclusive: u32,
}

impl ExfatAllocationBitmap {
    /// Loads the allocation bitmap from validated discovery facts.
    pub(super) fn load(
        block_device: &dyn BlockDevice,
        super_block: &ExfatSuperBlock,
        bitmap_facts: &ExfatSysRootBitmapDiscovery,
    ) -> Result<Self> {
        let cluster_size = super_block.cluster_size();
        if cluster_size == 0 {
            return Err(Error::with_message(
                Errno::EINVAL,
                "volume cluster size is invalid",
            ));
        }

        let byte_size = bitmap_facts.byte_size;
        let minimum_byte_size = minimum_bitmap_byte_size(super_block)?;
        if byte_size < minimum_byte_size {
            return Err(Error::with_message(
                Errno::EINVAL,
                "allocation bitmap is too small for the volume geometry",
            ));
        }

        let cluster_count = bitmap_cluster_count(byte_size, cluster_size)?;
        let bitmap_chain = ExfatChain::new(
            block_device,
            super_block,
            bitmap_facts.start_cluster,
            Some(cluster_count),
            ChainMode::Contiguous,
        )?;
        let bitmap_start_offset = bitmap_chain.physical_cluster_start_offset(super_block)?;

        let mut bytes = vec![0; byte_size];
        read_metadata_bytes(block_device, bitmap_start_offset, &mut bytes)?;
        validate_bitmap_self_coverage(
            super_block,
            bitmap_facts.start_cluster,
            cluster_count,
            &bytes,
        )?;

        Ok(Self {
            bytes,
            data_cluster_end_exclusive: super_block.data_cluster_end_exclusive(),
        })
    }

    /// Returns whether one legal data cluster is allocated.
    pub(super) fn is_cluster_allocated(&self, cluster: u32) -> Result<bool> {
        self.validate_cluster_id(cluster)?;
        self.is_cluster_allocated_unchecked(cluster)
    }

    /// Returns whether every cluster in the half-open range is allocated.
    pub(super) fn is_cluster_range_allocated(&self, clusters: Range<u32>) -> Result<bool> {
        self.validate_cluster_range(clusters.clone())?;

        for cluster in clusters {
            if !self.is_cluster_allocated_unchecked(cluster)? {
                return Ok(false);
            }
        }

        Ok(true)
    }

    fn validate_cluster_id(&self, cluster: u32) -> Result<()> {
        if cluster >= EXFAT_RESERVED_CLUSTERS && cluster < self.data_cluster_end_exclusive {
            Ok(())
        } else {
            Err(Error::with_message(
                Errno::EINVAL,
                "invalid data-region cluster",
            ))
        }
    }

    fn validate_cluster_range(&self, clusters: Range<u32>) -> Result<()> {
        if clusters.start >= EXFAT_RESERVED_CLUSTERS
            && clusters.start <= clusters.end
            && clusters.end <= self.data_cluster_end_exclusive
        {
            Ok(())
        } else {
            Err(Error::with_message(
                Errno::EINVAL,
                "invalid data-region cluster range",
            ))
        }
    }

    fn is_cluster_allocated_unchecked(&self, cluster: u32) -> Result<bool> {
        let bit_index = bitmap_bit_index(cluster)?;
        let byte_index = bit_index / 8;
        let bit_mask = 1u8 << (bit_index % 8);
        let byte = self.bytes.get(byte_index).ok_or_else(|| {
            Error::with_message(
                Errno::EINVAL,
                "allocation bitmap is too small for the volume geometry",
            )
        })?;

        Ok((*byte & bit_mask) != 0)
    }
}

fn minimum_bitmap_byte_size(super_block: &ExfatSuperBlock) -> Result<usize> {
    let data_cluster_count = usize::try_from(super_block.data_cluster_count()).map_err(|_| {
        Error::with_message(Errno::EINVAL, "volume cluster count does not fit in usize")
    })?;

    Ok(data_cluster_count.div_ceil(8))
}

fn bitmap_cluster_count(byte_size: usize, cluster_size: usize) -> Result<u32> {
    let cluster_count = byte_size.div_ceil(cluster_size);
    u32::try_from(cluster_count)
        .map_err(|_| Error::with_message(Errno::EINVAL, "bitmap spans too many clusters"))
}

fn validate_bitmap_self_coverage(
    super_block: &ExfatSuperBlock,
    start_cluster: u32,
    cluster_count: u32,
    bytes: &[u8],
) -> Result<()> {
    let end_cluster = start_cluster
        .checked_add(cluster_count)
        .ok_or_else(|| Error::with_message(Errno::EINVAL, "bitmap cluster coverage overflow"))?;

    if !super_block.is_data_cluster_range(start_cluster..end_cluster) {
        return Err(Error::with_message(
            Errno::EINVAL,
            "bitmap file cluster range is invalid",
        ));
    }

    for cluster in start_cluster..end_cluster {
        if !is_cluster_allocated_in_bytes(bytes, cluster)? {
            return Err(Error::with_message(
                Errno::EINVAL,
                "allocation bitmap does not mark its own clusters allocated",
            ));
        }
    }

    Ok(())
}

fn is_cluster_allocated_in_bytes(bytes: &[u8], cluster: u32) -> Result<bool> {
    let bit_index = bitmap_bit_index(cluster)?;
    let byte_index = bit_index / 8;
    let bit_mask = 1u8 << (bit_index % 8);
    let byte = bytes.get(byte_index).ok_or_else(|| {
        Error::with_message(
            Errno::EINVAL,
            "allocation bitmap is too small for the volume geometry",
        )
    })?;

    Ok((*byte & bit_mask) != 0)
}

fn bitmap_bit_index(cluster: u32) -> Result<usize> {
    let bit_index = cluster
        .checked_sub(EXFAT_RESERVED_CLUSTERS)
        .ok_or_else(|| Error::with_message(Errno::EINVAL, "invalid data-region cluster"))?;

    usize::try_from(bit_index)
        .map_err(|_| Error::with_message(Errno::EINVAL, "bitmap bit index does not fit in usize"))
}

#[cfg(ktest)]
mod tests {
    use alloc::vec;

    use ostd::prelude::ktest;

    use super::{
        EXFAT_RESERVED_CLUSTERS, ExfatAllocationBitmap, bitmap_bit_index, minimum_bitmap_byte_size,
    };
    use crate::{
        fs::fs_impls::exfat_refactor::{
            boot_sector::read_primary_super_block,
            fat::{ChainMode, ExfatChain},
            super_block::ExfatSuperBlock,
            sysroot::{ExfatSysRootBitmapDiscovery, scan_root_system_entries},
            test_support::{ExfatMemoryDisk, load_exfat_disk},
        },
        prelude::Errno,
    };

    fn bitmap_fixture() -> (
        ExfatMemoryDisk,
        ExfatSuperBlock,
        ExfatSysRootBitmapDiscovery,
    ) {
        let disk = load_exfat_disk();
        let super_block = read_primary_super_block(&disk).unwrap();
        let root_chain = ExfatChain::new(
            &disk,
            &super_block,
            super_block.root_dir,
            Some(1),
            ChainMode::Contiguous,
        )
        .unwrap();
        let facts = scan_root_system_entries(&disk, &super_block, root_chain).unwrap();
        let bitmap_facts = facts.bitmap.unwrap();

        (disk, super_block, bitmap_facts)
    }

    fn load_bitmap(
        disk: &ExfatMemoryDisk,
        super_block: &ExfatSuperBlock,
        bitmap_facts: &ExfatSysRootBitmapDiscovery,
    ) -> ExfatAllocationBitmap {
        ExfatAllocationBitmap::load(disk, super_block, bitmap_facts).unwrap()
    }

    fn bitmap_payload_offset(
        super_block: &ExfatSuperBlock,
        bitmap_facts: &ExfatSysRootBitmapDiscovery,
    ) -> usize {
        super_block
            .cluster_to_byte_offset(bitmap_facts.start_cluster)
            .unwrap()
    }

    fn first_free_cluster(bitmap: &ExfatAllocationBitmap, super_block: &ExfatSuperBlock) -> u32 {
        for cluster in EXFAT_RESERVED_CLUSTERS..super_block.data_cluster_end_exclusive() {
            if !bitmap.is_cluster_allocated(cluster).unwrap() {
                return cluster;
            }
        }

        panic!("expected at least one free data cluster in the test image");
    }

    // Confirms the loader keeps a valid bitmap read-only and answers occupied
    // and free occupancy queries without invoking search or mutation.
    #[ktest]
    fn loads_valid_bitmap_and_reports_occupied_and_free_clusters() {
        let (disk, super_block, bitmap_facts) = bitmap_fixture();
        let bitmap = load_bitmap(&disk, &super_block, &bitmap_facts);
        let occupied_cluster = bitmap_facts.start_cluster;
        let free_cluster = first_free_cluster(&bitmap, &super_block);

        assert!(bitmap.is_cluster_allocated(occupied_cluster).unwrap());
        assert!(
            bitmap
                .is_cluster_range_allocated(occupied_cluster..occupied_cluster + 1)
                .unwrap()
        );
        assert!(!bitmap.is_cluster_allocated(free_cluster).unwrap());
        assert!(
            !bitmap
                .is_cluster_range_allocated(free_cluster..free_cluster + 1)
                .unwrap()
        );
    }

    // Confirms the loader rejects a bitmap that does not meet the minimum size
    // required by the volume geometry.
    #[ktest]
    fn rejects_undersized_bitmap_payloads() {
        let (disk, super_block, mut bitmap_facts) = bitmap_fixture();
        let minimum_byte_size = minimum_bitmap_byte_size(&super_block).unwrap();

        assert!(minimum_byte_size > 0);
        bitmap_facts.byte_size = minimum_byte_size - 1;

        let error = match ExfatAllocationBitmap::load(&disk, &super_block, &bitmap_facts) {
            Ok(_) => panic!("expected undersized bitmap payload to be rejected"),
            Err(error) => error,
        };

        assert_eq!(error.error(), Errno::EINVAL);
    }

    // Confirms the loader rejects a bitmap whose own clusters are not all
    // marked allocated before the surface becomes visible.
    #[ktest]
    fn rejects_bitmaps_whose_own_clusters_are_not_allocated() {
        let (disk, super_block, bitmap_facts) = bitmap_fixture();
        let payload_offset = bitmap_payload_offset(&super_block, &bitmap_facts);
        let mut payload = vec![0; bitmap_facts.byte_size];
        disk.read_bytes(payload_offset, &mut payload);

        let bit_index = bitmap_bit_index(bitmap_facts.start_cluster).unwrap();
        payload[bit_index / 8] &= !(1u8 << (bit_index % 8));
        disk.write_bytes(payload_offset, &payload);

        let error = match ExfatAllocationBitmap::load(&disk, &super_block, &bitmap_facts) {
            Ok(_) => panic!("expected malformed bitmap payload to be rejected"),
            Err(error) => error,
        };

        assert_eq!(error.error(), Errno::EINVAL);
    }

    // Confirms the query surface rejects reserved cluster ids and the
    // one-past-end id instead of treating them as ordinary data clusters.
    #[ktest]
    fn rejects_out_of_range_cluster_queries() {
        let (disk, super_block, bitmap_facts) = bitmap_fixture();
        let bitmap = load_bitmap(&disk, &super_block, &bitmap_facts);
        let end_cluster = super_block.data_cluster_end_exclusive();

        for cluster in [0, 1, end_cluster] {
            let error = match bitmap.is_cluster_allocated(cluster) {
                Ok(_) => panic!("expected cluster {cluster} to be rejected"),
                Err(error) => error,
            };
            assert_eq!(error.error(), Errno::EINVAL);
        }
    }

    // Confirms the loader accepts a bitmap payload larger than the minimum
    // size as long as the data-cluster geometry is still covered.
    #[ktest]
    fn accepts_oversized_bitmap_payloads() {
        let (disk, super_block, bitmap_facts) = bitmap_fixture();
        let minimum_byte_size = minimum_bitmap_byte_size(&super_block).unwrap();

        assert!(bitmap_facts.byte_size > minimum_byte_size);
        let bitmap = load_bitmap(&disk, &super_block, &bitmap_facts);

        assert!(
            bitmap
                .is_cluster_allocated(bitmap_facts.start_cluster)
                .unwrap()
        );
    }
}
