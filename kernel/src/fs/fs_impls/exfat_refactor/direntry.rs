// SPDX-License-Identifier: MPL-2.0

// This module is the exFAT-local byte-backed directory-entry scan and authoring
// layer. On-disk entry-shape rules live here; higher-level mutation policy does
// not.

use alloc::{vec, vec::Vec};

use super::{
    boot::BootRegion, invalid_on_disk_layout, invalid_operation_input, upcase::UpcaseTable,
};
use crate::{fs::file::InodeType, prelude::*};

pub(super) const DIRECTORY_ENTRY_SIZE: usize = 32;
pub(super) const FILE_ATTRIBUTE_READ_ONLY: u16 = 0x0001;
pub(super) const FILE_ATTRIBUTE_DIRECTORY: u16 = 0x0010;
const CREATE_TIMESTAMP_OFFSET: usize = 8;
const LAST_MODIFIED_TIMESTAMP_OFFSET: usize = 12;
const LAST_ACCESSED_TIMESTAMP_OFFSET: usize = 16;
const CREATE_10MS_INCREMENT_OFFSET: usize = 20;
const LAST_MODIFIED_10MS_INCREMENT_OFFSET: usize = 21;
const CREATE_UTC_OFFSET_OFFSET: usize = 22;
const LAST_MODIFIED_UTC_OFFSET_OFFSET: usize = 23;
const LAST_ACCESSED_UTC_OFFSET_OFFSET: usize = 24;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct DirectoryEntrySlotRange {
    entry_count: usize,
    first_entry_index: usize,
}

impl DirectoryEntrySlotRange {
    pub(super) fn new(
        first_entry_index: usize,
        entry_count: usize,
    ) -> Result<Self> {
        if entry_count == 0 {
            return Err(invalid_on_disk_layout());
        }
        first_entry_index
            .checked_add(entry_count)
            .ok_or(invalid_on_disk_layout())?;
        Ok(Self {
            entry_count,
            first_entry_index,
        })
    }

    pub(super) fn entry_count(self) -> usize {
        self.entry_count
    }

    pub(super) fn first_entry_index(self) -> usize {
        self.first_entry_index
    }

    pub(super) fn next_entry_index(self) -> Result<usize> {
        self.first_entry_index
            .checked_add(self.entry_count)
            .ok_or(invalid_on_disk_layout())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum DirectoryEntryAnomalyKind {
    BenignUnrecognizedEntrySet,
    BrokenEntrySet,
    CriticalUnrecognizedEntrySet,
    UnexpectedSecondaryEntry,
}

#[derive(Clone, Copy)]
pub(super) struct FileEntryTimestamp {
    ten_ms_increment: Option<u8>,
    timestamp_bytes: [u8; 4],
    utc_offset_byte: u8,
}

impl FileEntryTimestamp {
    pub(super) fn new(
        timestamp_bytes: [u8; 4],
        ten_ms_increment: Option<u8>,
        utc_offset_byte: u8,
    ) -> Self {
        Self {
            ten_ms_increment,
            timestamp_bytes,
            utc_offset_byte,
        }
    }

    pub(super) fn ten_ms_increment(self) -> Option<u8> {
        self.ten_ms_increment
    }

    pub(super) fn timestamp_bytes(self) -> [u8; 4] {
        self.timestamp_bytes
    }

    pub(super) fn utc_offset_byte(self) -> u8 {
        self.utc_offset_byte
    }
}

#[derive(Clone, Copy)]
pub(super) struct FileEntryClusterMap {
    data_length: u64,
    first_cluster: u32,
    no_fat_chain: bool,
    valid_data_length: u64,
}

impl FileEntryClusterMap {
    pub(super) fn new(
        first_cluster: u32,
        data_length: u64,
        valid_data_length: u64,
        no_fat_chain: bool,
    ) -> Result<Self> {
        if valid_data_length > data_length {
            return Err(invalid_on_disk_layout());
        }
        Ok(Self {
            data_length,
            first_cluster,
            no_fat_chain,
            valid_data_length,
        })
    }

    pub(super) fn data_length(self) -> u64 {
        self.data_length
    }

    pub(super) fn first_cluster(self) -> u32 {
        self.first_cluster
    }

    pub(super) fn no_fat_chain(self) -> bool {
        self.no_fat_chain
    }

    pub(super) fn valid_data_length(self) -> u64 {
        self.valid_data_length
    }
}

// Borrowed read-only view produced only from one validated file entry set. This
// is not a general mutable entry buffer.
#[derive(Clone, Copy)]
pub(super) struct FileEntrySetView<'a> {
    entry_set: &'a [u8],
    primary_entry: &'a [u8],
    secondary_count: usize,
    slot_range: DirectoryEntrySlotRange,
    stream_entry: &'a [u8],
}

impl FileEntrySetView<'_> {
    pub(super) fn child_metadata(
        self,
        boot_region: &BootRegion,
    ) -> Result<(InodeType, u32, usize, bool)> {
        file_entry_child_metadata(self.primary_entry, self.stream_entry, boot_region)
    }

