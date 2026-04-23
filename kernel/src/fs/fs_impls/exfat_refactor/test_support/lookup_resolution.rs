// SPDX-License-Identifier: MPL-2.0

use alloc::{ffi::CString, format, string::String, sync::Arc, vec, vec::Vec};

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

const DIRECTORY_ENTRY_SIZE: usize = 32;
const FILE_ATTRIBUTE_DIRECTORY: u16 = 0x0010;
const FILE_ATTRIBUTE_REGULAR: u16 = 0x0020;
const FILE_DIRECTORY_ENTRY_TYPE: u8 = 0x85;
const FILE_NAME_ENTRY_TYPE: u8 = 0xC1;
const ROOT_FILE_ENTRY_INDEX: usize = 4;
const STREAM_EXTENSION_ENTRY_TYPE: u8 = 0xC0;
const TEST_PARENT_CLUSTER: u32 = 6;
const TEST_PARENT_NAME: &str = "CreateParent";

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
    mount_root_with_flags(disk, FsFlags::empty(), options)
}

fn mount_root_with_flags(
    disk: &Arc<ExfatLookupTestDisk>,
    flags: FsFlags,
    options: Option<&str>,
) -> (Arc<dyn FileSystem>, Arc<dyn Inode>) {
    let args = options.map(|options| CString::new(options).unwrap());
    let fs = ExfatFsType.create(flags, args, Some(disk.as_block_device())).unwrap();
    let root_inode = fs.root_inode();
    (fs, root_inode)
}

fn mount_create_parent(
    disk: &Arc<ExfatLookupTestDisk>,
    flags: FsFlags,
    options: Option<&str>,
) -> (Arc<dyn FileSystem>, Arc<dyn Inode>, Arc<dyn Inode>) {
    disk.install_root_directory(ROOT_FILE_ENTRY_INDEX, TEST_PARENT_NAME, TEST_PARENT_CLUSTER);
    let (fs, root_inode) = mount_root_with_flags(disk, flags, options);
    let parent_inode = root_inode.lookup(TEST_PARENT_NAME).unwrap();
    (fs, root_inode, parent_inode)
}

fn read_le_u16(bytes: &[u8]) -> u16 {
    u16::from_le_bytes([bytes[0], bytes[1]])
}

