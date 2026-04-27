// SPDX-License-Identifier: MPL-2.0

use alloc::{sync::Arc, vec, vec::Vec};
use core::{
    fmt,
    ops::Range,
    sync::atomic::{AtomicBool, Ordering},
};

use aster_block::{
    bio::{BioEnqueueError, BioSegment, BioStatus, BioType, SubmittedBio},
    BlockDevice, BlockDeviceMeta, SECTOR_SIZE,
};
use device_id::DeviceId;
use ostd::mm::{io::util::HasVmReaderWriter, FrameAllocOptions, HasSize, Segment, VmIo, PAGE_SIZE};
use spin::Mutex;

use super::load_validated_mount;

const DIRECTORY_ENTRY_SIZE: usize = 32;
const ALLOCATION_BITMAP_ENTRY_TYPE: u8 = 0x81;
const BENIGN_UNRECOGNIZED_ENTRY_TYPE: u8 = 0xA0;
const END_OF_DIRECTORY_ENTRY_TYPE: u8 = 0x00;
const FILE_ATTRIBUTE_DIRECTORY: u16 = 0x0010;
const FILE_ATTRIBUTE_REGULAR: u16 = 0x0020;
const FILE_DIRECTORY_ENTRY_TYPE: u8 = 0x85;
const FILE_NAME_ENTRY_TYPE: u8 = 0xC1;
const FAT_END_OF_CHAIN: u32 = 0xFFFF_FFFF;
const STREAM_EXTENSION_ENTRY_TYPE: u8 = 0xC0;
const VOLUME_FLAGS_OFFSET: usize = 106;
static EXFAT_IMAGE: &[u8] = include_bytes!("../../../../../../test/initramfs/build/exfat.img");

pub(in super::super) struct ExfatLookupTestDisk {
    blocks: Segment<()>,
    observed_bios: Mutex<Vec<ObservedBio>>,
}

pub(in super::super) struct ExfatLookupToggleFailingWriteDisk {
    fail_range: core::ops::Range<usize>,
    fail_writes: AtomicBool,
    inner: Arc<ExfatLookupTestDisk>,
}

pub(in super::super) struct ExfatLookupToggleFailingReadDisk {
    fail_range: core::ops::Range<usize>,
    fail_reads: AtomicBool,
    inner: Arc<ExfatLookupTestDisk>,
}

pub(in super::super) struct ExfatLookupFlushControlDisk {
    block_flush: AtomicBool,
    fail_flush: AtomicBool,
    flush_started: AtomicBool,
    inner: Arc<ExfatLookupTestDisk>,
}

#[derive(Clone, Debug, PartialEq)]
pub(in super::super) struct ObservedBio {
    pub(in super::super) byte_range: Range<usize>,
    pub(in super::super) segment_lengths: Vec<usize>,
    pub(in super::super) type_: BioType,
}

impl ExfatLookupTestDisk {
    pub(in super::super) fn new() -> Arc<Self> {
        let blocks = FrameAllocOptions::new()
            .zeroed(false)
            .alloc_segment(EXFAT_IMAGE.len().div_ceil(PAGE_SIZE))
            .unwrap();
        blocks.write_bytes(0, EXFAT_IMAGE).unwrap();
        Arc::new(Self {
            blocks,
            observed_bios: Mutex::new(Vec::new()),
        })
    }

    pub(in super::super) fn as_block_device(self: &Arc<Self>) -> Arc<dyn BlockDevice> {
        self.clone()
    }

    pub(in super::super) fn install_root_file(&self, entry_index: usize, name: &str) {
        self.install_directory_entry_set(
            self.validated_mount().boot_region.root_dir_cluster,
            entry_index,
            name,
            FILE_ATTRIBUTE_REGULAR,
            0,
            0,
            false,
        );
    }