    pub(super) fn name(self) -> Result<Vec<u16>> {
        file_name(self.entry_set, self.secondary_count, self.stream_entry)
    }

    pub(super) fn slot_range(self) -> DirectoryEntrySlotRange {
        self.slot_range
    }

    pub(super) fn file_attributes(self) -> u16 {
        u16::from_le_bytes([
            self.primary_entry[FILE_ATTRIBUTES_OFFSET],
            self.primary_entry[FILE_ATTRIBUTES_OFFSET + 1],
        ])
    }

    pub(super) fn is_directory(self) -> bool {
        self.file_attributes() & FILE_ATTRIBUTE_DIRECTORY != 0
    }

    pub(super) fn is_read_only(self) -> bool {
        self.file_attributes() & FILE_ATTRIBUTE_READ_ONLY != 0
    }

    pub(super) fn create_timestamp(self) -> FileEntryTimestamp {
        FileEntryTimestamp::new(
            [
                self.primary_entry[CREATE_TIMESTAMP_OFFSET],
                self.primary_entry[CREATE_TIMESTAMP_OFFSET + 1],
                self.primary_entry[CREATE_TIMESTAMP_OFFSET + 2],
                self.primary_entry[CREATE_TIMESTAMP_OFFSET + 3],
            ],
            Some(self.primary_entry[CREATE_10MS_INCREMENT_OFFSET]),
            self.primary_entry[CREATE_UTC_OFFSET_OFFSET],
        )
    }

    pub(super) fn last_modified_timestamp(self) -> FileEntryTimestamp {
        FileEntryTimestamp::new(
            [
                self.primary_entry[LAST_MODIFIED_TIMESTAMP_OFFSET],
                self.primary_entry[LAST_MODIFIED_TIMESTAMP_OFFSET + 1],
                self.primary_entry[LAST_MODIFIED_TIMESTAMP_OFFSET + 2],
                self.primary_entry[LAST_MODIFIED_TIMESTAMP_OFFSET + 3],
            ],
            Some(self.primary_entry[LAST_MODIFIED_10MS_INCREMENT_OFFSET]),
            self.primary_entry[LAST_MODIFIED_UTC_OFFSET_OFFSET],
        )
    }

    pub(super) fn last_accessed_timestamp(self) -> FileEntryTimestamp {
        FileEntryTimestamp::new(
            [
                self.primary_entry[LAST_ACCESSED_TIMESTAMP_OFFSET],
                self.primary_entry[LAST_ACCESSED_TIMESTAMP_OFFSET + 1],
                self.primary_entry[LAST_ACCESSED_TIMESTAMP_OFFSET + 2],
                self.primary_entry[LAST_ACCESSED_TIMESTAMP_OFFSET + 3],
            ],
            None,
            self.primary_entry[LAST_ACCESSED_UTC_OFFSET_OFFSET],
        )
    }

    pub(super) fn cluster_map(self) -> Result<FileEntryClusterMap> {
        FileEntryClusterMap::new(
            u32::from_le_bytes([
                self.stream_entry[STREAM_FIRST_CLUSTER_OFFSET],
                self.stream_entry[STREAM_FIRST_CLUSTER_OFFSET + 1],
                self.stream_entry[STREAM_FIRST_CLUSTER_OFFSET + 2],
                self.stream_entry[STREAM_FIRST_CLUSTER_OFFSET + 3],
            ]),
            u64::from_le_bytes([
                self.stream_entry[STREAM_DATA_LENGTH_OFFSET],
                self.stream_entry[STREAM_DATA_LENGTH_OFFSET + 1],
                self.stream_entry[STREAM_DATA_LENGTH_OFFSET + 2],
                self.stream_entry[STREAM_DATA_LENGTH_OFFSET + 3],
                self.stream_entry[STREAM_DATA_LENGTH_OFFSET + 4],
                self.stream_entry[STREAM_DATA_LENGTH_OFFSET + 5],
                self.stream_entry[STREAM_DATA_LENGTH_OFFSET + 6],
                self.stream_entry[STREAM_DATA_LENGTH_OFFSET + 7],
            ]),
            u64::from_le_bytes([
                self.stream_entry[STREAM_VALID_DATA_LENGTH_OFFSET],
                self.stream_entry[STREAM_VALID_DATA_LENGTH_OFFSET + 1],
                self.stream_entry[STREAM_VALID_DATA_LENGTH_OFFSET + 2],
                self.stream_entry[STREAM_VALID_DATA_LENGTH_OFFSET + 3],
                self.stream_entry[STREAM_VALID_DATA_LENGTH_OFFSET + 4],
                self.stream_entry[STREAM_VALID_DATA_LENGTH_OFFSET + 5],
                self.stream_entry[STREAM_VALID_DATA_LENGTH_OFFSET + 6],
                self.stream_entry[STREAM_VALID_DATA_LENGTH_OFFSET + 7],
            ]),
            self.stream_entry[STREAM_FLAGS_OFFSET] & 0x02 != 0,
        )
    }

