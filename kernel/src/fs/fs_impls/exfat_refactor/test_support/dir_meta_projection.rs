// SPDX-License-Identifier: MPL-2.0

use core::time::Duration;

use time::{Date, Month, PrimitiveDateTime, Time, UtcOffset};

use super::{
    ExfatLookupTestDisk, FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTES_OFFSET, ROOT_FILE_ENTRY_INDEX,
    ROOT_SECOND_FILE_ENTRY_INDEX, TEST_CHILD_DIRECTORY_CLUSTER, assert_metadata_unchanged,
    encode_exfat_date, encode_exfat_date_only, encode_exfat_date_time,
    encode_valid_utc_offset_byte, entry_set_checksum, expected_timestamp, init_lookup_test_runtime,
    lookup_error, mount_root, root_entry_set, set_directory_entry_metadata,
};

const CREATE_TIMESTAMP_OFFSET: usize = 8;
const LAST_MODIFIED_TIMESTAMP_OFFSET: usize = 12;
const LAST_ACCESSED_TIMESTAMP_OFFSET: usize = 16;
const CREATE_10MS_INCREMENT_OFFSET: usize = 20;
const LAST_MODIFIED_10MS_INCREMENT_OFFSET: usize = 21;
const CREATE_UTC_OFFSET_OFFSET: usize = 22;
const LAST_MODIFIED_UTC_OFFSET_OFFSET: usize = 23;
const LAST_ACCESSED_UTC_OFFSET_OFFSET: usize = 24;
const FILE_ATTRIBUTE_READ_ONLY: u16 = 0x0001;

pub(super) fn directory_metadata_projection_and_update_projection_substrate_projects_ordinary_directory_from_validated_self_entry_set()
 {
    init_lookup_test_runtime();

    let disk = ExfatLookupTestDisk::new();
    let cluster_size = disk.root_cluster_size();
    disk.install_root_directory(
        ROOT_FILE_ENTRY_INDEX,
        "ProjectedDir",
        TEST_CHILD_DIRECTORY_CLUSTER,
    );

    let create_date = Date::from_calendar_date(2026, Month::January, 9).unwrap();
    let create_time = Time::from_hms_milli(3, 4, 6, 120).unwrap();
    let create_offset = UtcOffset::from_whole_seconds(2 * 60 * 60).unwrap();
    let accessed_date = Date::from_calendar_date(2026, Month::February, 7).unwrap();
    let accessed_offset = UtcOffset::from_whole_seconds(60 * 60).unwrap();
    let modified_date = Date::from_calendar_date(2026, Month::March, 11).unwrap();
    let modified_time = Time::from_hms_milli(14, 16, 18, 230).unwrap();
    let modified_offset = UtcOffset::from_whole_seconds(-2 * 60 * 60).unwrap();
    set_directory_entry_metadata(
        &disk,
        ROOT_FILE_ENTRY_INDEX,
        FILE_ATTRIBUTE_DIRECTORY | FILE_ATTRIBUTE_READ_ONLY,
        (create_date, create_time, create_offset),
        (accessed_date, accessed_offset),
        (modified_date, modified_time, modified_offset),
    );

    let (_fs, root_inode) = mount_root(&disk, None);
    let directory_inode = root_inode.lookup("ProjectedDir").unwrap();
    let metadata = directory_inode.metadata();
    let expected_mode = chmod!(root_inode.mode().unwrap(), a-w);
    let expected_atime = expected_timestamp(accessed_date, Time::MIDNIGHT, accessed_offset);
    let expected_mtime = expected_timestamp(modified_date, modified_time, modified_offset);

    assert_eq!(directory_inode.type_(), InodeType::Dir);
    assert_eq!(metadata.type_, InodeType::Dir);
    assert_eq!(directory_inode.size(), cluster_size);
    assert_eq!(metadata.size, cluster_size);
    assert_eq!(metadata.nr_sectors_allocated, cluster_size / SECTOR_SIZE);
    assert_eq!(metadata.mode, expected_mode);
    assert_eq!(directory_inode.mode().unwrap(), expected_mode);
    assert_eq!(metadata.uid, root_inode.owner().unwrap());
    assert_eq!(metadata.gid, root_inode.group().unwrap());
    assert_eq!(directory_inode.owner().unwrap(), metadata.uid);
    assert_eq!(directory_inode.group().unwrap(), metadata.gid);
    assert_eq!(metadata.last_access_at, expected_atime);
    assert_eq!(directory_inode.atime(), expected_atime);
    assert_eq!(metadata.last_modify_at, expected_mtime);
    assert_eq!(directory_inode.mtime(), expected_mtime);
    assert_eq!(metadata.last_meta_change_at, expected_mtime);
    assert_eq!(directory_inode.ctime(), expected_mtime);
}