    pub(in super::super) fn install_root_file_with_contents(
        &self,
        entry_index: usize,
        name: &str,
        first_cluster: u32,
        contents: &[u8],
    ) {
        let cluster_size = self.root_cluster_size();
        assert!(contents.len() <= cluster_size);
        self.mark_cluster_allocated(first_cluster);
        let mut cluster_bytes = vec![0; cluster_size];
        cluster_bytes[..contents.len()].copy_from_slice(contents);
        self.write_cluster(first_cluster, &cluster_bytes);
        self.install_directory_entry_set(
            self.validated_mount().boot_region.root_dir_cluster,
            entry_index,
            name,
            FILE_ATTRIBUTE_REGULAR,
            first_cluster,
            contents.len(),
            true,
        );
    }

    pub(in super::super) fn install_root_file_with_cluster_chain(
        &self,
        entry_index: usize,
        name: &str,
        first_cluster: u32,
        data_length: usize,
        valid_data_length: usize,
        no_fat_chain: bool,
        clusters: &[u32],
    ) {
        assert!(!clusters.is_empty());
        assert_eq!(clusters[0], first_cluster);
        assert!(valid_data_length <= data_length);

        let cluster_size = self.root_cluster_size();
        let zeroed_cluster = vec![0; cluster_size];
        for cluster in clusters {
            self.mark_cluster_allocated(*cluster);
            self.write_cluster(*cluster, &zeroed_cluster);
        }
        self.install_directory_entry_set(
            self.validated_mount().boot_region.root_dir_cluster,
            entry_index,
            name,
            FILE_ATTRIBUTE_REGULAR,
            first_cluster,
            data_length,
            no_fat_chain,
        );
        self.set_root_stream_extension(entry_index, first_cluster, data_length, valid_data_length);
    }

    pub(in super::super) fn set_root_stream_extension(
        &self,
        entry_index: usize,
        first_cluster: u32,
        data_length: usize,
        valid_data_length: usize,
    ) {
        let secondary_count = usize::from(self.read_root_entries(entry_index, 1)[1]);
        let mut entry_set = self.read_root_entries(entry_index, secondary_count + 1);
        let stream_entry = &mut entry_set[DIRECTORY_ENTRY_SIZE..DIRECTORY_ENTRY_SIZE * 2];
        assert_eq!(stream_entry[0], STREAM_EXTENSION_ENTRY_TYPE);
        let data_length = u64::try_from(data_length).unwrap();
        let valid_data_length = u64::try_from(valid_data_length).unwrap();
        stream_entry[8..16].copy_from_slice(&valid_data_length.to_le_bytes());
        stream_entry[20..24].copy_from_slice(&first_cluster.to_le_bytes());
        stream_entry[24..32].copy_from_slice(&data_length.to_le_bytes());

        let checksum = entry_set_checksum(&entry_set, u8::try_from(secondary_count).unwrap());
        entry_set[2..4].copy_from_slice(&checksum.to_le_bytes());
        self.write_bytes(self.root_entry_offset(entry_index), &entry_set);
    }

    pub(in super::super) fn write_cluster_prefix(&self, cluster: u32, bytes: &[u8]) {
        let cluster_size = self.root_cluster_size();
        assert!(bytes.len() <= cluster_size);
        self.mark_cluster_allocated(cluster);
        let mut cluster_bytes = vec![0; cluster_size];
        cluster_bytes[..bytes.len()].copy_from_slice(bytes);
        self.write_cluster(cluster, &cluster_bytes);
    }

    pub(in super::super) fn set_fat_chain_step(&self, cluster: u32, next_cluster: u32) {
        self.write_bytes(self.fat_entry_offset(cluster), &next_cluster.to_le_bytes());
    }

    pub(in super::super) fn terminate_fat_chain(&self, cluster: u32) {
        self.set_fat_chain_step(cluster, FAT_END_OF_CHAIN);
    }

    pub(in super::super) fn fat_chain_step(&self, cluster: u32) -> u32 {
        let mut entry_bytes = [0u8; 4];
        self.blocks
            .read_bytes(self.fat_entry_offset(cluster), &mut entry_bytes)
            .unwrap();
        u32::from_le_bytes(entry_bytes)
    }

