// SPDX-License-Identifier: MPL-2.0

use alloc::{vec, vec::Vec};

use super::{
    boot::BootRegion,
    fs::MountVolumeStateError,
    upcase::UpcaseTable,
};
use crate::fs::file::InodeType;

pub(super) const DIRECTORY_ENTRY_SIZE: usize = 32;

const END_OF_DIRECTORY_ENTRY_TYPE: u8 = 0x00;
const ALLOCATION_BITMAP_ENTRY_TYPE: u8 = 0x81;
const UPCASE_TABLE_ENTRY_TYPE: u8 = 0x82;
const VOLUME_LABEL_ENTRY_TYPE: u8 = 0x83;
const FILE_DIRECTORY_ENTRY_TYPE: u8 = 0x85;
const STREAM_EXTENSION_ENTRY_TYPE: u8 = 0xC0;
const FILE_NAME_ENTRY_TYPE: u8 = 0xC1;
const ENTRY_TYPE_IMPORTANCE_BIT: u8 = 0x20;
const ENTRY_TYPE_CATEGORY_BIT: u8 = 0x40;
const ENTRY_TYPE_IN_USE_BIT: u8 = 0x80;
const FILE_ATTRIBUTE_DIRECTORY: u16 = 0x0010;

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
                    ALLOCATION_BITMAP_ENTRY_TYPE | UPCASE_TABLE_ENTRY_TYPE | VOLUME_LABEL_ENTRY_TYPE
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
    if validated_file_entry_set(directory_bytes, entry_offset, secondary_count, expected_checksum)
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
            candidate_name.push(u16::from_le_bytes([
                code_unit_bytes[0],
                code_unit_bytes[1],
            ]));
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
        checksum = ((checksum & 1) << 15) + (checksum >> 1) + u16::from(*byte);
    }
    checksum
}
