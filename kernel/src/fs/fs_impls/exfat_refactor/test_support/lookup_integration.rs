// SPDX-License-Identifier: MPL-2.0

use super::{
    CString, Errno, ExfatFsType, ExfatLookupTestDisk, FsFlags, ROOT_FILE_ENTRY_INDEX,
    collect_dirents, entry_names, init_lookup_test_runtime, lookup_error, mount_root,
};

#[ktest]
fn lookup_and_readdir_reject_non_utf8_iocharset_mount_option() {
    init_lookup_test_runtime();

    let utf8_disk = ExfatLookupTestDisk::new();
    utf8_disk.install_root_file(ROOT_FILE_ENTRY_INDEX, "Utf8Name");
    let (_fs, utf8_root) = mount_root(&utf8_disk, Some("iocharset=utf8"));
    let (_visited_count, utf8_entries) = collect_dirents(&utf8_root, 2);

    assert_eq!(
        utf8_root.lookup("utf8name").unwrap().ino(),
        utf8_entries[0].ino
    );
    assert_eq!(entry_names(&utf8_entries), vec!["Utf8Name"]);

    let non_utf8_disk = ExfatLookupTestDisk::new();
    let args = CString::new("iocharset=cp437").unwrap();
    let mount_error = match ExfatFsType.create(
        FsFlags::empty(),
        Some(args),
        Some(non_utf8_disk.as_block_device()),
    ) {
        Ok(_) => panic!("non-UTF-8 exFAT iocharset mount unexpectedly succeeded"),
        Err(error) => error,
    };

    assert_eq!(mount_error.error(), Errno::EINVAL);
}

pub(super) fn directory_lookup_and_identity_integration_success_path_coheres_lookup_and_readdir() {
    init_lookup_test_runtime();

    let disk = ExfatLookupTestDisk::new();
    disk.install_root_file(ROOT_FILE_ENTRY_INDEX, "MixedCase");
    disk.install_root_file(ROOT_FILE_ENTRY_INDEX + 3, "AliasName");
    let (_fs, root_inode) = mount_root(&disk, None);

    let mixed_canonical = root_inode.lookup("MixedCase").unwrap();
    let mixed_folded = root_inode.lookup("mixedcase").unwrap();
    let mixed_trailing_dot = root_inode.lookup("MIXEDCASE...").unwrap();
    let alias_canonical = root_inode.lookup("AliasName").unwrap();
    let alias_folded = root_inode.lookup("aliasname").unwrap();
    let alias_trailing_dot = root_inode.lookup("ALIASNAME.").unwrap();
    let (visited_count, entries) = collect_dirents(&root_inode, 0);

    assert_eq!(mixed_canonical.ino(), mixed_folded.ino());
    assert_eq!(mixed_canonical.ino(), mixed_trailing_dot.ino());
    assert_eq!(alias_canonical.ino(), alias_folded.ino());
    assert_eq!(alias_canonical.ino(), alias_trailing_dot.ino());
    assert_ne!(mixed_canonical.ino(), alias_canonical.ino());
    assert_eq!(visited_count, 4);
    assert_eq!(
        entry_names(&entries),
        vec![".", "..", "MixedCase", "AliasName"]
    );
    assert_eq!(entries[2].ino, mixed_canonical.ino());
    assert_eq!(entries[3].ino, alias_canonical.ino());

    let keep_last_dots_disk = ExfatLookupTestDisk::new();
    keep_last_dots_disk.install_root_file(ROOT_FILE_ENTRY_INDEX, "Trailing");
    let (_fs, keep_last_dots_root) = mount_root(&keep_last_dots_disk, Some("keep_last_dots"));
    let (_visited_count, keep_last_dots_entries) = collect_dirents(&keep_last_dots_root, 2);

    assert_eq!(
        keep_last_dots_root.lookup("trailing").unwrap().ino(),
        keep_last_dots_entries[0].ino
    );
    assert_eq!(
        lookup_error(&keep_last_dots_root, "TRAILING."),
        Errno::ENOENT
    );
    assert_eq!(entry_names(&keep_last_dots_entries), vec!["Trailing"]);
}

