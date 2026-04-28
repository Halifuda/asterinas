// SPDX-License-Identifier: MPL-2.0

use alloc::{sync::Arc, vec, vec::Vec};
use core::sync::atomic::AtomicUsize;

use aster_block::BlockDevice;
use ostd::prelude::ktest;

use super::{
    super::super::direntry::{
        self, DirectoryEntryAnomalyKind, DirectoryEntrySlotRange, FileEntrySetFieldUpdates,
        ScannedDirectoryEntry, WritableDirectoryEntrySlotSpan,
    },
    collect_dirents, decode_entry_name, entry_index_from_ino, entry_names, entry_set_checksum,
    init_lookup_test_runtime, lookup_error, mount_create_parent, mount_root,
    mount_root_from_block_device, visible_name_count, wait_for_concurrent_start, Errno,
    ExfatLookupTestDisk, ExfatLookupToggleFailingWriteDisk, FsFlags, InodeMode, InodeType,
    MountVolumeStateError, Ordering, ThreadOptions, DIRECTORY_ENTRY_SIZE, FILE_ATTRIBUTES_OFFSET,
    FILE_ATTRIBUTE_REGULAR, FILE_DIRECTORY_ENTRY_TYPE, FILE_NAME_ENTRY_TYPE, ROOT_FILE_ENTRY_INDEX,
    ROOT_SECOND_FILE_ENTRY_INDEX, STREAM_EXTENSION_ENTRY_TYPE, TEST_PARENT_CLUSTER,
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

fn root_directory_bytes(disk: &Arc<ExfatLookupTestDisk>) -> Vec<u8> {
    disk.read_root_entries(0, disk.root_directory_entry_capacity())
}

fn root_slot_bytes(slot_range: DirectoryEntrySlotRange) -> core::ops::Range<usize> {
    let start = slot_range.first_entry_index() * DIRECTORY_ENTRY_SIZE;
    let end = start + slot_range.entry_count() * DIRECTORY_ENTRY_SIZE;
    start..end
}

fn scan_root_entry(
    directory_bytes: &[u8],
    entry_index: usize,
) -> core::result::Result<ScannedDirectoryEntry<'_>, MountVolumeStateError> {
    direntry::scan_directory_entry(true, directory_bytes, entry_index)
}

