// SPDX-License-Identifier: MPL-2.0

use alloc::sync::Arc;
use core::{ops::Range, time::Duration};

use aster_block::BlockDevice;
use time::{Date, Month, PrimitiveDateTime, Time, UtcOffset};

use super::{
    assert_bytes_unchanged_except, assert_flush_only, assert_metadata_unchanged,
    assert_valid_entry_set_checksum, encode_exfat_date, encode_exfat_date_only,
    encode_exfat_date_time, encode_valid_utc_offset_byte, expected_timestamp,
    init_lookup_test_runtime, mount_root, root_entry_set, set_regular_file_entry_metadata,
    ExfatLookupTestDisk, ExfatLookupToggleFailingWriteDisk, FILE_ATTRIBUTES_OFFSET,
    FILE_ATTRIBUTE_REGULAR, ROOT_FILE_ENTRY_INDEX,
};
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

pub(super) fn file_metadata_projection_and_update_policy_and_timestamp_mutation_updates_durable_read_only_projection_and_metadata_only_dirty_state(
) {
    init_lookup_test_runtime();

    let disk = ExfatLookupTestDisk::new();
    disk.install_root_file(ROOT_FILE_ENTRY_INDEX, "ModeFile");
    set_regular_file_entry_metadata(
        &disk,
        ROOT_FILE_ENTRY_INDEX,
        FILE_ATTRIBUTE_REGULAR,
        (
            Date::from_calendar_date(2026, Month::January, 4).unwrap(),
            Time::from_hms_milli(1, 2, 4, 120).unwrap(),
            UtcOffset::from_whole_seconds(2 * 60 * 60).unwrap(),
        ),
        (
            Date::from_calendar_date(2026, Month::January, 7).unwrap(),
            UtcOffset::from_whole_seconds(60 * 60).unwrap(),
        ),
        (
            Date::from_calendar_date(2026, Month::January, 9).unwrap(),
            Time::from_hms_milli(5, 6, 8, 230).unwrap(),
            UtcOffset::from_whole_seconds(90 * 60).unwrap(),
        ),
    );

    let (_fs, root_inode) = mount_root(&disk, None);
    let file_inode = root_inode.lookup("ModeFile").unwrap();
    let metadata_before = file_inode.metadata();
    let entry_set_before = root_entry_set(&disk, ROOT_FILE_ENTRY_INDEX);

    let requested_mode = chmod!(metadata_before.mode, a-w);
    let _ = disk.take_observed_bios();
    file_inode.set_mode(requested_mode).unwrap();

    let entry_set_after = root_entry_set(&disk, ROOT_FILE_ENTRY_INDEX);
    assert_valid_entry_set_checksum(&entry_set_after);
    assert_eq!(
        u16::from_le_bytes([
            entry_set_after[FILE_ATTRIBUTES_OFFSET],
            entry_set_after[FILE_ATTRIBUTES_OFFSET + 1],
        ]),
        FILE_ATTRIBUTE_REGULAR | FILE_ATTRIBUTE_READ_ONLY
    );
    assert_bytes_unchanged_except(&entry_set_before, &entry_set_after, &[2..6]);
    assert_eq!(file_inode.mode().unwrap(), requested_mode);
    assert_eq!(file_inode.metadata().mode, requested_mode);

    let _ = disk.take_observed_bios();
    file_inode.sync_data().unwrap();
    assert!(disk.take_observed_bios().is_empty());

    file_inode.sync_all().unwrap();
    assert_flush_only(&disk.take_observed_bios());

    file_inode.sync_all().unwrap();
    assert!(disk.take_observed_bios().is_empty());
}

