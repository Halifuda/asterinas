// SPDX-License-Identifier: MPL-2.0

use core::mem;

use aster_block::BlockDevice;
use ostd::mm::VmIo;

use super::{
    bitmap::{ALLOCATION_BITMAP_ENTRY_TYPE, AllocationBitmap},
    fat::{ChainVisitControl, FatReader},
    fs::MountVolumeStateError,
    upcase::{UPCASE_TABLE_ENTRY_TYPE, UpcaseRecord, UpcaseTable},
};
use crate::prelude::*;

const END_OF_DIRECTORY_ENTRY_TYPE: u8 = 0x00;
const FIRST_DATA_CLUSTER: u32 = 2;
const MAX_CLUSTER_SIZE: usize = 32 * 1024 * 1024;

#[derive(Clone, Copy)]
pub(super) struct BootRegion {
    pub(super) cluster_count: u32,
    pub(super) cluster_heap_offset_sectors: u32,
    pub(super) cluster_size: usize,
    pub(super) fat_length_sectors: u32,
    pub(super) fat_offset_sectors: u32,
    pub(super) percent_in_use: u8,
    pub(super) root_dir_cluster: u32,
    pub(super) sector_size: usize,
    pub(super) sectors_per_cluster: usize,
    pub(super) volume_length_sectors: u64,
    pub(super) volume_serial_number: u32,
}

impl BootRegion {
    pub(super) fn load_mount_state(
        block_device: &dyn BlockDevice,
    ) -> core::result::Result<
        (
            Self,
            VolumeAnomalyState,
            AllocationBitmap,
            Arc<UpcaseTable>,
            usize,
            bool,
        ),
        MountVolumeStateError,
    > {
        let boot_region = Self::read(block_device)?;
        let anomaly = VolumeAnomalyState::read(block_device, &boot_region)?;
        let mut fat_reader = FatReader::new(block_device, &boot_region);
        let (bitmap, upcase) = Self::scan_root_directory(&boot_region, &mut fat_reader)?;
        let upcase_table = Arc::new(UpcaseTable::load(&boot_region, &mut fat_reader, upcase)?);
        let (used_clusters, used_clusters_from_recount) =
            bitmap.count_used_clusters(&boot_region, &mut fat_reader)?;
        Ok((
            boot_region,
            anomaly,
            bitmap,
            upcase_table,
            used_clusters,
            used_clusters_from_recount,
        ))
    }

    pub(super) fn read(
        block_device: &dyn BlockDevice,
    ) -> core::result::Result<Self, MountVolumeStateError> {
        let mut sector_header = [0u8; 512];
        block_device
            .read_bytes(0, &mut sector_header)
            .map_err(|_| MountVolumeStateError::DeviceIo)?;
        if &sector_header[3..11] != b"EXFAT   " {
            return Err(MountVolumeStateError::InvalidOnDiskLayout);
        }
        if u16::from_le_bytes([sector_header[510], sector_header[511]]) != 0xAA55 {
            return Err(MountVolumeStateError::InvalidOnDiskLayout);
        }

        let bytes_per_sector_shift = sector_header[108];
        let sectors_per_cluster_shift = sector_header[109];
        if !(9..=12).contains(&bytes_per_sector_shift) {
            return Err(MountVolumeStateError::InvalidOnDiskLayout);
        }
        let sector_size = 1usize
            .checked_shl(u32::from(bytes_per_sector_shift))
            .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
        let sectors_per_cluster = 1usize
            .checked_shl(u32::from(sectors_per_cluster_shift))
            .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
        let cluster_size = sector_size
            .checked_mul(sectors_per_cluster)
            .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
        if cluster_size == 0 || cluster_size > MAX_CLUSTER_SIZE {
            return Err(MountVolumeStateError::InvalidOnDiskLayout);
        }

        let volume_length_sectors = u64::from_le_bytes([
            sector_header[72],
            sector_header[73],
            sector_header[74],
            sector_header[75],
            sector_header[76],
            sector_header[77],
            sector_header[78],
            sector_header[79],
        ]);
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
        let cluster_heap_offset_sectors = u32::from_le_bytes([
            sector_header[88],
            sector_header[89],
            sector_header[90],
            sector_header[91],
        ]);
        let cluster_count = u32::from_le_bytes([
            sector_header[92],
            sector_header[93],
            sector_header[94],
            sector_header[95],
        ]);
        let root_dir_cluster = u32::from_le_bytes([
            sector_header[96],
            sector_header[97],
            sector_header[98],
            sector_header[99],
        ]);
        let volume_serial_number = u32::from_le_bytes([
            sector_header[100],
            sector_header[101],
            sector_header[102],
            sector_header[103],
        ]);
        let number_of_fats = sector_header[110];
        let percent_in_use = sector_header[112];

        if number_of_fats != 1
            || fat_offset_sectors == 0
            || fat_length_sectors == 0
            || cluster_count == 0
        {
            return Err(MountVolumeStateError::InvalidOnDiskLayout);
        }

        let boot_region = Self {
            cluster_count,
            cluster_heap_offset_sectors,
            cluster_size,
            fat_length_sectors,
            fat_offset_sectors,
            percent_in_use,
            root_dir_cluster,
            sector_size,
            sectors_per_cluster,
            volume_length_sectors,
            volume_serial_number,
        };
        boot_region.validate_geometry()?;
        boot_region.validate_checksum(block_device)?;
        Ok(boot_region)
    }