    pub(super) fn republished(self) -> RepublishedFileEntrySet {
        RepublishedFileEntrySet {
            entry_set: self.entry_set.to_vec(),
            secondary_count: self.secondary_count,
        }
    }

    pub(super) fn stored_name_hash(self) -> u16 {
        u16::from_le_bytes([
            self.stream_entry[STREAM_NAME_HASH_OFFSET],
            self.stream_entry[STREAM_NAME_HASH_OFFSET + 1],
        ])
    }
}

pub(super) struct RepublishedFileEntrySet {
    entry_set: Vec<u8>,
    secondary_count: usize,
}

impl RepublishedFileEntrySet {
    pub(super) fn set_file_attributes(&mut self, file_attributes: u16) {
        self.entry_set[FILE_ATTRIBUTES_OFFSET..FILE_ATTRIBUTES_OFFSET + 2]
            .copy_from_slice(&file_attributes.to_le_bytes());
    }

    pub(super) fn set_last_accessed_timestamp(&mut self, timestamp: FileEntryTimestamp) {
        self.entry_set[LAST_ACCESSED_TIMESTAMP_OFFSET..LAST_ACCESSED_TIMESTAMP_OFFSET + 4]
            .copy_from_slice(&timestamp.timestamp_bytes());
        self.entry_set[LAST_ACCESSED_UTC_OFFSET_OFFSET] = timestamp.utc_offset_byte();
    }

    pub(super) fn set_last_modified_timestamp(&mut self, timestamp: FileEntryTimestamp) {
        self.entry_set[LAST_MODIFIED_TIMESTAMP_OFFSET..LAST_MODIFIED_TIMESTAMP_OFFSET + 4]
            .copy_from_slice(&timestamp.timestamp_bytes());
        self.entry_set[LAST_MODIFIED_10MS_INCREMENT_OFFSET] =
            timestamp.ten_ms_increment().unwrap_or(0);
        self.entry_set[LAST_MODIFIED_UTC_OFFSET_OFFSET] = timestamp.utc_offset_byte();
    }

    pub(super) fn set_cluster_map(&mut self, cluster_map: FileEntryClusterMap) {
        self.entry_set[DIRECTORY_ENTRY_SIZE + STREAM_FLAGS_OFFSET] = if cluster_map.no_fat_chain() {
            0x03
        } else {
            0x01
        };
        self.entry_set[DIRECTORY_ENTRY_SIZE + STREAM_VALID_DATA_LENGTH_OFFSET
            ..DIRECTORY_ENTRY_SIZE + STREAM_VALID_DATA_LENGTH_OFFSET + 8]
            .copy_from_slice(&cluster_map.valid_data_length().to_le_bytes());
        self.entry_set[DIRECTORY_ENTRY_SIZE + STREAM_FIRST_CLUSTER_OFFSET
            ..DIRECTORY_ENTRY_SIZE + STREAM_FIRST_CLUSTER_OFFSET + 4]
            .copy_from_slice(&cluster_map.first_cluster().to_le_bytes());
        self.entry_set[DIRECTORY_ENTRY_SIZE + STREAM_DATA_LENGTH_OFFSET
            ..DIRECTORY_ENTRY_SIZE + STREAM_DATA_LENGTH_OFFSET + 8]
            .copy_from_slice(&cluster_map.data_length().to_le_bytes());
    }

    fn set_name_fields(
        &mut self,
        name: &[u16],
        name_hash: u16,
    ) -> Result<()> {
        let current_name_entry_count =
            usize::from(self.entry_set[DIRECTORY_ENTRY_SIZE + STREAM_NAME_LENGTH_OFFSET])
                .div_ceil(15);
        let requested_name_entry_count = file_entry_set_entry_count(name.len())?
            .checked_sub(2)
            .ok_or(invalid_operation_input())?;
        if requested_name_entry_count != current_name_entry_count {
            return Err(invalid_operation_input());
        }

        self.entry_set[DIRECTORY_ENTRY_SIZE + STREAM_NAME_LENGTH_OFFSET] =
            u8::try_from(name.len()).map_err(|_| invalid_operation_input())?;
        self.entry_set[DIRECTORY_ENTRY_SIZE + STREAM_NAME_HASH_OFFSET
            ..DIRECTORY_ENTRY_SIZE + STREAM_NAME_HASH_OFFSET + 2]
            .copy_from_slice(&name_hash.to_le_bytes());

        for name_entry_index in 0..current_name_entry_count {
            let name_entry_offset = (name_entry_index + 2)
                .checked_mul(DIRECTORY_ENTRY_SIZE)
                .ok_or(invalid_on_disk_layout())?;
            self.entry_set[name_entry_offset + 2..name_entry_offset + DIRECTORY_ENTRY_SIZE].fill(0);
        }

        for (name_entry_index, name_chunk) in name.chunks(15).enumerate() {
            let name_entry_offset = (name_entry_index + 2)
                .checked_mul(DIRECTORY_ENTRY_SIZE)
                .ok_or(invalid_operation_input())?;
            for (name_code_unit_index, name_code_unit) in name_chunk.iter().enumerate() {
                let code_unit_offset = name_entry_offset
                    .checked_add(2)
                    .and_then(|offset| offset.checked_add(name_code_unit_index * 2))
                    .ok_or(invalid_operation_input())?;
                self.entry_set[code_unit_offset..code_unit_offset + 2]
                    .copy_from_slice(&name_code_unit.to_le_bytes());
            }
        }
        Ok(())
    }

