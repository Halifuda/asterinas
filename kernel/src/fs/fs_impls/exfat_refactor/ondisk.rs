// SPDX-License-Identifier: MPL-2.0

use core::mem;

#[cfg(ktest)]
use alloc::collections::BTreeSet;

use aster_block::BlockDevice;
use ostd::mm::VmIo;

use super::fs::MountVolumeStateError;
use crate::prelude::*;

const ALLOCATION_BITMAP_ENTRY_TYPE: u8 = 0x81;
const END_OF_DIRECTORY_ENTRY_TYPE: u8 = 0x00;
const FIRST_DATA_CLUSTER: u32 = 2;
const MAX_CLUSTER_SIZE: usize = 32 * 1024 * 1024;
const UPCASE_TABLE_ENTRY_TYPE: u8 = 0x82;

#[derive(Clone, Copy)]
pub(super) struct AllocationBitmapRecord {
    pub(super) data_length: u64,
    pub(super) first_cluster: u32,
}

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
    fn cluster_offset(&self, cluster: u32) -> core::result::Result<usize, MountVolumeStateError> {
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

    pub(super) fn cluster_count_usize(&self) -> core::result::Result<usize, MountVolumeStateError> {
        usize::try_from(self.cluster_count).map_err(|_| MountVolumeStateError::InvalidOnDiskLayout)
    }

    fn data_capacity_bytes(&self) -> core::result::Result<u64, MountVolumeStateError> {
        let cluster_size = u64::try_from(self.cluster_size)
            .map_err(|_| MountVolumeStateError::InvalidOnDiskLayout)?;
        u64::from(self.cluster_count)
            .checked_mul(cluster_size)
            .ok_or(MountVolumeStateError::InvalidOnDiskLayout)
    }

    fn is_valid_cluster(&self, cluster: u32) -> bool {
        cluster >= FIRST_DATA_CLUSTER
            && cluster
                <= self
                    .cluster_count
                    .checked_add(FIRST_DATA_CLUSTER - 1)
                    .unwrap_or(u32::MAX)
    }
}

#[derive(Clone, Copy)]
enum ChainVisitControl {
    Continue,
    Stop,
}

#[derive(Clone, Copy)]
enum FatChainStep {
    Continue(u32),
    End,
}

#[derive(Clone, Copy)]
pub(super) struct VolumeAnomalyState {
    pub(super) clear_to_zero: bool,
    pub(super) media_failure: bool,
    pub(super) volume_dirty: bool,
}

#[derive(Clone)]
pub(super) struct UpcaseTable {
    pub(super) data: Vec<u8>,
}

impl UpcaseTable {
    pub(super) const NAME_MAX: usize = 255;
}

struct DirectoryBootstrap {
    bitmap: AllocationBitmapRecord,
    upcase: UpcaseRecord,
}

struct FatReader<'a> {
    block_device: &'a dyn BlockDevice,
    boot_region: &'a BootRegion,
    cached_sector_index: Option<u64>,
    cached_sector: Vec<u8>,
}

impl<'a> FatReader<'a> {
    fn new(block_device: &'a dyn BlockDevice, boot_region: &'a BootRegion) -> Self {
        Self {
            block_device,
            boot_region,
            cached_sector_index: None,
            cached_sector: vec![0; boot_region.sector_size],
        }
    }

