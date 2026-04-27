// SPDX-License-Identifier: MPL-2.0

use core::time::Duration;

use time::{Date, Month, PrimitiveDateTime, Time, UtcOffset};

use super::*;
use crate::process::{Gid, Uid};

const FILE_ATTRIBUTE_READ_ONLY: u16 = 0x0001;
const LAST_MODIFIED_TIMESTAMP_OFFSET: usize = 12;
const LAST_ACCESSED_TIMESTAMP_OFFSET: usize = 16;
const LAST_MODIFIED_10MS_INCREMENT_OFFSET: usize = 21;
const LAST_MODIFIED_UTC_OFFSET_OFFSET: usize = 23;
const LAST_ACCESSED_UTC_OFFSET_OFFSET: usize = 24;

fn encode_exfat_date(date: Date) -> u16 {
    let year = u16::try_from(date.year() - 1980).unwrap();
    let month = u16::from(u8::from(date.month()));
    let day = u16::from(date.day());
    (year << 9) | (month << 5) | day
}

fn encode_exfat_date_only(date: Date) -> [u8; 4] {
    let date_bytes = encode_exfat_date(date).to_le_bytes();
    [0, 0, date_bytes[0], date_bytes[1]]
}

fn encode_exfat_date_time(date: Date, time: Time) -> ([u8; 4], u8) {
    assert_eq!(time.second() % 2, 0);
    assert_eq!(time.millisecond() % 10, 0);

    let encoded_time = (u16::from(time.hour()) << 11)
        | (u16::from(time.minute()) << 5)
        | u16::from(time.second() / 2);
    let time_bytes = encoded_time.to_le_bytes();
    let date_bytes = encode_exfat_date(date).to_le_bytes();
    let ten_ms_increment = u8::try_from(time.millisecond() / 10).unwrap();
    (
        [time_bytes[0], time_bytes[1], date_bytes[0], date_bytes[1]],
        ten_ms_increment,
    )
}

fn encode_valid_utc_offset_byte(offset: UtcOffset) -> u8 {
    let quarter_hours = offset.whole_seconds() / (15 * 60);
    assert!((-64..=63).contains(&quarter_hours));
    0x80 | (u8::try_from(quarter_hours.rem_euclid(128)).unwrap() & 0x7f)
}

fn expected_timestamp(date: Date, time: Time, offset: UtcOffset) -> Duration {
    let timestamp = PrimitiveDateTime::new(date, time).assume_offset(offset);
    Duration::from_nanos(u64::try_from(timestamp.unix_timestamp_nanos()).unwrap())
}

