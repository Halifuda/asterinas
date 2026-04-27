// SPDX-License-Identifier: MPL-2.0

use alloc::sync::Arc;
use core::{ops::Range, time::Duration};

use aster_block::BlockDevice;
use time::{Date, Month, PrimitiveDateTime, Time, UtcOffset};

use super::{super::super::test_support::inode::entry_set_checksum, *};
use crate::process::{Gid, Uid};

const CREATE_TIMESTAMP_OFFSET: usize = 8;
const LAST_MODIFIED_TIMESTAMP_OFFSET: usize = 12;
const LAST_ACCESSED_TIMESTAMP_OFFSET: usize = 16;
const CREATE_10MS_INCREMENT_OFFSET: usize = 20;
const LAST_MODIFIED_10MS_INCREMENT_OFFSET: usize = 21;
const CREATE_UTC_OFFSET_OFFSET: usize = 22;
const LAST_MODIFIED_UTC_OFFSET_OFFSET: usize = 23;
const LAST_ACCESSED_UTC_OFFSET_OFFSET: usize = 24;
const FILE_ATTRIBUTE_READ_ONLY: u16 = 0x0001;

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

fn set_directory_entry_metadata(
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

    let checksum = entry_set_checksum(&entry_set, entry_set[1]);
    entry_set[2..4].copy_from_slice(&checksum.to_le_bytes());
    disk.write_root_entries(entry_index, &entry_set);
}

fn assert_valid_entry_set_checksum(entry_set: &[u8]) {
    let checksum = entry_set_checksum(entry_set, entry_set[1]);
    assert_eq!(u16::from_le_bytes([entry_set[2], entry_set[3]]), checksum);
}

