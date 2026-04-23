// SPDX-License-Identifier: MPL-2.0

use alloc::{ffi::CString, string::String, sync::Arc, vec, vec::Vec};

use ostd::prelude::ktest;

use super::super::{
    fs::ExfatFsType,
    test_support::inode::ExfatLookupTestDisk,
};
use super::*;
use crate::fs::vfs::{
    file_system::{FileSystem, FsFlags},
    registry::FsType,
};

const ROOT_FILE_ENTRY_INDEX: usize = 4;

#[derive(Debug, Eq, PartialEq)]
struct CapturedDirent {
    name: String,
    ino: u64,
    inode_type: InodeType,
    offset: usize,
}

impl DirentVisitor for Vec<CapturedDirent> {
    fn visit(
        &mut self,
        name: &str,
        ino: u64,
        inode_type: InodeType,
        offset: usize,
    ) -> Result<()> {
        self.push(CapturedDirent {
            name: name.into(),
            ino,
            inode_type,
            offset,
        });
        Ok(())
    }
}

struct RejectingDirentVisitor {
    entries: Vec<String>,
    reject_name: &'static str,
}

impl DirentVisitor for RejectingDirentVisitor {
    fn visit(
        &mut self,
        name: &str,
        _ino: u64,
        _inode_type: InodeType,
        _offset: usize,
    ) -> Result<()> {
        self.entries.push(name.into());
        if name == self.reject_name {
            return Err(Error::new(Errno::EOVERFLOW));
        }
        Ok(())
    }
}

fn init_lookup_test_runtime() {
    crate::time::clocks::init_for_ktest();
}

fn collect_dirents(inode: &Arc<dyn Inode>, offset: usize) -> (usize, Vec<CapturedDirent>) {
    let mut entries = Vec::new();
    let visited_count = inode.readdir_at(offset, &mut entries).unwrap();
    (visited_count, entries)
}

fn entry_names(entries: &[CapturedDirent]) -> Vec<&str> {
    entries.iter().map(|entry| entry.name.as_str()).collect()
}

fn entry_offsets(entries: &[CapturedDirent]) -> Vec<usize> {
    entries.iter().map(|entry| entry.offset).collect()
}

fn lookup_error(inode: &Arc<dyn Inode>, name: &str) -> Errno {
    inode.lookup(name).unwrap_err().error()
}

fn mount_root(
    disk: &Arc<ExfatLookupTestDisk>,
    options: Option<&str>,
) -> (Arc<dyn FileSystem>, Arc<dyn Inode>) {
    let args = options.map(|options| CString::new(options).unwrap());
    let fs = ExfatFsType
        .create(FsFlags::empty(), args, Some(disk.as_block_device()))
        .unwrap();
    let root_inode = fs.root_inode();
    (fs, root_inode)
}

#[ktest]
fn lookup_resolution_matches_mixed_case_and_trailing_dot_equivalence() {
    init_lookup_test_runtime();

    let disk = ExfatLookupTestDisk::new();
    disk.install_root_file(ROOT_FILE_ENTRY_INDEX, "MixedCase");
    let (_fs, root_inode) = mount_root(&disk, None);

    let mixed_case = root_inode.lookup("mixedcase").unwrap();
    let trailing_dot = root_inode.lookup("MIXEDCASE...").unwrap();
    assert_eq!(mixed_case.ino(), trailing_dot.ino());

    let keep_last_dots_disk = ExfatLookupTestDisk::new();
    keep_last_dots_disk.install_root_file(ROOT_FILE_ENTRY_INDEX, "MixedCase");
    let (_fs, keep_last_dots_root) = mount_root(&keep_last_dots_disk, Some("keep_last_dots"));

    assert_eq!(
        keep_last_dots_root.lookup("mixedcase").unwrap().ino(),
        mixed_case.ino()
    );
    assert_eq!(lookup_error(&keep_last_dots_root, "MIXEDCASE..."), Errno::ENOENT);
}

#[ktest]
fn lookup_resolution_reuses_identity_for_alias_equivalent_spellings() {
    init_lookup_test_runtime();

    let disk = ExfatLookupTestDisk::new();
    disk.install_root_file(ROOT_FILE_ENTRY_INDEX, "AliasName");
    let (_fs, root_inode) = mount_root(&disk, None);

    let canonical = root_inode.lookup("AliasName").unwrap();
    let alias = root_inode.lookup("aliasname").unwrap();
    let trailing_alias = root_inode.lookup("ALIASNAME.").unwrap();

    assert_eq!(canonical.ino(), alias.ino());
    assert_eq!(canonical.ino(), trailing_alias.ino());
}

#[ktest]
fn lookup_resolution_distinguishes_absence_from_invalid_input() {
    init_lookup_test_runtime();

    let disk = ExfatLookupTestDisk::new();
    disk.install_root_file(ROOT_FILE_ENTRY_INDEX, "Present");
    let (_fs, root_inode) = mount_root(&disk, None);

    assert_eq!(lookup_error(&root_inode, "missing"), Errno::ENOENT);
    assert_eq!(lookup_error(&root_inode, "invalid/name"), Errno::EINVAL);
    assert_eq!(lookup_error(&root_inode, &"a".repeat(256)), Errno::ENAMETOOLONG);
}

