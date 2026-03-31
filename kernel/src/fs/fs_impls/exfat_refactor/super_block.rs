// SPDX-License-Identifier: MPL-2.0
#![expect(
    dead_code,
    reason = "Superblock geometry is staged before mount integration."
)]

use super::boot_sector::{
    persistent_volume_flags, ExfatBootSector, EXFAT_FIRST_CLUSTER, EXFAT_RESERVED_CLUSTERS,
};

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct ExfatSuperBlock {
    pub(super) num_sectors: u64,
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

impl From<ExfatBootSector> for ExfatSuperBlock {
    fn from(boot_sector: ExfatBootSector) -> Self {
        let sector_size = 1u32 << boot_sector.sector_size_bits;
        let sect_per_cluster = 1u32 << boot_sector.sector_per_cluster_bits;
        let cluster_size_bits =
            u32::from(boot_sector.sector_size_bits + boot_sector.sector_per_cluster_bits);
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
            num_clusters: boot_sector.cluster_count + EXFAT_RESERVED_CLUSTERS,
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
            used_clusters: !0,
        }
    }
}