#[ktest]
fn directory_entry_field_update_substrate_republish_reopens_without_layout_drift() {
    init_lookup_test_runtime();

    let disk = ExfatLookupTestDisk::new();
    disk.install_root_file(ROOT_FILE_ENTRY_INDEX, "AlphaName");
    let mut root_bytes = root_directory_bytes(&disk);
    let source_view = match scan_root_entry(&root_bytes, ROOT_FILE_ENTRY_INDEX).unwrap() {
        ScannedDirectoryEntry::File(view) => view,
        _ => panic!("expected published file entry set"),
    };
    let source_slot_range = source_view.slot_range();
    let source_bytes = root_bytes[root_slot_bytes(source_slot_range)].to_vec();

    let republished_name: Vec<u16> = "GammaName".encode_utf16().collect();
    let republished_name_hash = 0x4A3C;
    let republished_entry_set = direntry::republished_entry_set(
        source_view,
        &FileEntrySetFieldUpdates {
            create_fields: Some(([0x11, 0x22, 0x33, 0x44], 0x55, 0x66)),
            file_attributes: Some(FILE_ATTRIBUTE_REGULAR),
            last_accessed_fields: Some(([0x77, 0x88, 0x99, 0xAA], 0xBB)),
            last_modified_fields: Some(([0xCC, 0xDD, 0xEE, 0x0F], 0x10, 0x11)),
            name: Some(&republished_name),
            name_hash: Some(republished_name_hash),
            ..Default::default()
        },
    )
    .unwrap();

    assert_eq!(republished_entry_set.len(), source_bytes.len());
    assert_eq!(
        &republished_entry_set[FILE_ATTRIBUTES_OFFSET..FILE_ATTRIBUTES_OFFSET + 2],
        &FILE_ATTRIBUTE_REGULAR.to_le_bytes()
    );
    assert_eq!(
        &republished_entry_set[CREATE_TIMESTAMP_OFFSET..CREATE_TIMESTAMP_OFFSET + 4],
        &[0x11, 0x22, 0x33, 0x44]
    );
    assert_eq!(republished_entry_set[CREATE_10MS_INCREMENT_OFFSET], 0x55);
    assert_eq!(republished_entry_set[CREATE_UTC_OFFSET_OFFSET], 0x66);
    assert_eq!(
        &republished_entry_set[LAST_MODIFIED_TIMESTAMP_OFFSET..LAST_MODIFIED_TIMESTAMP_OFFSET + 4],
        &[0xCC, 0xDD, 0xEE, 0x0F]
    );
    assert_eq!(
        republished_entry_set[LAST_MODIFIED_10MS_INCREMENT_OFFSET],
        0x10
    );
    assert_eq!(republished_entry_set[LAST_MODIFIED_UTC_OFFSET_OFFSET], 0x11);
    assert_eq!(
        &republished_entry_set[LAST_ACCESSED_TIMESTAMP_OFFSET..LAST_ACCESSED_TIMESTAMP_OFFSET + 4],
        &[0x77, 0x88, 0x99, 0xAA]
    );
    assert_eq!(republished_entry_set[LAST_ACCESSED_UTC_OFFSET_OFFSET], 0xBB);
    assert_ne!(&republished_entry_set[2..4], &source_bytes[2..4]);
    assert_eq!(
        u16::from_le_bytes([republished_entry_set[2], republished_entry_set[3]]),
        entry_set_checksum(&republished_entry_set, source_slot_range.entry_count() - 1)
    );

    root_bytes[root_slot_bytes(source_slot_range)].copy_from_slice(&republished_entry_set);
    let reopened_view = match scan_root_entry(&root_bytes, ROOT_FILE_ENTRY_INDEX).unwrap() {
        ScannedDirectoryEntry::File(view) => view,
        _ => panic!("expected republished file entry set"),
    };
    let reopened_slot_range = reopened_view.slot_range();
    let reopened_name = reopened_view.name().unwrap();

    assert_eq!(reopened_slot_range, source_slot_range);
    assert_eq!(reopened_name, republished_name);
    assert_eq!(reopened_view.stored_name_hash(), republished_name_hash);

    let no_drift_entry_set = direntry::republished_entry_set(
        reopened_view,
        &FileEntrySetFieldUpdates {
            create_fields: Some(([0x11, 0x22, 0x33, 0x44], 0x55, 0x66)),
            file_attributes: Some(FILE_ATTRIBUTE_REGULAR),
            last_accessed_fields: Some(([0x77, 0x88, 0x99, 0xAA], 0xBB)),
            last_modified_fields: Some(([0xCC, 0xDD, 0xEE, 0x0F], 0x10, 0x11)),
            name: Some(&republished_name),
            name_hash: Some(republished_name_hash),
            ..Default::default()
        },
    )
    .unwrap();

    assert_eq!(no_drift_entry_set, republished_entry_set);
}

#[ktest]
fn directory_entry_field_update_substrate_slot_cleanup_invalidates_reserved_bytes_without_publication(
) {
    init_lookup_test_runtime();

    let disk = ExfatLookupTestDisk::new();
    let mut root_bytes = root_directory_bytes(&disk);
    let slot_range = DirectoryEntrySlotRange::new(ROOT_FILE_ENTRY_INDEX, 3).unwrap();
    let staging_name: Vec<u16> = "StageFile".encode_utf16().collect();
    let staging_entry_set =
        direntry::encode_file_entry_set(&staging_name, 0x31B5, InodeType::File, 0, 0, false)
            .unwrap();
    root_bytes[root_slot_bytes(slot_range)].copy_from_slice(&staging_entry_set);

    let reserved_slot_bytes = root_bytes.get_mut(root_slot_bytes(slot_range)).unwrap();
    let mut reserved_slot_span =
        WritableDirectoryEntrySlotSpan::new(slot_range, reserved_slot_bytes).unwrap();
    direntry::invalidate_entry_set(&mut reserved_slot_span).unwrap();

    let invalidated_entry_set = &root_bytes[root_slot_bytes(slot_range)];
    match scan_root_entry(&root_bytes, ROOT_FILE_ENTRY_INDEX).unwrap() {
        ScannedDirectoryEntry::Vacant(found_slot_range) => {
            assert_eq!(found_slot_range.first_entry_index(), ROOT_FILE_ENTRY_INDEX);
        }
        _ => panic!("expected invalidated slot span to stay unpublished"),
    }
    assert_entry_set_invalidated(invalidated_entry_set);
}