#[ktest]
fn lookup_resolution_reports_integrity_failures_for_fractured_or_unrecognized_entry_sets() {
    init_lookup_test_runtime();

    let fractured_disk = ExfatLookupTestDisk::new();
    fractured_disk.install_root_fractured_entry_set(ROOT_FILE_ENTRY_INDEX, "Broken");
    let (_fs, fractured_root) = mount_root(&fractured_disk, None);
    assert_eq!(lookup_error(&fractured_root, "Broken"), Errno::EUCLEAN);

    let unrecognized_disk = ExfatLookupTestDisk::new();
    unrecognized_disk.install_root_unrecognized_critical_entry(ROOT_FILE_ENTRY_INDEX);
    let (_fs, unrecognized_root) = mount_root(&unrecognized_disk, None);
    assert_eq!(lookup_error(&unrecognized_root, "Broken"), Errno::EUCLEAN);
}

#[ktest]
fn lookup_resolution_rechecks_without_trusting_stale_negative_state() {
    init_lookup_test_runtime();

    let disk = ExfatLookupTestDisk::new();
    let (_fs, root_inode) = mount_root(&disk, None);

    assert_eq!(lookup_error(&root_inode, "FreshFile"), Errno::ENOENT);

    disk.install_root_file(ROOT_FILE_ENTRY_INDEX, "FreshFile");

    let fresh_lookup = root_inode.lookup("freshfile").unwrap();
    assert_ne!(fresh_lookup.ino(), root_inode.ino());
}

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
    assert_eq!(entry_names(&full_entries), vec![".", "..", "First", "Second"]);
    assert_eq!(visible_count, 2);
    assert_eq!(entry_names(&visible_entries), vec!["First", "Second"]);
    assert_eq!(entry_offsets(&visible_entries), vec![2, 3]);
    assert_eq!(last_count, 1);
    assert_eq!(entry_names(&last_entries), vec!["Second"]);
    assert_eq!(entry_offsets(&last_entries), vec![3]);
    assert_eq!(end_count, 0);
    assert!(end_entries.is_empty());
}

#[ktest]
fn lookup_and_readdir_reject_non_utf8_iocharset_mount_option() {
    init_lookup_test_runtime();

    let utf8_disk = ExfatLookupTestDisk::new();
    utf8_disk.install_root_file(ROOT_FILE_ENTRY_INDEX, "Utf8Name");
    let (_fs, utf8_root) = mount_root(&utf8_disk, Some("iocharset=utf8"));
    let (_visited_count, utf8_entries) = collect_dirents(&utf8_root, 2);

    assert_eq!(utf8_root.lookup("utf8name").unwrap().ino(), utf8_entries[0].ino);
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

#[ktest]
fn directory_lookup_and_identity_integration_success_path_coheres_lookup_and_readdir() {
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
    assert_eq!(entry_names(&entries), vec![".", "..", "MixedCase", "AliasName"]);
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
    assert_eq!(lookup_error(&keep_last_dots_root, "TRAILING."), Errno::ENOENT);
    assert_eq!(entry_names(&keep_last_dots_entries), vec!["Trailing"]);
}

#[ktest]
fn directory_lookup_and_identity_integration_failure_path_preserves_typed_boundaries() {
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
    assert_eq!(entry_names(&fractured_entries), vec![".", "..", "BeforeBroken"]);

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
    assert_eq!(entry_names(&critical_entries), vec![".", "..", "BeforeCritical"]);

    let benign_disk = ExfatLookupTestDisk::new();
    benign_disk.install_root_unrecognized_benign_entry(ROOT_FILE_ENTRY_INDEX);
    benign_disk.install_root_file(ROOT_FILE_ENTRY_INDEX + 1, "Visible");
    let (_fs, benign_root) = mount_root(&benign_disk, None);
    let (_visited_count, benign_entries) = collect_dirents(&benign_root, 0);

    assert_eq!(lookup_error(&benign_root, "Missing"), Errno::ENOENT);
    assert_eq!(benign_root.lookup("visible").unwrap().ino(), benign_entries[2].ino);
    assert_eq!(entry_names(&benign_entries), vec![".", "..", "Visible"]);

    let stale_negative_disk = ExfatLookupTestDisk::new();
    let (_fs, stale_negative_root) = mount_root(&stale_negative_disk, None);
    assert_eq!(lookup_error(&stale_negative_root, "FreshFile"), Errno::ENOENT);

    stale_negative_disk.install_root_file(ROOT_FILE_ENTRY_INDEX, "FreshFile");
    let fresh_lookup = stale_negative_root.lookup("freshfile").unwrap();
    let (_visited_count, refreshed_entries) = collect_dirents(&stale_negative_root, 2);

    assert_eq!(entry_names(&refreshed_entries), vec!["FreshFile"]);
    assert_eq!(fresh_lookup.ino(), refreshed_entries[0].ino);
}

#[ktest]
fn directory_lookup_and_identity_integration_repeated_calls_stay_stable() {
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
    assert_ne!(first_lookup.ino(), second_lookup.ino());
    assert_eq!(first_count, 4);
    assert_eq!(second_count, first_count);
    assert_eq!(visible_count, 2);
    assert_eq!(first_entries, second_entries);
    assert_eq!(entry_names(&first_entries), vec![".", "..", "RepeatOne", "RepeatTwo"]);
    assert_eq!(entry_names(&visible_entries), vec!["RepeatOne", "RepeatTwo"]);
    assert_eq!(visible_entries[0].ino, first_lookup.ino());
    assert_eq!(visible_entries[1].ino, second_lookup.ino());
}
