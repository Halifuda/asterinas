// SPDX-License-Identifier: MPL-2.0

use alloc::sync::Arc;
use core::{ops::Range, time::Duration};

use aster_block::BlockDevice;
use time::{Date, Month, PrimitiveDateTime, Time, UtcOffset};

use super::{
    assert_bytes_unchanged_except, assert_metadata_unchanged, assert_valid_entry_set_checksum,
    encode_exfat_date, encode_exfat_date_only, encode_exfat_date_time,
    encode_valid_utc_offset_byte, expected_timestamp, init_lookup_test_runtime, lookup_error,
    mount_root, root_entry_set, set_directory_entry_metadata, ExfatLookupTestDisk,
    ExfatLookupToggleFailingWriteDisk, FILE_ATTRIBUTE_DIRECTORY, RENAME_SOURCE_FILE_CLUSTER,
    RENAME_SOURCE_PARENT_CLUSTER, RENAME_SOURCE_PARENT_NAME, RENAME_TARGET_PARENT_CLUSTER,
    RENAME_TARGET_PARENT_NAME, ROOT_FILE_ENTRY_INDEX, ROOT_SECOND_FILE_ENTRY_INDEX,
    TEST_CHILD_DIRECTORY_CLUSTER, TEST_PARENT_CLUSTER, TEST_REGULAR_FILE_CLUSTER,
};

const CREATE_TIMESTAMP_OFFSET: usize = 8;
const LAST_MODIFIED_TIMESTAMP_OFFSET: usize = 12;
const LAST_ACCESSED_TIMESTAMP_OFFSET: usize = 16;
const CREATE_10MS_INCREMENT_OFFSET: usize = 20;
const LAST_MODIFIED_10MS_INCREMENT_OFFSET: usize = 21;
const CREATE_UTC_OFFSET_OFFSET: usize = 22;
const LAST_MODIFIED_UTC_OFFSET_OFFSET: usize = 23;
const LAST_ACCESSED_UTC_OFFSET_OFFSET: usize = 24;

fn install_timestamped_root_directory(
    disk: &Arc<ExfatLookupTestDisk>,
    entry_index: usize,
    name: &str,
    first_cluster: u32,
) -> Duration {
    disk.install_root_directory(entry_index, name, first_cluster);
    let modified_date = Date::from_calendar_date(2020, Month::March, 5).unwrap();
    let modified_time = Time::from_hms_milli(8, 10, 12, 140).unwrap();
    let modified_offset = UtcOffset::from_whole_seconds(-60 * 60).unwrap();
    set_directory_entry_metadata(
        disk,
        entry_index,
        FILE_ATTRIBUTE_DIRECTORY,
        (
            Date::from_calendar_date(2020, Month::January, 1).unwrap(),
            Time::from_hms_milli(2, 4, 6, 120).unwrap(),
            UtcOffset::UTC,
        ),
        (
            Date::from_calendar_date(2020, Month::February, 3).unwrap(),
            UtcOffset::from_whole_seconds(60 * 60).unwrap(),
        ),
        (modified_date, modified_time, modified_offset),
    );
    expected_timestamp(modified_date, modified_time, modified_offset)
}

fn assert_namespace_refresh_published(
    disk: &Arc<ExfatLookupTestDisk>,
    entry_index: usize,
    directory_inode: &Arc<dyn Inode>,
    entry_set_before: &[u8],
    metadata_before: Metadata,
) -> Vec<u8> {
    let entry_set_after = root_entry_set(disk, entry_index);
    assert_valid_entry_set_checksum(&entry_set_after);
    assert_bytes_unchanged_except(
        entry_set_before,
        &entry_set_after,
        &[
            2..4,
            LAST_MODIFIED_TIMESTAMP_OFFSET..LAST_MODIFIED_TIMESTAMP_OFFSET + 4,
            LAST_MODIFIED_10MS_INCREMENT_OFFSET..LAST_MODIFIED_10MS_INCREMENT_OFFSET + 1,
            LAST_MODIFIED_UTC_OFFSET_OFFSET..LAST_MODIFIED_UTC_OFFSET_OFFSET + 1,
        ],
    );
    assert_ne!(
        &entry_set_after[LAST_MODIFIED_TIMESTAMP_OFFSET..LAST_MODIFIED_TIMESTAMP_OFFSET + 4],
        &entry_set_before[LAST_MODIFIED_TIMESTAMP_OFFSET..LAST_MODIFIED_TIMESTAMP_OFFSET + 4],
    );

    let metadata_after = directory_inode.metadata();
    assert_eq!(directory_inode.mtime(), metadata_after.last_modify_at);
    assert_eq!(directory_inode.ctime(), metadata_after.last_meta_change_at);
    assert_eq!(
        metadata_after.last_meta_change_at,
        metadata_after.last_modify_at
    );
    assert_ne!(
        metadata_after.last_modify_at,
        metadata_before.last_modify_at
    );
    assert_eq!(
        metadata_after.last_access_at,
        metadata_before.last_access_at
    );
    assert_eq!(metadata_after.type_, InodeType::Dir);
    assert_eq!(metadata_after.ino, metadata_before.ino);
    entry_set_after
}