    pub(super) fn cluster_offset(
        &self,
        cluster: u32,
    ) -> core::result::Result<usize, MountVolumeStateError> {
        if !self.is_valid_cluster(cluster) {
            return Err(MountVolumeStateError::InvalidOnDiskLayout);
        }
        let cluster_index = u64::from(cluster - FIRST_DATA_CLUSTER);
        let sectors_per_cluster = u64::try_from(self.sectors_per_cluster)
            .map_err(|_| MountVolumeStateError::InvalidOnDiskLayout)?;
        let sector_index = cluster_index
            .checked_mul(sectors_per_cluster)
            .and_then(|offset| offset.checked_add(u64::from(self.cluster_heap_offset_sectors)))
            .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
        let sector_size = u64::try_from(self.sector_size)
            .map_err(|_| MountVolumeStateError::InvalidOnDiskLayout)?;
        let byte_offset = sector_index
            .checked_mul(sector_size)
            .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
        usize::try_from(byte_offset).map_err(|_| MountVolumeStateError::InvalidOnDiskLayout)
    }

    pub(super) fn cluster_count_usize(
        &self,
    ) -> core::result::Result<usize, MountVolumeStateError> {
        usize::try_from(self.cluster_count).map_err(|_| MountVolumeStateError::InvalidOnDiskLayout)
    }

    pub(super) fn cluster_from_index(
        &self,
        cluster_index: usize,
    ) -> core::result::Result<u32, MountVolumeStateError> {
        let cluster_index =
            u32::try_from(cluster_index).map_err(|_| MountVolumeStateError::InvalidOnDiskLayout)?;
        let cluster = FIRST_DATA_CLUSTER
            .checked_add(cluster_index)
            .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
        if !self.is_valid_cluster(cluster) {
            return Err(MountVolumeStateError::InvalidOnDiskLayout);
        }
        Ok(cluster)
    }

    pub(super) fn cluster_index(
        &self,
        cluster: u32,
    ) -> core::result::Result<usize, MountVolumeStateError> {
        if !self.is_valid_cluster(cluster) {
            return Err(MountVolumeStateError::InvalidOnDiskLayout);
        }
        usize::try_from(cluster - FIRST_DATA_CLUSTER)
            .map_err(|_| MountVolumeStateError::InvalidOnDiskLayout)
    }

    pub(super) fn data_capacity_bytes(
        &self,
    ) -> core::result::Result<u64, MountVolumeStateError> {
        let cluster_size = u64::try_from(self.cluster_size)
            .map_err(|_| MountVolumeStateError::InvalidOnDiskLayout)?;
        u64::from(self.cluster_count)
            .checked_mul(cluster_size)
            .ok_or(MountVolumeStateError::InvalidOnDiskLayout)
    }

    pub(super) fn is_valid_cluster(&self, cluster: u32) -> bool {
        cluster >= FIRST_DATA_CLUSTER
            && cluster
                <= self
                    .cluster_count
                    .checked_add(FIRST_DATA_CLUSTER - 1)
                    .unwrap_or(u32::MAX)
    }

    pub(super) fn validate_stream_data(
        &self,
        first_cluster: u32,
        data_length: u64,
    ) -> core::result::Result<(), MountVolumeStateError> {
        if !self.is_valid_cluster(first_cluster) || data_length == 0 {
            return Err(MountVolumeStateError::InvalidOnDiskLayout);
        }
        if data_length > self.data_capacity_bytes()? {
            return Err(MountVolumeStateError::InvalidOnDiskLayout);
        }
        Ok(())
    }

    pub(super) fn write_volume_anomaly_state(
        &self,
        block_device: &dyn BlockDevice,
        anomaly: VolumeAnomalyState,
    ) -> core::result::Result<(), MountVolumeStateError> {
        let mut boot_sector = vec![0; self.sector_size];
        block_device
            .read_bytes(0, &mut boot_sector)
            .map_err(|_| MountVolumeStateError::DeviceIo)?;

        let mut volume_flags = 0u16;
        if anomaly.volume_dirty {
            volume_flags |= 0x0002;
        }
        if anomaly.media_failure {
            volume_flags |= 0x0004;
        }
        if anomaly.clear_to_zero {
            volume_flags |= 0x0008;
        }
        boot_sector[106..108].copy_from_slice(&volume_flags.to_le_bytes());

        block_device
            .write_bytes(0, &boot_sector)
            .map_err(|_| MountVolumeStateError::DeviceIo)?;
        Ok(())
    }

