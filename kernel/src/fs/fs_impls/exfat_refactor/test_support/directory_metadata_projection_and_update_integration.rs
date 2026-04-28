// SPDX-License-Identifier: MPL-2.0

use alloc::sync::Arc;
use core::{
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
    time::Duration,
};

use aster_block::BlockDevice;
use spin::Mutex;
use time::{Date, Month, PrimitiveDateTime, Time, UtcOffset};

use super::{
    assert_metadata_unchanged, assert_valid_entry_set_checksum, collect_dirents, decode_entry_name,
    encode_exfat_date, encode_exfat_date_only, encode_exfat_date_time,
    encode_valid_utc_offset_byte, entry_set_checksum, expected_timestamp, init_lookup_test_runtime,
    lookup_error, mount_root, root_entry_set, set_directory_entry_metadata, ExfatLookupTestDisk,
    ExfatLookupToggleFailingWriteDisk, ExfatLookupWriteControlDisk, DIRECTORY_ENTRY_SIZE,
    FILE_ATTRIBUTES_OFFSET, FILE_ATTRIBUTE_DIRECTORY, FILE_DIRECTORY_ENTRY_TYPE,
    FILE_NAME_ENTRY_TYPE, RENAME_SOURCE_PARENT_CLUSTER, RENAME_TARGET_PARENT_CLUSTER,
    ROOT_FILE_ENTRY_INDEX, ROOT_SECOND_FILE_ENTRY_INDEX, STREAM_EXTENSION_ENTRY_TYPE,
    TEST_PARENT_CLUSTER,
};
use crate::{
    process::{Gid, Uid},
    thread::{kernel_thread::ThreadOptions, Thread},
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
const SOURCE_DIRECTORY_NAME: &str = "SrcDir";
const TARGET_DIRECTORY_NAME: &str = "DstDir";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ProjectionSnapshot {
    atime: Duration,
    ctime: Duration,
    gid: Gid,
    ino: u64,
    mode: InodeMode,
    mtime: Duration,
    nr_sectors_allocated: usize,
    size: usize,
    type_: InodeType,
    uid: Uid,
}

fn install_timestamped_root_directory(
    disk: &Arc<ExfatLookupTestDisk>,
    entry_index: usize,
    name: &str,
    first_cluster: u32,
    read_only: bool,
) -> Duration {
    disk.install_root_directory(entry_index, name, first_cluster);
    let file_attributes = if read_only {
        FILE_ATTRIBUTE_DIRECTORY | FILE_ATTRIBUTE_READ_ONLY
    } else {
        FILE_ATTRIBUTE_DIRECTORY
    };
    let modified_date = Date::from_calendar_date(2026, Month::April, 8).unwrap();
    let modified_time = Time::from_hms_milli(10, 12, 14, 230).unwrap();
    let modified_offset = UtcOffset::from_whole_seconds(-2 * 60 * 60).unwrap();
    set_directory_entry_metadata(
        disk,
        entry_index,
        file_attributes,
        (
            Date::from_calendar_date(2026, Month::April, 2).unwrap(),
            Time::from_hms_milli(1, 2, 4, 120).unwrap(),
            UtcOffset::UTC,
        ),
        (
            Date::from_calendar_date(2026, Month::April, 5).unwrap(),
            UtcOffset::from_whole_seconds(60 * 60).unwrap(),
        ),
        (modified_date, modified_time, modified_offset),
    );
    expected_timestamp(modified_date, modified_time, modified_offset)
}

fn projection_snapshot(inode: &Arc<dyn Inode>) -> ProjectionSnapshot {
    let metadata = inode.metadata();
    ProjectionSnapshot {
        atime: metadata.last_access_at,
        ctime: metadata.last_meta_change_at,
        gid: metadata.gid,
        ino: metadata.ino,
        mode: metadata.mode,
        mtime: metadata.last_modify_at,
        nr_sectors_allocated: metadata.nr_sectors_allocated,
        size: metadata.size,
        type_: metadata.type_,
        uid: metadata.uid,
    }
}

fn assert_directory_self_entry_set(
    entry_set: &[u8],
    expected_name: &str,
    expected_read_only: bool,
) {
    assert_valid_entry_set_checksum(entry_set);
    assert_eq!(entry_set[0], FILE_DIRECTORY_ENTRY_TYPE);
    assert_eq!(entry_set[DIRECTORY_ENTRY_SIZE], STREAM_EXTENSION_ENTRY_TYPE);
    assert_eq!(entry_set[DIRECTORY_ENTRY_SIZE * 2], FILE_NAME_ENTRY_TYPE);
    assert_eq!(
        decode_entry_name(entry_set),
        expected_name.encode_utf16().collect::<Vec<_>>()
    );
    let file_attributes = u16::from_le_bytes([
        entry_set[FILE_ATTRIBUTES_OFFSET],
        entry_set[FILE_ATTRIBUTES_OFFSET + 1],
    ]);
    assert_ne!(file_attributes & FILE_ATTRIBUTE_DIRECTORY, 0);
    assert_eq!(
        file_attributes & FILE_ATTRIBUTE_READ_ONLY != 0,
        expected_read_only
    );
}

pub(super) fn directory_metadata_projection_and_update_integration_namespace_mutation_sequence_preserves_projection_and_durable_self_entry_sets(
) {
    init_lookup_test_runtime();

    let disk = ExfatLookupTestDisk::new();
    let source_mtime_before = install_timestamped_root_directory(
        &disk,
        ROOT_FILE_ENTRY_INDEX,
        SOURCE_DIRECTORY_NAME,
        RENAME_SOURCE_PARENT_CLUSTER,
        false,
    );
    let target_mtime_before = install_timestamped_root_directory(
        &disk,
        ROOT_SECOND_FILE_ENTRY_INDEX,
        TARGET_DIRECTORY_NAME,
        RENAME_TARGET_PARENT_CLUSTER,
        true,
    );
    let (_fs, root_inode) = mount_root(&disk, Some("zero_size_dir"));
    let source_directory = root_inode.lookup(SOURCE_DIRECTORY_NAME).unwrap();
    let target_directory = root_inode.lookup(TARGET_DIRECTORY_NAME).unwrap();
    let source_snapshot_before = projection_snapshot(&source_directory);
    let target_snapshot_before = projection_snapshot(&target_directory);

    let source_owner = source_directory.owner().unwrap();
    let source_group = source_directory.group().unwrap();
    let target_owner = target_directory.owner().unwrap();
    let target_group = target_directory.group().unwrap();
    let source_mode_before = source_directory.mode().unwrap();
    let target_mode_before = target_directory.mode().unwrap();

    source_directory
        .create("MoveMe", InodeType::File, InodeMode::all())
        .unwrap();
    source_directory
        .create("EmptyDir", InodeType::Dir, InodeMode::all())
        .unwrap();
    source_directory
        .rename("MoveMe", &target_directory, "MovedFile")
        .unwrap();
    target_directory.unlink("MovedFile").unwrap();
    source_directory.rmdir("EmptyDir").unwrap();

    let (_source_visited_count, source_entries) = collect_dirents(&source_directory, 2);
    let (_target_visited_count, target_entries) = collect_dirents(&target_directory, 2);
    let source_snapshot_after = projection_snapshot(&source_directory);
    let target_snapshot_after = projection_snapshot(&target_directory);

    assert!(source_entries.is_empty());
    assert!(target_entries.is_empty());
    assert_eq!(lookup_error(&source_directory, "MoveMe"), Errno::ENOENT);
    assert_eq!(lookup_error(&source_directory, "EmptyDir"), Errno::ENOENT);
    assert_eq!(lookup_error(&target_directory, "MovedFile"), Errno::ENOENT);

    assert_eq!(source_directory.mode().unwrap(), source_mode_before);
    assert_eq!(target_directory.mode().unwrap(), target_mode_before);
    assert_eq!(source_snapshot_after.mode, source_mode_before);
    assert_eq!(target_snapshot_after.mode, target_mode_before);
    assert!(source_mode_before.intersects(mkmod!(a+w)));
    assert!(!target_mode_before.intersects(mkmod!(a+w)));

    assert_eq!(source_directory.owner().unwrap(), source_owner);
    assert_eq!(source_directory.group().unwrap(), source_group);
    assert_eq!(target_directory.owner().unwrap(), target_owner);
    assert_eq!(target_directory.group().unwrap(), target_group);
    assert_eq!(source_snapshot_after.uid, source_owner);
    assert_eq!(source_snapshot_after.gid, source_group);
    assert_eq!(target_snapshot_after.uid, target_owner);
    assert_eq!(target_snapshot_after.gid, target_group);

    assert_eq!(source_snapshot_before.mtime, source_mtime_before);
    assert_eq!(source_snapshot_before.ctime, source_mtime_before);
    assert_eq!(target_snapshot_before.mtime, target_mtime_before);
    assert_eq!(target_snapshot_before.ctime, target_mtime_before);
    assert_eq!(source_snapshot_after.atime, source_snapshot_before.atime);
    assert_eq!(target_snapshot_after.atime, target_snapshot_before.atime);
    assert_ne!(source_snapshot_after.mtime, source_snapshot_before.mtime);
    assert_ne!(target_snapshot_after.mtime, target_snapshot_before.mtime);
    assert_eq!(source_snapshot_after.mtime, source_snapshot_after.ctime);
    assert_eq!(target_snapshot_after.mtime, target_snapshot_after.ctime);

    assert_directory_self_entry_set(
        &root_entry_set(&disk, ROOT_FILE_ENTRY_INDEX),
        SOURCE_DIRECTORY_NAME,
        false,
    );
    assert_directory_self_entry_set(
        &root_entry_set(&disk, ROOT_SECOND_FILE_ENTRY_INDEX),
        TARGET_DIRECTORY_NAME,
        true,
    );
}

pub(super) fn directory_metadata_projection_and_update_integration_failure_maintenance_preserves_last_good_directory_metadata_publication(
) {
    init_lookup_test_runtime();

    let integrity_disk = ExfatLookupTestDisk::new();
    install_timestamped_root_directory(
        &integrity_disk,
        ROOT_FILE_ENTRY_INDEX,
        "IntegrityDir",
        TEST_PARENT_CLUSTER,
        false,
    );
    let (_integrity_fs, integrity_root) = mount_root(&integrity_disk, None);
    let integrity_directory = integrity_root.lookup("IntegrityDir").unwrap();
    let integrity_metadata_before = integrity_directory.metadata();
    let integrity_mode_request = chmod!(integrity_directory.mode().unwrap(), a-w);
    let mut corrupted_entry_set = root_entry_set(&integrity_disk, ROOT_FILE_ENTRY_INDEX);
    corrupted_entry_set[2] ^= 0x5A;
    integrity_disk.write_root_entries(ROOT_FILE_ENTRY_INDEX, &corrupted_entry_set);

    let integrity_error = integrity_directory
        .set_mode(integrity_mode_request)
        .unwrap_err();

    assert_eq!(integrity_error.error(), Errno::EUCLEAN);
    assert_eq!(
        root_entry_set(&integrity_disk, ROOT_FILE_ENTRY_INDEX),
        corrupted_entry_set
    );
    assert_eq!(
        integrity_directory.mode().unwrap_err().error(),
        Errno::EUCLEAN
    );
    let fallback_metadata = integrity_directory.metadata();
    assert_eq!(fallback_metadata.ino, integrity_metadata_before.ino);
    assert_eq!(fallback_metadata.size, integrity_metadata_before.size);
    assert_eq!(fallback_metadata.type_, InodeType::Dir);
    assert_eq!(fallback_metadata.mode, integrity_metadata_before.mode);
    assert_eq!(fallback_metadata.uid, integrity_metadata_before.uid);
    assert_eq!(fallback_metadata.gid, integrity_metadata_before.gid);
    assert_eq!(
        integrity_directory.atime(),
        fallback_metadata.last_access_at
    );
    assert_eq!(
        integrity_directory.mtime(),
        fallback_metadata.last_modify_at
    );
    assert_eq!(
        integrity_directory.ctime(),
        fallback_metadata.last_meta_change_at
    );

    let writable_disk = ExfatLookupTestDisk::new();
    let expected_mtime = install_timestamped_root_directory(
        &writable_disk,
        ROOT_FILE_ENTRY_INDEX,
        "RefreshFailureParent",
        TEST_PARENT_CLUSTER,
        false,
    );
    let failing_write_disk = ExfatLookupToggleFailingWriteDisk::new(
        writable_disk.clone(),
        writable_disk.root_directory_offset(),
        writable_disk.root_cluster_size(),
    );
    let block_device: Arc<dyn BlockDevice> = failing_write_disk.clone();
    let (_fs, root_inode) = mount_root_from_block_device(block_device, FsFlags::empty(), None);
    let parent_inode = root_inode.lookup("RefreshFailureParent").unwrap();
    let metadata_before = parent_inode.metadata();
    let mode_before = parent_inode.mode().unwrap();
    let entry_set_before = root_entry_set(&writable_disk, ROOT_FILE_ENTRY_INDEX);

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
    assert_eq!(parent_inode.mode().unwrap(), mode_before);
    assert_eq!(parent_inode.mtime(), expected_mtime);
    assert_eq!(parent_inode.ctime(), expected_mtime);
    assert!(parent_inode.lookup("RefreshFail").is_ok());
}

pub(super) fn directory_metadata_projection_and_update_integration_concurrency_observes_only_pre_or_post_projection_views(
) {
    init_lookup_test_runtime();

    let disk = ExfatLookupTestDisk::new();
    install_timestamped_root_directory(
        &disk,
        ROOT_FILE_ENTRY_INDEX,
        "ConcurrentDir",
        TEST_PARENT_CLUSTER,
        false,
    );
    let write_control_disk = ExfatLookupWriteControlDisk::new(
        disk.clone(),
        disk.root_directory_offset(),
        disk.root_cluster_size(),
    );
    let block_device: Arc<dyn BlockDevice> = write_control_disk.clone();
    let (_fs, root_inode) = mount_root_from_block_device(block_device, FsFlags::empty(), None);
    let directory_inode = root_inode.lookup("ConcurrentDir").unwrap();
    let snapshot_before = projection_snapshot(&directory_inode);
    let requested_mtime = expected_timestamp(
        Date::from_calendar_date(2026, Month::June, 14).unwrap(),
        Time::from_hms_milli(6, 8, 10, 240).unwrap(),
        UtcOffset::from_whole_seconds(-2 * 60 * 60).unwrap(),
    );
    let snapshot_after = ProjectionSnapshot {
        atime: snapshot_before.atime,
        ctime: requested_mtime,
        gid: snapshot_before.gid,
        ino: snapshot_before.ino,
        mode: snapshot_before.mode,
        mtime: requested_mtime,
        nr_sectors_allocated: snapshot_before.nr_sectors_allocated,
        size: snapshot_before.size,
        type_: snapshot_before.type_,
        uid: snapshot_before.uid,
    };

    let pre_projection_count = Arc::new(AtomicUsize::new(0));
    let post_projection_count = Arc::new(AtomicUsize::new(0));
    let reader_stop = Arc::new(AtomicBool::new(false));
    let unexpected_snapshot = Arc::new(Mutex::new(None));
    let writer_finished = Arc::new(AtomicBool::new(false));

    write_control_disk.enable_blocking_writes();

    let reader_thread = {
        let directory_inode = directory_inode.clone();
        let post_projection_count = post_projection_count.clone();
        let pre_projection_count = pre_projection_count.clone();
        let reader_stop = reader_stop.clone();
        let unexpected_snapshot = unexpected_snapshot.clone();
        let writer_finished = writer_finished.clone();
        ThreadOptions::new(move || {
            while !reader_stop.load(Ordering::Relaxed) {
                let snapshot = projection_snapshot(&directory_inode);
                if snapshot == snapshot_before {
                    pre_projection_count.fetch_add(1, Ordering::Relaxed);
                } else if snapshot == snapshot_after {
                    post_projection_count.fetch_add(1, Ordering::Relaxed);
                } else {
                    *unexpected_snapshot.lock() = Some(snapshot);
                    break;
                }

                if writer_finished.load(Ordering::Relaxed)
                    && post_projection_count.load(Ordering::Relaxed) > 0
                {
                    break;
                }
                Thread::yield_now();
            }
        })
        .spawn()
    };

    let writer_thread = {
        let directory_inode = directory_inode.clone();
        let writer_finished = writer_finished.clone();
        ThreadOptions::new(move || {
            directory_inode.set_mtime(requested_mtime);
            writer_finished.store(true, Ordering::Relaxed);
        })
        .spawn()
    };

    while !write_control_disk.write_started() {
        Thread::yield_now();
    }
    for _ in 0..10_000 {
        if pre_projection_count.load(Ordering::Relaxed) > 0 {
            break;
        }
        Thread::yield_now();
    }

    assert!(!writer_finished.load(Ordering::Relaxed));
    assert!(pre_projection_count.load(Ordering::Relaxed) > 0);

    write_control_disk.release_blocked_writes();
    writer_thread.join();

    for _ in 0..10_000 {
        if post_projection_count.load(Ordering::Relaxed) > 0 {
            break;
        }
        Thread::yield_now();
    }

    reader_stop.store(true, Ordering::Relaxed);
    reader_thread.join();

    assert_eq!(*unexpected_snapshot.lock(), None);
    assert!(post_projection_count.load(Ordering::Relaxed) > 0);
    assert_eq!(projection_snapshot(&directory_inode), snapshot_after);
    assert_eq!(directory_inode.mtime(), requested_mtime);
    assert_eq!(directory_inode.ctime(), requested_mtime);
    assert_directory_self_entry_set(
        &root_entry_set(&disk, ROOT_FILE_ENTRY_INDEX),
        "ConcurrentDir",
        false,
    );
}