#[ktest]
fn directory_entry_field_update_substrate_refuses_invalid_targets_and_layout_changes() {
    init_lookup_test_runtime();

    let fractured_disk = ExfatLookupTestDisk::new();
    fractured_disk.install_root_fractured_entry_set(ROOT_FILE_ENTRY_INDEX, "Broken");
    let fractured_root_bytes = root_directory_bytes(&fractured_disk);
    match scan_root_entry(&fractured_root_bytes, ROOT_FILE_ENTRY_INDEX).unwrap() {
        ScannedDirectoryEntry::Anomaly { kind, .. } => {
            assert_eq!(kind, DirectoryEntryAnomalyKind::BrokenEntrySet);
        }
        _ => panic!("expected checksum-broken anomaly"),
    }

    let critical_disk = ExfatLookupTestDisk::new();
    critical_disk.install_root_unrecognized_critical_entry(ROOT_FILE_ENTRY_INDEX);
    let critical_root_bytes = root_directory_bytes(&critical_disk);
    match scan_root_entry(&critical_root_bytes, ROOT_FILE_ENTRY_INDEX).unwrap() {
        ScannedDirectoryEntry::Anomaly { .. } => {}
        _ => panic!("expected critical anomaly"),
    }

    let fractured_bytes_before = fractured_disk.read_root_entries(ROOT_FILE_ENTRY_INDEX, 3);
    let (_fs, fractured_root) = mount_root(&fractured_disk, None);
    let fractured_error = fractured_root
        .rename("Broken", &fractured_root, "Retried")
        .unwrap_err();

    assert_eq!(fractured_error.error(), Errno::EUCLEAN);
    assert_eq!(
        fractured_disk.read_root_entries(ROOT_FILE_ENTRY_INDEX, 3),
        fractured_bytes_before
    );

    let critical_bytes_before = critical_disk.read_root_entries(ROOT_FILE_ENTRY_INDEX, 2);
    let (_fs, critical_root) = mount_root(&critical_disk, None);
    let critical_error = critical_root
        .rename("Broken", &critical_root, "Retried")
        .unwrap_err();

    assert_eq!(critical_error.error(), Errno::EUCLEAN);
    assert_eq!(
        critical_disk.read_root_entries(ROOT_FILE_ENTRY_INDEX, 2),
        critical_bytes_before
    );

    let valid_disk = ExfatLookupTestDisk::new();
    valid_disk.install_root_file(ROOT_FILE_ENTRY_INDEX, "ShortName");
    let valid_root_bytes = root_directory_bytes(&valid_disk);
    let source_view = match scan_root_entry(&valid_root_bytes, ROOT_FILE_ENTRY_INDEX).unwrap() {
        ScannedDirectoryEntry::File(view) => view,
        _ => panic!("expected published file entry set"),
    };
    let longer_name: Vec<u16> = "LongerLayoutChangeName".encode_utf16().collect();
    let error = direntry::republished_entry_set(
        source_view,
        &FileEntrySetFieldUpdates {
            name: Some(&longer_name),
            name_hash: Some(0x5E17),
            ..Default::default()
        },
    )
    .unwrap_err();

    assert_eq!(error, MountVolumeStateError::InvalidOperationInput);
}

