// SPDX-License-Identifier: MPL-2.0

use core::mem;

use aster_block::BlockDevice;
use ostd::mm::VmIo;

use super::{
    bitmap::{ALLOCATION_BITMAP_ENTRY_TYPE, AllocationBitmap},
    fat::{ChainVisitControl, FatReader},
    device_io, invalid_on_disk_layout,
    fs::VolumeAnomalyState,
    upcase::{UPCASE_TABLE_ENTRY_TYPE, UpcaseTable},
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
    ) -> Result<(Self, VolumeAnomalyState, AllocationBitmap, Arc<UpcaseTable>)> {
        let boot_region = Self::load(block_device)?;
        let anomaly = VolumeAnomalyState::read(block_device, &boot_region)?;
        let mut fat_reader = FatReader::new(block_device, &boot_region);
        let (mut bitmap, upcase_entry) =
            Self::scan_root_directory(&boot_region, &mut fat_reader)?;
        let upcase_table =
            Arc::new(UpcaseTable::load(&boot_region, &mut fat_reader, upcase_entry)?);
        let used_clusters = bitmap.count_used_clusters(&boot_region, &mut fat_reader)?;
        bitmap.set_used_clusters(used_clusters);
        Ok((boot_region, anomaly, bitmap, upcase_table))
    }

    fn load(block_device: &dyn BlockDevice) -> Result<Self> {
        let mut sector_header = [0u8; 512];
        block_device
            .read_bytes(0, &mut sector_header)
            .map_err(|_| device_io())?;
        if &sector_header[3..11] != b"EXFAT   " {
            return Err(invalid_on_disk_layout());
        }
        if u16::from_le_bytes([sector_header[510], sector_header[511]]) != 0xAA55 {
            return Err(invalid_on_disk_layout());
        }

        let bytes_per_sector_shift = sector_header[108];
        let sectors_per_cluster_shift = sector_header[109];
        if !(9..=12).contains(&bytes_per_sector_shift) {
            return Err(invalid_on_disk_layout());
        }
        let sector_size = 1usize
            .checked_shl(u32::from(bytes_per_sector_shift))
            .ok_or_else(invalid_on_disk_layout)?;
        let sectors_per_cluster = 1usize
            .checked_shl(u32::from(sectors_per_cluster_shift))
            .ok_or_else(invalid_on_disk_layout)?;
        let cluster_size = sector_size
            .checked_mul(sectors_per_cluster)
            .ok_or_else(invalid_on_disk_layout)?;
        if cluster_size == 0 || cluster_size > MAX_CLUSTER_SIZE {
            return Err(invalid_on_disk_layout());
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
            return Err(invalid_on_disk_layout());
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

    pub(super) fn cluster_offset(&self, cluster: u32) -> Result<usize> {
        if !self.is_valid_cluster(cluster) {
            return Err(invalid_on_disk_layout());
        }
        let cluster_index = u64::from(cluster - FIRST_DATA_CLUSTER);
        let sectors_per_cluster = u64::try_from(self.sectors_per_cluster)
            .map_err(|_| invalid_on_disk_layout())?;
        let sector_index = cluster_index
            .checked_mul(sectors_per_cluster)
            .and_then(|offset| offset.checked_add(u64::from(self.cluster_heap_offset_sectors)))
            .ok_or_else(invalid_on_disk_layout)?;
        let sector_size = u64::try_from(self.sector_size).map_err(|_| invalid_on_disk_layout())?;
        let byte_offset = sector_index
            .checked_mul(sector_size)
            .ok_or_else(invalid_on_disk_layout)?;
        usize::try_from(byte_offset).map_err(|_| invalid_on_disk_layout())
    }

    pub(super) fn cluster_count_usize(&self) -> Result<usize> {
        usize::try_from(self.cluster_count).map_err(|_| invalid_on_disk_layout())
    }

    pub(super) fn cluster_from_index(
        &self,
        cluster_index: usize,
    ) -> Result<u32> {
        let cluster_index = u32::try_from(cluster_index).map_err(|_| invalid_on_disk_layout())?;
        let cluster = FIRST_DATA_CLUSTER
            .checked_add(cluster_index)
            .ok_or_else(invalid_on_disk_layout)?;
        if !self.is_valid_cluster(cluster) {
            return Err(invalid_on_disk_layout());
        }
        Ok(cluster)
    }

    pub(super) fn cluster_index(&self, cluster: u32) -> Result<usize> {
        if !self.is_valid_cluster(cluster) {
            return Err(invalid_on_disk_layout());
        }
        usize::try_from(cluster - FIRST_DATA_CLUSTER).map_err(|_| invalid_on_disk_layout())
    }

    pub(super) fn data_capacity_bytes(&self) -> Result<u64> {
        let cluster_size = u64::try_from(self.cluster_size).map_err(|_| invalid_on_disk_layout())?;
        u64::from(self.cluster_count)
            .checked_mul(cluster_size)
            .ok_or_else(invalid_on_disk_layout)
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
    ) -> Result<()> {
        if !self.is_valid_cluster(first_cluster) || data_length == 0 {
            return Err(invalid_on_disk_layout());
        }
        if data_length > self.data_capacity_bytes()? {
            return Err(invalid_on_disk_layout());
        }
        Ok(())
    }

    fn validate_checksum(
        &self,
        block_device: &dyn BlockDevice,
    ) -> Result<()> {
        let checksum_region_len = self
            .sector_size
            .checked_mul(11)
            .ok_or_else(invalid_on_disk_layout)?;
        let mut checksum_region = vec![0; checksum_region_len];
        block_device
            .read_bytes(0, &mut checksum_region)
            .map_err(|_| device_io())?;
        let expected_checksum = Self::checksum(&checksum_region);

        let mut checksum_sector = vec![0; self.sector_size];
        block_device
            .read_bytes(checksum_region_len, &mut checksum_sector)
            .map_err(|_| device_io())?;
        for chunk in checksum_sector.chunks_exact(mem::size_of::<u32>()) {
            if u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]) != expected_checksum {
                return Err(invalid_on_disk_layout());
            }
        }
        Ok(())
    }

    fn validate_geometry(&self) -> Result<()> {
        if !self.is_valid_cluster(self.root_dir_cluster) {
            return Err(invalid_on_disk_layout());
        }
        let data_sectors = u64::from(self.cluster_count)
            .checked_mul(
                u64::try_from(self.sectors_per_cluster)
                    .map_err(|_| invalid_on_disk_layout())?,
            )
            .ok_or_else(invalid_on_disk_layout)?;
        let heap_end = u64::from(self.cluster_heap_offset_sectors)
            .checked_add(data_sectors)
            .ok_or_else(invalid_on_disk_layout)?;
        if heap_end > self.volume_length_sectors {
            return Err(invalid_on_disk_layout());
        }
        let fat_end = u64::from(self.fat_offset_sectors)
            .checked_add(u64::from(self.fat_length_sectors))
            .ok_or_else(invalid_on_disk_layout)?;
        if fat_end > u64::from(self.cluster_heap_offset_sectors) {
            return Err(invalid_on_disk_layout());
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
    ) -> Result<(AllocationBitmap, [u8; 32])> {
        let mut bitmap = None;
        let mut upcase_entry = None;
        fat_reader.walk_cluster_chain(boot_region.root_dir_cluster, |_, cluster_bytes| {
            for entry in cluster_bytes.chunks_exact(32) {
                match entry[0] {
                    END_OF_DIRECTORY_ENTRY_TYPE => return Ok(ChainVisitControl::Stop),
                    ALLOCATION_BITMAP_ENTRY_TYPE => bitmap = Some(AllocationBitmap::parse(entry)?),
                    UPCASE_TABLE_ENTRY_TYPE => {
                        upcase_entry = Some(
                            <[u8; 32]>::try_from(entry).map_err(|_| invalid_on_disk_layout())?,
                        );
                    }
                    _ => (),
                }
                if bitmap.is_some() && upcase_entry.is_some() {
                    return Ok(ChainVisitControl::Stop);
                }
            }
            Ok(ChainVisitControl::Continue)
        })?;
        Ok((
            bitmap.ok_or_else(invalid_on_disk_layout)?,
            upcase_entry.ok_or_else(invalid_on_disk_layout)?,
        ))
    }

    pub(super) fn write_volume_anomaly_state(
        &self,
        block_device: &dyn BlockDevice,
        anomaly: VolumeAnomalyState,
    ) -> Result<()> {
        let mut boot_sector = vec![0; self.sector_size];
        block_device
            .read_bytes(0, &mut boot_sector)
            .map_err(|_| device_io())?;

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
            .map_err(|_| device_io())?;
        Ok(())
    }
}
