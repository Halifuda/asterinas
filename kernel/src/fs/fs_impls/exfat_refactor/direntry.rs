// SPDX-License-Identifier: MPL-2.0

// This module is the exFAT-local byte-backed directory-entry scan and authoring
// layer. On-disk entry-shape rules live here; higher-level mutation policy does
// not.

use alloc::{vec, vec::Vec};

use super::{boot::BootRegion, fs::MountVolumeStateError, upcase::UpcaseTable};
use crate::fs::file::InodeType;

pub(super) const DIRECTORY_ENTRY_SIZE: usize = 32;

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
const VOLUME_LABEL_ENTRY_LENGTH_OFFSET: usize = 1;
const VOLUME_LABEL_UTF16_OFFSET: usize = 2;
const VOLUME_LABEL_MAX_CODE_UNITS: usize = 11;
pub(super) const FILE_ATTRIBUTE_READ_ONLY: u16 = 0x0001;
pub(super) const FILE_ATTRIBUTE_DIRECTORY: u16 = 0x0010;
const FILE_ATTRIBUTES_OFFSET: usize = 4;
pub(super) const CREATE_TIMESTAMP_OFFSET: usize = 8;
pub(super) const LAST_MODIFIED_TIMESTAMP_OFFSET: usize = 12;
pub(super) const LAST_ACCESSED_TIMESTAMP_OFFSET: usize = 16;
pub(super) const CREATE_10MS_INCREMENT_OFFSET: usize = 20;
pub(super) const LAST_MODIFIED_10MS_INCREMENT_OFFSET: usize = 21;
pub(super) const CREATE_UTC_OFFSET_OFFSET: usize = 22;
pub(super) const LAST_MODIFIED_UTC_OFFSET_OFFSET: usize = 23;
pub(super) const LAST_ACCESSED_UTC_OFFSET_OFFSET: usize = 24;
const STREAM_FLAGS_OFFSET: usize = 1;
const STREAM_NAME_LENGTH_OFFSET: usize = 3;
const STREAM_NAME_HASH_OFFSET: usize = 4;
const STREAM_VALID_DATA_LENGTH_OFFSET: usize = 8;
const STREAM_FIRST_CLUSTER_OFFSET: usize = 20;
const STREAM_DATA_LENGTH_OFFSET: usize = 24;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct DirectoryEntrySlotRange {
    entry_count: usize,
    first_entry_index: usize,
}