    fn validate_checksum(
        &self,
        block_device: &dyn BlockDevice,
    ) -> core::result::Result<(), MountVolumeStateError> {
        let checksum_region_len = self
            .sector_size
            .checked_mul(11)
            .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
        let mut checksum_region = vec![0; checksum_region_len];
        block_device
            .read_bytes(0, &mut checksum_region)
            .map_err(|_| MountVolumeStateError::DeviceIo)?;
        let expected_checksum = Self::checksum(&checksum_region);

        let mut checksum_sector = vec![0; self.sector_size];
        block_device
            .read_bytes(checksum_region_len, &mut checksum_sector)
            .map_err(|_| MountVolumeStateError::DeviceIo)?;
        for chunk in checksum_sector.chunks_exact(mem::size_of::<u32>()) {
            if u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]) != expected_checksum {
                return Err(MountVolumeStateError::InvalidOnDiskLayout);
            }
        }
        Ok(())
    }

    fn validate_geometry(&self) -> core::result::Result<(), MountVolumeStateError> {
        if !self.is_valid_cluster(self.root_dir_cluster) {
            return Err(MountVolumeStateError::InvalidOnDiskLayout);
        }
        let data_sectors = u64::from(self.cluster_count)
            .checked_mul(
                u64::try_from(self.sectors_per_cluster)
                    .map_err(|_| MountVolumeStateError::InvalidOnDiskLayout)?,
            )
            .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
        let heap_end = u64::from(self.cluster_heap_offset_sectors)
            .checked_add(data_sectors)
            .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
        if heap_end > self.volume_length_sectors {
            return Err(MountVolumeStateError::InvalidOnDiskLayout);
        }
        let fat_end = u64::from(self.fat_offset_sectors)
            .checked_add(u64::from(self.fat_length_sectors))
            .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
        if fat_end > u64::from(self.cluster_heap_offset_sectors) {
            return Err(MountVolumeStateError::InvalidOnDiskLayout);
        }
        Ok(())
    }

    fn checksum(bytes: &[u8]) -> u32 {
        let mut checksum = 0u32;
        for (offset, byte) in bytes.iter().enumerate() {
            if offset == 106 || offset == 107 || offset == 112 {
                continue;
            }
            checksum = checksum.rotate_right(1).wrapping_add(u32::from(*byte));
        }
        checksum
    }

    fn scan_root_directory(
        boot_region: &BootRegion,
        fat_reader: &mut FatReader<'_>,
    ) -> core::result::Result<(AllocationBitmap, UpcaseRecord), MountVolumeStateError> {
        let mut bitmap = None;
        let mut upcase = None;
        fat_reader.walk_cluster_chain(boot_region.root_dir_cluster, |_, cluster_bytes| {
            for entry in cluster_bytes.chunks_exact(32) {
                match entry[0] {
                    END_OF_DIRECTORY_ENTRY_TYPE => return Ok(ChainVisitControl::Stop),
                    ALLOCATION_BITMAP_ENTRY_TYPE => {
                        bitmap = Some(AllocationBitmap::parse(entry)?)
                    }
                    UPCASE_TABLE_ENTRY_TYPE => upcase = Some(UpcaseRecord::parse(entry)?),
                    _ => (),
                }
                if bitmap.is_some() && upcase.is_some() {
                    return Ok(ChainVisitControl::Stop);
                }
            }
            Ok(ChainVisitControl::Continue)
        })?;
        Self::finalize_root_records(bitmap, upcase)
    }

    fn finalize_root_records(
        bitmap: Option<AllocationBitmap>,
        upcase: Option<UpcaseRecord>,
    ) -> core::result::Result<(AllocationBitmap, UpcaseRecord), MountVolumeStateError> {
        Ok((
            bitmap.ok_or(MountVolumeStateError::InvalidOnDiskLayout)?,
            upcase.ok_or(MountVolumeStateError::InvalidOnDiskLayout)?,
        ))
    }
}

#[derive(Clone, Copy)]
pub(super) struct VolumeAnomalyState {
    pub(super) clear_to_zero: bool,
    pub(super) media_failure: bool,
    pub(super) volume_dirty: bool,
}

impl VolumeAnomalyState {
    fn read(
        block_device: &dyn BlockDevice,
        boot_region: &BootRegion,
    ) -> core::result::Result<Self, MountVolumeStateError> {
        let mut boot_sector = vec![0; boot_region.sector_size];
        block_device
            .read_bytes(0, &mut boot_sector)
            .map_err(|_| MountVolumeStateError::DeviceIo)?;
        let volume_flags = u16::from_le_bytes([boot_sector[106], boot_sector[107]]);
        Ok(Self {
            clear_to_zero: volume_flags & 0x0008 != 0,
            media_failure: volume_flags & 0x0004 != 0,
            volume_dirty: volume_flags & 0x0002 != 0,
        })
    }
}