    fn next_cluster(
        &mut self,
        current_cluster: u32,
    ) -> core::result::Result<FatChainStep, MountVolumeStateError> {
        let entry_offset = u64::from(self.boot_region.fat_offset_sectors)
            .checked_mul(
                u64::try_from(self.boot_region.sector_size)
                    .map_err(|_| MountVolumeStateError::InvalidOnDiskLayout)?,
            )
            .and_then(|offset| offset.checked_add(u64::from(current_cluster) * 4))
            .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
        let sector_size = u64::try_from(self.boot_region.sector_size)
            .map_err(|_| MountVolumeStateError::InvalidOnDiskLayout)?;
        let sector_index = entry_offset / sector_size;
        if self.cached_sector_index != Some(sector_index) {
            let sector_offset = sector_index
                .checked_mul(sector_size)
                .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
            read_device_bytes(
                self.block_device,
                usize::try_from(sector_offset)
                    .map_err(|_| MountVolumeStateError::InvalidOnDiskLayout)?,
                &mut self.cached_sector,
            )?;
            self.cached_sector_index = Some(sector_index);
        }
        let entry_within_sector = usize::try_from(entry_offset % sector_size)
            .map_err(|_| MountVolumeStateError::InvalidOnDiskLayout)?;
        let entry_end = entry_within_sector
            .checked_add(mem::size_of::<u32>())
            .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
        let next_cluster = read_le_u32(
            self.cached_sector
                .get(entry_within_sector..entry_end)
                .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?,
        );
        if next_cluster == 0xFFFF_FFF7 {
            return Err(MountVolumeStateError::InvalidOnDiskLayout);
        }
        if next_cluster >= 0xFFFF_FFF8 {
            return Ok(FatChainStep::End);
        }
        if !self.boot_region.is_valid_cluster(next_cluster) {
            return Err(MountVolumeStateError::InvalidOnDiskLayout);
        }
        Ok(FatChainStep::Continue(next_cluster))
    }
}

#[derive(Clone, Copy)]
struct UpcaseRecord {
    checksum: u32,
    data_length: u64,
    first_cluster: u32,
}

pub(super) struct ValidatedMount {
    pub(super) anomaly: VolumeAnomalyState,
    pub(super) bitmap: AllocationBitmapRecord,
    pub(super) boot_region: BootRegion,
    pub(super) upcase_table: Arc<UpcaseTable>,
    pub(super) used_clusters: usize,
    pub(super) used_clusters_from_recount: bool,
}

pub(super) fn load_validated_mount(
    block_device: &dyn BlockDevice,
) -> core::result::Result<ValidatedMount, MountVolumeStateError> {
    let boot_region = read_boot_region(block_device)?;
    let anomaly = read_anomaly_state(block_device, &boot_region)?;
    let mut fat_reader = FatReader::new(block_device, &boot_region);
    let directory_bootstrap = scan_root_directory(block_device, &boot_region, &mut fat_reader)?;
    let upcase_table = Arc::new(load_upcase_table(
        block_device,
        &boot_region,
        &mut fat_reader,
        directory_bootstrap.upcase,
    )?);
    let (used_clusters, used_clusters_from_recount) = count_used_clusters(
        block_device,
        &boot_region,
        &mut fat_reader,
        directory_bootstrap.bitmap,
    )?;
    Ok(ValidatedMount {
        anomaly,
        bitmap: directory_bootstrap.bitmap,
        boot_region,
        upcase_table,
        used_clusters,
        used_clusters_from_recount,
    })
}

fn count_used_clusters(
    block_device: &dyn BlockDevice,
    boot_region: &BootRegion,
    fat_reader: &mut FatReader<'_>,
    bitmap: AllocationBitmapRecord,
) -> core::result::Result<(usize, bool), MountVolumeStateError> {
    validate_stream_record(boot_region, bitmap.first_cluster, bitmap.data_length)?;
    let cluster_count = boot_region.cluster_count_usize()?;
    let required_bytes = cluster_count.div_ceil(8);
    let bitmap_bytes = usize::try_from(bitmap.data_length)
        .map_err(|_| MountVolumeStateError::InvalidOnDiskLayout)?;
    if bitmap_bytes < required_bytes {
        return Err(MountVolumeStateError::InconsistentAccounting);
    }

    let mut bits_remaining = cluster_count;
    let mut bitmap_bytes_remaining = bitmap_bytes;
    let mut used_clusters = 0usize;
    let result = walk_cluster_chain(
        block_device,
        boot_region,
        fat_reader,
        bitmap.first_cluster,
        |_, cluster_bytes| {
            let bytes_to_visit = bitmap_bytes_remaining.min(cluster_bytes.len());
            for byte in &cluster_bytes[..bytes_to_visit] {
                if bits_remaining == 0 {
                    if *byte != 0 {
                        return Err(MountVolumeStateError::InconsistentAccounting);
                    }
                    continue;
                }
                let relevant_bits = bits_remaining.min(u8::BITS as usize);
                let mask = if relevant_bits == u8::BITS as usize {
                    u8::MAX
                } else {
                    (1u16
                        .checked_shl(
                            u32::try_from(relevant_bits)
                                .map_err(|_| MountVolumeStateError::InconsistentAccounting)?,
                        )
                        .ok_or(MountVolumeStateError::InconsistentAccounting)?
                        - 1) as u8
                };
                let masked_byte = *byte & mask;
                if masked_byte != *byte && (*byte & !mask) != 0 {
                    return Err(MountVolumeStateError::InconsistentAccounting);
                }
                used_clusters = used_clusters
                    .checked_add(masked_byte.count_ones() as usize)
                    .ok_or(MountVolumeStateError::InconsistentAccounting)?;
                bits_remaining -= relevant_bits;
            }
            bitmap_bytes_remaining -= bytes_to_visit;
            if bitmap_bytes_remaining == 0 {
                return Ok(ChainVisitControl::Stop);
            }
            Ok(ChainVisitControl::Continue)
        },
    );
    match result {
        Ok(()) => (),
        Err(MountVolumeStateError::InvalidOnDiskLayout) => {
            return Err(MountVolumeStateError::InconsistentAccounting);
        }
        Err(error) => return Err(error),
    }
    if bits_remaining != 0 || bitmap_bytes_remaining != 0 {
        return Err(MountVolumeStateError::InconsistentAccounting);
    }

    let counted_percent = if cluster_count == 0 {
        return Err(MountVolumeStateError::InvalidOnDiskLayout);
    } else {
        (used_clusters.saturating_mul(100) + cluster_count / 2) / cluster_count
    };
    let used_clusters_from_recount = match boot_region.percent_in_use {
        0xFF => true,
        percent_in_use => counted_percent != usize::from(percent_in_use),
    };
    Ok((used_clusters, used_clusters_from_recount))
}

