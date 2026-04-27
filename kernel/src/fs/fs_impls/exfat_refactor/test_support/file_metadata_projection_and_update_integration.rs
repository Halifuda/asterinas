// SPDX-License-Identifier: MPL-2.0

use alloc::{sync::Arc, vec::Vec};
use core::{ops::Range, time::Duration};

use aster_block::BlockDevice;
use time::{Date, Month, PrimitiveDateTime, Time, UtcOffset};

use super::*;
use crate::process::Uid;

const CREATE_TIMESTAMP_OFFSET: usize = 8;
const LAST_MODIFIED_TIMESTAMP_OFFSET: usize = 12;
const LAST_ACCESSED_TIMESTAMP_OFFSET: usize = 16;
const CREATE_10MS_INCREMENT_OFFSET: usize = 20;
const LAST_MODIFIED_10MS_INCREMENT_OFFSET: usize = 21;
const CREATE_UTC_OFFSET_OFFSET: usize = 22;
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

fn set_regular_file_entry_metadata(
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

fn assert_valid_entry_set_checksum(entry_set: &[u8]) {
    let checksum = entry_set_checksum(entry_set, usize::from(entry_set[1]));
    assert_eq!(u16::from_le_bytes([entry_set[2], entry_set[3]]), checksum);
}

fn assert_bytes_unchanged_except(before: &[u8], after: &[u8], allowed_ranges: &[Range<usize>]) {
    assert_eq!(before.len(), after.len());

    for index in 0..before.len() {
        if allowed_ranges.iter().any(|range| range.contains(&index)) {
            continue;
        }
        assert_eq!(
            after[index],
            before[index],
            "unexpected durable byte change at offset {index}",
        );
    }
}

fn assert_projected_identity(metadata: Metadata, size: usize, allocated_sectors: usize) {
    assert_eq!(metadata.size, size);
    assert_eq!(metadata.nr_sectors_allocated, allocated_sectors);
    assert_eq!(metadata.uid, Uid::new_root());
    assert_eq!(metadata.gid, Gid::new_root());
    assert_eq!(metadata.type_, InodeType::File);
}

fn wait_for_blocked_flush(flush_control_disk: &ExfatLookupFlushControlDisk) {
    while !flush_control_disk.flush_started() {
        Thread::yield_now();
    }
}

pub(super) fn file_metadata_projection_update_integration_success_path_live_and_reread_projection_agree_after_sync() {
    init_lookup_test_runtime();

    let disk = ExfatLookupTestDisk::new();
    let cluster_size = disk.root_cluster_size();
    let file_size = cluster_size + 37;
    disk.install_root_file_with_cluster_chain(
        ROOT_FILE_ENTRY_INDEX,
        "IntegratedMeta",
        TEST_REGULAR_FILE_CLUSTER,
        file_size,
        file_size,
        true,
        &[TEST_REGULAR_FILE_CLUSTER, TEST_CONTIGUOUS_SECOND_CLUSTER],
    );

    let create_date = Date::from_calendar_date(2026, Month::January, 2).unwrap();
    let create_time = Time::from_hms_milli(3, 4, 6, 120).unwrap();
    let create_offset = UtcOffset::from_whole_seconds(2 * 60 * 60).unwrap();
    let accessed_date = Date::from_calendar_date(2026, Month::January, 5).unwrap();
    let accessed_offset = UtcOffset::from_whole_seconds(60 * 60).unwrap();
    let modified_date = Date::from_calendar_date(2026, Month::January, 8).unwrap();
    let modified_time = Time::from_hms_milli(9, 10, 12, 340).unwrap();
    let modified_offset = UtcOffset::from_whole_seconds(-60 * 60).unwrap();
    set_regular_file_entry_metadata(
        &disk,
        ROOT_FILE_ENTRY_INDEX,
        FILE_ATTRIBUTE_REGULAR,
        (create_date, create_time, create_offset),
        (accessed_date, accessed_offset),
        (modified_date, modified_time, modified_offset),
    );

    let (_fs, root_inode) = mount_root(&disk, None);
    let file_inode = root_inode.lookup("IntegratedMeta").unwrap();
    let entry_set_before = root_entry_set(&disk, ROOT_FILE_ENTRY_INDEX);
    let expected_atime = expected_timestamp(accessed_date, Time::MIDNIGHT, accessed_offset);
    let requested_mtime = expected_timestamp(
        Date::from_calendar_date(2026, Month::February, 11).unwrap(),
        Time::from_hms_milli(14, 16, 18, 170).unwrap(),
        modified_offset,
    );
    let allocated_sectors = 2 * (cluster_size / SECTOR_SIZE);

    assert_projected_identity(file_inode.metadata(), file_size, allocated_sectors);
    assert_eq!(file_inode.owner().unwrap(), Uid::new_root());
    assert_eq!(file_inode.group().unwrap(), Gid::new_root());
    assert_eq!(file_inode.atime(), expected_atime);

    file_inode
        .set_mode(chmod!(file_inode.mode().unwrap(), a-w))
        .unwrap();
    file_inode.set_mtime(requested_mtime);
    let _ = disk.take_observed_bios();
    file_inode.sync_all().unwrap();
    assert_flush_only(&disk.take_observed_bios());

    let live_metadata = file_inode.metadata();
    let reread_inode = root_inode.lookup("IntegratedMeta").unwrap();
    let reread_metadata = reread_inode.metadata();
    let entry_set_after = root_entry_set(&disk, ROOT_FILE_ENTRY_INDEX);

    assert_projected_identity(live_metadata, file_size, allocated_sectors);
    assert_projected_identity(reread_metadata, file_size, allocated_sectors);
    assert_eq!(live_metadata.mode, chmod!(mkmod!(u+rw, g+r, o+r), a-w));
    assert_eq!(reread_metadata.mode, live_metadata.mode);
    assert_eq!(live_metadata.uid, reread_metadata.uid);
    assert_eq!(live_metadata.gid, reread_metadata.gid);
    assert_eq!(live_metadata.last_access_at, expected_atime);
    assert_eq!(reread_metadata.last_access_at, expected_atime);
    assert_eq!(live_metadata.last_modify_at, requested_mtime);
    assert_eq!(reread_metadata.last_modify_at, requested_mtime);
    assert_eq!(reread_inode.size(), file_size);
    assert_eq!(
        stream_lengths(&entry_set_after),
        (file_size as u64, file_size as u64)
    );
    assert_eq!(decode_entry_name(&entry_set_after), "IntegratedMeta".encode_utf16().collect::<Vec<_>>());
    assert_valid_entry_set_checksum(&entry_set_after);
    assert_bytes_unchanged_except(
        &entry_set_before,
        &entry_set_after,
        &[
            2..6,
            LAST_MODIFIED_TIMESTAMP_OFFSET..LAST_MODIFIED_TIMESTAMP_OFFSET + 4,
            LAST_MODIFIED_10MS_INCREMENT_OFFSET..LAST_MODIFIED_10MS_INCREMENT_OFFSET + 1,
            LAST_MODIFIED_UTC_OFFSET_OFFSET..LAST_MODIFIED_UTC_OFFSET_OFFSET + 1,
        ],
    );
    assert_eq!(
        &entry_set_after[CREATE_TIMESTAMP_OFFSET..CREATE_TIMESTAMP_OFFSET + 4],
        &entry_set_before[CREATE_TIMESTAMP_OFFSET..CREATE_TIMESTAMP_OFFSET + 4],
    );
    assert_eq!(
        entry_set_after[CREATE_10MS_INCREMENT_OFFSET],
        entry_set_before[CREATE_10MS_INCREMENT_OFFSET]
    );
    assert_eq!(
        entry_set_after[CREATE_UTC_OFFSET_OFFSET],
        entry_set_before[CREATE_UTC_OFFSET_OFFSET]
    );
}

pub(super) fn file_metadata_projection_update_integration_failure_maintenance_preserves_state_and_retry() {
    init_lookup_test_runtime();

    let denied_disk = ExfatLookupTestDisk::new();
    denied_disk.install_root_file(ROOT_FILE_ENTRY_INDEX, "DeniedMeta");
    let denied_entry_set_before = root_entry_set(&denied_disk, ROOT_FILE_ENTRY_INDEX);
    let (_fs, denied_root) = mount_root_with_flags(&denied_disk, FsFlags::RDONLY, None);
    let denied_file = denied_root.lookup("DeniedMeta").unwrap();
    let denied_metadata_before = denied_file.metadata();

    assert_eq!(
        denied_file.set_owner(Uid::new(42)).unwrap_err().error(),
        Errno::EROFS
    );
    denied_file.set_mtime(expected_timestamp(
        Date::from_calendar_date(2026, Month::March, 3).unwrap(),
        Time::from_hms_milli(10, 12, 14, 160).unwrap(),
        UtcOffset::UTC,
    ));
    assert_eq!(
        root_entry_set(&denied_disk, ROOT_FILE_ENTRY_INDEX),
        denied_entry_set_before
    );
    assert_metadata_unchanged(denied_file.metadata(), denied_metadata_before);

    let writable_disk = ExfatLookupTestDisk::new();
    writable_disk.install_root_file(ROOT_FILE_ENTRY_INDEX, "RetryMeta");
    set_regular_file_entry_metadata(
        &writable_disk,
        ROOT_FILE_ENTRY_INDEX,
        FILE_ATTRIBUTE_REGULAR,
        (
            Date::from_calendar_date(2026, Month::April, 1).unwrap(),
            Time::from_hms_milli(2, 4, 6, 120).unwrap(),
            UtcOffset::UTC,
        ),
        (
            Date::from_calendar_date(2026, Month::April, 2).unwrap(),
            UtcOffset::UTC,
        ),
        (
            Date::from_calendar_date(2026, Month::April, 3).unwrap(),
            Time::from_hms_milli(8, 10, 12, 140).unwrap(),
            UtcOffset::UTC,
        ),
    );
    let failing_write_disk = ExfatLookupToggleFailingWriteDisk::new(
        writable_disk.clone(),
        writable_disk.root_directory_offset(),
        writable_disk.root_cluster_size(),
    );
    let block_device: Arc<dyn BlockDevice> = failing_write_disk.clone();
    let (_fs, failing_root) = mount_root_from_block_device(block_device, FsFlags::empty(), None);
    let failing_file = failing_root.lookup("RetryMeta").unwrap();
    let io_entry_set_before = root_entry_set(&writable_disk, ROOT_FILE_ENTRY_INDEX);
    let io_metadata_before = failing_file.metadata();

    failing_write_disk.enable_failures();
    assert_eq!(
        failing_file
            .set_mode(chmod!(io_metadata_before.mode, a-w))
            .unwrap_err()
            .error(),
        Errno::EIO
    );
    failing_file.set_mtime(expected_timestamp(
        Date::from_calendar_date(2026, Month::April, 9).unwrap(),
        Time::from_hms_milli(14, 16, 18, 160).unwrap(),
        UtcOffset::UTC,
    ));
    assert_eq!(
        root_entry_set(&writable_disk, ROOT_FILE_ENTRY_INDEX),
        io_entry_set_before
    );
    assert_metadata_unchanged(failing_file.metadata(), io_metadata_before);

    let (_retry_fs, retry_root) = mount_root(&writable_disk, None);
    let retry_file = retry_root.lookup("RetryMeta").unwrap();
    let retry_mtime = expected_timestamp(
        Date::from_calendar_date(2026, Month::April, 11).unwrap(),
        Time::from_hms_milli(20, 22, 24, 180).unwrap(),
        UtcOffset::UTC,
    );
    retry_file
        .set_mode(chmod!(retry_file.mode().unwrap(), a-w))
        .unwrap();
    retry_file.set_mtime(retry_mtime);

    let retry_entry_set = root_entry_set(&writable_disk, ROOT_FILE_ENTRY_INDEX);
    assert_valid_entry_set_checksum(&retry_entry_set);
    assert_eq!(retry_file.mtime(), retry_mtime);
    assert_eq!(retry_file.mode().unwrap(), chmod!(mkmod!(u+rw, g+r, o+r), a-w));
}

pub(super) fn file_metadata_projection_update_integration_repeated_calls_keep_metadata_stable() {
    init_lookup_test_runtime();

    let disk = ExfatLookupTestDisk::new();
    disk.install_root_file(ROOT_FILE_ENTRY_INDEX, "StableMeta");
    set_regular_file_entry_metadata(
        &disk,
        ROOT_FILE_ENTRY_INDEX,
        FILE_ATTRIBUTE_REGULAR,
        (
            Date::from_calendar_date(2026, Month::May, 1).unwrap(),
            Time::from_hms_milli(2, 4, 6, 120).unwrap(),
            UtcOffset::UTC,
        ),
        (
            Date::from_calendar_date(2026, Month::May, 2).unwrap(),
            UtcOffset::UTC,
        ),
        (
            Date::from_calendar_date(2026, Month::May, 3).unwrap(),
            Time::from_hms_milli(8, 10, 12, 140).unwrap(),
            UtcOffset::UTC,
        ),
    );

    let (_fs, root_inode) = mount_root(&disk, None);
    let file_inode = root_inode.lookup("StableMeta").unwrap();
    let entry_set_before = root_entry_set(&disk, ROOT_FILE_ENTRY_INDEX);
    let metadata_before = file_inode.metadata();

    for _ in 0..3 {
        assert_metadata_unchanged(file_inode.metadata(), metadata_before);
        assert_eq!(file_inode.owner().unwrap(), Uid::new_root());
        assert_eq!(file_inode.group().unwrap(), Gid::new_root());
        file_inode.set_owner(Uid::new_root()).unwrap();
        file_inode.set_group(Gid::new_root()).unwrap();
        file_inode.set_mode(file_inode.mode().unwrap()).unwrap();
    }
    assert_eq!(root_entry_set(&disk, ROOT_FILE_ENTRY_INDEX), entry_set_before);
    assert_metadata_unchanged(file_inode.metadata(), metadata_before);

    let read_only_mode = chmod!(file_inode.mode().unwrap(), a-w);
    file_inode.set_mode(read_only_mode).unwrap();
    let entry_set_after_transition = root_entry_set(&disk, ROOT_FILE_ENTRY_INDEX);
    assert_valid_entry_set_checksum(&entry_set_after_transition);
    assert_bytes_unchanged_except(
        &entry_set_before,
        &entry_set_after_transition,
        &[2..6],
    );

    file_inode.set_mode(read_only_mode).unwrap();
    assert_eq!(
        root_entry_set(&disk, ROOT_FILE_ENTRY_INDEX),
        entry_set_after_transition
    );
    assert_eq!(file_inode.mode().unwrap(), read_only_mode);
    assert_eq!(file_inode.owner().unwrap(), Uid::new_root());
    assert_eq!(file_inode.group().unwrap(), Gid::new_root());
}

pub(super) fn file_metadata_projection_update_integration_concurrency_serializes_metadata_and_content_updates() {
    init_lookup_test_runtime();

    let disk = ExfatLookupTestDisk::new();
    disk.install_root_file_with_contents(
        ROOT_FILE_ENTRY_INDEX,
        "RaceMeta",
        TEST_REGULAR_FILE_CLUSTER,
        b"abcdefgh",
    );
    set_regular_file_entry_metadata(
        &disk,
        ROOT_FILE_ENTRY_INDEX,
        FILE_ATTRIBUTE_REGULAR,
        (
            Date::from_calendar_date(2026, Month::June, 1).unwrap(),
            Time::from_hms_milli(2, 4, 6, 120).unwrap(),
            UtcOffset::UTC,
        ),
        (
            Date::from_calendar_date(2026, Month::June, 2).unwrap(),
            UtcOffset::UTC,
        ),
        (
            Date::from_calendar_date(2026, Month::June, 3).unwrap(),
            Time::from_hms_milli(8, 10, 12, 140).unwrap(),
            UtcOffset::UTC,
        ),
    );

    let flush_control_disk = ExfatLookupFlushControlDisk::new(disk.clone());
    let block_device: Arc<dyn BlockDevice> = flush_control_disk.clone();
    let (_fs, root_inode) = mount_root_from_block_device(block_device, FsFlags::empty(), None);
    let file_inode = root_inode.lookup("RaceMeta").unwrap();
    let requested_atime = expected_timestamp(
        Date::from_calendar_date(2026, Month::June, 9).unwrap(),
        Time::MIDNIGHT,
        UtcOffset::UTC,
    );
    let initial_size = file_inode.size();
    let _ = disk.take_observed_bios();

    flush_control_disk.enable_blocking_flush();
    let metadata_done = Arc::new(Mutex::new(false));
    let metadata_thread = {
        let file_inode = file_inode.clone();
        let metadata_done = metadata_done.clone();
        ThreadOptions::new(move || {
            file_inode.set_atime(requested_atime);
            *metadata_done.lock() = true;
        })
        .spawn()
    };

    wait_for_blocked_flush(&flush_control_disk);
    let append_result = Arc::new(Mutex::new(None));
    let append_thread = {
        let file_inode = file_inode.clone();
        let append_result = append_result.clone();
        ThreadOptions::new(move || {
            *append_result.lock() = Some(write_bytes_append(&file_inode, b"TAIL"));
        })
        .spawn()
    };
    Thread::yield_now();
    flush_control_disk.release_blocked_flush();
    metadata_thread.join();
    append_thread.join();

    assert!(*metadata_done.lock());
    let append_result = append_result.lock().take().unwrap();
    assert_eq!(append_result.map_err(|error| error.error()), Ok(4));
    assert_eq!(file_inode.atime(), requested_atime);
    assert_eq!(file_inode.size(), initial_size + 4);

    let reread_inode = root_inode.lookup("RaceMeta").unwrap();
    let entry_set_after = root_entry_set(&disk, ROOT_FILE_ENTRY_INDEX);
    let mut visible_bytes = Vec::from([0u8; 12]);

    assert_eq!(reread_inode.atime(), requested_atime);
    assert_eq!(reread_inode.size(), initial_size + 4);
    assert_eq!(reread_inode.read_bytes_at(0, &mut visible_bytes).unwrap(), 12);
    assert_eq!(visible_bytes.as_slice(), b"abcdefghTAIL");
    assert_eq!(
        stream_lengths(&entry_set_after),
        ((initial_size + 4) as u64, (initial_size + 4) as u64)
    );
    assert_valid_entry_set_checksum(&entry_set_after);
    assert_eq!(
        &entry_set_after[LAST_ACCESSED_TIMESTAMP_OFFSET..LAST_ACCESSED_TIMESTAMP_OFFSET + 4],
        &encode_exfat_date_only(Date::from_calendar_date(2026, Month::June, 9).unwrap()),
    );
}