fn assert_bytes_unchanged_except(before: &[u8], after: &[u8], allowed_ranges: &[Range<usize>]) {
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

pub(super) fn directory_metadata_projection_and_update_policy_and_timestamp_mutation_updates_only_dos_read_only_for_ordinary_directories()
 {
    init_lookup_test_runtime();

    let disk = ExfatLookupTestDisk::new();
    disk.install_root_directory(
        ROOT_FILE_ENTRY_INDEX,
        "ModeDir",
        TEST_CHILD_DIRECTORY_CLUSTER,
    );
    set_directory_entry_metadata(
        &disk,
        ROOT_FILE_ENTRY_INDEX,
        FILE_ATTRIBUTE_DIRECTORY,
        (
            Date::from_calendar_date(2026, Month::January, 14).unwrap(),
            Time::from_hms_milli(1, 2, 4, 120).unwrap(),
            UtcOffset::from_whole_seconds(2 * 60 * 60).unwrap(),
        ),
        (
            Date::from_calendar_date(2026, Month::January, 18).unwrap(),
            UtcOffset::from_whole_seconds(60 * 60).unwrap(),
        ),
        (
            Date::from_calendar_date(2026, Month::January, 22).unwrap(),
            Time::from_hms_milli(5, 6, 8, 230).unwrap(),
            UtcOffset::from_whole_seconds(-90 * 60).unwrap(),
        ),
    );

    let (_fs, root_inode) = mount_root(&disk, None);
    let directory_inode = root_inode.lookup("ModeDir").unwrap();
    let metadata_before = directory_inode.metadata();
    let entry_set_before = root_entry_set(&disk, ROOT_FILE_ENTRY_INDEX);

    let requested_mode = chmod!(metadata_before.mode, a-w);
    directory_inode.set_mode(requested_mode).unwrap();

    let entry_set_after = root_entry_set(&disk, ROOT_FILE_ENTRY_INDEX);
    assert_valid_entry_set_checksum(&entry_set_after);
    assert_eq!(
        u16::from_le_bytes([
            entry_set_after[FILE_ATTRIBUTES_OFFSET],
            entry_set_after[FILE_ATTRIBUTES_OFFSET + 1],
        ]),
        FILE_ATTRIBUTE_DIRECTORY | FILE_ATTRIBUTE_READ_ONLY
    );
    assert_bytes_unchanged_except(&entry_set_before, &entry_set_after, &[2..6]);
    assert_eq!(directory_inode.type_(), InodeType::Dir);
    assert_eq!(directory_inode.mode().unwrap(), requested_mode);
    assert_eq!(directory_inode.metadata().mode, requested_mode);
    assert_eq!(directory_inode.metadata().size, metadata_before.size);
    assert_eq!(
        directory_inode.metadata().nr_sectors_allocated,
        metadata_before.nr_sectors_allocated
    );
}

pub(super) fn directory_metadata_projection_and_update_policy_and_timestamp_mutation_owner_group_follow_mount_envelope()
 {
    init_lookup_test_runtime();

    let disk = ExfatLookupTestDisk::new();
    disk.install_root_directory(
        ROOT_FILE_ENTRY_INDEX,
        "OwnerDir",
        TEST_CHILD_DIRECTORY_CLUSTER,
    );

    let (_fs, root_inode) = mount_root(&disk, None);
    let directory_inode = root_inode.lookup("OwnerDir").unwrap();
    let metadata_before = directory_inode.metadata();
    let entry_set_before = root_entry_set(&disk, ROOT_FILE_ENTRY_INDEX);

    assert_eq!(directory_inode.owner().unwrap(), Uid::new_root());
    assert_eq!(directory_inode.group().unwrap(), Gid::new_root());

    directory_inode.set_owner(Uid::new_root()).unwrap();
    directory_inode.set_group(Gid::new_root()).unwrap();
    assert_eq!(
        directory_inode.set_owner(Uid::new(42)).unwrap_err().error(),
        Errno::EPERM
    );
    assert_eq!(
        directory_inode.set_group(Gid::new(24)).unwrap_err().error(),
        Errno::EPERM
    );

    assert_eq!(
        root_entry_set(&disk, ROOT_FILE_ENTRY_INDEX),
        entry_set_before
    );
    assert_metadata_unchanged(directory_inode.metadata(), metadata_before);
}

pub(super) fn directory_metadata_projection_and_update_policy_and_timestamp_mutation_rewrites_only_directory_timestamp_families()
 {
    init_lookup_test_runtime();

    let disk = ExfatLookupTestDisk::new();
    disk.install_root_directory(
        ROOT_FILE_ENTRY_INDEX,
        "TimeDir",
        TEST_CHILD_DIRECTORY_CLUSTER,
    );

    let create_date = Date::from_calendar_date(2026, Month::February, 2).unwrap();
    let create_time = Time::from_hms_milli(3, 4, 6, 120).unwrap();
    let create_offset = UtcOffset::from_whole_seconds(2 * 60 * 60).unwrap();
    let accessed_date = Date::from_calendar_date(2026, Month::February, 5).unwrap();
    let accessed_offset = UtcOffset::from_whole_seconds(60 * 60).unwrap();
    let modified_date = Date::from_calendar_date(2026, Month::February, 9).unwrap();
    let modified_time = Time::from_hms_milli(10, 12, 14, 230).unwrap();
    let modified_offset = UtcOffset::from_whole_seconds(-2 * 60 * 60).unwrap();
    set_directory_entry_metadata(
        &disk,
        ROOT_FILE_ENTRY_INDEX,
        FILE_ATTRIBUTE_DIRECTORY,
        (create_date, create_time, create_offset),
        (accessed_date, accessed_offset),
        (modified_date, modified_time, modified_offset),
    );

    let (_fs, root_inode) = mount_root(&disk, None);
    let directory_inode = root_inode.lookup("TimeDir").unwrap();
    let modified_before = directory_inode.mtime();
    let entry_set_before = root_entry_set(&disk, ROOT_FILE_ENTRY_INDEX);

    let requested_atime = expected_timestamp(
        Date::from_calendar_date(2026, Month::March, 12).unwrap(),
        Time::from_hms(16, 14, 0).unwrap(),
        accessed_offset,
    );
    directory_inode.set_atime(requested_atime);

    let entry_set_after_atime = root_entry_set(&disk, ROOT_FILE_ENTRY_INDEX);
    assert_valid_entry_set_checksum(&entry_set_after_atime);
    assert_bytes_unchanged_except(
        &entry_set_before,
        &entry_set_after_atime,
        &[
            2..4,
            LAST_ACCESSED_TIMESTAMP_OFFSET..LAST_ACCESSED_TIMESTAMP_OFFSET + 4,
            LAST_ACCESSED_UTC_OFFSET_OFFSET..LAST_ACCESSED_UTC_OFFSET_OFFSET + 1,
        ],
    );
    assert_eq!(
        &entry_set_after_atime[LAST_ACCESSED_TIMESTAMP_OFFSET..LAST_ACCESSED_TIMESTAMP_OFFSET + 4],
        &encode_exfat_date_only(Date::from_calendar_date(2026, Month::March, 12).unwrap()),
    );
    assert_eq!(
        entry_set_after_atime[LAST_ACCESSED_UTC_OFFSET_OFFSET],
        encode_valid_utc_offset_byte(accessed_offset)
    );
    let expected_projected_atime = expected_timestamp(
        Date::from_calendar_date(2026, Month::March, 12).unwrap(),
        Time::MIDNIGHT,
        accessed_offset,
    );
    assert_eq!(directory_inode.atime(), expected_projected_atime);
    assert_eq!(directory_inode.mtime(), modified_before);
    assert_eq!(directory_inode.ctime(), modified_before);

    let requested_mtime = expected_timestamp(
        Date::from_calendar_date(2026, Month::April, 18).unwrap(),
        Time::from_hms_milli(20, 22, 24, 170).unwrap(),
        modified_offset,
    );
    directory_inode.set_mtime(requested_mtime);

    let entry_set_after_mtime = root_entry_set(&disk, ROOT_FILE_ENTRY_INDEX);
    let (expected_modified_timestamp, expected_modified_ten_ms_increment) = encode_exfat_date_time(
        Date::from_calendar_date(2026, Month::April, 18).unwrap(),
        Time::from_hms_milli(20, 22, 24, 170).unwrap(),
    );
    assert_valid_entry_set_checksum(&entry_set_after_mtime);
    assert_bytes_unchanged_except(
        &entry_set_after_atime,
        &entry_set_after_mtime,
        &[
            2..4,
            LAST_MODIFIED_TIMESTAMP_OFFSET..LAST_MODIFIED_TIMESTAMP_OFFSET + 4,
            LAST_MODIFIED_10MS_INCREMENT_OFFSET..LAST_MODIFIED_10MS_INCREMENT_OFFSET + 1,
            LAST_MODIFIED_UTC_OFFSET_OFFSET..LAST_MODIFIED_UTC_OFFSET_OFFSET + 1,
        ],
    );
    assert_eq!(
        &entry_set_after_mtime[LAST_MODIFIED_TIMESTAMP_OFFSET..LAST_MODIFIED_TIMESTAMP_OFFSET + 4],
        &expected_modified_timestamp,
    );
    assert_eq!(
        entry_set_after_mtime[LAST_MODIFIED_10MS_INCREMENT_OFFSET],
        expected_modified_ten_ms_increment
    );
    assert_eq!(
        entry_set_after_mtime[LAST_MODIFIED_UTC_OFFSET_OFFSET],
        encode_valid_utc_offset_byte(modified_offset)
    );
    assert_eq!(
        &entry_set_after_mtime[CREATE_TIMESTAMP_OFFSET..CREATE_TIMESTAMP_OFFSET + 4],
        &entry_set_before[CREATE_TIMESTAMP_OFFSET..CREATE_TIMESTAMP_OFFSET + 4],
    );
    assert_eq!(
        entry_set_after_mtime[CREATE_10MS_INCREMENT_OFFSET],
        entry_set_before[CREATE_10MS_INCREMENT_OFFSET]
    );
    assert_eq!(
        entry_set_after_mtime[CREATE_UTC_OFFSET_OFFSET],
        entry_set_before[CREATE_UTC_OFFSET_OFFSET]
    );
    assert_eq!(directory_inode.atime(), expected_projected_atime);
    assert_eq!(directory_inode.mtime(), requested_mtime);
    assert_eq!(directory_inode.ctime(), requested_mtime);
}

pub(super) fn directory_metadata_projection_and_update_policy_and_timestamp_mutation_root_and_ctime_requests_stay_bounded()
 {
    init_lookup_test_runtime();

    let ordinary_disk = ExfatLookupTestDisk::new();
    ordinary_disk.install_root_directory(
        ROOT_FILE_ENTRY_INDEX,
        "SyntheticDirCtime",
        TEST_CHILD_DIRECTORY_CLUSTER,
    );
    set_directory_entry_metadata(
        &ordinary_disk,
        ROOT_FILE_ENTRY_INDEX,
        FILE_ATTRIBUTE_DIRECTORY,
        (
            Date::from_calendar_date(2026, Month::May, 1).unwrap(),
            Time::from_hms_milli(4, 6, 8, 120).unwrap(),
            UtcOffset::from_whole_seconds(2 * 60 * 60).unwrap(),
        ),
        (
            Date::from_calendar_date(2026, Month::May, 3).unwrap(),
            UtcOffset::from_whole_seconds(60 * 60).unwrap(),
        ),
        (
            Date::from_calendar_date(2026, Month::May, 5).unwrap(),
            Time::from_hms_milli(10, 12, 14, 230).unwrap(),
            UtcOffset::from_whole_seconds(-2 * 60 * 60).unwrap(),
        ),
    );

    let (_fs, ordinary_root_inode) = mount_root(&ordinary_disk, None);
    let ordinary_directory_inode = ordinary_root_inode.lookup("SyntheticDirCtime").unwrap();
    let metadata_before = ordinary_directory_inode.metadata();
    let entry_set_before = root_entry_set(&ordinary_disk, ROOT_FILE_ENTRY_INDEX);
    ordinary_directory_inode.set_ctime(expected_timestamp(
        Date::from_calendar_date(2026, Month::June, 9).unwrap(),
        Time::from_hms_milli(7, 8, 10, 340).unwrap(),
        UtcOffset::UTC,
    ));

    assert_eq!(
        root_entry_set(&ordinary_disk, ROOT_FILE_ENTRY_INDEX),
        entry_set_before
    );
    assert_metadata_unchanged(ordinary_directory_inode.metadata(), metadata_before);
    assert_eq!(
        ordinary_directory_inode.atime(),
        metadata_before.last_access_at
    );
    assert_eq!(
        ordinary_directory_inode.mtime(),
        metadata_before.last_modify_at
    );
    assert_eq!(
        ordinary_directory_inode.ctime(),
        metadata_before.last_meta_change_at
    );

    let root_disk = ExfatLookupTestDisk::new();
    root_disk.install_root_fractured_entry_set(ROOT_FILE_ENTRY_INDEX, "BrokenRootNeighbor");

    let (_fs, root_inode) = mount_root(&root_disk, None);
    let root_metadata_before = root_inode.metadata();
    assert_eq!(
        root_inode
            .set_mode(chmod!(root_metadata_before.mode, a-w))
            .unwrap_err()
            .error(),
        Errno::EOPNOTSUPP
    );
    root_inode.set_atime(expected_timestamp(
        Date::from_calendar_date(2026, Month::July, 3).unwrap(),
        Time::from_hms(13, 0, 0).unwrap(),
        UtcOffset::UTC,
    ));
    root_inode.set_mtime(expected_timestamp(
        Date::from_calendar_date(2026, Month::July, 5).unwrap(),
        Time::from_hms_milli(18, 20, 22, 120).unwrap(),
        UtcOffset::UTC,
    ));
    root_inode.set_ctime(expected_timestamp(
        Date::from_calendar_date(2026, Month::July, 7).unwrap(),
        Time::from_hms_milli(9, 10, 12, 340).unwrap(),
        UtcOffset::UTC,
    ));

    assert_metadata_unchanged(root_inode.metadata(), root_metadata_before);
    assert_eq!(
        lookup_error(&root_inode, "BrokenRootNeighbor"),
        Errno::EUCLEAN
    );
}

pub(super) fn directory_metadata_projection_and_update_policy_and_timestamp_mutation_denials_and_failures_preserve_last_good_state()
 {
    init_lookup_test_runtime();

    let read_only_disk = ExfatLookupTestDisk::new();
    read_only_disk.install_root_directory(
        ROOT_FILE_ENTRY_INDEX,
        "DeniedDir",
        TEST_CHILD_DIRECTORY_CLUSTER,
    );
    let denied_entry_set_before = root_entry_set(&read_only_disk, ROOT_FILE_ENTRY_INDEX);
    let (_fs, read_only_root) = mount_root_with_flags(&read_only_disk, FsFlags::RDONLY, None);
    let denied_directory = read_only_root.lookup("DeniedDir").unwrap();
    let denied_metadata_before = denied_directory.metadata();

    assert_eq!(
        denied_directory
            .set_mode(chmod!(denied_metadata_before.mode, a-w))
            .unwrap_err()
            .error(),
        Errno::EROFS
    );
    assert_eq!(
        denied_directory
            .set_owner(Uid::new_root())
            .unwrap_err()
            .error(),
        Errno::EROFS
    );
    assert_eq!(
        denied_directory
            .set_group(Gid::new_root())
            .unwrap_err()
            .error(),
        Errno::EROFS
    );
    denied_directory.set_atime(expected_timestamp(
        Date::from_calendar_date(2026, Month::August, 3).unwrap(),
        Time::from_hms(13, 0, 0).unwrap(),
        UtcOffset::UTC,
    ));
    denied_directory.set_mtime(expected_timestamp(
        Date::from_calendar_date(2026, Month::August, 5).unwrap(),
        Time::from_hms_milli(18, 20, 22, 120).unwrap(),
        UtcOffset::UTC,
    ));
    denied_directory.set_ctime(expected_timestamp(
        Date::from_calendar_date(2026, Month::August, 7).unwrap(),
        Time::from_hms_milli(9, 10, 12, 340).unwrap(),
        UtcOffset::UTC,
    ));

    assert_eq!(
        root_entry_set(&read_only_disk, ROOT_FILE_ENTRY_INDEX),
        denied_entry_set_before
    );
    assert_metadata_unchanged(denied_directory.metadata(), denied_metadata_before);

    let integrity_disk = ExfatLookupTestDisk::new();
    integrity_disk.install_root_directory(
        ROOT_FILE_ENTRY_INDEX,
        "BrokenDir",
        TEST_CHILD_DIRECTORY_CLUSTER,
    );
    let (_fs, integrity_root) = mount_root(&integrity_disk, None);
    let broken_directory = integrity_root.lookup("BrokenDir").unwrap();
    let integrity_metadata_before = broken_directory.metadata();
    let integrity_entry_set_before = root_entry_set(&integrity_disk, ROOT_FILE_ENTRY_INDEX);
    let mut corrupted_entry_set = integrity_entry_set_before.clone();
    corrupted_entry_set[2] ^= 0x5A;
    integrity_disk.write_root_entries(ROOT_FILE_ENTRY_INDEX, &corrupted_entry_set);
    let metadata_after_corruption = broken_directory.metadata();
    assert_eq!(metadata_after_corruption.ino, integrity_metadata_before.ino);
    assert_eq!(
        metadata_after_corruption.size,
        integrity_metadata_before.size
    );
    assert_eq!(metadata_after_corruption.type_, InodeType::Dir);

    assert_eq!(
        broken_directory
            .set_mode(chmod!(integrity_metadata_before.mode, a-w))
            .unwrap_err()
            .error(),
        Errno::EUCLEAN
    );
    broken_directory.set_atime(expected_timestamp(
        Date::from_calendar_date(2026, Month::September, 2).unwrap(),
        Time::from_hms(6, 0, 0).unwrap(),
        UtcOffset::UTC,
    ));
    broken_directory.set_mtime(expected_timestamp(
        Date::from_calendar_date(2026, Month::September, 4).unwrap(),
        Time::from_hms_milli(8, 10, 12, 140).unwrap(),
        UtcOffset::UTC,
    ));

    assert_eq!(
        root_entry_set(&integrity_disk, ROOT_FILE_ENTRY_INDEX),
        corrupted_entry_set
    );
    assert_metadata_unchanged(broken_directory.metadata(), metadata_after_corruption);

    let writable_disk = ExfatLookupTestDisk::new();
    writable_disk.install_root_directory(
        ROOT_FILE_ENTRY_INDEX,
        "IoFailureDir",
        TEST_CHILD_DIRECTORY_CLUSTER,
    );
    set_directory_entry_metadata(
        &writable_disk,
        ROOT_FILE_ENTRY_INDEX,
        FILE_ATTRIBUTE_DIRECTORY,
        (
            Date::from_calendar_date(2026, Month::October, 1).unwrap(),
            Time::from_hms_milli(2, 4, 6, 120).unwrap(),
            UtcOffset::UTC,
        ),
        (
            Date::from_calendar_date(2026, Month::October, 3).unwrap(),
            UtcOffset::from_whole_seconds(60 * 60).unwrap(),
        ),
        (
            Date::from_calendar_date(2026, Month::October, 5).unwrap(),
            Time::from_hms_milli(8, 10, 12, 140).unwrap(),
            UtcOffset::from_whole_seconds(-60 * 60).unwrap(),
        ),
    );
    let failing_write_disk = ExfatLookupToggleFailingWriteDisk::new(
        writable_disk.clone(),
        writable_disk.root_directory_offset(),
        writable_disk.root_cluster_size(),
    );
    let block_device: Arc<dyn BlockDevice> = failing_write_disk.clone();
    let (_fs, io_root) = mount_root_from_block_device(block_device, FsFlags::empty(), None);
    let io_directory = io_root.lookup("IoFailureDir").unwrap();
    let io_entry_set_before = root_entry_set(&writable_disk, ROOT_FILE_ENTRY_INDEX);
    let io_metadata_before = io_directory.metadata();

    failing_write_disk.enable_failures();
    assert_eq!(
        io_directory
            .set_mode(chmod!(io_metadata_before.mode, a-w))
            .unwrap_err()
            .error(),
        Errno::EIO
    );
    io_directory.set_mtime(expected_timestamp(
        Date::from_calendar_date(2026, Month::October, 9).unwrap(),
        Time::from_hms_milli(14, 16, 18, 160).unwrap(),
        UtcOffset::from_whole_seconds(-60 * 60).unwrap(),
    ));

    assert_eq!(
        root_entry_set(&writable_disk, ROOT_FILE_ENTRY_INDEX),
        io_entry_set_before
    );
    assert_metadata_unchanged(io_directory.metadata(), io_metadata_before);
}