#[ktest]
fn directory_entry_field_update_substrate_repeated_republish_preserves_identity_and_checksum() {
    init_lookup_test_runtime();

    let disk = ExfatLookupTestDisk::new();
    let (_fs, _root_inode, parent_inode) = mount_create_parent(&disk, FsFlags::empty(), None);
    let created = parent_inode
        .create("AlphaOne", InodeType::File, InodeMode::all())
        .unwrap();
    let created_entry_index = entry_index_from_ino(created.ino());

    parent_inode
        .rename("AlphaOne", &parent_inode, "GammaOne")
        .unwrap();

    let first_republished = parent_inode.lookup("gammaone").unwrap();
    let first_entry_set = disk.read_directory_entries(TEST_PARENT_CLUSTER, created_entry_index, 3);

    assert_eq!(first_republished.ino(), created.ino());
    assert_eq!(usize::from(first_entry_set[1]), 2);
    assert_eq!(
        u16::from_le_bytes([first_entry_set[2], first_entry_set[3]]),
        entry_set_checksum(&first_entry_set, 2)
    );
    assert_eq!(
        decode_entry_name(&first_entry_set),
        "GammaOne".encode_utf16().collect::<Vec<_>>()
    );

    parent_inode
        .rename("GammaOne", &parent_inode, "DeltaOne")
        .unwrap();

    let second_republished = parent_inode.lookup("deltaone").unwrap();
    let second_entry_set = disk.read_directory_entries(TEST_PARENT_CLUSTER, created_entry_index, 3);
    let (_visited_count, second_entries) = collect_dirents(&parent_inode, 2);

    assert_eq!(second_republished.ino(), created.ino());
    assert_eq!(second_republished.ino(), first_republished.ino());
    assert_eq!(usize::from(second_entry_set[1]), 2);
    assert_eq!(
        u16::from_le_bytes([second_entry_set[2], second_entry_set[3]]),
        entry_set_checksum(&second_entry_set, 2)
    );
    assert_eq!(
        decode_entry_name(&second_entry_set),
        "DeltaOne".encode_utf16().collect::<Vec<_>>()
    );
    assert_eq!(entry_names(&second_entries), vec!["DeltaOne"]);
    assert_eq!(visible_name_count(&second_entries, "GammaOne"), 0);

    let bytes_before_noop = second_entry_set.clone();
    parent_inode
        .rename("DeltaOne", &parent_inode, "DeltaOne")
        .unwrap();

    assert_eq!(
        disk.read_directory_entries(TEST_PARENT_CLUSTER, created_entry_index, 3),
        bytes_before_noop
    );
}

#[ktest]
fn directory_entry_field_update_substrate_integration_publish_republish_rescan_invalidate_stays_coherent(
) {
    init_lookup_test_runtime();

    let disk = ExfatLookupTestDisk::new();
    let (_fs, _root_inode, parent_inode) = mount_create_parent(&disk, FsFlags::empty(), None);

    let created = parent_inode
        .create("AlphaOne", InodeType::File, InodeMode::all())
        .unwrap();
    let created_entry_index = entry_index_from_ino(created.ino());

    parent_inode
        .rename("AlphaOne", &parent_inode, "GammaOne")
        .unwrap();

    let republished = parent_inode.lookup("gammaone").unwrap();
    let (_visited_count, republished_entries) = collect_dirents(&parent_inode, 2);
    let republished_entry_set =
        disk.read_directory_entries(TEST_PARENT_CLUSTER, created_entry_index, 3);

    assert_eq!(republished.ino(), created.ino());
    assert_eq!(entry_names(&republished_entries), vec!["GammaOne"]);
    assert_eq!(visible_name_count(&republished_entries, "AlphaOne"), 0);
    assert_eq!(republished_entry_set[0], FILE_DIRECTORY_ENTRY_TYPE);
    assert_eq!(usize::from(republished_entry_set[1]), 2);
    assert_eq!(
        u16::from_le_bytes([republished_entry_set[2], republished_entry_set[3]]),
        entry_set_checksum(&republished_entry_set, 2)
    );
    assert_eq!(
        decode_entry_name(&republished_entry_set),
        "GammaOne".encode_utf16().collect::<Vec<_>>()
    );

    parent_inode.unlink("GammaOne").unwrap();

    let removed_entry_set =
        disk.read_directory_entries(TEST_PARENT_CLUSTER, created_entry_index, 3);
    let (_visited_count, removed_entries) = collect_dirents(&parent_inode, 2);

    assert_eq!(lookup_error(&parent_inode, "GammaOne"), Errno::ENOENT);
    assert!(removed_entries.is_empty());
    assert_entry_set_invalidated(&removed_entry_set);
}