pub(super) fn file_metadata_projection_and_update_policy_and_timestamp_mutation_owner_group_confirm_projection_and_refuse_escape(
) {
    init_lookup_test_runtime();

    let disk = ExfatLookupTestDisk::new();
    disk.install_root_file(ROOT_FILE_ENTRY_INDEX, "OwnerFile");

    let (_fs, root_inode) = mount_root(&disk, None);
    let file_inode = root_inode.lookup("OwnerFile").unwrap();
    let metadata_before = file_inode.metadata();
    let entry_set_before = root_entry_set(&disk, ROOT_FILE_ENTRY_INDEX);

    assert_eq!(file_inode.owner().unwrap(), Uid::new_root());
    assert_eq!(file_inode.group().unwrap(), Gid::new_root());

    file_inode.set_owner(Uid::new_root()).unwrap();
    file_inode.set_group(Gid::new_root()).unwrap();
    assert_eq!(
        file_inode.set_owner(Uid::new(42)).unwrap_err().error(),
        Errno::EPERM
    );
    assert_eq!(
        file_inode.set_group(Gid::new(24)).unwrap_err().error(),
        Errno::EPERM
    );

    assert_eq!(
        root_entry_set(&disk, ROOT_FILE_ENTRY_INDEX),
        entry_set_before
    );
    assert_metadata_unchanged(file_inode.metadata(), metadata_before);
}

pub(super) fn file_metadata_projection_and_update_policy_and_timestamp_mutation_rewrites_only_owned_timestamp_families(
) {
    init_lookup_test_runtime();

    let disk = ExfatLookupTestDisk::new();
    disk.install_root_file(ROOT_FILE_ENTRY_INDEX, "TimeFile");

    let create_date = Date::from_calendar_date(2026, Month::February, 2).unwrap();
    let create_time = Time::from_hms_milli(3, 4, 6, 120).unwrap();
    let create_offset = UtcOffset::from_whole_seconds(2 * 60 * 60).unwrap();
    let accessed_date = Date::from_calendar_date(2026, Month::February, 5).unwrap();
    let accessed_offset = UtcOffset::from_whole_seconds(60 * 60).unwrap();
    let modified_date = Date::from_calendar_date(2026, Month::February, 9).unwrap();
    let modified_time = Time::from_hms_milli(10, 12, 14, 230).unwrap();
    let modified_offset = UtcOffset::from_whole_seconds(-2 * 60 * 60).unwrap();
    set_regular_file_entry_metadata(
        &disk,
        ROOT_FILE_ENTRY_INDEX,
        FILE_ATTRIBUTE_REGULAR,
        (create_date, create_time, create_offset),
        (accessed_date, accessed_offset),
        (modified_date, modified_time, modified_offset),
    );

    let (_fs, root_inode) = mount_root(&disk, None);
    let file_inode = root_inode.lookup("TimeFile").unwrap();
    let modified_before = file_inode.mtime();
    let entry_set_before = root_entry_set(&disk, ROOT_FILE_ENTRY_INDEX);

    let requested_atime = expected_timestamp(
        Date::from_calendar_date(2026, Month::March, 12).unwrap(),
        Time::from_hms(16, 14, 0).unwrap(),
        accessed_offset,
    );
    file_inode.set_atime(requested_atime);

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
    assert_eq!(file_inode.atime(), requested_atime);
    assert_eq!(file_inode.mtime(), modified_before);

    let requested_mtime = expected_timestamp(
        Date::from_calendar_date(2026, Month::April, 18).unwrap(),
        Time::from_hms_milli(20, 22, 24, 170).unwrap(),
        modified_offset,
    );
    file_inode.set_mtime(requested_mtime);

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
    assert_eq!(file_inode.atime(), requested_atime);
    assert_eq!(file_inode.mtime(), requested_mtime);
}