fn read_le_u32(bytes: &[u8]) -> u32 {
    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

fn read_le_u64(bytes: &[u8]) -> u64 {
    u64::from_le_bytes([
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
    ])
}

fn entry_set_checksum(entry_set: &[u8], secondary_count: usize) -> u16 {
    let mut checksum = 0u16;
    let byte_count = (secondary_count + 1) * DIRECTORY_ENTRY_SIZE;
    for (index, byte) in entry_set.iter().take(byte_count).enumerate() {
        if index == 2 || index == 3 {
            continue;
        }
        checksum = ((checksum & 1) << 15) + (checksum >> 1) + u16::from(*byte);
    }
    checksum
}

fn decode_entry_name(entry_set: &[u8]) -> Vec<u16> {
    let name_length = usize::from(entry_set[DIRECTORY_ENTRY_SIZE + 3]);
    let mut name = Vec::with_capacity(name_length);
    for name_entry in entry_set[DIRECTORY_ENTRY_SIZE * 2..].chunks_exact(DIRECTORY_ENTRY_SIZE) {
        if name_entry[0] != FILE_NAME_ENTRY_TYPE {
            break;
        }
        for code_unit_bytes in name_entry[2..].chunks_exact(2) {
            if name.len() == name_length {
                break;
            }
            name.push(read_le_u16(code_unit_bytes));
        }
        if name.len() == name_length {
            break;
        }
    }
    name
}

fn entry_index_from_ino(ino: u64) -> usize {
    usize::try_from(ino & u64::from(u32::MAX)).unwrap()
}

fn assert_parent_directory_unchanged(
    disk: &Arc<ExfatLookupTestDisk>,
    parent_inode: &Arc<dyn Inode>,
    expected_bytes: &[u8],
    expected_names: &[&str],
) {
    assert_eq!(disk.read_cluster(TEST_PARENT_CLUSTER), expected_bytes);
    let (_visited_count, entries) = collect_dirents(parent_inode, 2);
    assert_eq!(entry_names(&entries), expected_names);
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
fn directory_entry_mutation_create_file_publishes_checksum_valid_entry_set() {
    init_lookup_test_runtime();

    let disk = ExfatLookupTestDisk::new();
    let (_fs, _root_inode, parent_inode) =
        mount_create_parent(&disk, FsFlags::empty(), None);

    let created = parent_inode
        .create("CreateFile", InodeType::File, InodeMode::all())
        .unwrap();
    let lookup = parent_inode.lookup("createfile").unwrap();
    let (_visited_count, entries) = collect_dirents(&parent_inode, 2);
    let parent_cluster = disk.read_cluster(TEST_PARENT_CLUSTER);
    let entry_set = parent_cluster[..DIRECTORY_ENTRY_SIZE * 3].to_vec();

    assert_eq!(created.ino(), lookup.ino());
    assert_eq!(created.type_(), InodeType::File);
    assert_eq!(created.size(), 0);
    assert_eq!(entry_names(&entries), vec!["CreateFile"]);
    assert_eq!(entry_index_from_ino(created.ino()), 0);
    assert_eq!(entry_set[0], FILE_DIRECTORY_ENTRY_TYPE);
    assert_eq!(usize::from(entry_set[1]), 2);
    assert_eq!(read_le_u16(&entry_set[2..4]), entry_set_checksum(&entry_set, 2));
    assert_eq!(read_le_u16(&entry_set[4..6]), FILE_ATTRIBUTE_REGULAR);
    assert_eq!(entry_set[DIRECTORY_ENTRY_SIZE], STREAM_EXTENSION_ENTRY_TYPE);
    assert_eq!(entry_set[DIRECTORY_ENTRY_SIZE + 1], 0x01);
    assert_eq!(
        usize::from(entry_set[DIRECTORY_ENTRY_SIZE + 3]),
        "CreateFile".encode_utf16().count()
    );
    assert_eq!(read_le_u32(&entry_set[DIRECTORY_ENTRY_SIZE + 20..]), 0);
    assert_eq!(read_le_u64(&entry_set[DIRECTORY_ENTRY_SIZE + 24..]), 0);
    assert_eq!(decode_entry_name(&entry_set), "CreateFile".encode_utf16().collect::<Vec<_>>());
}

#[ktest]
fn directory_entry_mutation_mkdir_publishes_checksum_valid_entry_set() {
    init_lookup_test_runtime();

    let disk = ExfatLookupTestDisk::new();
    let (_fs, _root_inode, parent_inode) =
        mount_create_parent(&disk, FsFlags::empty(), None);

    let created = parent_inode
        .create("CreateDir", InodeType::Dir, InodeMode::all())
        .unwrap();
    let lookup = parent_inode.lookup("createdir").unwrap();
    let (_visited_count, entries) = collect_dirents(&parent_inode, 2);
    let parent_cluster = disk.read_cluster(TEST_PARENT_CLUSTER);
    let entry_set = parent_cluster[..DIRECTORY_ENTRY_SIZE * 3].to_vec();
    let first_cluster = read_le_u32(&entry_set[DIRECTORY_ENTRY_SIZE + 20..]);

    assert_eq!(created.ino(), lookup.ino());
    assert_eq!(created.type_(), InodeType::Dir);
    assert_eq!(created.size(), disk.root_cluster_size());
    assert_eq!(entry_names(&entries), vec!["CreateDir"]);
    assert_eq!(entry_index_from_ino(created.ino()), 0);
    assert_eq!(entry_set[0], FILE_DIRECTORY_ENTRY_TYPE);
    assert_eq!(usize::from(entry_set[1]), 2);
    assert_eq!(read_le_u16(&entry_set[2..4]), entry_set_checksum(&entry_set, 2));
    assert_eq!(read_le_u16(&entry_set[4..6]), FILE_ATTRIBUTE_DIRECTORY);
    assert_eq!(entry_set[DIRECTORY_ENTRY_SIZE], STREAM_EXTENSION_ENTRY_TYPE);
    assert_eq!(entry_set[DIRECTORY_ENTRY_SIZE + 1], 0x03);
    assert_ne!(first_cluster, 0);
    assert_eq!(
        usize::try_from(read_le_u64(&entry_set[DIRECTORY_ENTRY_SIZE + 24..])).unwrap(),
        disk.root_cluster_size()
    );
    assert_eq!(decode_entry_name(&entry_set), "CreateDir".encode_utf16().collect::<Vec<_>>());
    assert!(disk.read_cluster(first_cluster).iter().all(|byte| *byte == 0));
}

#[ktest]
fn directory_entry_mutation_zero_size_dir_changes_only_newborn_shape() {
    init_lookup_test_runtime();

    let default_disk = ExfatLookupTestDisk::new();
    let (_default_fs, _default_root_inode, default_parent_inode) =
        mount_create_parent(&default_disk, FsFlags::empty(), None);
    let default_child = default_parent_inode
        .create("ShapeDir", InodeType::Dir, InodeMode::all())
        .unwrap();
    let (_default_visited_count, default_entries) = collect_dirents(&default_parent_inode, 2);
    let default_parent_cluster = default_disk.read_cluster(TEST_PARENT_CLUSTER);
    let default_entry_set = default_parent_cluster[..DIRECTORY_ENTRY_SIZE * 3].to_vec();

    let zero_size_disk = ExfatLookupTestDisk::new();
    let (_zero_size_fs, _zero_size_root_inode, zero_size_parent_inode) =
        mount_create_parent(&zero_size_disk, FsFlags::empty(), Some("zero_size_dir"));
    let zero_size_child = zero_size_parent_inode
        .create("ShapeDir", InodeType::Dir, InodeMode::all())
        .unwrap();
    let (_zero_size_visited_count, zero_size_entries) =
        collect_dirents(&zero_size_parent_inode, 2);
    let zero_size_parent_cluster = zero_size_disk.read_cluster(TEST_PARENT_CLUSTER);
    let zero_size_entry_set = zero_size_parent_cluster[..DIRECTORY_ENTRY_SIZE * 3].to_vec();

    assert_eq!(default_child.type_(), InodeType::Dir);
    assert_eq!(zero_size_child.type_(), InodeType::Dir);
    assert_eq!(default_child.size(), default_disk.root_cluster_size());
    assert_eq!(zero_size_child.size(), 0);
    assert_eq!(entry_names(&default_entries), vec!["ShapeDir"]);
    assert_eq!(entry_names(&zero_size_entries), vec!["ShapeDir"]);
    assert_eq!(entry_index_from_ino(default_child.ino()), 0);
    assert_eq!(entry_index_from_ino(zero_size_child.ino()), 0);
    assert_eq!(usize::from(default_entry_set[1]), usize::from(zero_size_entry_set[1]));
    assert_eq!(
        decode_entry_name(&default_entry_set),
        decode_entry_name(&zero_size_entry_set)
    );
    assert_eq!(
        read_le_u16(&default_entry_set[4..6]),
        read_le_u16(&zero_size_entry_set[4..6])
    );
    assert_eq!(default_entry_set[DIRECTORY_ENTRY_SIZE], STREAM_EXTENSION_ENTRY_TYPE);
    assert_eq!(zero_size_entry_set[DIRECTORY_ENTRY_SIZE], STREAM_EXTENSION_ENTRY_TYPE);
    assert_eq!(default_entry_set[DIRECTORY_ENTRY_SIZE + 1], 0x03);
    assert_eq!(zero_size_entry_set[DIRECTORY_ENTRY_SIZE + 1], 0x01);
    assert_ne!(read_le_u32(&default_entry_set[DIRECTORY_ENTRY_SIZE + 20..]), 0);
    assert_eq!(read_le_u32(&zero_size_entry_set[DIRECTORY_ENTRY_SIZE + 20..]), 0);
    assert_eq!(
        usize::try_from(read_le_u64(&default_entry_set[DIRECTORY_ENTRY_SIZE + 24..])).unwrap(),
        default_disk.root_cluster_size()
    );
    assert_eq!(read_le_u64(&zero_size_entry_set[DIRECTORY_ENTRY_SIZE + 24..]), 0);
    assert_eq!(read_le_u16(&default_entry_set[2..4]), entry_set_checksum(&default_entry_set, 2));
    assert_eq!(
        read_le_u16(&zero_size_entry_set[2..4]),
        entry_set_checksum(&zero_size_entry_set, 2)
    );
}

#[ktest]
fn directory_entry_mutation_parent_growth_extends_directory_before_publication() {
    init_lookup_test_runtime();

    let disk = ExfatLookupTestDisk::new();
    let (_fs, _root_inode, parent_inode) =
        mount_create_parent(&disk, FsFlags::empty(), None);
    let fill_count = disk.root_directory_entry_capacity() / 3;
    let mut inserted_names = Vec::with_capacity(fill_count);
    for index in 0..fill_count {
        let name = format!("Fill{:02}", index);
        parent_inode
            .create(&name, InodeType::File, InodeMode::all())
            .unwrap();
        inserted_names.push(name);
    }

    let before_size = parent_inode.size();
    let (_before_visited_count, before_entries) = collect_dirents(&parent_inode, 2);

    let created = parent_inode
        .create("GrowthFile", InodeType::File, InodeMode::all())
        .unwrap();
    let lookup = parent_inode.lookup("growthfile").unwrap();
    let (_after_visited_count, after_entries) = collect_dirents(&parent_inode, 2);

    assert_eq!(entry_names(&before_entries).len(), inserted_names.len());
    assert_eq!(created.ino(), lookup.ino());
    assert_eq!(created.type_(), InodeType::File);
    assert_eq!(parent_inode.size(), before_size + disk.root_cluster_size());
    assert!(entry_index_from_ino(created.ino()) >= disk.root_directory_entry_capacity() - 2);
    assert_eq!(after_entries.len(), inserted_names.len() + 1);
    assert_eq!(after_entries.last().unwrap().name, "GrowthFile");
}

#[ktest]
fn directory_entry_mutation_create_refusals_preserve_parent_visibility() {
    init_lookup_test_runtime();

    let duplicate_disk = ExfatLookupTestDisk::new();
    let (_duplicate_fs, _duplicate_root_inode, duplicate_parent_inode) =
        mount_create_parent(&duplicate_disk, FsFlags::empty(), None);
    duplicate_parent_inode
        .create("Existing", InodeType::File, InodeMode::all())
        .unwrap();
    let duplicate_parent_bytes = duplicate_disk.read_cluster(TEST_PARENT_CLUSTER);
    let (_duplicate_visited_count, duplicate_entries) = collect_dirents(&duplicate_parent_inode, 2);
    let duplicate_error = duplicate_parent_inode
        .create("existing", InodeType::File, InodeMode::all())
        .unwrap_err();
    assert_eq!(duplicate_error.error(), Errno::EEXIST);
    assert_parent_directory_unchanged(
        &duplicate_disk,
        &duplicate_parent_inode,
        &duplicate_parent_bytes,
        &entry_names(&duplicate_entries),
    );

    let invalid_disk = ExfatLookupTestDisk::new();
    let (_invalid_fs, _invalid_root_inode, invalid_parent_inode) =
        mount_create_parent(&invalid_disk, FsFlags::empty(), None);
    let invalid_parent_bytes = invalid_disk.read_cluster(TEST_PARENT_CLUSTER);
    let invalid_error = invalid_parent_inode
        .create("invalid/name", InodeType::File, InodeMode::all())
        .unwrap_err();
    assert_eq!(invalid_error.error(), Errno::EINVAL);
    assert_parent_directory_unchanged(
        &invalid_disk,
        &invalid_parent_inode,
        &invalid_parent_bytes,
        &[],
    );
    let name_too_long_error = invalid_parent_inode
        .create(&"a".repeat(256), InodeType::File, InodeMode::all())
        .unwrap_err();
    assert_eq!(name_too_long_error.error(), Errno::ENAMETOOLONG);
    assert_parent_directory_unchanged(
        &invalid_disk,
        &invalid_parent_inode,
        &invalid_parent_bytes,
        &[],
    );

    let read_only_disk = ExfatLookupTestDisk::new();
    let (_read_only_fs, _read_only_root_inode, read_only_parent_inode) =
        mount_create_parent(&read_only_disk, FsFlags::RDONLY, None);
    let read_only_parent_bytes = read_only_disk.read_cluster(TEST_PARENT_CLUSTER);
    let read_only_error = read_only_parent_inode
        .create("ReadOnly", InodeType::File, InodeMode::all())
        .unwrap_err();
    assert_eq!(read_only_error.error(), Errno::EROFS);
    assert_parent_directory_unchanged(
        &read_only_disk,
        &read_only_parent_inode,
        &read_only_parent_bytes,
        &[],
    );

    let unsupported_disk = ExfatLookupTestDisk::new();
    let (_unsupported_fs, _unsupported_root_inode, unsupported_parent_inode) =
        mount_create_parent(&unsupported_disk, FsFlags::empty(), None);
    let unsupported_parent_bytes = unsupported_disk.read_cluster(TEST_PARENT_CLUSTER);
    let link_error = unsupported_parent_inode
        .link(&unsupported_parent_inode, "HardLink")
        .unwrap_err();
    assert_eq!(link_error.error(), Errno::EOPNOTSUPP);
    let mknod_error = unsupported_parent_inode
        .mknod("Node", InodeMode::all(), MknodType::CharDevice(0))
        .unwrap_err();
    assert_eq!(mknod_error.error(), Errno::EOPNOTSUPP);
    let symlink_error = unsupported_parent_inode
        .create("Link", InodeType::SymLink, InodeMode::all())
        .unwrap_err();
    assert_eq!(symlink_error.error(), Errno::EOPNOTSUPP);
    assert_parent_directory_unchanged(
        &unsupported_disk,
        &unsupported_parent_inode,
        &unsupported_parent_bytes,
        &[],
    );
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