fn load_upcase_table(
    block_device: &dyn BlockDevice,
    boot_region: &BootRegion,
    fat_reader: &mut FatReader<'_>,
    upcase: UpcaseRecord,
) -> core::result::Result<UpcaseTable, MountVolumeStateError> {
    validate_stream_record(boot_region, upcase.first_cluster, upcase.data_length)?;
    let data_length = usize::try_from(upcase.data_length)
        .map_err(|_| MountVolumeStateError::InvalidOnDiskLayout)?;
    let mut remaining = data_length;
    let mut table_bytes = Vec::with_capacity(data_length);
    walk_cluster_chain(
        block_device,
        boot_region,
        fat_reader,
        upcase.first_cluster,
        |_, cluster_bytes| {
            let bytes_to_copy = remaining.min(cluster_bytes.len());
            table_bytes.extend_from_slice(&cluster_bytes[..bytes_to_copy]);
            remaining -= bytes_to_copy;
            if remaining == 0 {
                return Ok(ChainVisitControl::Stop);
            }
            Ok(ChainVisitControl::Continue)
        },
    )?;
    if remaining != 0 {
        return Err(MountVolumeStateError::InvalidOnDiskLayout);
    }
    if stream_checksum(&table_bytes) != upcase.checksum {
        return Err(MountVolumeStateError::InvalidOnDiskLayout);
    }
    Ok(UpcaseTable { data: table_bytes })
}

fn parse_bitmap_record(entry: &[u8]) -> core::result::Result<AllocationBitmapRecord, MountVolumeStateError> {
    if entry.len() != 32 {
        return Err(MountVolumeStateError::InvalidOnDiskLayout);
    }
    Ok(AllocationBitmapRecord {
        first_cluster: read_le_u32(&entry[20..24]),
        data_length: read_le_u64(&entry[24..32]),
    })
}

fn parse_upcase_record(entry: &[u8]) -> core::result::Result<UpcaseRecord, MountVolumeStateError> {
    if entry.len() != 32 {
        return Err(MountVolumeStateError::InvalidOnDiskLayout);
    }
    Ok(UpcaseRecord {
        checksum: read_le_u32(&entry[4..8]),
        first_cluster: read_le_u32(&entry[20..24]),
        data_length: read_le_u64(&entry[24..32]),
    })
}

fn read_anomaly_state(
    block_device: &dyn BlockDevice,
    boot_region: &BootRegion,
) -> core::result::Result<VolumeAnomalyState, MountVolumeStateError> {
    let mut boot_sector = vec![0; boot_region.sector_size];
    read_device_bytes(block_device, 0, &mut boot_sector)?;
    let volume_flags = read_le_u16(&boot_sector[106..108]);
    Ok(VolumeAnomalyState {
        clear_to_zero: volume_flags & 0x0008 != 0,
        media_failure: volume_flags & 0x0004 != 0,
        volume_dirty: volume_flags & 0x0002 != 0,
    })
}