pub(super) fn file_metadata_projection_and_update_policy_and_timestamp_mutation_treats_ctime_as_synthetic_only(
) {
    init_lookup_test_runtime();

    let disk = ExfatLookupTestDisk::new();
    disk.install_root_file(ROOT_FILE_ENTRY_INDEX, "SyntheticCtime");
    set_regular_file_entry_metadata(
        &disk,
        ROOT_FILE_ENTRY_INDEX,
        FILE_ATTRIBUTE_REGULAR,
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

    let (_fs, root_inode) = mount_root(&disk, None);
    let file_inode = root_inode.lookup("SyntheticCtime").unwrap();
    let metadata_before = file_inode.metadata();
    let entry_set_before = root_entry_set(&disk, ROOT_FILE_ENTRY_INDEX);
    let requested_ctime = expected_timestamp(
        Date::from_calendar_date(2026, Month::June, 9).unwrap(),
        Time::from_hms_milli(7, 8, 10, 340).unwrap(),
        UtcOffset::UTC,
    );

    let _ = disk.take_observed_bios();
    file_inode.set_ctime(requested_ctime);

    assert_eq!(
        root_entry_set(&disk, ROOT_FILE_ENTRY_INDEX),
        entry_set_before
    );
    assert_eq!(file_inode.atime(), metadata_before.last_access_at);
    assert_eq!(file_inode.mtime(), metadata_before.last_modify_at);
    assert_eq!(file_inode.ctime(), requested_ctime);
    assert_eq!(file_inode.metadata().last_meta_change_at, requested_ctime);

    let _ = disk.take_observed_bios();
    file_inode.sync_all().unwrap();
    assert!(disk.take_observed_bios().is_empty());
}

pub(super) fn file_metadata_projection_and_update_policy_and_timestamp_mutation_policy_denial_and_io_failure_preserve_last_good_state(
) {
    init_lookup_test_runtime();

    let read_only_disk = ExfatLookupTestDisk::new();
    read_only_disk.install_root_file(ROOT_FILE_ENTRY_INDEX, "DeniedFile");
    let denied_entry_set_before = root_entry_set(&read_only_disk, ROOT_FILE_ENTRY_INDEX);
    let (_fs, read_only_root) = mount_root_with_flags(&read_only_disk, FsFlags::RDONLY, None);
    let denied_file = read_only_root.lookup("DeniedFile").unwrap();
    let denied_metadata_before = denied_file.metadata();

    assert_eq!(
        denied_file
            .set_mode(chmod!(denied_metadata_before.mode, a-w))
            .unwrap_err()
            .error(),
        Errno::EROFS
    );
    denied_file.set_atime(expected_timestamp(
        Date::from_calendar_date(2026, Month::July, 3).unwrap(),
        Time::from_hms(13, 0, 0).unwrap(),
        UtcOffset::UTC,
    ));
    denied_file.set_mtime(expected_timestamp(
        Date::from_calendar_date(2026, Month::July, 5).unwrap(),
        Time::from_hms_milli(18, 20, 22, 120).unwrap(),
        UtcOffset::UTC,
    ));
    denied_file.set_ctime(expected_timestamp(
        Date::from_calendar_date(2026, Month::July, 7).unwrap(),
        Time::from_hms_milli(9, 10, 12, 340).unwrap(),
        UtcOffset::UTC,
    ));

    assert_eq!(
        root_entry_set(&read_only_disk, ROOT_FILE_ENTRY_INDEX),
        denied_entry_set_before
    );
    assert_metadata_unchanged(denied_file.metadata(), denied_metadata_before);

    let writable_disk = ExfatLookupTestDisk::new();
    writable_disk.install_root_file(ROOT_FILE_ENTRY_INDEX, "IoFailureFile");
    set_regular_file_entry_metadata(
        &writable_disk,
        ROOT_FILE_ENTRY_INDEX,
        FILE_ATTRIBUTE_REGULAR,
        (
            Date::from_calendar_date(2026, Month::August, 1).unwrap(),
            Time::from_hms_milli(2, 4, 6, 120).unwrap(),
            UtcOffset::UTC,
        ),
        (
            Date::from_calendar_date(2026, Month::August, 3).unwrap(),
            UtcOffset::from_whole_seconds(60 * 60).unwrap(),
        ),
        (
            Date::from_calendar_date(2026, Month::August, 5).unwrap(),
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
    let io_file = io_root.lookup("IoFailureFile").unwrap();
    let io_entry_set_before = root_entry_set(&writable_disk, ROOT_FILE_ENTRY_INDEX);
    let io_metadata_before = io_file.metadata();

    failing_write_disk.enable_failures();
    assert_eq!(
        io_file
            .set_mode(chmod!(io_metadata_before.mode, a-w))
            .unwrap_err()
            .error(),
        Errno::EIO
    );
    io_file.set_mtime(expected_timestamp(
        Date::from_calendar_date(2026, Month::August, 9).unwrap(),
        Time::from_hms_milli(14, 16, 18, 160).unwrap(),
        UtcOffset::from_whole_seconds(-60 * 60).unwrap(),
    ));

    assert_eq!(
        root_entry_set(&writable_disk, ROOT_FILE_ENTRY_INDEX),
        io_entry_set_before
    );
    assert_metadata_unchanged(io_file.metadata(), io_metadata_before);
}