    pub(in super::super) fn install_root_directory(
        &self,
        entry_index: usize,
        name: &str,
        first_cluster: u32,
    ) {
        self.mark_cluster_allocated(first_cluster);
        self.write_cluster(first_cluster, &vec![0; self.root_cluster_size()]);
        self.install_directory_entry_set(
            self.validated_mount().boot_region.root_dir_cluster,
            entry_index,
            name,
            FILE_ATTRIBUTE_DIRECTORY,
            first_cluster,
            self.root_cluster_size(),
            true,
        );
    }

    pub(in super::super) fn install_directory_file(
        &self,
        directory_cluster: u32,
        entry_index: usize,
        name: &str,
        first_cluster: u32,
        data_length: usize,
    ) {
        self.mark_cluster_allocated(first_cluster);
        self.install_directory_entry_set(
            directory_cluster,
            entry_index,
            name,
            FILE_ATTRIBUTE_REGULAR,
            first_cluster,
            data_length,
            true,
        );
    }

    pub(in super::super) fn install_directory_subdirectory(
        &self,
        directory_cluster: u32,
        entry_index: usize,
        name: &str,
        first_cluster: u32,
    ) {
        self.mark_cluster_allocated(first_cluster);
        self.write_cluster(first_cluster, &vec![0; self.root_cluster_size()]);
        self.install_directory_entry_set(
            directory_cluster,
            entry_index,
            name,
            FILE_ATTRIBUTE_DIRECTORY,
            first_cluster,
            self.root_cluster_size(),
            true,
        );
    }

    fn install_directory_entry_set(
        &self,
        directory_cluster: u32,
        entry_index: usize,
        name: &str,
        file_attributes: u16,
        first_cluster: u32,
        data_length: usize,
        no_fat_chain: bool,
    ) {
        let validated_mount = self.validated_mount();
        let entry_offset = self.directory_entry_offset(directory_cluster, entry_index);
        let name_utf16: Vec<u16> = name.encode_utf16().collect();
        let name_entry_count = name_utf16.len().div_ceil(15);
        let secondary_count = name_entry_count.checked_add(1).unwrap();
        let mut entry_set = vec![0u8; (secondary_count + 1) * DIRECTORY_ENTRY_SIZE];

        entry_set[0] = FILE_DIRECTORY_ENTRY_TYPE;
        entry_set[1] = u8::try_from(secondary_count).unwrap();
        entry_set[4..6].copy_from_slice(&file_attributes.to_le_bytes());

        let stream_entry = &mut entry_set[DIRECTORY_ENTRY_SIZE..DIRECTORY_ENTRY_SIZE * 2];
        stream_entry[0] = STREAM_EXTENSION_ENTRY_TYPE;
        stream_entry[1] = if no_fat_chain { 0x03 } else { 0x01 };
        stream_entry[3] = u8::try_from(name_utf16.len()).unwrap();
        stream_entry[4..6].copy_from_slice(
            &validated_mount
                .upcase_table
                .name_hash(&name_utf16)
                .to_le_bytes(),
        );
        let data_length = u64::try_from(data_length).unwrap();
        stream_entry[8..16].copy_from_slice(&data_length.to_le_bytes());
        stream_entry[20..24].copy_from_slice(&first_cluster.to_le_bytes());
        stream_entry[24..32].copy_from_slice(&data_length.to_le_bytes());

        for (name_entry_index, name_chunk) in name_utf16.chunks(15).enumerate() {
            let entry_offset = (name_entry_index + 2) * DIRECTORY_ENTRY_SIZE;
            let name_entry = &mut entry_set[entry_offset..entry_offset + DIRECTORY_ENTRY_SIZE];
            name_entry[0] = FILE_NAME_ENTRY_TYPE;
            for (chunk_index, code_unit) in name_chunk.iter().enumerate() {
                let byte_offset = 2 + chunk_index * 2;
                name_entry[byte_offset..byte_offset + 2].copy_from_slice(&code_unit.to_le_bytes());
            }
        }

        let checksum = entry_set_checksum(&entry_set, u8::try_from(secondary_count).unwrap());
        entry_set[2..4].copy_from_slice(&checksum.to_le_bytes());
        self.write_bytes(entry_offset, &entry_set);
        self.write_bytes(
            self.directory_entry_offset(directory_cluster, entry_index + secondary_count + 1),
            &[END_OF_DIRECTORY_ENTRY_TYPE; DIRECTORY_ENTRY_SIZE],
        );
    }