    pub(super) fn into_bytes(mut self) -> Vec<u8> {
        let checksum = entry_set_checksum(&self.entry_set, self.secondary_count);
        self.entry_set[2..4].copy_from_slice(&checksum.to_le_bytes());
        self.entry_set
    }
}

pub(super) fn file_entry_set_entry_count(
    name_length: usize,
) -> Result<usize> {
    if name_length == 0 || name_length > UpcaseTable::NAME_MAX {
        return Err(invalid_operation_input());
    }
    name_length
        .div_ceil(15)
        .checked_add(2)
        .ok_or(invalid_operation_input())
}

pub(super) fn encode_file_entry_set(
    name: &[u16],
    name_hash: u16,
    inode_type: InodeType,
    first_cluster: u32,
    data_length: usize,
    no_fat_chain: bool,
) -> Result<Vec<u8>> {
    let entry_count = file_entry_set_entry_count(name.len())?;
    let secondary_count = entry_count
        .checked_sub(1)
        .ok_or(invalid_operation_input())?;
    let secondary_count =
        u8::try_from(secondary_count).map_err(|_| invalid_operation_input())?;
    let entry_set_len = entry_count
        .checked_mul(DIRECTORY_ENTRY_SIZE)
        .ok_or(invalid_operation_input())?;
    let mut entry_set = vec![0; entry_set_len];

    entry_set[0] = FILE_DIRECTORY_ENTRY_TYPE;
    entry_set[1] = secondary_count;
    let file_attributes = match inode_type {
        InodeType::Dir => FILE_ATTRIBUTE_DIRECTORY,
        InodeType::File => 0x0020,
        _ => return Err(invalid_operation_input()),
    };
    entry_set[4..6].copy_from_slice(&file_attributes.to_le_bytes());

    let stream_entry_offset = DIRECTORY_ENTRY_SIZE;
    entry_set[stream_entry_offset] = STREAM_EXTENSION_ENTRY_TYPE;
    entry_set[stream_entry_offset + 1] = if no_fat_chain { 0x03 } else { 0x01 };
    entry_set[stream_entry_offset + 3] =
        u8::try_from(name.len()).map_err(|_| invalid_operation_input())?;
    entry_set[stream_entry_offset + 4..stream_entry_offset + 6]
        .copy_from_slice(&name_hash.to_le_bytes());
    let data_length =
        u64::try_from(data_length).map_err(|_| invalid_operation_input())?;
    entry_set[stream_entry_offset + 8..stream_entry_offset + 16]
        .copy_from_slice(&data_length.to_le_bytes());
    entry_set[stream_entry_offset + 20..stream_entry_offset + 24]
        .copy_from_slice(&first_cluster.to_le_bytes());
    entry_set[stream_entry_offset + 24..stream_entry_offset + 32]
        .copy_from_slice(&data_length.to_le_bytes());

    for (name_entry_index, name_chunk) in name.chunks(15).enumerate() {
        let name_entry_offset = (name_entry_index + 2)
            .checked_mul(DIRECTORY_ENTRY_SIZE)
            .ok_or(invalid_operation_input())?;
        entry_set[name_entry_offset] = FILE_NAME_ENTRY_TYPE;
        for (name_code_unit_index, name_code_unit) in name_chunk.iter().enumerate() {
            let code_unit_offset = name_entry_offset
                .checked_add(2)
                .and_then(|offset| offset.checked_add(name_code_unit_index * 2))
                .ok_or(invalid_operation_input())?;
            entry_set[code_unit_offset..code_unit_offset + 2]
                .copy_from_slice(&name_code_unit.to_le_bytes());
        }
    }

    let checksum = entry_set_checksum(&entry_set, usize::from(secondary_count));
    entry_set[2..4].copy_from_slice(&checksum.to_le_bytes());
    Ok(entry_set)
}

