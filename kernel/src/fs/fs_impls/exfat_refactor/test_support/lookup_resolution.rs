// SPDX-License-Identifier: MPL-2.0

use alloc::{ffi::CString, sync::Arc};

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

fn init_lookup_test_runtime() {
    crate::time::clocks::init_for_ktest();
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