    pub(in super::super) fn install_root_fractured_entry_set(
        &self,
        entry_index: usize,
        name: &str,
    ) {
        let validated_mount = self.validated_mount();
        let root_entry_offset = self.root_entry_offset(entry_index);
        let name_utf16: Vec<u16> = name.encode_utf16().collect();
        let mut entry_set = vec![0u8; DIRECTORY_ENTRY_SIZE * 3];

        entry_set[0] = FILE_DIRECTORY_ENTRY_TYPE;
        entry_set[1] = 2;

        let stream_entry = &mut entry_set[DIRECTORY_ENTRY_SIZE..DIRECTORY_ENTRY_SIZE * 2];
        stream_entry[0] = STREAM_EXTENSION_ENTRY_TYPE;
        stream_entry[1] = 0x01;
        stream_entry[3] = u8::try_from(name_utf16.len()).unwrap();
        stream_entry[4..6].copy_from_slice(
            &validated_mount
                .upcase_table
                .name_hash(&name_utf16)
                .to_le_bytes(),
        );

        let checksum = entry_set_checksum(&entry_set, 2);
        entry_set[2..4].copy_from_slice(&checksum.to_le_bytes());
        self.write_bytes(root_entry_offset, &entry_set);
    }

    pub(in super::super) fn install_root_unrecognized_critical_entry(&self, entry_index: usize) {
        self.write_bytes(
            self.root_entry_offset(entry_index),
            &[FILE_NAME_ENTRY_TYPE; DIRECTORY_ENTRY_SIZE],
        );
        self.write_bytes(
            self.root_entry_offset(entry_index + 1),
            &[END_OF_DIRECTORY_ENTRY_TYPE; DIRECTORY_ENTRY_SIZE],
        );
    }

    pub(in super::super) fn install_root_unrecognized_benign_entry(&self, entry_index: usize) {
        let mut entry_set = [0u8; DIRECTORY_ENTRY_SIZE];
        entry_set[0] = BENIGN_UNRECOGNIZED_ENTRY_TYPE;
        entry_set[1] = 0;
        let checksum = entry_set_checksum(&entry_set, 0);
        entry_set[2..4].copy_from_slice(&checksum.to_le_bytes());

        self.write_bytes(self.root_entry_offset(entry_index), &entry_set);
        self.write_bytes(
            self.root_entry_offset(entry_index + 1),
            &[END_OF_DIRECTORY_ENTRY_TYPE; DIRECTORY_ENTRY_SIZE],
        );
    }

    pub(in super::super) fn root_directory_offset(&self) -> usize {
        let boot_region = self.validated_mount().boot_region;
        boot_region
            .cluster_offset(boot_region.root_dir_cluster)
            .unwrap()
    }

    pub(in super::super) fn set_volume_flags(&self, flags: u16) {
        self.write_bytes(VOLUME_FLAGS_OFFSET, &flags.to_le_bytes());
    }

    pub(in super::super) fn root_cluster_size(&self) -> usize {
        self.validated_mount().boot_region.cluster_size
    }

    pub(in super::super) fn cluster_offset(&self, cluster: u32) -> usize {
        self.validated_mount().boot_region.cluster_offset(cluster).unwrap()
    }

    pub(in super::super) fn fat_entry_offset(&self, cluster: u32) -> usize {
        let boot_region = self.validated_mount().boot_region;
        let fat_offset = usize::try_from(
            u64::from(boot_region.fat_offset_sectors)
                .checked_mul(u64::try_from(boot_region.sector_size).unwrap())
                .unwrap(),
        )
        .unwrap();
        fat_offset
            .checked_add(
                usize::try_from(cluster)
                    .unwrap()
                    .checked_mul(core::mem::size_of::<u32>())
                    .unwrap(),
            )
            .unwrap()
    }