// Slot-aligned writable directory-entry bytes reserved by the owner for
// invalidation, staging, or pre-publication cleanup. This does not prove the
// bytes are a validated published file entry set.
pub(super) struct WritableDirectoryEntrySlotSpan<'a> {
    slot_span: &'a mut [u8],
}

impl<'a> WritableDirectoryEntrySlotSpan<'a> {
    pub(super) fn new(
        slot_range: DirectoryEntrySlotRange,
        slot_span: &'a mut [u8],
    ) -> Result<Self> {
        let expected_len = slot_range
            .entry_count()
            .checked_mul(DIRECTORY_ENTRY_SIZE)
            .ok_or(invalid_on_disk_layout())?;
        if slot_span.is_empty()
            || slot_span.len() != expected_len
            || slot_span.len() % DIRECTORY_ENTRY_SIZE != 0
        {
            return Err(invalid_on_disk_layout());
        }
        Ok(Self { slot_span })
    }

    pub(super) fn bytes_mut(&mut self) -> &mut [u8] {
        self.slot_span
    }
}

pub(super) fn invalidate_entry_set(
    slot_span: &mut WritableDirectoryEntrySlotSpan<'_>,
) -> Result<()> {
    for entry in slot_span.bytes_mut().chunks_exact_mut(DIRECTORY_ENTRY_SIZE) {
        entry[0] &= !ENTRY_TYPE_IN_USE_BIT;
    }
    Ok(())
}

pub(super) fn renamed_entry_set(
    source_entry_set: FileEntrySetView<'_>,
    name: &[u16],
    name_hash: u16,
) -> Result<Vec<u8>> {
    let entry_count = file_entry_set_entry_count(name.len())?;
    let new_name_entry_count = entry_count
        .checked_sub(2)
        .ok_or(invalid_operation_input())?;
    let current_name_entry_count =
        usize::from(source_entry_set.stream_entry[STREAM_NAME_LENGTH_OFFSET]).div_ceil(15);
    if new_name_entry_count == current_name_entry_count {
        let mut renamed_entry_set = source_entry_set.republished();
        renamed_entry_set.set_name_fields(name, name_hash)?;
        return Ok(renamed_entry_set.into_bytes());
    }
    let required_secondary_count = current_name_entry_count
        .checked_add(1)
        .ok_or(invalid_on_disk_layout())?;
    let trailing_secondary_count = source_entry_set
        .secondary_count
        .checked_sub(required_secondary_count)
        .ok_or(invalid_on_disk_layout())?;
    let secondary_count = new_name_entry_count
        .checked_add(1)
        .and_then(|count| count.checked_add(trailing_secondary_count))
        .ok_or(invalid_operation_input())?;
    let secondary_count =
        u8::try_from(secondary_count).map_err(|_| invalid_operation_input())?;
    let entry_set_len = usize::from(secondary_count)
        .checked_add(1)
        .and_then(|entry_count| entry_count.checked_mul(DIRECTORY_ENTRY_SIZE))
        .ok_or(invalid_operation_input())?;
    let mut renamed_entry_set = vec![0; entry_set_len];
    renamed_entry_set[..DIRECTORY_ENTRY_SIZE].copy_from_slice(source_entry_set.primary_entry);
    renamed_entry_set[DIRECTORY_ENTRY_SIZE..DIRECTORY_ENTRY_SIZE * 2]
        .copy_from_slice(source_entry_set.stream_entry);
    renamed_entry_set[1] = secondary_count;
    renamed_entry_set[DIRECTORY_ENTRY_SIZE + STREAM_NAME_LENGTH_OFFSET] =
        u8::try_from(name.len()).map_err(|_| invalid_operation_input())?;
    renamed_entry_set[DIRECTORY_ENTRY_SIZE + STREAM_NAME_HASH_OFFSET
        ..DIRECTORY_ENTRY_SIZE + STREAM_NAME_HASH_OFFSET + 2]
        .copy_from_slice(&name_hash.to_le_bytes());

    for (name_entry_index, name_chunk) in name.chunks(15).enumerate() {
        let name_entry_offset = (name_entry_index + 2)
            .checked_mul(DIRECTORY_ENTRY_SIZE)
            .ok_or(invalid_operation_input())?;
        renamed_entry_set[name_entry_offset] = FILE_NAME_ENTRY_TYPE;
        for (name_code_unit_index, name_code_unit) in name_chunk.iter().enumerate() {
            let code_unit_offset = name_entry_offset
                .checked_add(2)
                .and_then(|offset| offset.checked_add(name_code_unit_index * 2))
                .ok_or(invalid_operation_input())?;
            renamed_entry_set[code_unit_offset..code_unit_offset + 2]
                .copy_from_slice(&name_code_unit.to_le_bytes());
        }
    }

    let trailing_source_offset = (current_name_entry_count + 2)
        .checked_mul(DIRECTORY_ENTRY_SIZE)
        .ok_or(invalid_on_disk_layout())?;
    let trailing_destination_offset = (new_name_entry_count + 2)
        .checked_mul(DIRECTORY_ENTRY_SIZE)
        .ok_or(invalid_on_disk_layout())?;
    renamed_entry_set[trailing_destination_offset..]
        .copy_from_slice(&source_entry_set.entry_set[trailing_source_offset..]);

    let checksum = entry_set_checksum(&renamed_entry_set, usize::from(secondary_count));
    renamed_entry_set[2..4].copy_from_slice(&checksum.to_le_bytes());
    Ok(renamed_entry_set)
}

