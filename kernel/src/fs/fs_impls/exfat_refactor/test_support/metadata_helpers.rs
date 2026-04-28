// SPDX-License-Identifier: MPL-2.0

use alloc::sync::Arc;
use core::ops::Range;

use time::{Date, Time, UtcOffset};

use super::{
    super::direntry::entry_set_checksum,
    disk::ExfatLookupTestDisk,
    inode_fixtures::root_entry_set,
    timestamp::{encode_exfat_date_only, encode_exfat_date_time, encode_valid_utc_offset_byte},
};

const FILE_ATTRIBUTES_OFFSET: usize = 4;
const CREATE_TIMESTAMP_OFFSET: usize = 8;
const LAST_MODIFIED_TIMESTAMP_OFFSET: usize = 12;
const LAST_ACCESSED_TIMESTAMP_OFFSET: usize = 16;
const CREATE_10MS_INCREMENT_OFFSET: usize = 20;
const LAST_MODIFIED_10MS_INCREMENT_OFFSET: usize = 21;
const CREATE_UTC_OFFSET_OFFSET: usize = 22;
const LAST_MODIFIED_UTC_OFFSET_OFFSET: usize = 23;
const LAST_ACCESSED_UTC_OFFSET_OFFSET: usize = 24;

pub(in super::super) fn set_directory_entry_metadata(
    disk: &Arc<ExfatLookupTestDisk>,
    entry_index: usize,
    file_attributes: u16,
    create: (Date, Time, UtcOffset),
    accessed: (Date, UtcOffset),
    modified: (Date, Time, UtcOffset),
) {
    let mut entry_set = root_entry_set(disk, entry_index);
    entry_set[FILE_ATTRIBUTES_OFFSET..FILE_ATTRIBUTES_OFFSET + 2]
        .copy_from_slice(&file_attributes.to_le_bytes());

    let (create_timestamp, create_ten_ms_increment) = encode_exfat_date_time(create.0, create.1);
    entry_set[CREATE_TIMESTAMP_OFFSET..CREATE_TIMESTAMP_OFFSET + 4]
        .copy_from_slice(&create_timestamp);
    entry_set[CREATE_10MS_INCREMENT_OFFSET] = create_ten_ms_increment;
    entry_set[CREATE_UTC_OFFSET_OFFSET] = encode_valid_utc_offset_byte(create.2);

    entry_set[LAST_ACCESSED_TIMESTAMP_OFFSET..LAST_ACCESSED_TIMESTAMP_OFFSET + 4]
        .copy_from_slice(&encode_exfat_date_only(accessed.0));
    entry_set[LAST_ACCESSED_UTC_OFFSET_OFFSET] = encode_valid_utc_offset_byte(accessed.1);

    let (modified_timestamp, modified_ten_ms_increment) =
        encode_exfat_date_time(modified.0, modified.1);
    entry_set[LAST_MODIFIED_TIMESTAMP_OFFSET..LAST_MODIFIED_TIMESTAMP_OFFSET + 4]
        .copy_from_slice(&modified_timestamp);
    entry_set[LAST_MODIFIED_10MS_INCREMENT_OFFSET] = modified_ten_ms_increment;
    entry_set[LAST_MODIFIED_UTC_OFFSET_OFFSET] = encode_valid_utc_offset_byte(modified.2);

    let checksum = entry_set_checksum(&entry_set, usize::from(entry_set[1]));
    entry_set[2..4].copy_from_slice(&checksum.to_le_bytes());
    disk.write_root_entries(entry_index, &entry_set);
}

pub(in super::super) fn set_regular_file_entry_metadata(
    disk: &Arc<ExfatLookupTestDisk>,
    entry_index: usize,
    file_attributes: u16,
    create: (Date, Time, UtcOffset),
    accessed: (Date, UtcOffset),
    modified: (Date, Time, UtcOffset),
) {
    set_directory_entry_metadata(
        disk,
        entry_index,
        file_attributes,
        create,
        accessed,
        modified,
    );
}

pub(in super::super) fn assert_valid_entry_set_checksum(entry_set: &[u8]) {
    let checksum = entry_set_checksum(entry_set, usize::from(entry_set[1]));
    assert_eq!(u16::from_le_bytes([entry_set[2], entry_set[3]]), checksum);
}

pub(in super::super) fn assert_bytes_unchanged_except(
    before: &[u8],
    after: &[u8],
    allowed_ranges: &[Range<usize>],
) {
    assert_eq!(before.len(), after.len());

    for index in 0..before.len() {
        if allowed_ranges.iter().any(|range| range.contains(&index)) {
            continue;
        }
        assert_eq!(
            after[index], before[index],
            "unexpected durable byte change at offset {index}",
        );
    }
}