    pub(in super::super) fn root_directory_entry_capacity(&self) -> usize {
        self.root_cluster_size() / DIRECTORY_ENTRY_SIZE
    }

    pub(in super::super) fn read_cluster(&self, cluster: u32) -> Vec<u8> {
        let boot_region = self.validated_mount().boot_region;
        let mut bytes = vec![0; boot_region.cluster_size];
        self.blocks
            .read_bytes(boot_region.cluster_offset(cluster).unwrap(), &mut bytes)
            .unwrap();
        bytes
    }

    pub(in super::super) fn read_root_entries(
        &self,
        entry_index: usize,
        entry_count: usize,
    ) -> Vec<u8> {
        self.read_directory_entries(
            self.validated_mount().boot_region.root_dir_cluster,
            entry_index,
            entry_count,
        )
    }

    pub(in super::super) fn write_root_entries(&self, entry_index: usize, bytes: &[u8]) {
        self.write_directory_entries(
            self.validated_mount().boot_region.root_dir_cluster,
            entry_index,
            bytes,
        );
    }

    pub(in super::super) fn read_directory_entries(
        &self,
        directory_cluster: u32,
        entry_index: usize,
        entry_count: usize,
    ) -> Vec<u8> {
        let mut bytes = vec![0; entry_count * DIRECTORY_ENTRY_SIZE];
        self.blocks
            .read_bytes(
                self.directory_entry_offset(directory_cluster, entry_index),
                &mut bytes,
            )
            .unwrap();
        bytes
    }

    pub(in super::super) fn write_directory_entries(
        &self,
        directory_cluster: u32,
        entry_index: usize,
        bytes: &[u8],
    ) {
        self.write_bytes(self.directory_entry_offset(directory_cluster, entry_index), bytes);
    }

    pub(in super::super) fn take_observed_bios(&self) -> Vec<ObservedBio> {
        core::mem::take(&mut *self.observed_bios.lock())
    }

    pub(in super::super) fn allocation_bitmap_byte_offset_for_cluster(
        &self,
        cluster: u32,
    ) -> usize {
        let bit_index = usize::try_from(cluster.checked_sub(2).unwrap()).unwrap();
        self.allocation_bitmap_offset() + bit_index / 8
    }

    pub(in super::super) fn is_cluster_allocated(&self, cluster: u32) -> bool {
        let bit_index = usize::try_from(cluster.checked_sub(2).unwrap()).unwrap();
        let byte_offset = self.allocation_bitmap_byte_offset_for_cluster(cluster);
        let mut byte = [0u8; 1];
        self.blocks.read_bytes(byte_offset, &mut byte).unwrap();
        byte[0] & (1 << (bit_index % 8)) != 0
    }

    fn allocation_bitmap_offset(&self) -> usize {
        for entry_index in 0..self.root_directory_entry_capacity() {
            let entry = self.read_root_entries(entry_index, 1);
            if entry[0] == ALLOCATION_BITMAP_ENTRY_TYPE {
                let first_cluster =
                    u32::from_le_bytes([entry[20], entry[21], entry[22], entry[23]]);
                return self
                    .validated_mount()
                    .boot_region
                    .cluster_offset(first_cluster)
                    .unwrap();
            }
        }
        panic!("exFAT lookup test image has no allocation bitmap entry");
    }

    fn mark_cluster_allocated(&self, cluster: u32) {
        let bit_index = usize::try_from(cluster.checked_sub(2).unwrap()).unwrap();
        let byte_offset = self.allocation_bitmap_offset() + bit_index / 8;
        let mut byte = [0u8; 1];
        self.blocks.read_bytes(byte_offset, &mut byte).unwrap();
        byte[0] |= 1 << (bit_index % 8);
        self.write_bytes(byte_offset, &byte);
    }

