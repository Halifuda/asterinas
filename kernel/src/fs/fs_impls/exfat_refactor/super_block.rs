// SPDX-License-Identifier: MPL-2.0
#![expect(
    dead_code,
    reason = "Superblock geometry is staged before mount integration."
)]

use core::ops::Range;

use super::boot_sector::{
    persistent_volume_flags, ValidatedBootSector, EXFAT_FIRST_CLUSTER, EXFAT_RESERVED_CLUSTERS,
};
use crate::prelude::*;

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct ExfatSuperBlock {
    pub(super) num_sectors: u64,
    // BPB ClusterCount is the number of data clusters, not a one-past-end id.
    pub(super) num_clusters: u32,
    pub(super) sector_size: u32,
    pub(super) cluster_size: u32,
    pub(super) cluster_size_bits: u32,
    pub(super) sect_per_cluster: u32,
    pub(super) fat1_start_sector: u64,
    pub(super) fat2_start_sector: u64,
    pub(super) data_start_sector: u64,
    pub(super) num_fat_sectors: u32,
    pub(super) root_dir: u32,
    pub(super) dentries_per_clu: u32,
    pub(super) vol_flags: u32,
    pub(super) vol_flags_persistent: u32,
    pub(super) cluster_search_ptr: u32,
    pub(super) used_clusters: u32,
}

impl From<ValidatedBootSector> for ExfatSuperBlock {
    fn from(validated_boot_sector: ValidatedBootSector) -> Self {
        let boot_sector = validated_boot_sector.into_inner();
        let sector_size = 1u32 << boot_sector.sector_size_bits;
        let sect_per_cluster = 1u32 << boot_sector.sector_per_cluster_bits;
        let cluster_size_bits = u32::from(
            boot_sector
                .sector_size_bits
                .checked_add(boot_sector.sector_per_cluster_bits)
                .expect("validated boot sector must cap cluster size bits"),
        );
        let cluster_size = 1u32 << cluster_size_bits;
        let fat1_start_sector = u64::from(boot_sector.fat_offset);
        // A single-FAT volume aliases the second start sector to the primary
        // FAT start so downstream code can treat both slots uniformly.
        let fat2_start_sector = if boot_sector.num_fats == 1 {
            fat1_start_sector
        } else {
            fat1_start_sector + u64::from(boot_sector.fat_length)
        };

        Self {
            num_sectors: boot_sector.vol_length,
            num_clusters: boot_sector.cluster_count,
            sector_size,
            cluster_size,
            cluster_size_bits,
            sect_per_cluster,
            fat1_start_sector,
            fat2_start_sector,
            data_start_sector: u64::from(boot_sector.cluster_offset),
            num_fat_sectors: boot_sector.fat_length,
            root_dir: boot_sector.root_cluster,
            dentries_per_clu: cluster_size / 32,
            vol_flags: u32::from(boot_sector.vol_flags),
            vol_flags_persistent: persistent_volume_flags(&boot_sector),
            // Allocation scanning starts at the first allocatable cluster and
            // advances from there once mount-time allocation is wired in.
            cluster_search_ptr: EXFAT_FIRST_CLUSTER,
            used_clusters: u32::MAX,
        }
    }
}

impl ExfatSuperBlock {
    /// Returns the raw BPB `ClusterCount`, which counts usable data clusters.
    pub(super) fn data_cluster_count(&self) -> u32 {
        self.num_clusters
    }

    /// Returns the exclusive upper bound of legal data-cluster ids, i.e. `ClusterCount + 2`.
    pub(super) fn data_cluster_end_exclusive(&self) -> u32 {
        self.num_clusters
            .checked_add(EXFAT_RESERVED_CLUSTERS)
            .expect("validated boot sector must cap cluster count")
    }

    pub(super) fn sector_size(&self) -> usize {
        self.sector_size as usize
    }

    pub(super) fn cluster_size(&self) -> usize {
        self.cluster_size as usize
    }

    /// Returns whether `cluster` lies in the legal data-region id range `2..ClusterCount + 2`.
    pub(super) fn is_data_cluster_id(&self, cluster: u32) -> bool {
        cluster >= EXFAT_RESERVED_CLUSTERS && cluster < self.data_cluster_end_exclusive()
    }

    /// Returns whether `range` stays within the half-open legal data-cluster range.
    pub(super) fn is_data_cluster_range(&self, range: Range<u32>) -> bool {
        let range_end_limit = self.data_cluster_end_exclusive();

        range.start >= EXFAT_RESERVED_CLUSTERS
            && range.start <= range.end
            && range.end <= range_end_limit
    }