impl DirectoryEntrySlotRange {
    pub(super) fn new(
        first_entry_index: usize,
        entry_count: usize,
    ) -> core::result::Result<Self, MountVolumeStateError> {
        if entry_count == 0 {
            return Err(MountVolumeStateError::InvalidOnDiskLayout);
        }
        first_entry_index
            .checked_add(entry_count)
            .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
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

    pub(super) fn next_entry_index(self) -> core::result::Result<usize, MountVolumeStateError> {
        self.first_entry_index
            .checked_add(self.entry_count)
            .ok_or(MountVolumeStateError::InvalidOnDiskLayout)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum DirectoryEntryAnomalyKind {
    BenignUnrecognizedEntrySet,
    BrokenEntrySet,
    CriticalUnrecognizedEntrySet,
    UnexpectedSecondaryEntry,
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
    ) -> core::result::Result<(InodeType, u32, usize, bool), MountVolumeStateError> {
        file_entry_child_metadata(self.primary_entry, self.stream_entry, boot_region)
    }

    pub(super) fn name(self) -> core::result::Result<Vec<u16>, MountVolumeStateError> {
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

    pub(super) fn stored_name_hash(self) -> u16 {
        u16::from_le_bytes([self.stream_entry[4], self.stream_entry[5]])
    }
}

pub(super) fn file_entry_set_entry_count(
    name_length: usize,
) -> core::result::Result<usize, MountVolumeStateError> {
    if name_length == 0 || name_length > UpcaseTable::NAME_MAX {
        return Err(MountVolumeStateError::InvalidOperationInput);
    }
    name_length
        .div_ceil(15)
        .checked_add(2)
        .ok_or(MountVolumeStateError::InvalidOperationInput)
}

pub(super) fn encode_file_entry_set(
    name: &[u16],
    name_hash: u16,
    inode_type: InodeType,
    first_cluster: u32,
    data_length: usize,
    no_fat_chain: bool,
) -> core::result::Result<Vec<u8>, MountVolumeStateError> {
    let entry_count = file_entry_set_entry_count(name.len())?;
    let secondary_count = entry_count
        .checked_sub(1)
        .ok_or(MountVolumeStateError::InvalidOperationInput)?;
    let secondary_count =
        u8::try_from(secondary_count).map_err(|_| MountVolumeStateError::InvalidOperationInput)?;
    let entry_set_len = entry_count
        .checked_mul(DIRECTORY_ENTRY_SIZE)
        .ok_or(MountVolumeStateError::InvalidOperationInput)?;
    let mut entry_set = vec![0; entry_set_len];

    entry_set[0] = FILE_DIRECTORY_ENTRY_TYPE;
    entry_set[1] = secondary_count;
    let file_attributes = match inode_type {
        InodeType::Dir => FILE_ATTRIBUTE_DIRECTORY,
        InodeType::File => 0x0020,
        _ => return Err(MountVolumeStateError::InvalidOperationInput),
    };
    entry_set[4..6].copy_from_slice(&file_attributes.to_le_bytes());

    let stream_entry_offset = DIRECTORY_ENTRY_SIZE;
    entry_set[stream_entry_offset] = STREAM_EXTENSION_ENTRY_TYPE;
    entry_set[stream_entry_offset + 1] = if no_fat_chain { 0x03 } else { 0x01 };
    entry_set[stream_entry_offset + 3] =
        u8::try_from(name.len()).map_err(|_| MountVolumeStateError::InvalidOperationInput)?;
    entry_set[stream_entry_offset + 4..stream_entry_offset + 6]
        .copy_from_slice(&name_hash.to_le_bytes());
    let data_length =
        u64::try_from(data_length).map_err(|_| MountVolumeStateError::InvalidOperationInput)?;
    entry_set[stream_entry_offset + 8..stream_entry_offset + 16]
        .copy_from_slice(&data_length.to_le_bytes());
    entry_set[stream_entry_offset + 20..stream_entry_offset + 24]
        .copy_from_slice(&first_cluster.to_le_bytes());
    entry_set[stream_entry_offset + 24..stream_entry_offset + 32]
        .copy_from_slice(&data_length.to_le_bytes());

    for (name_entry_index, name_chunk) in name.chunks(15).enumerate() {
        let name_entry_offset = (name_entry_index + 2)
            .checked_mul(DIRECTORY_ENTRY_SIZE)
            .ok_or(MountVolumeStateError::InvalidOperationInput)?;
        entry_set[name_entry_offset] = FILE_NAME_ENTRY_TYPE;
        for (name_code_unit_index, name_code_unit) in name_chunk.iter().enumerate() {
            let code_unit_offset = name_entry_offset
                .checked_add(2)
                .and_then(|offset| offset.checked_add(name_code_unit_index * 2))
                .ok_or(MountVolumeStateError::InvalidOperationInput)?;
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
    ) -> core::result::Result<Self, MountVolumeStateError> {
        let expected_len = slot_range
            .entry_count()
            .checked_mul(DIRECTORY_ENTRY_SIZE)
            .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
        if slot_span.is_empty()
            || slot_span.len() != expected_len
            || slot_span.len() % DIRECTORY_ENTRY_SIZE != 0
        {
            return Err(MountVolumeStateError::InvalidOnDiskLayout);
        }
        Ok(Self { slot_span })
    }

    pub(super) fn bytes_mut(&mut self) -> &mut [u8] {
        self.slot_span
    }
}

pub(super) fn invalidate_entry_set(
    slot_span: &mut WritableDirectoryEntrySlotSpan<'_>,
) -> core::result::Result<(), MountVolumeStateError> {
    for entry in slot_span.bytes_mut().chunks_exact_mut(DIRECTORY_ENTRY_SIZE) {
        entry[0] &= !ENTRY_TYPE_IN_USE_BIT;
    }
    Ok(())
}

pub(super) fn read_volume_label(
    directory_bytes: &[u8],
) -> core::result::Result<Option<Vec<u16>>, MountVolumeStateError> {
    let Some(entry_index) = locate_volume_label_entry(directory_bytes)? else {
        return Ok(None);
    };
    let entry_offset = entry_index
        .checked_mul(DIRECTORY_ENTRY_SIZE)
        .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
    let entry_end = entry_offset
        .checked_add(DIRECTORY_ENTRY_SIZE)
        .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
    let entry = directory_bytes
        .get(entry_offset..entry_end)
        .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
    let label_length = usize::from(entry[VOLUME_LABEL_ENTRY_LENGTH_OFFSET]);
    if label_length > VOLUME_LABEL_MAX_CODE_UNITS {
        return Err(MountVolumeStateError::InvalidOnDiskLayout);
    }
    if label_length == 0 {
        return Ok(None);
    }

    let mut label = Vec::with_capacity(label_length);
    let label_end = VOLUME_LABEL_UTF16_OFFSET
        .checked_add(
            label_length
                .checked_mul(2)
                .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?,
        )
        .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
    for code_unit_bytes in entry[VOLUME_LABEL_UTF16_OFFSET..label_end].chunks_exact(2) {
        label.push(u16::from_le_bytes([code_unit_bytes[0], code_unit_bytes[1]]));
    }
    Ok(Some(label))
}

pub(super) fn write_volume_label(
    directory_bytes: &mut [u8],
    label: Option<&[u16]>,
) -> core::result::Result<(), MountVolumeStateError> {
    let existing_entry_index = locate_volume_label_entry(directory_bytes)?;
    let Some(label) = label.filter(|label| !label.is_empty()) else {
        if let Some(existing_entry_index) = existing_entry_index {
            let slot_range = DirectoryEntrySlotRange::new(existing_entry_index, 1)?;
            let slot_range_bytes = slot_range_bytes(slot_range)?;
            let slot_bytes = directory_bytes
                .get_mut(slot_range_bytes)
                .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
            let mut slot_span = WritableDirectoryEntrySlotSpan::new(slot_range, slot_bytes)?;
            invalidate_entry_set(&mut slot_span)?;
        }
        return Ok(());
    };
    if label.len() > VOLUME_LABEL_MAX_CODE_UNITS {
        return Err(MountVolumeStateError::InvalidOperationInput);
    }

    let destination_entry_index = match existing_entry_index {
        Some(existing_entry_index) => existing_entry_index,
        None => {
            let mut destination_entry_index = None;
            for (entry_index, entry) in directory_bytes
                .chunks_exact(DIRECTORY_ENTRY_SIZE)
                .enumerate()
            {
                if entry[0] == END_OF_DIRECTORY_ENTRY_TYPE || entry[0] & ENTRY_TYPE_IN_USE_BIT == 0
                {
                    destination_entry_index = Some(entry_index);
                    break;
                }
            }
            destination_entry_index.ok_or(MountVolumeStateError::InvalidOnDiskLayout)?
        }
    };
    let mut encoded_entry = [0u8; DIRECTORY_ENTRY_SIZE];
    encoded_entry[0] = VOLUME_LABEL_ENTRY_TYPE;
    encoded_entry[VOLUME_LABEL_ENTRY_LENGTH_OFFSET] =
        u8::try_from(label.len()).map_err(|_| MountVolumeStateError::InvalidOperationInput)?;
    for (index, code_unit) in label.iter().enumerate() {
        let code_unit_offset = VOLUME_LABEL_UTF16_OFFSET
            .checked_add(
                index
                    .checked_mul(2)
                    .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?,
            )
            .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
        encoded_entry[code_unit_offset..code_unit_offset + 2]
            .copy_from_slice(&code_unit.to_le_bytes());
    }
    let entry_offset = destination_entry_index
        .checked_mul(DIRECTORY_ENTRY_SIZE)
        .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
    let entry_end = entry_offset
        .checked_add(DIRECTORY_ENTRY_SIZE)
        .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
    let entry = directory_bytes
        .get_mut(entry_offset..entry_end)
        .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
    entry.copy_from_slice(&encoded_entry);
    Ok(())
}

// Recognized field updates for one already-admitted published file entry set.
// This preserves the existing entry-set topology and refuses layout-changing
// name rewrites.
#[derive(Default)]
pub(super) struct FileEntrySetFieldUpdates<'a> {
    pub(super) create_fields: Option<([u8; 4], u8, u8)>,
    pub(super) data_length: Option<u64>,
    pub(super) file_attributes: Option<u16>,
    pub(super) first_cluster: Option<u32>,
    pub(super) last_accessed_fields: Option<([u8; 4], u8)>,
    pub(super) last_modified_fields: Option<([u8; 4], u8, u8)>,
    pub(super) name: Option<&'a [u16]>,
    pub(super) name_hash: Option<u16>,
    pub(super) stream_flags: Option<u8>,
    pub(super) valid_data_length: Option<u64>,
}

pub(super) fn republished_entry_set(
    source_entry_set: FileEntrySetView<'_>,
    updates: &FileEntrySetFieldUpdates<'_>,
) -> core::result::Result<Vec<u8>, MountVolumeStateError> {
    let mut republished_entry_set = source_entry_set.entry_set.to_vec();
    let stream_entry_offset = DIRECTORY_ENTRY_SIZE;

    if let Some(file_attributes) = updates.file_attributes {
        republished_entry_set[FILE_ATTRIBUTES_OFFSET..FILE_ATTRIBUTES_OFFSET + 2]
            .copy_from_slice(&file_attributes.to_le_bytes());
    }

    if let Some((timestamp, ten_ms_increment, utc_offset)) = updates.create_fields {
        republished_entry_set[CREATE_TIMESTAMP_OFFSET..CREATE_TIMESTAMP_OFFSET + 4]
            .copy_from_slice(&timestamp);
        republished_entry_set[CREATE_10MS_INCREMENT_OFFSET] = ten_ms_increment;
        republished_entry_set[CREATE_UTC_OFFSET_OFFSET] = utc_offset;
    }

    if let Some((timestamp, ten_ms_increment, utc_offset)) = updates.last_modified_fields {
        republished_entry_set[LAST_MODIFIED_TIMESTAMP_OFFSET..LAST_MODIFIED_TIMESTAMP_OFFSET + 4]
            .copy_from_slice(&timestamp);
        republished_entry_set[LAST_MODIFIED_10MS_INCREMENT_OFFSET] = ten_ms_increment;
        republished_entry_set[LAST_MODIFIED_UTC_OFFSET_OFFSET] = utc_offset;
    }

    if let Some((timestamp, utc_offset)) = updates.last_accessed_fields {
        republished_entry_set[LAST_ACCESSED_TIMESTAMP_OFFSET..LAST_ACCESSED_TIMESTAMP_OFFSET + 4]
            .copy_from_slice(&timestamp);
        republished_entry_set[LAST_ACCESSED_UTC_OFFSET_OFFSET] = utc_offset;
    }

    if let Some(name) = updates.name {
        let current_name_entry_count =
            usize::from(source_entry_set.stream_entry[STREAM_NAME_LENGTH_OFFSET]).div_ceil(15);
        let requested_name_entry_count = file_entry_set_entry_count(name.len())?
            .checked_sub(2)
            .ok_or(MountVolumeStateError::InvalidOperationInput)?;
        if requested_name_entry_count != current_name_entry_count {
            return Err(MountVolumeStateError::InvalidOperationInput);
        }

        republished_entry_set[stream_entry_offset + STREAM_NAME_LENGTH_OFFSET] =
            u8::try_from(name.len()).map_err(|_| MountVolumeStateError::InvalidOperationInput)?;

        for name_entry_index in 0..current_name_entry_count {
            let name_entry_offset = (name_entry_index + 2)
                .checked_mul(DIRECTORY_ENTRY_SIZE)
                .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
            republished_entry_set[name_entry_offset + 2..name_entry_offset + DIRECTORY_ENTRY_SIZE]
                .fill(0);
        }

        for (name_entry_index, name_chunk) in name.chunks(15).enumerate() {
            let name_entry_offset = (name_entry_index + 2)
                .checked_mul(DIRECTORY_ENTRY_SIZE)
                .ok_or(MountVolumeStateError::InvalidOperationInput)?;
            for (name_code_unit_index, name_code_unit) in name_chunk.iter().enumerate() {
                let code_unit_offset = name_entry_offset
                    .checked_add(2)
                    .and_then(|offset| offset.checked_add(name_code_unit_index * 2))
                    .ok_or(MountVolumeStateError::InvalidOperationInput)?;
                republished_entry_set[code_unit_offset..code_unit_offset + 2]
                    .copy_from_slice(&name_code_unit.to_le_bytes());
            }
        }
    }

    if let Some(name_hash) = updates.name_hash {
        republished_entry_set[stream_entry_offset + STREAM_NAME_HASH_OFFSET
            ..stream_entry_offset + STREAM_NAME_HASH_OFFSET + 2]
            .copy_from_slice(&name_hash.to_le_bytes());
    }

    if let Some(stream_flags) = updates.stream_flags {
        republished_entry_set[stream_entry_offset + STREAM_FLAGS_OFFSET] = stream_flags;
    }

    if let Some(valid_data_length) = updates.valid_data_length {
        republished_entry_set[stream_entry_offset + STREAM_VALID_DATA_LENGTH_OFFSET
            ..stream_entry_offset + STREAM_VALID_DATA_LENGTH_OFFSET + 8]
            .copy_from_slice(&valid_data_length.to_le_bytes());
    }

    if let Some(first_cluster) = updates.first_cluster {
        republished_entry_set[stream_entry_offset + STREAM_FIRST_CLUSTER_OFFSET
            ..stream_entry_offset + STREAM_FIRST_CLUSTER_OFFSET + 4]
            .copy_from_slice(&first_cluster.to_le_bytes());
    }

    if let Some(data_length) = updates.data_length {
        republished_entry_set[stream_entry_offset + STREAM_DATA_LENGTH_OFFSET
            ..stream_entry_offset + STREAM_DATA_LENGTH_OFFSET + 8]
            .copy_from_slice(&data_length.to_le_bytes());
    }

    let checksum = entry_set_checksum(
        &republished_entry_set,
        usize::from(source_entry_set.secondary_count),
    );
    republished_entry_set[2..4].copy_from_slice(&checksum.to_le_bytes());
    Ok(republished_entry_set)
}

pub(super) fn renamed_entry_set(
    source_entry_set: FileEntrySetView<'_>,
    name: &[u16],
    name_hash: u16,
) -> core::result::Result<Vec<u8>, MountVolumeStateError> {
    let entry_count = file_entry_set_entry_count(name.len())?;
    let new_name_entry_count = entry_count
        .checked_sub(2)
        .ok_or(MountVolumeStateError::InvalidOperationInput)?;
    let current_name_entry_count =
        usize::from(source_entry_set.stream_entry[STREAM_NAME_LENGTH_OFFSET]).div_ceil(15);
    if new_name_entry_count == current_name_entry_count {
        return republished_entry_set(
            source_entry_set,
            &FileEntrySetFieldUpdates {
                name: Some(name),
                name_hash: Some(name_hash),
                ..Default::default()
            },
        );
    }
    let required_secondary_count = current_name_entry_count
        .checked_add(1)
        .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
    let trailing_secondary_count = source_entry_set
        .secondary_count
        .checked_sub(required_secondary_count)
        .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
    let secondary_count = new_name_entry_count
        .checked_add(1)
        .and_then(|count| count.checked_add(trailing_secondary_count))
        .ok_or(MountVolumeStateError::InvalidOperationInput)?;
    let secondary_count =
        u8::try_from(secondary_count).map_err(|_| MountVolumeStateError::InvalidOperationInput)?;
    let entry_set_len = usize::from(secondary_count)
        .checked_add(1)
        .and_then(|entry_count| entry_count.checked_mul(DIRECTORY_ENTRY_SIZE))
        .ok_or(MountVolumeStateError::InvalidOperationInput)?;
    let mut renamed_entry_set = vec![0; entry_set_len];
    renamed_entry_set[..DIRECTORY_ENTRY_SIZE].copy_from_slice(source_entry_set.primary_entry);
    renamed_entry_set[DIRECTORY_ENTRY_SIZE..DIRECTORY_ENTRY_SIZE * 2]
        .copy_from_slice(source_entry_set.stream_entry);
    renamed_entry_set[1] = secondary_count;
    renamed_entry_set[DIRECTORY_ENTRY_SIZE + STREAM_NAME_LENGTH_OFFSET] =
        u8::try_from(name.len()).map_err(|_| MountVolumeStateError::InvalidOperationInput)?;
    renamed_entry_set[DIRECTORY_ENTRY_SIZE + STREAM_NAME_HASH_OFFSET
        ..DIRECTORY_ENTRY_SIZE + STREAM_NAME_HASH_OFFSET + 2]
        .copy_from_slice(&name_hash.to_le_bytes());

    for (name_entry_index, name_chunk) in name.chunks(15).enumerate() {
        let name_entry_offset = (name_entry_index + 2)
            .checked_mul(DIRECTORY_ENTRY_SIZE)
            .ok_or(MountVolumeStateError::InvalidOperationInput)?;
        renamed_entry_set[name_entry_offset] = FILE_NAME_ENTRY_TYPE;
        for (name_code_unit_index, name_code_unit) in name_chunk.iter().enumerate() {
            let code_unit_offset = name_entry_offset
                .checked_add(2)
                .and_then(|offset| offset.checked_add(name_code_unit_index * 2))
                .ok_or(MountVolumeStateError::InvalidOperationInput)?;
            renamed_entry_set[code_unit_offset..code_unit_offset + 2]
                .copy_from_slice(&name_code_unit.to_le_bytes());
        }
    }

    let trailing_source_offset = (current_name_entry_count + 2)
        .checked_mul(DIRECTORY_ENTRY_SIZE)
        .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
    let trailing_destination_offset = (new_name_entry_count + 2)
        .checked_mul(DIRECTORY_ENTRY_SIZE)
        .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
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

pub(super) fn scan_directory_entry(
    is_root_directory: bool,
    directory_bytes: &[u8],
    mut entry_index: usize,
) -> core::result::Result<ScannedDirectoryEntry<'_>, MountVolumeStateError> {
    loop {
        let slot_range = DirectoryEntrySlotRange::new(entry_index, 1)?;
        let entry_offset = entry_index
            .checked_mul(DIRECTORY_ENTRY_SIZE)
            .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
        let entry_end = entry_offset
            .checked_add(DIRECTORY_ENTRY_SIZE)
            .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
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
                        .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
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
) -> core::result::Result<ScannedDirectoryEntry<'a>, MountVolumeStateError> {
    let secondary_count = usize::from(primary_entry[1]);
    let slot_range = DirectoryEntrySlotRange::new(
        entry_index,
        secondary_count
            .checked_add(1)
            .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?,
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
) -> core::result::Result<ScannedDirectoryEntry<'a>, MountVolumeStateError> {
    let secondary_count = usize::from(primary_entry[1]);
    let slot_range = DirectoryEntrySlotRange::new(
        entry_index,
        secondary_count
            .checked_add(1)
            .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?,
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
) -> core::result::Result<&[u8], MountVolumeStateError> {
    let entry_set_len = secondary_count
        .checked_add(1)
        .and_then(|entries| entries.checked_mul(DIRECTORY_ENTRY_SIZE))
        .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
    let entry_set_end = entry_offset
        .checked_add(entry_set_len)
        .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
    let entry_set = directory_bytes
        .get(entry_offset..entry_set_end)
        .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
    if entry_set_checksum(entry_set, secondary_count) != expected_checksum {
        return Err(MountVolumeStateError::InvalidOnDiskLayout);
    }
    Ok(entry_set)
}

fn file_stream_entry(entry_set: &[u8]) -> core::result::Result<&[u8], MountVolumeStateError> {
    let stream_entry = entry_set
        .get(DIRECTORY_ENTRY_SIZE..DIRECTORY_ENTRY_SIZE * 2)
        .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
    if stream_entry[0] != STREAM_EXTENSION_ENTRY_TYPE {
        return Err(MountVolumeStateError::InvalidOnDiskLayout);
    }
    if stream_entry[1] & 0x01 == 0 {
        return Err(MountVolumeStateError::InvalidOnDiskLayout);
    }
    Ok(stream_entry)
}

fn file_name(
    entry_set: &[u8],
    secondary_count: usize,
    stream_entry: &[u8],
) -> core::result::Result<Vec<u16>, MountVolumeStateError> {
    let name_length = usize::from(stream_entry[3]);
    if name_length == 0 || name_length > UpcaseTable::NAME_MAX {
        return Err(MountVolumeStateError::InvalidOnDiskLayout);
    }

    let name_entry_count = name_length.div_ceil(15);
    let required_secondary_count = name_entry_count
        .checked_add(1)
        .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
    if secondary_count < required_secondary_count {
        return Err(MountVolumeStateError::InvalidOnDiskLayout);
    }

    let mut candidate_name = Vec::with_capacity(name_length);
    for name_entry_index in 0..name_entry_count {
        let name_entry_offset = (name_entry_index + 2)
            .checked_mul(DIRECTORY_ENTRY_SIZE)
            .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
        let name_entry_end = name_entry_offset
            .checked_add(DIRECTORY_ENTRY_SIZE)
            .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
        let name_entry = entry_set
            .get(name_entry_offset..name_entry_end)
            .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
        if name_entry[0] != FILE_NAME_ENTRY_TYPE {
            return Err(MountVolumeStateError::InvalidOnDiskLayout);
        }
        for code_unit_bytes in name_entry[2..].chunks_exact(2) {
            if candidate_name.len() == name_length {
                break;
            }
            candidate_name.push(u16::from_le_bytes([code_unit_bytes[0], code_unit_bytes[1]]));
        }
    }
    if candidate_name.len() != name_length {
        return Err(MountVolumeStateError::InvalidOnDiskLayout);
    }

    validate_trailing_secondaries(entry_set, required_secondary_count, secondary_count)?;
    Ok(candidate_name)
}

fn validate_trailing_secondaries(
    entry_set: &[u8],
    required_secondary_count: usize,
    secondary_count: usize,
) -> core::result::Result<(), MountVolumeStateError> {
    for trailing_secondary_index in required_secondary_count..secondary_count {
        let trailing_secondary_offset = (trailing_secondary_index + 1)
            .checked_mul(DIRECTORY_ENTRY_SIZE)
            .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
        let trailing_secondary_end = trailing_secondary_offset
            .checked_add(DIRECTORY_ENTRY_SIZE)
            .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
        let trailing_secondary = entry_set
            .get(trailing_secondary_offset..trailing_secondary_end)
            .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
        if trailing_secondary[0] & ENTRY_TYPE_IN_USE_BIT == 0
            || trailing_secondary[0] & ENTRY_TYPE_CATEGORY_BIT == 0
            || trailing_secondary[0] & ENTRY_TYPE_IMPORTANCE_BIT == 0
        {
            return Err(MountVolumeStateError::InvalidOnDiskLayout);
        }
    }
    Ok(())
}

fn file_entry_child_metadata(
    entry: &[u8],
    stream_entry: &[u8],
    boot_region: &BootRegion,
) -> core::result::Result<(InodeType, u32, usize, bool), MountVolumeStateError> {
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
    .map_err(|_| MountVolumeStateError::InvalidOnDiskLayout)?;
    let no_fat_chain = stream_entry[1] & 0x02 != 0;
    if data_length != 0 {
        boot_region.validate_stream_data(
            first_cluster,
            u64::try_from(data_length).map_err(|_| MountVolumeStateError::InvalidOnDiskLayout)?,
        )?;
    } else if first_cluster != 0 {
        return Err(MountVolumeStateError::InvalidOnDiskLayout);
    }
    Ok((inode_type, first_cluster, data_length, no_fat_chain))
}

fn entry_set_checksum(entry_set: &[u8], secondary_count: usize) -> u16 {
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

fn locate_volume_label_entry(
    directory_bytes: &[u8],
) -> core::result::Result<Option<usize>, MountVolumeStateError> {
    if directory_bytes.len() % DIRECTORY_ENTRY_SIZE != 0 {
        return Err(MountVolumeStateError::InvalidOnDiskLayout);
    }

    let mut label_entry_index = None;
    for (entry_index, entry) in directory_bytes
        .chunks_exact(DIRECTORY_ENTRY_SIZE)
        .enumerate()
    {
        if entry[0] == END_OF_DIRECTORY_ENTRY_TYPE {
            break;
        }
        if entry[0] == VOLUME_LABEL_ENTRY_TYPE {
            if label_entry_index.replace(entry_index).is_some() {
                return Err(MountVolumeStateError::InvalidOnDiskLayout);
            }
        }
    }
    Ok(label_entry_index)
}

pub(super) fn slot_range_bytes(
    slot_range: DirectoryEntrySlotRange,
) -> core::result::Result<core::ops::Range<usize>, MountVolumeStateError> {
    let byte_start = slot_range
        .first_entry_index()
        .checked_mul(DIRECTORY_ENTRY_SIZE)
        .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
    let byte_len = slot_range
        .entry_count()
        .checked_mul(DIRECTORY_ENTRY_SIZE)
        .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
    let byte_end = byte_start
        .checked_add(byte_len)
        .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
    Ok(byte_start..byte_end)
}