    fn write_cluster(&self, cluster: u32, bytes: &[u8]) {
        let boot_region = self.validated_mount().boot_region;
        assert_eq!(bytes.len(), boot_region.cluster_size);
        self.write_bytes(boot_region.cluster_offset(cluster).unwrap(), bytes);
    }

    fn validated_mount(&self) -> super::LoadedMountState {
        load_validated_mount(self).unwrap_or_else(|error| {
            panic!(
                "lookup fixture load_validated_mount failed with {:?}; diagnostic gate: {}",
                error,
                super::diagnose_invalid_on_disk_layout_gate(self)
            );
        })
    }

    fn root_entry_offset(&self, entry_index: usize) -> usize {
        self.root_directory_offset()
            .checked_add(entry_index * DIRECTORY_ENTRY_SIZE)
            .unwrap()
    }

    fn directory_entry_offset(&self, directory_cluster: u32, entry_index: usize) -> usize {
        self.validated_mount()
            .boot_region
            .cluster_offset(directory_cluster)
            .unwrap()
            .checked_add(entry_index * DIRECTORY_ENTRY_SIZE)
            .unwrap()
    }

    fn sectors_count(&self) -> usize {
        self.blocks.size() / SECTOR_SIZE
    }

    fn write_bytes(&self, offset: usize, bytes: &[u8]) {
        self.blocks.write_bytes(offset, bytes).unwrap();
    }
}

impl fmt::Debug for ExfatLookupTestDisk {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ExfatLookupTestDisk")
            .field("sectors_count", &self.sectors_count())
            .finish()
    }
}

impl ExfatLookupToggleFailingWriteDisk {
    pub(in super::super) fn new(
        inner: Arc<ExfatLookupTestDisk>,
        fail_offset: usize,
        fail_len: usize,
    ) -> Arc<Self> {
        Arc::new(Self {
            fail_range: fail_offset..fail_offset.checked_add(fail_len).unwrap(),
            fail_writes: AtomicBool::new(false),
            inner,
        })
    }

    pub(in super::super) fn enable_failures(&self) {
        self.fail_writes.store(true, Ordering::Relaxed);
    }

    fn overlaps_failure_range(&self, start: usize, end: usize) -> bool {
        start < self.fail_range.end && self.fail_range.start < end
    }
}

impl ExfatLookupToggleFailingReadDisk {
    pub(in super::super) fn new(
        inner: Arc<ExfatLookupTestDisk>,
        fail_offset: usize,
        fail_len: usize,
    ) -> Arc<Self> {
        Arc::new(Self {
            fail_range: fail_offset..fail_offset.checked_add(fail_len).unwrap(),
            fail_reads: AtomicBool::new(false),
            inner,
        })
    }

    pub(in super::super) fn enable_failures(&self) {
        self.fail_reads.store(true, Ordering::Relaxed);
    }

    fn overlaps_failure_range(&self, start: usize, end: usize) -> bool {
        start < self.fail_range.end && self.fail_range.start < end
    }
}

impl ExfatLookupFlushControlDisk {
    pub(in super::super) fn new(inner: Arc<ExfatLookupTestDisk>) -> Arc<Self> {
        Arc::new(Self {
            block_flush: AtomicBool::new(false),
            fail_flush: AtomicBool::new(false),
            flush_started: AtomicBool::new(false),
            inner,
        })
    }

    pub(in super::super) fn enable_blocking_flush(&self) {
        self.flush_started.store(false, Ordering::Relaxed);
        self.block_flush.store(true, Ordering::Relaxed);
    }

    pub(in super::super) fn release_blocked_flush(&self) {
        self.block_flush.store(false, Ordering::Relaxed);
    }

    pub(in super::super) fn flush_started(&self) -> bool {
        self.flush_started.load(Ordering::Relaxed)
    }

    pub(in super::super) fn enable_flush_failures(&self) {
        self.fail_flush.store(true, Ordering::Relaxed);
    }