// Scan result category, not a write-side capability object.
#[derive(Clone, Copy)]
pub(super) enum ScannedDirectoryEntry<'a> {
    Anomaly {
        kind: DirectoryEntryAnomalyKind,
        slot_range: DirectoryEntrySlotRange,
    },
    EndOfDirectory {
        entry_index: usize,
    },
    File(FileEntrySetView<'a>),
    Vacant(DirectoryEntrySlotRange),
}

const END_OF_DIRECTORY_ENTRY_TYPE: u8 = 0x00;
const ALLOCATION_BITMAP_ENTRY_TYPE: u8 = 0x81;
const UPCASE_TABLE_ENTRY_TYPE: u8 = 0x82;
const VOLUME_LABEL_ENTRY_TYPE: u8 = 0x83;
const VOLUME_GUID_ENTRY_TYPE: u8 = 0xA0;
const FILE_DIRECTORY_ENTRY_TYPE: u8 = 0x85;
const STREAM_EXTENSION_ENTRY_TYPE: u8 = 0xC0;
const FILE_NAME_ENTRY_TYPE: u8 = 0xC1;
const ENTRY_TYPE_IMPORTANCE_BIT: u8 = 0x20;
const ENTRY_TYPE_CATEGORY_BIT: u8 = 0x40;
const ENTRY_TYPE_IN_USE_BIT: u8 = 0x80;
const FILE_ATTRIBUTES_OFFSET: usize = 4;
const STREAM_FLAGS_OFFSET: usize = 1;
const STREAM_NAME_LENGTH_OFFSET: usize = 3;
const STREAM_NAME_HASH_OFFSET: usize = 4;
const STREAM_VALID_DATA_LENGTH_OFFSET: usize = 8;
const STREAM_FIRST_CLUSTER_OFFSET: usize = 20;
const STREAM_DATA_LENGTH_OFFSET: usize = 24;

pub(super) fn scan_directory_entry(
    is_root_directory: bool,
    directory_bytes: &[u8],
    mut entry_index: usize,
) -> Result<ScannedDirectoryEntry<'_>> {
    loop {
        let slot_range = DirectoryEntrySlotRange::new(entry_index, 1)?;
        let entry_offset = entry_index
            .checked_mul(DIRECTORY_ENTRY_SIZE)
            .ok_or(invalid_on_disk_layout())?;
        let entry_end = entry_offset
            .checked_add(DIRECTORY_ENTRY_SIZE)
            .ok_or(invalid_on_disk_layout())?;
        let Some(entry) = directory_bytes.get(entry_offset..entry_end) else {
            return Ok(ScannedDirectoryEntry::EndOfDirectory { entry_index });
        };

        match entry[0] {
            END_OF_DIRECTORY_ENTRY_TYPE => {
                return Ok(ScannedDirectoryEntry::EndOfDirectory { entry_index });
            }
            0x01..=0x7F => return Ok(ScannedDirectoryEntry::Vacant(slot_range)),
            FILE_DIRECTORY_ENTRY_TYPE => {
                return scan_file_entry_set(directory_bytes, entry_index, entry_offset, entry);
            }
            entry_type => {
                if entry_type & ENTRY_TYPE_IN_USE_BIT == 0 {
                    return Ok(ScannedDirectoryEntry::Vacant(slot_range));
                }

                let is_root_metadata = matches!(
                    entry_type,
                    ALLOCATION_BITMAP_ENTRY_TYPE
                        | UPCASE_TABLE_ENTRY_TYPE
                        | VOLUME_LABEL_ENTRY_TYPE
                        | VOLUME_GUID_ENTRY_TYPE
                );
                if is_root_directory && is_root_metadata {
                    entry_index = entry_index
                        .checked_add(1)
                        .ok_or(invalid_on_disk_layout())?;
                    continue;
                }

                if entry_type & ENTRY_TYPE_CATEGORY_BIT != 0 {
                    return Ok(ScannedDirectoryEntry::Anomaly {
                        kind: DirectoryEntryAnomalyKind::UnexpectedSecondaryEntry,
                        slot_range,
                    });
                }

                return scan_unrecognized_entry_set(
                    directory_bytes,
                    entry_index,
                    entry_offset,
                    entry,
                    entry_type,
                );
            }
        }
    }
}