pub(super) fn directory_metadata_projection_and_update_namespace_refresh_create_and_mkdir_refresh_parent_timestamp(
) {
    init_lookup_test_runtime();

    let create_disk = ExfatLookupTestDisk::new();
    install_timestamped_root_directory(
        &create_disk,
        ROOT_FILE_ENTRY_INDEX,
        "CreateRefreshParent",
        TEST_PARENT_CLUSTER,
    );
    let (_create_fs, create_root) = mount_root(&create_disk, None);
    let create_parent = create_root.lookup("CreateRefreshParent").unwrap();
    let create_metadata_before = create_parent.metadata();
    let create_entry_before = root_entry_set(&create_disk, ROOT_FILE_ENTRY_INDEX);

    let created_file = create_parent
        .create("CreatedFile", InodeType::File, InodeMode::all())
        .unwrap();

    assert_eq!(created_file.type_(), InodeType::File);
    assert_eq!(
        create_parent.lookup("createdfile").unwrap().ino(),
        created_file.ino()
    );
    assert_namespace_refresh_published(
        &create_disk,
        ROOT_FILE_ENTRY_INDEX,
        &create_parent,
        &create_entry_before,
        create_metadata_before,
    );

    let mkdir_disk = ExfatLookupTestDisk::new();
    install_timestamped_root_directory(
        &mkdir_disk,
        ROOT_FILE_ENTRY_INDEX,
        "MkdirRefreshParent",
        TEST_PARENT_CLUSTER,
    );
    let (_mkdir_fs, mkdir_root) = mount_root(&mkdir_disk, None);
    let mkdir_parent = mkdir_root.lookup("MkdirRefreshParent").unwrap();
    let mkdir_metadata_before = mkdir_parent.metadata();
    let mkdir_entry_before = root_entry_set(&mkdir_disk, ROOT_FILE_ENTRY_INDEX);

    let created_directory = mkdir_parent
        .create("CreatedDir", InodeType::Dir, InodeMode::all())
        .unwrap();

    assert_eq!(created_directory.type_(), InodeType::Dir);
    assert_eq!(
        mkdir_parent.lookup("createddir").unwrap().ino(),
        created_directory.ino()
    );
    assert_namespace_refresh_published(
        &mkdir_disk,
        ROOT_FILE_ENTRY_INDEX,
        &mkdir_parent,
        &mkdir_entry_before,
        mkdir_metadata_before,
    );
}