    pub(in super::super) fn disable_flush_failures(&self) {
        self.fail_flush.store(false, Ordering::Relaxed);
    }
}

impl fmt::Debug for ExfatLookupToggleFailingWriteDisk {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ExfatLookupToggleFailingWriteDisk")
            .field("fail_range", &self.fail_range)
            .field("sectors_count", &self.inner.sectors_count())
            .finish()
    }
}

impl fmt::Debug for ExfatLookupToggleFailingReadDisk {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ExfatLookupToggleFailingReadDisk")
            .field("fail_range", &self.fail_range)
            .field("sectors_count", &self.inner.sectors_count())
            .finish()
    }
}

impl fmt::Debug for ExfatLookupFlushControlDisk {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ExfatLookupFlushControlDisk")
            .field("block_flush", &self.block_flush.load(Ordering::Relaxed))
            .field("fail_flush", &self.fail_flush.load(Ordering::Relaxed))
            .field("flush_started", &self.flush_started.load(Ordering::Relaxed))
            .field("sectors_count", &self.inner.sectors_count())
            .finish()
    }
}

impl BlockDevice for ExfatLookupTestDisk {
    fn enqueue(&self, bio: SubmittedBio) -> core::result::Result<(), BioEnqueueError> {
        let bio_type = bio.type_();
        self.observed_bios.lock().push(ObservedBio {
            byte_range: bio.sid_range().start.to_offset()..bio.sid_range().end.to_offset(),
            segment_lengths: bio.segments().iter().map(BioSegment::nbytes).collect(),
            type_: bio_type,
        });
        if bio_type == BioType::Flush {
            bio.complete(BioStatus::Complete);
            return Ok(());
        }

        let mut current_offset = bio.sid_range().start.to_offset();
        for segment in bio.segments() {
            let size = match bio_type {
                BioType::Read => segment
                    .inner_dma_slice()
                    .writer()
                    .unwrap()
                    .write(self.blocks.reader().skip(current_offset)),
                BioType::Write => self
                    .blocks
                    .writer()
                    .skip(current_offset)
                    .write(&mut segment.inner_dma_slice().reader().unwrap()),
                _ => 0,
            };
            current_offset += size;
        }
        bio.complete(BioStatus::Complete);
        Ok(())
    }

    fn metadata(&self) -> BlockDeviceMeta {
        BlockDeviceMeta {
            max_nr_segments_per_bio: usize::MAX,
            nr_sectors: self.sectors_count(),
        }
    }

    fn name(&self) -> &str {
        "exfat-lookup-test"
    }

    fn id(&self) -> DeviceId {
        DeviceId::null()
    }
}

impl BlockDevice for ExfatLookupToggleFailingWriteDisk {
    fn enqueue(&self, bio: SubmittedBio) -> core::result::Result<(), BioEnqueueError> {
        let bio_type = bio.type_();
        if bio_type == BioType::Flush {
            bio.complete(BioStatus::Complete);
            return Ok(());
        }

        let mut current_offset = bio.sid_range().start.to_offset();
        for segment in bio.segments() {
            let segment_end = current_offset.checked_add(segment.nbytes()).unwrap();
            if bio_type == BioType::Write
                && self.fail_writes.load(Ordering::Relaxed)
                && self.overlaps_failure_range(current_offset, segment_end)
            {
                bio.complete(BioStatus::IoError);
                return Ok(());
            }

            let size = match bio_type {
                BioType::Read => segment
                    .inner_dma_slice()
                    .writer()
                    .unwrap()
                    .write(self.inner.blocks.reader().skip(current_offset)),
                BioType::Write => self
                    .inner
                    .blocks
                    .writer()
                    .skip(current_offset)
                    .write(&mut segment.inner_dma_slice().reader().unwrap()),
                _ => 0,
            };
            current_offset += size;
        }
        bio.complete(BioStatus::Complete);
        Ok(())
    }

    fn metadata(&self) -> BlockDeviceMeta {
        self.inner.metadata()
    }