fn scan_file_entry_set<'a>(
    directory_bytes: &'a [u8],
    entry_index: usize,
    entry_offset: usize,
    primary_entry: &'a [u8],
) -> Result<ScannedDirectoryEntry<'a>> {
    let secondary_count = usize::from(primary_entry[1]);
    let slot_range = DirectoryEntrySlotRange::new(
        entry_index,
        secondary_count
            .checked_add(1)
            .ok_or(invalid_on_disk_layout())?,
    )?;
    let expected_checksum = u16::from_le_bytes([primary_entry[2], primary_entry[3]]);
    let Ok(entry_set) = validated_file_entry_set(
        directory_bytes,
        entry_offset,
        secondary_count,
        expected_checksum,
    ) else {
        return Ok(ScannedDirectoryEntry::Anomaly {
            kind: DirectoryEntryAnomalyKind::BrokenEntrySet,
            slot_range,
        });
    };
    let Ok(stream_entry) = file_stream_entry(entry_set) else {
        return Ok(ScannedDirectoryEntry::Anomaly {
            kind: DirectoryEntryAnomalyKind::BrokenEntrySet,
            slot_range,
        });
    };
    if file_name(entry_set, secondary_count, stream_entry).is_err() {
        return Ok(ScannedDirectoryEntry::Anomaly {
            kind: DirectoryEntryAnomalyKind::BrokenEntrySet,
            slot_range,
        });
    }
    Ok(ScannedDirectoryEntry::File(FileEntrySetView {
        entry_set,
        primary_entry,
        secondary_count,
        slot_range,
        stream_entry,
    }))
}

fn scan_unrecognized_entry_set<'a>(
    directory_bytes: &'a [u8],
    entry_index: usize,
    entry_offset: usize,
    primary_entry: &[u8],
    entry_type: u8,
) -> Result<ScannedDirectoryEntry<'a>> {
    let secondary_count = usize::from(primary_entry[1]);
    let slot_range = DirectoryEntrySlotRange::new(
        entry_index,
        secondary_count
            .checked_add(1)
            .ok_or(invalid_on_disk_layout())?,
    )?;
    let expected_checksum = u16::from_le_bytes([primary_entry[2], primary_entry[3]]);
    if validated_file_entry_set(
        directory_bytes,
        entry_offset,
        secondary_count,
        expected_checksum,
    )
    .is_err()
    {
        return Ok(ScannedDirectoryEntry::Anomaly {
            kind: DirectoryEntryAnomalyKind::BrokenEntrySet,
            slot_range,
        });
    }

    let kind = if entry_type & ENTRY_TYPE_IMPORTANCE_BIT == 0 {
        DirectoryEntryAnomalyKind::CriticalUnrecognizedEntrySet
    } else {
        DirectoryEntryAnomalyKind::BenignUnrecognizedEntrySet
    };
    Ok(ScannedDirectoryEntry::Anomaly { kind, slot_range })
}

fn validated_file_entry_set(
    directory_bytes: &[u8],
    entry_offset: usize,
    secondary_count: usize,
    expected_checksum: u16,
) -> Result<&[u8]> {
    let entry_set_len = secondary_count
        .checked_add(1)
        .and_then(|entries| entries.checked_mul(DIRECTORY_ENTRY_SIZE))
        .ok_or(invalid_on_disk_layout())?;
    let entry_set_end = entry_offset
        .checked_add(entry_set_len)
        .ok_or(invalid_on_disk_layout())?;
    let entry_set = directory_bytes
        .get(entry_offset..entry_set_end)
        .ok_or(invalid_on_disk_layout())?;
    if entry_set_checksum(entry_set, secondary_count) != expected_checksum {
        return Err(invalid_on_disk_layout());
    }
    Ok(entry_set)
}

fn file_stream_entry(entry_set: &[u8]) -> Result<&[u8]> {
    let stream_entry = entry_set
        .get(DIRECTORY_ENTRY_SIZE..DIRECTORY_ENTRY_SIZE * 2)
        .ok_or(invalid_on_disk_layout())?;
    if stream_entry[0] != STREAM_EXTENSION_ENTRY_TYPE {
        return Err(invalid_on_disk_layout());
    }
    if stream_entry[1] & 0x01 == 0 {
        return Err(invalid_on_disk_layout());
    }
    Ok(stream_entry)
}