pub(super) fn directory_metadata_projection_and_update_namespace_refresh_unlink_and_rmdir_refresh_parent_timestamp(
) {
    init_lookup_test_runtime();

    let unlink_disk = ExfatLookupTestDisk::new();
    install_timestamped_root_directory(
        &unlink_disk,
        ROOT_FILE_ENTRY_INDEX,
        "UnlinkRefreshParent",
        TEST_PARENT_CLUSTER,
    );
    unlink_disk.install_directory_file(
        TEST_PARENT_CLUSTER,
        0,
        "GoneFile",
        TEST_REGULAR_FILE_CLUSTER,
        unlink_disk.root_cluster_size(),
    );
    let (_unlink_fs, unlink_root) = mount_root(&unlink_disk, None);
    let unlink_parent = unlink_root.lookup("UnlinkRefreshParent").unwrap();
    let unlink_metadata_before = unlink_parent.metadata();
    let unlink_entry_before = root_entry_set(&unlink_disk, ROOT_FILE_ENTRY_INDEX);

    unlink_parent.unlink("GoneFile").unwrap();

    assert_eq!(lookup_error(&unlink_parent, "GoneFile"), Errno::ENOENT);
    assert_namespace_refresh_published(
        &unlink_disk,
        ROOT_FILE_ENTRY_INDEX,
        &unlink_parent,
        &unlink_entry_before,
        unlink_metadata_before,
    );

    let rmdir_disk = ExfatLookupTestDisk::new();
    install_timestamped_root_directory(
        &rmdir_disk,
        ROOT_FILE_ENTRY_INDEX,
        "RmdirRefreshParent",
        TEST_PARENT_CLUSTER,
    );
    rmdir_disk.install_directory_subdirectory(
        TEST_PARENT_CLUSTER,
        0,
        "GoneDir",
        TEST_CHILD_DIRECTORY_CLUSTER,
    );
    let (_rmdir_fs, rmdir_root) = mount_root(&rmdir_disk, None);
    let rmdir_parent = rmdir_root.lookup("RmdirRefreshParent").unwrap();
    let rmdir_metadata_before = rmdir_parent.metadata();
    let rmdir_entry_before = root_entry_set(&rmdir_disk, ROOT_FILE_ENTRY_INDEX);

    rmdir_parent.rmdir("GoneDir").unwrap();

    assert_eq!(lookup_error(&rmdir_parent, "GoneDir"), Errno::ENOENT);
    assert_namespace_refresh_published(
        &rmdir_disk,
        ROOT_FILE_ENTRY_INDEX,
        &rmdir_parent,
        &rmdir_entry_before,
        rmdir_metadata_before,
    );
}

pub(super) fn directory_metadata_projection_and_update_namespace_refresh_rename_refreshes_affected_directories(
) {
    init_lookup_test_runtime();

    let same_directory_disk = ExfatLookupTestDisk::new();
    install_timestamped_root_directory(
        &same_directory_disk,
        ROOT_FILE_ENTRY_INDEX,
        "SameRenameParent",
        RENAME_SOURCE_PARENT_CLUSTER,
    );
    same_directory_disk.install_directory_file(
        RENAME_SOURCE_PARENT_CLUSTER,
        0,
        "OldName",
        RENAME_SOURCE_FILE_CLUSTER,
        same_directory_disk.root_cluster_size(),
    );
    let (_same_fs, same_root) = mount_root(&same_directory_disk, None);
    let same_parent = same_root.lookup("SameRenameParent").unwrap();
    let same_metadata_before = same_parent.metadata();
    let same_entry_before = root_entry_set(&same_directory_disk, ROOT_FILE_ENTRY_INDEX);

    same_parent
        .rename("OldName", &same_parent, "NewName")
        .unwrap();

    assert_eq!(lookup_error(&same_parent, "OldName"), Errno::ENOENT);
    assert!(same_parent.lookup("NewName").is_ok());
    assert_namespace_refresh_published(
        &same_directory_disk,
        ROOT_FILE_ENTRY_INDEX,
        &same_parent,
        &same_entry_before,
        same_metadata_before,
    );

    let noop_disk = ExfatLookupTestDisk::new();
    install_timestamped_root_directory(
        &noop_disk,
        ROOT_FILE_ENTRY_INDEX,
        "NoopRenameParent",
        RENAME_SOURCE_PARENT_CLUSTER,
    );
    noop_disk.install_directory_file(
        RENAME_SOURCE_PARENT_CLUSTER,
        0,
        "StableName",
        RENAME_SOURCE_FILE_CLUSTER,
        noop_disk.root_cluster_size(),
    );
    let (_noop_fs, noop_root) = mount_root(&noop_disk, None);
    let noop_parent = noop_root.lookup("NoopRenameParent").unwrap();
    let noop_metadata_before = noop_parent.metadata();
    let noop_entry_before = root_entry_set(&noop_disk, ROOT_FILE_ENTRY_INDEX);

    noop_parent
        .rename("StableName", &noop_parent, "StableName")
        .unwrap();

    assert!(noop_parent.lookup("StableName").is_ok());
    assert_eq!(
        root_entry_set(&noop_disk, ROOT_FILE_ENTRY_INDEX),
        noop_entry_before
    );
    assert_metadata_unchanged(noop_parent.metadata(), noop_metadata_before);

    let cross_directory_disk = ExfatLookupTestDisk::new();
    install_timestamped_root_directory(
        &cross_directory_disk,
        ROOT_FILE_ENTRY_INDEX,
        RENAME_SOURCE_PARENT_NAME,
        RENAME_SOURCE_PARENT_CLUSTER,
    );
    install_timestamped_root_directory(
        &cross_directory_disk,
        ROOT_SECOND_FILE_ENTRY_INDEX,
        RENAME_TARGET_PARENT_NAME,
        RENAME_TARGET_PARENT_CLUSTER,
    );
    cross_directory_disk.install_directory_file(
        RENAME_SOURCE_PARENT_CLUSTER,
        0,
        "MoveMe",
        RENAME_SOURCE_FILE_CLUSTER,
        cross_directory_disk.root_cluster_size(),
    );
    let (_cross_fs, cross_root) = mount_root(&cross_directory_disk, None);
    let source_parent = cross_root.lookup(RENAME_SOURCE_PARENT_NAME).unwrap();
    let target_parent = cross_root.lookup(RENAME_TARGET_PARENT_NAME).unwrap();
    let source_metadata_before = source_parent.metadata();
    let target_metadata_before = target_parent.metadata();
    let source_entry_before = root_entry_set(&cross_directory_disk, ROOT_FILE_ENTRY_INDEX);
    let target_entry_before = root_entry_set(&cross_directory_disk, ROOT_SECOND_FILE_ENTRY_INDEX);

    source_parent
        .rename("MoveMe", &target_parent, "MovedFile")
        .unwrap();

    assert_eq!(lookup_error(&source_parent, "MoveMe"), Errno::ENOENT);
    assert!(target_parent.lookup("MovedFile").is_ok());
    assert_namespace_refresh_published(
        &cross_directory_disk,
        ROOT_FILE_ENTRY_INDEX,
        &source_parent,
        &source_entry_before,
        source_metadata_before,
    );
    assert_namespace_refresh_published(
        &cross_directory_disk,
        ROOT_SECOND_FILE_ENTRY_INDEX,
        &target_parent,
        &target_entry_before,
        target_metadata_before,
    );
}