    pub(super) fn cluster_to_byte_offset(&self, cluster: u32) -> Result<usize> {
        let sector = self.cluster_to_sector(cluster)?;
        let byte_offset = sector
            .checked_mul(self.sector_size as u64)
            .ok_or_else(|| Error::with_message(Errno::EINVAL, "cluster byte offset overflow"))?;

        usize::try_from(byte_offset)
            .map_err(|_| Error::with_message(Errno::EINVAL, "cluster byte offset overflow"))
    }

    pub(super) fn cluster_to_sector(&self, cluster: u32) -> Result<u64> {
        let cluster_index = self.cluster_data_index(cluster)?;
        // Translate by whole-cluster strides from the data-region base.
        let sector_offset = cluster_index
            .checked_mul(self.sect_per_cluster as u64)
            .ok_or_else(|| Error::with_message(Errno::EINVAL, "cluster sector offset overflow"))?;

        self.data_start_sector
            .checked_add(sector_offset)
            .ok_or_else(|| Error::with_message(Errno::EINVAL, "cluster sector offset overflow"))
    }

    fn cluster_data_index(&self, cluster: u32) -> Result<u64> {
        if !self.is_data_cluster_id(cluster) {
            return Err(Error::with_message(
                Errno::EINVAL,
                "invalid data-region cluster",
            ));
        }

        Ok((cluster - EXFAT_RESERVED_CLUSTERS) as u64)
    }
}

#[cfg(ktest)]
mod tests {
    use ostd::prelude::ktest;

    use super::EXFAT_RESERVED_CLUSTERS;
    use crate::fs::fs_impls::exfat_refactor::{
        boot_sector::{read_primary_boot_sector, read_primary_super_block},
        test_support::load_exfat_disk,
    };

    #[ktest]
    fn cluster_translation_matches_super_block_geometry() {
        // Confirms cluster translation helpers agree with the geometry derived
        // from the boot sector for the root directory cluster.
        let disk = load_exfat_disk();
        let boot_sector = read_primary_boot_sector(&disk).unwrap();
        let super_block = read_primary_super_block(&disk).unwrap();
        let root_cluster = boot_sector.root_cluster;
        let expected_sector = u64::from(boot_sector.cluster_offset)
            + u64::from(root_cluster - EXFAT_RESERVED_CLUSTERS)
                * u64::from(1u32 << boot_sector.sector_per_cluster_bits);
        let expected_byte_offset = expected_sector * u64::from(super_block.sector_size);

        assert_eq!(super_block.sector_size(), super_block.sector_size as usize);
        assert_eq!(
            super_block.cluster_size(),
            super_block.cluster_size as usize
        );
        assert_eq!(
            super_block.sect_per_cluster,
            1u32 << boot_sector.sector_per_cluster_bits
        );
        assert_eq!(
            super_block.cluster_to_sector(root_cluster).unwrap(),
            expected_sector
        );
        assert_eq!(
            super_block.cluster_to_byte_offset(root_cluster).unwrap(),
            expected_byte_offset as usize
        );
    }

    #[ktest]
    fn cluster_translation_rejects_invalid_clusters() {
        // Confirms geometry helpers reject reserved or out-of-range cluster
        // numbers instead of silently translating them into data offsets.
        // The exclusive upper bound is `ClusterCount + 2`.
        let disk = load_exfat_disk();
        let super_block = read_primary_super_block(&disk).unwrap();
        let invalid_cluster = super_block.data_cluster_count() + EXFAT_RESERVED_CLUSTERS;

        assert_eq!(super_block.data_cluster_count(), super_block.num_clusters);
        assert_eq!(invalid_cluster, super_block.data_cluster_end_exclusive());
        assert!(!super_block.is_data_cluster_id(0));
        assert!(!super_block.is_data_cluster_id(1));
        assert!(!super_block.is_data_cluster_id(invalid_cluster));
        assert!(super_block.is_data_cluster_id(invalid_cluster - 1));
        assert!(super_block.cluster_to_sector(0).is_err());
        assert!(super_block.cluster_to_byte_offset(invalid_cluster).is_err());
    }

    #[ktest]
    fn cluster_range_validation_uses_half_open_semantics() {
        // Confirms range validation accepts the canonical half-open data range
        // and rejects ranges that cross reserved or one-past-the-end bounds.
        let disk = load_exfat_disk();
        let super_block = read_primary_super_block(&disk).unwrap();
        let range_end = super_block.data_cluster_end_exclusive();

        assert!(super_block.is_data_cluster_range(EXFAT_RESERVED_CLUSTERS..range_end));
        assert!(super_block.is_data_cluster_range(range_end..range_end));
        assert!(!super_block.is_data_cluster_range(0..range_end));
        assert!(!super_block.is_data_cluster_range(EXFAT_RESERVED_CLUSTERS..range_end + 1));
    }
}