pub(super) fn directory_lookup_and_identity_integration_failure_path_preserves_typed_boundaries() {
    init_lookup_test_runtime();

    let fractured_disk = ExfatLookupTestDisk::new();
    fractured_disk.install_root_file(ROOT_FILE_ENTRY_INDEX, "BeforeBroken");
    fractured_disk.install_root_fractured_entry_set(ROOT_FILE_ENTRY_INDEX + 3, "Broken");
    let (_fs, fractured_root) = mount_root(&fractured_disk, None);
    let mut fractured_entries = Vec::<CapturedDirent>::new();
    let fractured_readdir_error = fractured_root
        .readdir_at(0, &mut fractured_entries)
        .unwrap_err();

    assert_eq!(fractured_readdir_error.error(), Errno::EUCLEAN);
    assert_eq!(lookup_error(&fractured_root, "Broken"), Errno::EUCLEAN);
    assert_eq!(
        entry_names(&fractured_entries),
        vec![".", "..", "BeforeBroken"]
    );

    let critical_disk = ExfatLookupTestDisk::new();
    critical_disk.install_root_file(ROOT_FILE_ENTRY_INDEX, "BeforeCritical");
    critical_disk.install_root_unrecognized_critical_entry(ROOT_FILE_ENTRY_INDEX + 3);
    let (_fs, critical_root) = mount_root(&critical_disk, None);
    let mut critical_entries = Vec::<CapturedDirent>::new();
    let critical_readdir_error = critical_root
        .readdir_at(0, &mut critical_entries)
        .unwrap_err();

    assert_eq!(critical_readdir_error.error(), Errno::EUCLEAN);
    assert_eq!(lookup_error(&critical_root, "Missing"), Errno::EUCLEAN);
    assert_eq!(
        entry_names(&critical_entries),
        vec![".", "..", "BeforeCritical"]
    );

    let benign_disk = ExfatLookupTestDisk::new();
    benign_disk.install_root_unrecognized_benign_entry(ROOT_FILE_ENTRY_INDEX);
    benign_disk.install_root_file(ROOT_FILE_ENTRY_INDEX + 1, "Visible");
    let (_fs, benign_root) = mount_root(&benign_disk, None);
    let (_visited_count, benign_entries) = collect_dirents(&benign_root, 0);

    assert_eq!(lookup_error(&benign_root, "Missing"), Errno::ENOENT);
    assert_eq!(
        benign_root.lookup("visible").unwrap().ino(),
        benign_entries[2].ino
    );
    assert_eq!(entry_names(&benign_entries), vec![".", "..", "Visible"]);

    let stale_negative_disk = ExfatLookupTestDisk::new();
    let (_fs, stale_negative_root) = mount_root(&stale_negative_disk, None);
    assert_eq!(
        lookup_error(&stale_negative_root, "FreshFile"),
        Errno::ENOENT
    );

    stale_negative_disk.install_root_file(ROOT_FILE_ENTRY_INDEX, "FreshFile");
    let fresh_lookup = stale_negative_root.lookup("freshfile").unwrap();
    let (_visited_count, refreshed_entries) = collect_dirents(&stale_negative_root, 2);

    assert_eq!(entry_names(&refreshed_entries), vec!["FreshFile"]);
    assert_eq!(fresh_lookup.ino(), refreshed_entries[0].ino);
}

pub(super) fn directory_lookup_and_identity_integration_repeated_calls_stay_stable() {
    init_lookup_test_runtime();

    let disk = ExfatLookupTestDisk::new();
    disk.install_root_file(ROOT_FILE_ENTRY_INDEX, "RepeatOne");
    disk.install_root_file(ROOT_FILE_ENTRY_INDEX + 3, "RepeatTwo");
    let (_fs, root_inode) = mount_root(&disk, None);

    let first_lookup = root_inode.lookup("repeatone").unwrap();
    let repeated_lookup = root_inode.lookup("RepeatOne").unwrap();
    let alias_lookup = root_inode.lookup("REPEATONE.").unwrap();
    let second_lookup = root_inode.lookup("repeattwo").unwrap();
    let (first_count, first_entries) = collect_dirents(&root_inode, 0);
    let (second_count, second_entries) = collect_dirents(&root_inode, 0);
    let (visible_count, visible_entries) = collect_dirents(&root_inode, 2);

    assert_eq!(first_lookup.ino(), repeated_lookup.ino());
    assert_eq!(first_lookup.ino(), alias_lookup.ino());
    assert_eq!(first_count, second_count);
    assert_eq!(first_entries, second_entries);
    assert_eq!(visible_count, 2);
    assert_eq!(
        entry_names(&visible_entries),
        vec!["RepeatOne", "RepeatTwo"]
    );
    assert_eq!(visible_entries[0].ino, first_lookup.ino());
    assert_eq!(visible_entries[1].ino, second_lookup.ino());
}