fn read_boot_region(
    block_device: &dyn BlockDevice,
) -> core::result::Result<BootRegion, MountVolumeStateError> {
    let mut sector_header = [0u8; 512];
    read_device_bytes(block_device, 0, &mut sector_header)?;
    if &sector_header[3..11] != b"EXFAT   " {
        return Err(MountVolumeStateError::InvalidOnDiskLayout);
    }
    if read_le_u16(&sector_header[510..512]) != 0xAA55 {
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

    let volume_length_sectors = read_le_u64(&sector_header[72..80]);
    let fat_offset_sectors = read_le_u32(&sector_header[80..84]);
    let fat_length_sectors = read_le_u32(&sector_header[84..88]);
    let cluster_heap_offset_sectors = read_le_u32(&sector_header[88..92]);
    let cluster_count = read_le_u32(&sector_header[92..96]);
    let root_dir_cluster = read_le_u32(&sector_header[96..100]);
    let volume_serial_number = read_le_u32(&sector_header[100..104]);
    let number_of_fats = sector_header[110];
    let percent_in_use = sector_header[112];

    if number_of_fats != 1
        || fat_offset_sectors == 0
        || fat_length_sectors == 0
        || cluster_count == 0
    {
        return Err(MountVolumeStateError::InvalidOnDiskLayout);
    }

    let boot_region = BootRegion {
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
    validate_boot_geometry(&boot_region)?;
    validate_boot_checksum(block_device, &boot_region)?;
    Ok(boot_region)
}

fn read_device_bytes(
    block_device: &dyn BlockDevice,
    offset: usize,
    buffer: &mut [u8],
) -> core::result::Result<(), MountVolumeStateError> {
    block_device
        .read_bytes(offset, buffer)
        .map_err(|_| MountVolumeStateError::DeviceIo)
}

fn read_le_u16(bytes: &[u8]) -> u16 {
    u16::from_le_bytes([bytes[0], bytes[1]])
}

fn read_le_u32(bytes: &[u8]) -> u32 {
    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

fn read_le_u64(bytes: &[u8]) -> u64 {
    u64::from_le_bytes([
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
    ])
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

fn stream_checksum(bytes: &[u8]) -> u32 {
    let mut checksum = 0u32;
    for byte in bytes {
        checksum = checksum.rotate_right(1).wrapping_add(u32::from(*byte));
    }
    checksum
}

fn scan_root_directory(
    block_device: &dyn BlockDevice,
    boot_region: &BootRegion,
    fat_reader: &mut FatReader<'_>,
) -> core::result::Result<DirectoryBootstrap, MountVolumeStateError> {
    let mut bitmap = None;
    let mut upcase = None;
    walk_cluster_chain(
        block_device,
        boot_region,
        fat_reader,
        boot_region.root_dir_cluster,
        |_, cluster_bytes| {
            for entry in cluster_bytes.chunks_exact(32) {
                match entry[0] {
                    END_OF_DIRECTORY_ENTRY_TYPE => return Ok(ChainVisitControl::Stop),
                    ALLOCATION_BITMAP_ENTRY_TYPE => {
                        bitmap = Some(parse_bitmap_record(entry)?);
                    }
                    UPCASE_TABLE_ENTRY_TYPE => {
                        upcase = Some(parse_upcase_record(entry)?);
                    }
                    _ => (),
                }
                if bitmap.is_some() && upcase.is_some() {
                    return Ok(ChainVisitControl::Stop);
                }
            }
            Ok(ChainVisitControl::Continue)
        },
    )?;
    Ok(DirectoryBootstrap {
        bitmap: bitmap.ok_or(MountVolumeStateError::InvalidOnDiskLayout)?,
        upcase: upcase.ok_or(MountVolumeStateError::InvalidOnDiskLayout)?,
    })
}

fn validate_boot_checksum(
    block_device: &dyn BlockDevice,
    boot_region: &BootRegion,
) -> core::result::Result<(), MountVolumeStateError> {
    let checksum_region_len = boot_region
        .sector_size
        .checked_mul(11)
        .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
    let mut checksum_region = vec![0; checksum_region_len];
    read_device_bytes(block_device, 0, &mut checksum_region)?;
    let expected_checksum = boot_region_checksum(&checksum_region);

    let mut checksum_sector = vec![0; boot_region.sector_size];
    read_device_bytes(
        block_device,
        checksum_region_len,
        &mut checksum_sector,
    )?;
    for chunk in checksum_sector.chunks_exact(mem::size_of::<u32>()) {
        if read_le_u32(chunk) != expected_checksum {
            return Err(MountVolumeStateError::InvalidOnDiskLayout);
        }
    }
    Ok(())
}

fn validate_boot_geometry(
    boot_region: &BootRegion,
) -> core::result::Result<(), MountVolumeStateError> {
    if !boot_region.is_valid_cluster(boot_region.root_dir_cluster) {
        return Err(MountVolumeStateError::InvalidOnDiskLayout);
    }
    let data_sectors = u64::from(boot_region.cluster_count)
        .checked_mul(
            u64::try_from(boot_region.sectors_per_cluster)
                .map_err(|_| MountVolumeStateError::InvalidOnDiskLayout)?,
        )
        .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
    let heap_end = u64::from(boot_region.cluster_heap_offset_sectors)
        .checked_add(data_sectors)
        .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
    if heap_end > boot_region.volume_length_sectors {
        return Err(MountVolumeStateError::InvalidOnDiskLayout);
    }
    let fat_end = u64::from(boot_region.fat_offset_sectors)
        .checked_add(u64::from(boot_region.fat_length_sectors))
        .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
    if fat_end > u64::from(boot_region.cluster_heap_offset_sectors) {
        return Err(MountVolumeStateError::InvalidOnDiskLayout);
    }
    Ok(())
}

fn validate_stream_record(
    boot_region: &BootRegion,
    first_cluster: u32,
    data_length: u64,
) -> core::result::Result<(), MountVolumeStateError> {
    if !boot_region.is_valid_cluster(first_cluster) || data_length == 0 {
        return Err(MountVolumeStateError::InvalidOnDiskLayout);
    }
    if data_length > boot_region.data_capacity_bytes()? {
        return Err(MountVolumeStateError::InvalidOnDiskLayout);
    }
    Ok(())
}

fn walk_cluster_chain<F>(
    block_device: &dyn BlockDevice,
    boot_region: &BootRegion,
    fat_reader: &mut FatReader<'_>,
    start_cluster: u32,
    mut visit_cluster_fn: F,
) -> core::result::Result<(), MountVolumeStateError>
where
    F: FnMut(u32, &[u8]) -> core::result::Result<ChainVisitControl, MountVolumeStateError>,
{
    if !boot_region.is_valid_cluster(start_cluster) {
        return Err(MountVolumeStateError::InvalidOnDiskLayout);
    }
    let mut cluster_buffer = vec![0; boot_region.cluster_size];
    let mut current_cluster = start_cluster;
    let mut visited_clusters = BTreeSet::new();
    loop {
        if !visited_clusters.insert(current_cluster) {
            return Err(MountVolumeStateError::InvalidOnDiskLayout);
        }
        let cluster_offset = boot_region.cluster_offset(current_cluster)?;
        read_device_bytes(block_device, cluster_offset, &mut cluster_buffer)?;
        if matches!(
            visit_cluster_fn(current_cluster, &cluster_buffer)?,
            ChainVisitControl::Stop
        ) {
            return Ok(());
        }
        current_cluster = match fat_reader.next_cluster(current_cluster)? {
            FatChainStep::Continue(next_cluster) => next_cluster,
            FatChainStep::End => return Ok(()),
        };
    }
}

#[cfg(ktest)]
pub(super) fn diagnose_invalid_on_disk_layout_gate(block_device: &dyn BlockDevice) -> &'static str {
    let boot_region = match diagnose_boot_region(block_device) {
        Ok(boot_region) => boot_region,
        Err(gate) => return gate,
    };
    if read_anomaly_state(block_device, &boot_region).is_err() {
        return "read_anomaly_state:device_io";
    }
    let mut fat_reader = FatReader::new(block_device, &boot_region);
    let directory_bootstrap =
        match diagnose_scan_root_directory(block_device, &boot_region, &mut fat_reader) {
            Ok(directory_bootstrap) => directory_bootstrap,
            Err(gate) => return gate,
        };
    if let Err(gate) = diagnose_load_upcase_table(
        block_device,
        &boot_region,
        &mut fat_reader,
        directory_bootstrap.upcase,
    ) {
        return gate;
    }
    match diagnose_count_used_clusters(
        block_device,
        &boot_region,
        &mut fat_reader,
        directory_bootstrap.bitmap,
    ) {
        Ok(()) => "accepted",
        Err(gate) => gate,
    }
}

#[cfg(ktest)]
fn diagnose_boot_region(block_device: &dyn BlockDevice) -> core::result::Result<BootRegion, &'static str> {
    let mut sector_header = [0u8; 512];
    if read_device_bytes(block_device, 0, &mut sector_header).is_err() {
        return Err("read_boot_region:device_io");
    }
    if &sector_header[3..11] != b"EXFAT   " {
        return Err("read_boot_region:oem_name");
    }
    if read_le_u16(&sector_header[510..512]) != 0xAA55 {
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

    let fat_offset_sectors = read_le_u32(&sector_header[80..84]);
    let fat_length_sectors = read_le_u32(&sector_header[84..88]);
    let cluster_count = read_le_u32(&sector_header[92..96]);
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
        cluster_heap_offset_sectors: read_le_u32(&sector_header[88..92]),
        cluster_size,
        fat_length_sectors,
        fat_offset_sectors,
        percent_in_use: sector_header[112],
        root_dir_cluster: read_le_u32(&sector_header[96..100]),
        sector_size,
        sectors_per_cluster,
        volume_length_sectors: read_le_u64(&sector_header[72..80]),
        volume_serial_number: read_le_u32(&sector_header[100..104]),
    };
    diagnose_validate_boot_geometry(&boot_region)?;
    diagnose_validate_boot_checksum(block_device, &boot_region)?;
    Ok(boot_region)
}

#[cfg(ktest)]
fn diagnose_validate_boot_checksum(
    block_device: &dyn BlockDevice,
    boot_region: &BootRegion,
) -> core::result::Result<(), &'static str> {
    let checksum_region_len = match boot_region.sector_size.checked_mul(11) {
        Some(checksum_region_len) => checksum_region_len,
        None => return Err("validate_boot_checksum:checksum_region_len_overflow"),
    };
    let mut checksum_region = vec![0; checksum_region_len];
    if read_device_bytes(block_device, 0, &mut checksum_region).is_err() {
        return Err("validate_boot_checksum:checksum_region_device_io");
    }
    let expected_checksum = boot_region_checksum(&checksum_region);

    let mut checksum_sector = vec![0; boot_region.sector_size];
    if read_device_bytes(block_device, checksum_region_len, &mut checksum_sector).is_err() {
        return Err("validate_boot_checksum:checksum_sector_device_io");
    }
    for chunk in checksum_sector.chunks_exact(mem::size_of::<u32>()) {
        if read_le_u32(chunk) != expected_checksum {
            return Err("validate_boot_checksum:mismatched_checksum_sector");
        }
    }
    Ok(())
}

#[cfg(ktest)]
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
    let heap_end = match u64::from(boot_region.cluster_heap_offset_sectors).checked_add(data_sectors)
    {
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

#[cfg(ktest)]
fn diagnose_scan_root_directory(
    block_device: &dyn BlockDevice,
    boot_region: &BootRegion,
    fat_reader: &mut FatReader<'_>,
) -> core::result::Result<DirectoryBootstrap, &'static str> {
    let mut cluster_buffer = vec![0; boot_region.cluster_size];
    let mut current_cluster = boot_region.root_dir_cluster;
    let mut visited_clusters = BTreeSet::new();
    let mut bitmap = None;
    let mut upcase = None;

    loop {
        if !visited_clusters.insert(current_cluster) {
            return Err("scan_root_directory:cluster_chain_loop");
        }
        let cluster_offset = match boot_region.cluster_offset(current_cluster) {
            Ok(cluster_offset) => cluster_offset,
            Err(_) => return Err("scan_root_directory:cluster_offset_invalid"),
        };
        if read_device_bytes(block_device, cluster_offset, &mut cluster_buffer).is_err() {
            return Err("scan_root_directory:device_io");
        }
        for entry in cluster_buffer.chunks_exact(32) {
            match entry[0] {
                END_OF_DIRECTORY_ENTRY_TYPE => {
                    return match (bitmap, upcase) {
                        (Some(bitmap), Some(upcase)) => Ok(DirectoryBootstrap { bitmap, upcase }),
                        (None, _) => Err("scan_root_directory:missing_allocation_bitmap_record"),
                        (_, None) => Err("scan_root_directory:missing_upcase_record"),
                    };
                }
                ALLOCATION_BITMAP_ENTRY_TYPE => match parse_bitmap_record(entry) {
                    Ok(record) => bitmap = Some(record),
                    Err(_) => return Err("scan_root_directory:invalid_allocation_bitmap_record"),
                },
                UPCASE_TABLE_ENTRY_TYPE => match parse_upcase_record(entry) {
                    Ok(record) => upcase = Some(record),
                    Err(_) => return Err("scan_root_directory:invalid_upcase_record"),
                },
                _ => (),
            }
            if let (Some(bitmap), Some(upcase)) = (bitmap, upcase) {
                return Ok(DirectoryBootstrap { bitmap, upcase });
            }
        }

        current_cluster = match fat_reader.next_cluster(current_cluster) {
            Ok(FatChainStep::Continue(next_cluster)) => next_cluster,
            Ok(FatChainStep::End) => {
                return match (bitmap, upcase) {
                    (Some(bitmap), Some(upcase)) => Ok(DirectoryBootstrap { bitmap, upcase }),
                    (None, _) => Err("scan_root_directory:missing_allocation_bitmap_record"),
                    (_, None) => Err("scan_root_directory:missing_upcase_record"),
                };
            }
            Err(MountVolumeStateError::DeviceIo) => return Err("scan_root_directory:fat_device_io"),
            Err(_) => return Err("scan_root_directory:fat_chain_invalid"),
        };
    }
}

#[cfg(ktest)]
fn diagnose_load_upcase_table(
    block_device: &dyn BlockDevice,
    boot_region: &BootRegion,
    fat_reader: &mut FatReader<'_>,
    upcase: UpcaseRecord,
) -> core::result::Result<(), &'static str> {
    if !boot_region.is_valid_cluster(upcase.first_cluster) {
        return Err("load_upcase_table:first_cluster_out_of_range");
    }
    if upcase.data_length == 0 {
        return Err("load_upcase_table:data_length_zero");
    }
    let data_capacity = match boot_region.data_capacity_bytes() {
        Ok(data_capacity) => data_capacity,
        Err(_) => return Err("load_upcase_table:data_capacity_overflow"),
    };
    if upcase.data_length > data_capacity {
        return Err("load_upcase_table:data_length_exceeds_data_capacity");
    }
    let data_length = match usize::try_from(upcase.data_length) {
        Ok(data_length) => data_length,
        Err(_) => return Err("load_upcase_table:data_length_usize_conversion"),
    };
    let mut remaining = data_length;
    let mut table_bytes = Vec::with_capacity(data_length);
    let mut cluster_buffer = vec![0; boot_region.cluster_size];
    let mut current_cluster = upcase.first_cluster;
    let mut visited_clusters = BTreeSet::new();

    loop {
        if !visited_clusters.insert(current_cluster) {
            return Err("load_upcase_table:cluster_chain_loop");
        }
        let cluster_offset = match boot_region.cluster_offset(current_cluster) {
            Ok(cluster_offset) => cluster_offset,
            Err(_) => return Err("load_upcase_table:cluster_offset_invalid"),
        };
        if read_device_bytes(block_device, cluster_offset, &mut cluster_buffer).is_err() {
            return Err("load_upcase_table:device_io");
        }

        let bytes_to_copy = remaining.min(cluster_buffer.len());
        table_bytes.extend_from_slice(&cluster_buffer[..bytes_to_copy]);
        remaining -= bytes_to_copy;
        if remaining == 0 {
            break;
        }

        current_cluster = match fat_reader.next_cluster(current_cluster) {
            Ok(FatChainStep::Continue(next_cluster)) => next_cluster,
            Ok(FatChainStep::End) => return Err("load_upcase_table:stream_shorter_than_data_length"),
            Err(MountVolumeStateError::DeviceIo) => return Err("load_upcase_table:fat_device_io"),
            Err(_) => return Err("load_upcase_table:fat_chain_invalid"),
        };
    }

    if stream_checksum(&table_bytes) != upcase.checksum {
        return Err("load_upcase_table:stream_checksum_mismatch");
    }
    Ok(())
}