#[ktest]
fn directory_entry_field_update_substrate_failure_maintenance_preserves_neighboring_unknown_bytes()
{
    init_lookup_test_runtime();

    let disk = ExfatLookupTestDisk::new();
    disk.install_root_file(ROOT_FILE_ENTRY_INDEX, "AlphaOne");
    disk.install_root_unrecognized_benign_entry(ROOT_SECOND_FILE_ENTRY_INDEX);
    let root_bytes_before = disk.read_root_entries(ROOT_FILE_ENTRY_INDEX, 5);
    let failing_disk = ExfatLookupToggleFailingWriteDisk::new(
        disk.clone(),
        disk.root_directory_offset(),
        disk.root_cluster_size(),
    );
    let block_device: Arc<dyn BlockDevice> = failing_disk.clone();
    let (_fs, root_inode) = mount_root_from_block_device(block_device, FsFlags::empty(), None);

    failing_disk.enable_failures();
    let rename_error = root_inode
        .rename("AlphaOne", &root_inode, "GammaOne")
        .unwrap_err();
    let (_visited_count, final_entries) = collect_dirents(&root_inode, 2);

    assert_eq!(rename_error.error(), Errno::EIO);
    assert_eq!(
        disk.read_root_entries(ROOT_FILE_ENTRY_INDEX, 5),
        root_bytes_before
    );
    assert!(root_inode.lookup("alphaone").is_ok());
    assert_eq!(lookup_error(&root_inode, "GammaOne"), Errno::ENOENT);
    assert_eq!(entry_names(&final_entries), vec!["AlphaOne"]);
}

#[ktest]
fn directory_entry_field_update_substrate_concurrency_serializes_republish_with_removal_and_reuse()
{
    init_lookup_test_runtime();

    let disk = ExfatLookupTestDisk::new();
    let (_fs, _root_inode, parent_inode) = mount_create_parent(&disk, FsFlags::empty(), None);
    let created = parent_inode
        .create("RaceFile", InodeType::File, InodeMode::all())
        .unwrap();
    let created_entry_index = entry_index_from_ino(created.ino());

    let ready_count = Arc::new(AtomicUsize::new(0));
    let rename_result = Arc::new(Mutex::new(None));
    let remove_and_reuse_result = Arc::new(Mutex::new(None));

    let rename_thread = {
        let ready_count = ready_count.clone();
        let rename_result = rename_result.clone();
        let parent_inode = parent_inode.clone();

        ThreadOptions::new(move || {
            wait_for_concurrent_start(&ready_count, 2);
            *rename_result.lock() = Some(
                parent_inode
                    .rename("RaceFile", &parent_inode, "FaceFile")
                    .map_err(|error| error.error()),
            );
        })
        .spawn()
    };

    let remove_and_reuse_thread = {
        let ready_count = ready_count.clone();
        let remove_and_reuse_result = remove_and_reuse_result.clone();
        let parent_inode = parent_inode.clone();

        ThreadOptions::new(move || {
            wait_for_concurrent_start(&ready_count, 2);
            let unlink_result = parent_inode
                .unlink("RaceFile")
                .map_err(|error| error.error());
            let create_result = parent_inode
                .create("ReuseNew", InodeType::File, InodeMode::all())
                .map(|inode| inode.ino())
                .map_err(|error| error.error());
            *remove_and_reuse_result.lock() = Some((unlink_result, create_result));
        })
        .spawn()
    };

    rename_thread.join();
    remove_and_reuse_thread.join();

    let rename_result = rename_result.lock().take().unwrap();
    let (unlink_result, create_result) = remove_and_reuse_result.lock().take().unwrap();
    let (_visited_count, final_entries) = collect_dirents(&parent_inode, 2);

    assert_eq!(lookup_error(&parent_inode, "RaceFile"), Errno::ENOENT);
    assert!(matches!(create_result, Ok(_)));

    match (rename_result, unlink_result, create_result.unwrap()) {
        (Ok(()), Err(Errno::ENOENT), _) => {
            let renamed = parent_inode.lookup("facefile").unwrap();

            assert_eq!(renamed.ino(), created.ino());
            assert_eq!(visible_name_count(&final_entries, "FaceFile"), 1);
            assert_eq!(visible_name_count(&final_entries, "ReuseNew"), 1);
        }
        (Err(Errno::ENOENT), Ok(()), reused_ino) => {
            assert_eq!(lookup_error(&parent_inode, "FaceFile"), Errno::ENOENT);
            assert_eq!(entry_index_from_ino(reused_ino), created_entry_index);
            assert_eq!(visible_name_count(&final_entries, "FaceFile"), 0);
            assert_eq!(visible_name_count(&final_entries, "ReuseNew"), 1);
        }
        other => panic!("unexpected serialized outcome: {:?}", other),
    }
}