    fn name(&self) -> &str {
        "exfat-lookup-failing-write-test"
    }

    fn id(&self) -> DeviceId {
        DeviceId::null()
    }
}

impl BlockDevice for ExfatLookupToggleFailingReadDisk {
    fn enqueue(&self, bio: SubmittedBio) -> core::result::Result<(), BioEnqueueError> {
        let bio_type = bio.type_();
        if bio_type == BioType::Flush {
            bio.complete(BioStatus::Complete);
            return Ok(());
        }

        let mut current_offset = bio.sid_range().start.to_offset();
        for segment in bio.segments() {
            let segment_end = current_offset.checked_add(segment.nbytes()).unwrap();
            if bio_type == BioType::Read
                && self.fail_reads.load(Ordering::Relaxed)
                && self.overlaps_failure_range(current_offset, segment_end)
            {
                bio.complete(BioStatus::IoError);
                return Ok(());
            }

            let size = match bio_type {
                BioType::Read => segment
                    .inner_dma_slice()
                    .writer()
                    .unwrap()
                    .write(self.inner.blocks.reader().skip(current_offset)),
                BioType::Write => self
                    .inner
                    .blocks
                    .writer()
                    .skip(current_offset)
                    .write(&mut segment.inner_dma_slice().reader().unwrap()),
                _ => 0,
            };
            current_offset += size;
        }
        bio.complete(BioStatus::Complete);
        Ok(())
    }

    fn metadata(&self) -> BlockDeviceMeta {
        self.inner.metadata()
    }

    fn name(&self) -> &str {
        "exfat-lookup-failing-read-test"
    }

    fn id(&self) -> DeviceId {
        DeviceId::null()
    }
}

impl BlockDevice for ExfatLookupFlushControlDisk {
    fn enqueue(&self, bio: SubmittedBio) -> core::result::Result<(), BioEnqueueError> {
        let bio_type = bio.type_();
        self.inner.observed_bios.lock().push(ObservedBio {
            byte_range: bio.sid_range().start.to_offset()..bio.sid_range().end.to_offset(),
            segment_lengths: bio.segments().iter().map(BioSegment::nbytes).collect(),
            type_: bio_type,
        });
        if bio_type == BioType::Flush {
            self.flush_started.store(true, Ordering::Relaxed);
            while self.block_flush.load(Ordering::Relaxed) {
                crate::thread::Thread::yield_now();
            }
            let flush_status = if self.fail_flush.load(Ordering::Relaxed) {
                BioStatus::IoError
            } else {
                BioStatus::Complete
            };
            bio.complete(flush_status);
            return Ok(());
        }

        let mut current_offset = bio.sid_range().start.to_offset();
        for segment in bio.segments() {
            let size = match bio_type {
                BioType::Read => segment
                    .inner_dma_slice()
                    .writer()
                    .unwrap()
                    .write(self.inner.blocks.reader().skip(current_offset)),
                BioType::Write => self
                    .inner
                    .blocks
                    .writer()
                    .skip(current_offset)
                    .write(&mut segment.inner_dma_slice().reader().unwrap()),
                _ => 0,
            };
            current_offset += size;
        }
        bio.complete(BioStatus::Complete);
        Ok(())
    }

    fn metadata(&self) -> BlockDeviceMeta {
        self.inner.metadata()
    }

    fn name(&self) -> &str {
        "exfat-lookup-flush-control-test"
    }

    fn id(&self) -> DeviceId {
        DeviceId::null()
    }
}

pub(in super::super) fn entry_set_checksum(entry_set: &[u8], secondary_count: u8) -> u16 {
    let mut checksum = 0u16;
    let number_of_bytes = (usize::from(secondary_count) + 1) * DIRECTORY_ENTRY_SIZE;
    for (index, byte) in entry_set.iter().take(number_of_bytes).enumerate() {
        if index == 2 || index == 3 {
            continue;
        }
        checksum = checksum.rotate_right(1).wrapping_add(u16::from(*byte));
    }
    checksum
}
