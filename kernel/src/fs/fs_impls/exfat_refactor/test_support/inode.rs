// SPDX-License-Identifier: MPL-2.0

use alloc::{sync::Arc, vec, vec::Vec};
use core::fmt;

use aster_block::{
    BlockDevice, BlockDeviceMeta, SECTOR_SIZE,
    bio::{BioEnqueueError, BioStatus, BioType, SubmittedBio},
};
use device_id::DeviceId;
use ostd::mm::{FrameAllocOptions, HasSize, PAGE_SIZE, Segment, VmIo, io::util::HasVmReaderWriter};

use super::load_validated_mount;
use crate::prelude::*;

const DIRECTORY_ENTRY_SIZE: usize = 32;
const BENIGN_UNRECOGNIZED_ENTRY_TYPE: u8 = 0xA0;
const END_OF_DIRECTORY_ENTRY_TYPE: u8 = 0x00;
const FILE_DIRECTORY_ENTRY_TYPE: u8 = 0x85;
const FILE_NAME_ENTRY_TYPE: u8 = 0xC1;
const STREAM_EXTENSION_ENTRY_TYPE: u8 = 0xC0;
static EXFAT_IMAGE: &[u8] = include_bytes!("../../../../../../test/initramfs/build/exfat.img");

pub(in super::super) struct ExfatLookupTestDisk {
    blocks: Segment<()>,
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
        })
    }

    pub(in super::super) fn as_block_device(self: &Arc<Self>) -> Arc<dyn BlockDevice> {
        self.clone()
    }

    pub(in super::super) fn install_root_file(&self, entry_index: usize, name: &str) {
        let validated_mount = self.validated_mount();
        let root_entry_offset = self.root_entry_offset(entry_index);
        let name_utf16: Vec<u16> = name.encode_utf16().collect();
        let name_entry_count = name_utf16.len().div_ceil(15);
        let secondary_count = name_entry_count.checked_add(1).unwrap();
        let mut entry_set = vec![0u8; (secondary_count + 1) * DIRECTORY_ENTRY_SIZE];

        entry_set[0] = FILE_DIRECTORY_ENTRY_TYPE;
        entry_set[1] = u8::try_from(secondary_count).unwrap();

        let stream_entry = &mut entry_set[DIRECTORY_ENTRY_SIZE..DIRECTORY_ENTRY_SIZE * 2];
        stream_entry[0] = STREAM_EXTENSION_ENTRY_TYPE;
        stream_entry[1] = 0x01;
        stream_entry[3] = u8::try_from(name_utf16.len()).unwrap();
        stream_entry[4..6]
            .copy_from_slice(&validated_mount.upcase_table.name_hash(&name_utf16).to_le_bytes());

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
        self.write_bytes(root_entry_offset, &entry_set);
        self.write_bytes(
            self.root_entry_offset(entry_index + secondary_count + 1),
            &[END_OF_DIRECTORY_ENTRY_TYPE; DIRECTORY_ENTRY_SIZE],
        );
    }

    pub(in super::super) fn install_root_fractured_entry_set(&self, entry_index: usize, name: &str) {
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
        stream_entry[4..6]
            .copy_from_slice(&validated_mount.upcase_table.name_hash(&name_utf16).to_le_bytes());

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
        boot_region.cluster_offset(boot_region.root_dir_cluster).unwrap()
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

impl BlockDevice for ExfatLookupTestDisk {
    fn enqueue(&self, bio: SubmittedBio) -> core::result::Result<(), BioEnqueueError> {
        let bio_type = bio.type_();
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

fn entry_set_checksum(entry_set: &[u8], secondary_count: u8) -> u16 {
    let mut checksum = 0u16;
    let number_of_bytes = (usize::from(secondary_count) + 1) * DIRECTORY_ENTRY_SIZE;
    for (index, byte) in entry_set.iter().take(number_of_bytes).enumerate() {
        if index == 2 || index == 3 {
            continue;
        }
        checksum = ((checksum & 1) << 15) + (checksum >> 1) + u16::from(*byte);
    }
    checksum
}