pub(super) fn file_metadata_projection_and_update_projection_substrate_projects_regular_file_snapshot_from_entry_set_and_stream_state() {
    init_lookup_test_runtime();

    let disk = ExfatLookupTestDisk::new();
    let cluster_size = disk.root_cluster_size();
    let file_size = cluster_size + 111;
    disk.install_root_file_with_cluster_chain(
        ROOT_FILE_ENTRY_INDEX,
        "MetaFile",
        TEST_REGULAR_FILE_CLUSTER,
        file_size,
        file_size - 17,
        true,
        &[TEST_REGULAR_FILE_CLUSTER, TEST_CONTIGUOUS_SECOND_CLUSTER],
    );

    let accessed_date = Date::from_calendar_date(2026, Month::February, 3).unwrap();
    let modified_date = Date::from_calendar_date(2026, Month::April, 9).unwrap();
    let modified_time = Time::from_hms_milli(5, 6, 8, 230).unwrap();
    let accessed_offset = UtcOffset::from_whole_seconds(60 * 60).unwrap();
    let modified_offset = UtcOffset::from_whole_seconds(90 * 60).unwrap();
    let expected_atime = expected_timestamp(accessed_date, Time::MIDNIGHT, accessed_offset);
    let expected_mtime = expected_timestamp(modified_date, modified_time, modified_offset);

    {
        let mut entry_set = root_entry_set(&disk, ROOT_FILE_ENTRY_INDEX);
        entry_set[FILE_ATTRIBUTES_OFFSET..FILE_ATTRIBUTES_OFFSET + 2]
            .copy_from_slice(&(FILE_ATTRIBUTE_REGULAR | FILE_ATTRIBUTE_READ_ONLY).to_le_bytes());
        entry_set[LAST_ACCESSED_TIMESTAMP_OFFSET..LAST_ACCESSED_TIMESTAMP_OFFSET + 4]
            .copy_from_slice(&encode_exfat_date_only(accessed_date));
        entry_set[LAST_ACCESSED_UTC_OFFSET_OFFSET] =
            encode_valid_utc_offset_byte(accessed_offset);
        let (modified_timestamp_bytes, modified_ten_ms_increment) =
            encode_exfat_date_time(modified_date, modified_time);
        entry_set[LAST_MODIFIED_TIMESTAMP_OFFSET..LAST_MODIFIED_TIMESTAMP_OFFSET + 4]
            .copy_from_slice(&modified_timestamp_bytes);
        entry_set[LAST_MODIFIED_10MS_INCREMENT_OFFSET] = modified_ten_ms_increment;
        entry_set[LAST_MODIFIED_UTC_OFFSET_OFFSET] =
            encode_valid_utc_offset_byte(modified_offset);
        let checksum = entry_set_checksum(&entry_set, usize::from(entry_set[1]));
        entry_set[2..4].copy_from_slice(&checksum.to_le_bytes());
        disk.write_root_entries(ROOT_FILE_ENTRY_INDEX, &entry_set);
    }

    let (_fs, root_inode) = mount_root(&disk, None);
    let file_inode = root_inode.lookup("MetaFile").unwrap();
    let metadata = file_inode.metadata();
    let expected_mode = chmod!(mkmod!(u+rw, g+r, o+r), a-w);

    assert_eq!(file_inode.type_(), InodeType::File);
    assert_eq!(file_inode.size(), file_size);
    assert_eq!(metadata.size, file_size);
    assert_eq!(
        metadata.nr_sectors_allocated,
        2 * (cluster_size / SECTOR_SIZE)
    );
    assert_eq!(metadata.mode, expected_mode);
    assert_eq!(file_inode.mode().unwrap(), expected_mode);
    assert_eq!(metadata.uid, Uid::new_root());
    assert_eq!(metadata.gid, Gid::new_root());
    assert_eq!(file_inode.owner().unwrap(), Uid::new_root());
    assert_eq!(file_inode.group().unwrap(), Gid::new_root());
    assert_eq!(metadata.last_access_at, expected_atime);
    assert_eq!(file_inode.atime(), expected_atime);
    assert_eq!(metadata.last_modify_at, expected_mtime);
    assert_eq!(file_inode.mtime(), expected_mtime);
    assert_eq!(metadata.last_meta_change_at, expected_mtime);
    assert_eq!(file_inode.ctime(), expected_mtime);
}

pub(super) fn file_metadata_projection_and_update_projection_substrate_rejects_invalid_timestamp_layout_without_disturbing_neighbor_lookups() {
    init_lookup_test_runtime();

    let disk = ExfatLookupTestDisk::new();
    disk.install_root_file(ROOT_FILE_ENTRY_INDEX, "Healthy");
    disk.install_root_file(ROOT_SECOND_FILE_ENTRY_INDEX, "BrokenTime");

    let mut broken_entry_set = root_entry_set(&disk, ROOT_SECOND_FILE_ENTRY_INDEX);
    let invalid_date = Date::from_calendar_date(2026, Month::May, 6).unwrap();
    let invalid_time = Time::from_hms_milli(3, 4, 6, 0).unwrap();
    let (invalid_timestamp_bytes, _) = encode_exfat_date_time(invalid_date, invalid_time);
    broken_entry_set[LAST_MODIFIED_TIMESTAMP_OFFSET..LAST_MODIFIED_TIMESTAMP_OFFSET + 4]
        .copy_from_slice(&invalid_timestamp_bytes);
    broken_entry_set[LAST_MODIFIED_10MS_INCREMENT_OFFSET] = 200;
    let checksum = entry_set_checksum(&broken_entry_set, usize::from(broken_entry_set[1]));
    broken_entry_set[2..4].copy_from_slice(&checksum.to_le_bytes());
    disk.write_root_entries(ROOT_SECOND_FILE_ENTRY_INDEX, &broken_entry_set);

    let (_fs, root_inode) = mount_root(&disk, None);
    let healthy_lookup = root_inode.lookup("healthy").unwrap();
    let broken_lookup_error = root_inode.lookup("brokentime").unwrap_err();

    assert_eq!(healthy_lookup.type_(), InodeType::File);
    assert_eq!(healthy_lookup.size(), 0);
    assert_eq!(broken_lookup_error.error(), Errno::EUCLEAN);
    assert_eq!(
        disk.read_root_entries(
            ROOT_SECOND_FILE_ENTRY_INDEX,
            broken_entry_set.len() / DIRECTORY_ENTRY_SIZE,
        ),
        broken_entry_set
    );
    assert_eq!(root_inode.lookup("Healthy").unwrap().ino(), healthy_lookup.ino());
}