pub(super) fn directory_metadata_projection_and_update_projection_substrate_keeps_root_projection_synthetic_without_self_entry_fabrication()
 {
    init_lookup_test_runtime();

    let disk = ExfatLookupTestDisk::new();
    disk.install_root_fractured_entry_set(ROOT_FILE_ENTRY_INDEX, "BrokenRootNeighbor");

    let (_fs, root_inode) = mount_root(&disk, None);
    let metadata = root_inode.metadata();

    assert_eq!(root_inode.type_(), InodeType::Dir);
    assert_eq!(metadata.type_, InodeType::Dir);
    assert_eq!(root_inode.mode().unwrap(), metadata.mode);
    assert_eq!(root_inode.owner().unwrap(), metadata.uid);
    assert_eq!(root_inode.group().unwrap(), metadata.gid);
    assert_eq!(root_inode.atime(), metadata.last_access_at);
    assert_eq!(root_inode.mtime(), metadata.last_modify_at);
    assert_eq!(root_inode.ctime(), metadata.last_meta_change_at);
    assert_eq!(
        lookup_error(&root_inode, "BrokenRootNeighbor"),
        Errno::EUCLEAN
    );
    assert_metadata_unchanged(root_inode.metadata(), metadata);
}

pub(super) fn directory_metadata_projection_and_update_projection_substrate_rejects_broken_ordinary_self_entry_sets_through_result_getters()
 {
    init_lookup_test_runtime();

    let disk = ExfatLookupTestDisk::new();
    disk.install_root_file(ROOT_FILE_ENTRY_INDEX, "Healthy");
    disk.install_root_directory(
        ROOT_SECOND_FILE_ENTRY_INDEX,
        "BrokenDir",
        TEST_CHILD_DIRECTORY_CLUSTER,
    );

    set_directory_entry_metadata(
        &disk,
        ROOT_SECOND_FILE_ENTRY_INDEX,
        FILE_ATTRIBUTE_DIRECTORY,
        (
            Date::from_calendar_date(2026, Month::April, 2).unwrap(),
            Time::from_hms_milli(1, 2, 4, 120).unwrap(),
            UtcOffset::UTC,
        ),
        (
            Date::from_calendar_date(2026, Month::April, 5).unwrap(),
            UtcOffset::UTC,
        ),
        (
            Date::from_calendar_date(2026, Month::April, 8).unwrap(),
            Time::from_hms_milli(9, 10, 12, 140).unwrap(),
            UtcOffset::UTC,
        ),
    );

    let (_fs, root_inode) = mount_root(&disk, None);
    let healthy_inode = root_inode.lookup("Healthy").unwrap();
    let broken_directory_inode = root_inode.lookup("BrokenDir").unwrap();
    let metadata_before_corruption = broken_directory_inode.metadata();
    let broken_entry_ino = broken_directory_inode.ino();

    let mut corrupted_entry_set = root_entry_set(&disk, ROOT_SECOND_FILE_ENTRY_INDEX);
    corrupted_entry_set[2] ^= 0x5A;
    disk.write_root_entries(ROOT_SECOND_FILE_ENTRY_INDEX, &corrupted_entry_set);

    assert_eq!(
        root_inode.lookup("healthy").unwrap().ino(),
        healthy_inode.ino()
    );
    assert_eq!(
        broken_directory_inode.mode().unwrap_err().error(),
        Errno::EUCLEAN
    );
    assert_eq!(
        broken_directory_inode.owner().unwrap_err().error(),
        Errno::EUCLEAN
    );
    assert_eq!(
        broken_directory_inode.group().unwrap_err().error(),
        Errno::EUCLEAN
    );
    assert_eq!(broken_directory_inode.ino(), broken_entry_ino);
    let fallback_metadata = broken_directory_inode.metadata();
    assert_eq!(fallback_metadata.ino, broken_entry_ino);
    assert_eq!(fallback_metadata.size, metadata_before_corruption.size);
    assert_eq!(fallback_metadata.type_, InodeType::Dir);
    assert_eq!(
        broken_directory_inode.atime(),
        fallback_metadata.last_access_at
    );
    assert_eq!(
        broken_directory_inode.mtime(),
        fallback_metadata.last_modify_at
    );
    assert_eq!(
        broken_directory_inode.ctime(),
        fallback_metadata.last_meta_change_at
    );
}