fn file_name(
    entry_set: &[u8],
    secondary_count: usize,
    stream_entry: &[u8],
) -> Result<Vec<u16>> {
    let name_length = usize::from(stream_entry[3]);
    if name_length == 0 || name_length > UpcaseTable::NAME_MAX {
        return Err(invalid_on_disk_layout());
    }

    let name_entry_count = name_length.div_ceil(15);
    let required_secondary_count = name_entry_count
        .checked_add(1)
        .ok_or(invalid_on_disk_layout())?;
    if secondary_count < required_secondary_count {
        return Err(invalid_on_disk_layout());
    }

    let mut candidate_name = Vec::with_capacity(name_length);
    for name_entry_index in 0..name_entry_count {
        let name_entry_offset = (name_entry_index + 2)
            .checked_mul(DIRECTORY_ENTRY_SIZE)
            .ok_or(invalid_on_disk_layout())?;
        let name_entry_end = name_entry_offset
            .checked_add(DIRECTORY_ENTRY_SIZE)
            .ok_or(invalid_on_disk_layout())?;
        let name_entry = entry_set
            .get(name_entry_offset..name_entry_end)
            .ok_or(invalid_on_disk_layout())?;
        if name_entry[0] != FILE_NAME_ENTRY_TYPE {
            return Err(invalid_on_disk_layout());
        }
        for code_unit_bytes in name_entry[2..].chunks_exact(2) {
            if candidate_name.len() == name_length {
                break;
            }
            candidate_name.push(u16::from_le_bytes([code_unit_bytes[0], code_unit_bytes[1]]));
        }
    }
    if candidate_name.len() != name_length {
        return Err(invalid_on_disk_layout());
    }

    validate_trailing_secondaries(entry_set, required_secondary_count, secondary_count)?;
    Ok(candidate_name)
}

fn validate_trailing_secondaries(
    entry_set: &[u8],
    required_secondary_count: usize,
    secondary_count: usize,
) -> Result<()> {
    for trailing_secondary_index in required_secondary_count..secondary_count {
        let trailing_secondary_offset = (trailing_secondary_index + 1)
            .checked_mul(DIRECTORY_ENTRY_SIZE)
            .ok_or(invalid_on_disk_layout())?;
        let trailing_secondary_end = trailing_secondary_offset
            .checked_add(DIRECTORY_ENTRY_SIZE)
            .ok_or(invalid_on_disk_layout())?;
        let trailing_secondary = entry_set
            .get(trailing_secondary_offset..trailing_secondary_end)
            .ok_or(invalid_on_disk_layout())?;
        if trailing_secondary[0] & ENTRY_TYPE_IN_USE_BIT == 0
            || trailing_secondary[0] & ENTRY_TYPE_CATEGORY_BIT == 0
            || trailing_secondary[0] & ENTRY_TYPE_IMPORTANCE_BIT == 0
        {
            return Err(invalid_on_disk_layout());
        }
    }
    Ok(())
}

fn file_entry_child_metadata(
    entry: &[u8],
    stream_entry: &[u8],
    boot_region: &BootRegion,
) -> Result<(InodeType, u32, usize, bool)> {
    let file_attributes = u16::from_le_bytes([entry[4], entry[5]]);
    let inode_type = if file_attributes & FILE_ATTRIBUTE_DIRECTORY != 0 {
        InodeType::Dir
    } else {
        InodeType::File
    };
    let first_cluster = u32::from_le_bytes([
        stream_entry[20],
        stream_entry[21],
        stream_entry[22],
        stream_entry[23],
    ]);
    let data_length = usize::try_from(u64::from_le_bytes([
        stream_entry[24],
        stream_entry[25],
        stream_entry[26],
        stream_entry[27],
        stream_entry[28],
        stream_entry[29],
        stream_entry[30],
        stream_entry[31],
    ]))
    .map_err(|_| invalid_on_disk_layout())?;
    let no_fat_chain = stream_entry[1] & 0x02 != 0;
    if data_length != 0 {
        boot_region.validate_stream_data(
            first_cluster,
            u64::try_from(data_length).map_err(|_| invalid_on_disk_layout())?,
        )?;
    } else if first_cluster != 0 {
        return Err(invalid_on_disk_layout());
    }
    Ok((inode_type, first_cluster, data_length, no_fat_chain))
}

pub(super) fn entry_set_checksum(entry_set: &[u8], secondary_count: usize) -> u16 {
    let mut checksum = 0u16;
    let number_of_bytes = (secondary_count + 1) * DIRECTORY_ENTRY_SIZE;
    for (index, byte) in entry_set.iter().take(number_of_bytes).enumerate() {
        if index == 2 || index == 3 {
            continue;
        }
        checksum = checksum.rotate_right(1).wrapping_add(u16::from(*byte));
    }
    checksum
}

pub(super) fn slot_range_bytes(
    slot_range: DirectoryEntrySlotRange,
) -> Result<core::ops::Range<usize>> {
    let byte_start = slot_range
        .first_entry_index()
        .checked_mul(DIRECTORY_ENTRY_SIZE)
        .ok_or(invalid_on_disk_layout())?;
    let byte_len = slot_range
        .entry_count()
        .checked_mul(DIRECTORY_ENTRY_SIZE)
        .ok_or(invalid_on_disk_layout())?;
    let byte_end = byte_start
        .checked_add(byte_len)
        .ok_or(invalid_on_disk_layout())?;
    Ok(byte_start..byte_end)
}