pub(super) fn directory_metadata_projection_and_update_namespace_refresh_failure_preserves_last_good_state(
) {
    init_lookup_test_runtime();

    let writable_disk = ExfatLookupTestDisk::new();
    let expected_mtime = install_timestamped_root_directory(
        &writable_disk,
        ROOT_FILE_ENTRY_INDEX,
        "RefreshFailureParent",
        TEST_PARENT_CLUSTER,
    );
    let failing_write_disk = ExfatLookupToggleFailingWriteDisk::new(
        writable_disk.clone(),
        writable_disk.root_directory_offset(),
        writable_disk.root_cluster_size(),
    );
    let block_device: Arc<dyn BlockDevice> = failing_write_disk.clone();
    let (_fs, root_inode) = mount_root_from_block_device(block_device, FsFlags::empty(), None);
    let parent_inode = root_inode.lookup("RefreshFailureParent").unwrap();
    let entry_set_before = root_entry_set(&writable_disk, ROOT_FILE_ENTRY_INDEX);
    let metadata_before = parent_inode.metadata();

    failing_write_disk.enable_failures();
    let error = match parent_inode.create("RefreshFail", InodeType::File, InodeMode::all()) {
        Ok(_) => panic!("namespace refresh write failure unexpectedly reported success"),
        Err(error) => error,
    };

    assert_eq!(error.error(), Errno::EIO);
    assert_eq!(
        root_entry_set(&writable_disk, ROOT_FILE_ENTRY_INDEX),
        entry_set_before
    );
    assert_metadata_unchanged(parent_inode.metadata(), metadata_before);
    assert_eq!(parent_inode.mtime(), expected_mtime);
    assert_eq!(parent_inode.ctime(), expected_mtime);
    assert_eq!(parent_inode.metadata().last_modify_at, expected_mtime);
    assert_eq!(parent_inode.metadata().last_meta_change_at, expected_mtime);
    assert!(parent_inode.lookup("RefreshFail").is_ok());
}