#[cfg(ktest)]
fn diagnose_count_used_clusters(
    block_device: &dyn BlockDevice,
    boot_region: &BootRegion,
    fat_reader: &mut FatReader<'_>,
    bitmap: AllocationBitmapRecord,
) -> core::result::Result<(), &'static str> {
    if !boot_region.is_valid_cluster(bitmap.first_cluster) {
        return Err("count_used_clusters:first_cluster_out_of_range");
    }
    if bitmap.data_length == 0 {
        return Err("count_used_clusters:data_length_zero");
    }
    let data_capacity = match boot_region.data_capacity_bytes() {
        Ok(data_capacity) => data_capacity,
        Err(_) => return Err("count_used_clusters:data_capacity_overflow"),
    };
    if bitmap.data_length > data_capacity {
        return Err("count_used_clusters:data_length_exceeds_data_capacity");
    }
    let cluster_count = match boot_region.cluster_count_usize() {
        Ok(cluster_count) => cluster_count,
        Err(_) => return Err("count_used_clusters:cluster_count_usize_conversion"),
    };
    let required_bytes = cluster_count.div_ceil(8);
    let bitmap_bytes = match usize::try_from(bitmap.data_length) {
        Ok(bitmap_bytes) => bitmap_bytes,
        Err(_) => return Err("count_used_clusters:data_length_usize_conversion"),
    };
    if bitmap_bytes < required_bytes {
        return Err("count_used_clusters:bitmap_shorter_than_cluster_map");
    }

    let mut bits_remaining = cluster_count;
    let mut bitmap_bytes_remaining = bitmap_bytes;
    let mut cluster_buffer = vec![0; boot_region.cluster_size];
    let mut current_cluster = bitmap.first_cluster;
    let mut visited_clusters = BTreeSet::new();

    loop {
        if !visited_clusters.insert(current_cluster) {
            return Err("count_used_clusters:cluster_chain_loop");
        }
        let cluster_offset = match boot_region.cluster_offset(current_cluster) {
            Ok(cluster_offset) => cluster_offset,
            Err(_) => return Err("count_used_clusters:cluster_offset_invalid"),
        };
        if read_device_bytes(block_device, cluster_offset, &mut cluster_buffer).is_err() {
            return Err("count_used_clusters:device_io");
        }

        let bytes_to_visit = bitmap_bytes_remaining.min(cluster_buffer.len());
        for byte in &cluster_buffer[..bytes_to_visit] {
            if bits_remaining == 0 {
                if *byte != 0 {
                    return Err("count_used_clusters:nonzero_trailing_stream_padding");
                }
                continue;
            }
            let relevant_bits = bits_remaining.min(u8::BITS as usize);
            let mask = if relevant_bits == u8::BITS as usize {
                u8::MAX
            } else {
                match 1u16.checked_shl(u32::try_from(relevant_bits).unwrap()) {
                    Some(mask) => (mask - 1) as u8,
                    None => return Err("count_used_clusters:padding_mask_overflow"),
                }
            };
            if (*byte & !mask) != 0 {
                return Err("count_used_clusters:nonzero_unused_bits");
            }
            bits_remaining -= relevant_bits;
        }
        bitmap_bytes_remaining -= bytes_to_visit;
        if bitmap_bytes_remaining == 0 {
            break;
        }

        current_cluster = match fat_reader.next_cluster(current_cluster) {
            Ok(FatChainStep::Continue(next_cluster)) => next_cluster,
            Ok(FatChainStep::End) => {
                return Err("count_used_clusters:stream_shorter_than_data_length");
            }
            Err(MountVolumeStateError::DeviceIo) => return Err("count_used_clusters:fat_device_io"),
            Err(_) => return Err("count_used_clusters:fat_chain_invalid"),
        };
    }

    if bits_remaining != 0 {
        return Err("count_used_clusters:bitmap_shorter_than_cluster_map");
    }
    Ok(())
}
