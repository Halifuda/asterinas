// SPDX-License-Identifier: MPL-2.0

use ostd::prelude::ktest;

use super::*;

#[ktest]
fn readdir_visibility_emits_visible_entries_in_scan_order() {
    init_lookup_test_runtime();

    let disk = ExfatLookupTestDisk::new();
    disk.install_root_file(ROOT_FILE_ENTRY_INDEX, "Alpha");
    disk.install_root_unrecognized_benign_entry(ROOT_FILE_ENTRY_INDEX + 3);
    disk.install_root_file(ROOT_FILE_ENTRY_INDEX + 4, "Beta");
    let (_fs, root_inode) = mount_root(&disk, None);

    let (visited_count, entries) = collect_dirents(&root_inode, 0);

    assert_eq!(visited_count, 4);
    assert_eq!(entry_names(&entries), vec![".", "..", "Alpha", "Beta"]);
    assert_eq!(entry_offsets(&entries), vec![0, 1, 2, 3]);
    assert_eq!(entries[0].ino, root_inode.ino());
    assert_eq!(entries[1].ino, root_inode.ino());
    assert_eq!(entries[2].inode_type, InodeType::File);
    assert_eq!(entries[3].inode_type, InodeType::File);
    assert_eq!(entries[2].ino, root_inode.lookup("alpha").unwrap().ino());
    assert_eq!(entries[3].ino, root_inode.lookup("BETA").unwrap().ino());
    assert_ne!(entries[2].ino, entries[3].ino);
}

#[ktest]
fn readdir_visibility_stops_on_visitor_rejection() {
    init_lookup_test_runtime();

    let disk = ExfatLookupTestDisk::new();
    disk.install_root_file(ROOT_FILE_ENTRY_INDEX, "Alpha");
    disk.install_root_file(ROOT_FILE_ENTRY_INDEX + 3, "StopHere");
    disk.install_root_file(ROOT_FILE_ENTRY_INDEX + 6, "Later");
    let (_fs, root_inode) = mount_root(&disk, None);
    let mut visitor = RejectingDirentVisitor {
        entries: Vec::new(),
        reject_name: "StopHere",
    };

    let error = root_inode.readdir_at(0, &mut visitor).unwrap_err();

    assert_eq!(error.error(), Errno::EOVERFLOW);
    assert_eq!(visitor.entries, vec![".", "..", "Alpha", "StopHere"]);
}

#[ktest]
fn readdir_visibility_reports_integrity_failures_for_fractured_or_unrecognized_entry_sets() {
    init_lookup_test_runtime();

    let fractured_disk = ExfatLookupTestDisk::new();
    fractured_disk.install_root_fractured_entry_set(ROOT_FILE_ENTRY_INDEX, "Broken");
    let (_fs, fractured_root) = mount_root(&fractured_disk, None);
    let mut fractured_entries = Vec::<CapturedDirent>::new();
    let fractured_error = fractured_root
        .readdir_at(0, &mut fractured_entries)
        .unwrap_err();

    assert_eq!(fractured_error.error(), Errno::EUCLEAN);
    assert_eq!(lookup_error(&fractured_root, "Broken"), Errno::EUCLEAN);
    assert_eq!(entry_names(&fractured_entries), vec![".", ".."]);

    let unrecognized_disk = ExfatLookupTestDisk::new();
    unrecognized_disk.install_root_unrecognized_critical_entry(ROOT_FILE_ENTRY_INDEX);
    let (_fs, unrecognized_root) = mount_root(&unrecognized_disk, None);
    let mut unrecognized_entries = Vec::<CapturedDirent>::new();
    let unrecognized_error = unrecognized_root
        .readdir_at(0, &mut unrecognized_entries)
        .unwrap_err();

    assert_eq!(unrecognized_error.error(), Errno::EUCLEAN);
    assert_eq!(lookup_error(&unrecognized_root, "Broken"), Errno::EUCLEAN);
    assert_eq!(entry_names(&unrecognized_entries), vec![".", ".."]);
}

#[ktest]
fn readdir_visibility_preserves_progress_across_repeated_calls() {
    init_lookup_test_runtime();

    let disk = ExfatLookupTestDisk::new();
    disk.install_root_file(ROOT_FILE_ENTRY_INDEX, "First");
    disk.install_root_file(ROOT_FILE_ENTRY_INDEX + 3, "Second");
    let (_fs, root_inode) = mount_root(&disk, None);

    let (full_count, full_entries) = collect_dirents(&root_inode, 0);
    let (visible_count, visible_entries) = collect_dirents(&root_inode, 2);
    let (last_count, last_entries) = collect_dirents(&root_inode, 3);
    let (end_count, end_entries) = collect_dirents(&root_inode, 2 + visible_count);

    assert_eq!(full_count, 4);
    assert_eq!(
        entry_names(&full_entries),
        vec![".", "..", "First", "Second"]
    );
    assert_eq!(visible_count, 2);
    assert_eq!(entry_names(&visible_entries), vec!["First", "Second"]);
    assert_eq!(entry_offsets(&visible_entries), vec![2, 3]);
    assert_eq!(last_count, 1);
    assert_eq!(entry_names(&last_entries), vec!["Second"]);
    assert_eq!(entry_offsets(&last_entries), vec![3]);
    assert_eq!(end_count, 0);
    assert!(end_entries.is_empty());
}
