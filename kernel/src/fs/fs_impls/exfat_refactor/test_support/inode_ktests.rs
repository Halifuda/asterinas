// SPDX-License-Identifier: MPL-2.0

use alloc::{ffi::CString, format, string::String, sync::Arc, vec, vec::Vec};
use core::sync::atomic::{AtomicUsize, Ordering};

use ostd::prelude::ktest;

use super::super::{
    fs::ExfatFsType,
    test_support::inode::{ExfatLookupTestDisk, ExfatLookupToggleFailingWriteDisk},
};
use super::*;
use aster_block::BlockDevice;
use crate::fs::vfs::{
    file_system::{FileSystem, FsFlags},
    registry::FsType,
};
use crate::thread::{Thread, kernel_thread::ThreadOptions};

const DIRECTORY_ENTRY_SIZE: usize = 32;
const FILE_ATTRIBUTE_DIRECTORY: u16 = 0x0010;
const FILE_ATTRIBUTE_REGULAR: u16 = 0x0020;
const FILE_DIRECTORY_ENTRY_TYPE: u8 = 0x85;
const FILE_NAME_ENTRY_TYPE: u8 = 0xC1;
const ROOT_FILE_ENTRY_INDEX: usize = 4;
const ROOT_SECOND_FILE_ENTRY_INDEX: usize = ROOT_FILE_ENTRY_INDEX + 3;
const ROOT_THIRD_FILE_ENTRY_INDEX: usize = ROOT_FILE_ENTRY_INDEX + 6;
const STREAM_EXTENSION_ENTRY_TYPE: u8 = 0xC0;
const TEST_PARENT_CLUSTER: u32 = 6;
const TEST_CHILD_DIRECTORY_CLUSTER: u32 = 8;
const TEST_CHILD_FILE_CLUSTER: u32 = 9;
const TEST_REGULAR_FILE_CLUSTER: u32 = 7;
const TEST_PARENT_NAME: &str = "CreateParent";
const RENAME_SOURCE_PARENT_CLUSTER: u32 = 10;
const RENAME_TARGET_PARENT_CLUSTER: u32 = 11;
const RENAME_SOURCE_FILE_CLUSTER: u32 = 12;
const RENAME_TARGET_FILE_CLUSTER: u32 = 13;
const RENAME_SOURCE_DIRECTORY_CLUSTER: u32 = 14;
const RENAME_TARGET_DIRECTORY_CLUSTER: u32 = 15;
const RENAME_TARGET_DIRECTORY_CHILD_CLUSTER: u32 = 16;
const RENAME_SOURCE_PARENT_NAME: &str = "SrcParent";
const RENAME_TARGET_PARENT_NAME: &str = "DstParent";
const READ_AT_TEST_VOLUME_FLAGS: u16 = 0x000E;

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
    let block_device: Arc<dyn BlockDevice> = disk.as_block_device();
    mount_root_from_block_device(block_device, flags, options)
}

fn mount_root_from_block_device(
    block_device: Arc<dyn BlockDevice>,
    flags: FsFlags,
    options: Option<&str>,
) -> (Arc<dyn FileSystem>, Arc<dyn Inode>) {
    let args = options.map(|options| CString::new(options).unwrap());
    let fs = ExfatFsType.create(flags, args, Some(block_device)).unwrap();
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
            name.push(u16::from_le_bytes([code_unit_bytes[0], code_unit_bytes[1]]));
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

fn assert_directory_unchanged(
    disk: &Arc<ExfatLookupTestDisk>,
    directory_inode: &Arc<dyn Inode>,
    directory_cluster: u32,
    expected_bytes: &[u8],
    expected_names: &[&str],
) {
    assert_eq!(disk.read_cluster(directory_cluster), expected_bytes);
    let (_visited_count, entries) = collect_dirents(directory_inode, 2);
    assert_eq!(entry_names(&entries), expected_names);
}

fn assert_entry_set_invalidated(entry_set: &[u8]) {
    assert_eq!(entry_set[0], FILE_DIRECTORY_ENTRY_TYPE & !0x80);
    assert_eq!(entry_set[DIRECTORY_ENTRY_SIZE], STREAM_EXTENSION_ENTRY_TYPE & !0x80);
    assert_eq!(entry_set[DIRECTORY_ENTRY_SIZE * 2], FILE_NAME_ENTRY_TYPE & !0x80);
}

fn mount_rename_parent_pair(
    disk: &Arc<ExfatLookupTestDisk>,
) -> (
    Arc<dyn FileSystem>,
    Arc<dyn Inode>,
    Arc<dyn Inode>,
    Arc<dyn Inode>,
) {
    mount_rename_parent_pair_with_flags(disk, FsFlags::empty(), None)
}

fn mount_rename_parent_pair_with_flags(
    disk: &Arc<ExfatLookupTestDisk>,
    flags: FsFlags,
    options: Option<&str>,
) -> (
    Arc<dyn FileSystem>,
    Arc<dyn Inode>,
    Arc<dyn Inode>,
    Arc<dyn Inode>,
) {
    disk.install_root_directory(
        ROOT_FILE_ENTRY_INDEX,
        RENAME_SOURCE_PARENT_NAME,
        RENAME_SOURCE_PARENT_CLUSTER,
    );
    disk.install_root_directory(
        ROOT_SECOND_FILE_ENTRY_INDEX,
        RENAME_TARGET_PARENT_NAME,
        RENAME_TARGET_PARENT_CLUSTER,
    );
    let (fs, root_inode) = mount_root_with_flags(disk, flags, options);
    let source_parent = root_inode.lookup(RENAME_SOURCE_PARENT_NAME).unwrap();
    let target_parent = root_inode.lookup(RENAME_TARGET_PARENT_NAME).unwrap();
    (fs, root_inode, source_parent, target_parent)
}

fn visible_name_count(entries: &[CapturedDirent], expected_name: &str) -> usize {
    entries
        .iter()
        .filter(|entry| entry.name == expected_name)
        .count()
}

fn wait_for_concurrent_start(ready_count: &AtomicUsize, participant_count: usize) {
    ready_count.fetch_add(1, Ordering::Relaxed);
    while ready_count.load(Ordering::Relaxed) < participant_count {
        Thread::yield_now();
    }
}

#[path = "lookup_resolution.rs"]
mod lookup_resolution;

#[path = "readdir_visibility.rs"]
mod readdir_visibility;

#[path = "directory_lookup_and_identity_integration.rs"]
mod directory_lookup_and_identity_integration;

#[path = "directory_entry_field_update_substrate.rs"]
mod directory_entry_field_update_substrate;

#[ktest]
fn directory_entry_mutation_create_file_publishes_checksum_valid_entry_set() {
    init_lookup_test_runtime();

    let disk = ExfatLookupTestDisk::new();
    let (_fs, _root_inode, parent_inode) = mount_create_parent(&disk, FsFlags::empty(), None);

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
    assert_eq!(
        u16::from_le_bytes([entry_set[2], entry_set[3]]),
        entry_set_checksum(&entry_set, 2)
    );
    assert_eq!(u16::from_le_bytes([entry_set[4], entry_set[5]]), FILE_ATTRIBUTE_REGULAR);
    assert_eq!(entry_set[DIRECTORY_ENTRY_SIZE], STREAM_EXTENSION_ENTRY_TYPE);
    assert_eq!(entry_set[DIRECTORY_ENTRY_SIZE + 1], 0x01);
    assert_eq!(
        usize::from(entry_set[DIRECTORY_ENTRY_SIZE + 3]),
        "CreateFile".encode_utf16().count()
    );
    assert_eq!(
        u32::from_le_bytes([
            entry_set[DIRECTORY_ENTRY_SIZE + 20],
            entry_set[DIRECTORY_ENTRY_SIZE + 21],
            entry_set[DIRECTORY_ENTRY_SIZE + 22],
            entry_set[DIRECTORY_ENTRY_SIZE + 23],
        ]),
        0
    );
    assert_eq!(
        u64::from_le_bytes([
            entry_set[DIRECTORY_ENTRY_SIZE + 24],
            entry_set[DIRECTORY_ENTRY_SIZE + 25],
            entry_set[DIRECTORY_ENTRY_SIZE + 26],
            entry_set[DIRECTORY_ENTRY_SIZE + 27],
            entry_set[DIRECTORY_ENTRY_SIZE + 28],
            entry_set[DIRECTORY_ENTRY_SIZE + 29],
            entry_set[DIRECTORY_ENTRY_SIZE + 30],
            entry_set[DIRECTORY_ENTRY_SIZE + 31],
        ]),
        0
    );
    assert_eq!(decode_entry_name(&entry_set), "CreateFile".encode_utf16().collect::<Vec<_>>());
}

#[ktest]
fn directory_entry_mutation_mkdir_publishes_checksum_valid_entry_set() {
    init_lookup_test_runtime();

    let disk = ExfatLookupTestDisk::new();
    let (_fs, _root_inode, parent_inode) = mount_create_parent(&disk, FsFlags::empty(), None);

    let created = parent_inode
        .create("CreateDir", InodeType::Dir, InodeMode::all())
        .unwrap();
    let lookup = parent_inode.lookup("createdir").unwrap();
    let (_visited_count, entries) = collect_dirents(&parent_inode, 2);
    let parent_cluster = disk.read_cluster(TEST_PARENT_CLUSTER);
    let entry_set = parent_cluster[..DIRECTORY_ENTRY_SIZE * 3].to_vec();
    let first_cluster = u32::from_le_bytes([
        entry_set[DIRECTORY_ENTRY_SIZE + 20],
        entry_set[DIRECTORY_ENTRY_SIZE + 21],
        entry_set[DIRECTORY_ENTRY_SIZE + 22],
        entry_set[DIRECTORY_ENTRY_SIZE + 23],
    ]);

    assert_eq!(created.ino(), lookup.ino());
    assert_eq!(created.type_(), InodeType::Dir);
    assert_eq!(created.size(), disk.root_cluster_size());
    assert_eq!(entry_names(&entries), vec!["CreateDir"]);
    assert_eq!(entry_index_from_ino(created.ino()), 0);
    assert_eq!(entry_set[0], FILE_DIRECTORY_ENTRY_TYPE);
    assert_eq!(usize::from(entry_set[1]), 2);
    assert_eq!(
        u16::from_le_bytes([entry_set[2], entry_set[3]]),
        entry_set_checksum(&entry_set, 2)
    );
    assert_eq!(
        u16::from_le_bytes([entry_set[4], entry_set[5]]),
        FILE_ATTRIBUTE_DIRECTORY
    );
    assert_eq!(entry_set[DIRECTORY_ENTRY_SIZE], STREAM_EXTENSION_ENTRY_TYPE);
    assert_eq!(entry_set[DIRECTORY_ENTRY_SIZE + 1], 0x03);
    assert_ne!(first_cluster, 0);
    assert_eq!(
        usize::try_from(u64::from_le_bytes([
            entry_set[DIRECTORY_ENTRY_SIZE + 24],
            entry_set[DIRECTORY_ENTRY_SIZE + 25],
            entry_set[DIRECTORY_ENTRY_SIZE + 26],
            entry_set[DIRECTORY_ENTRY_SIZE + 27],
            entry_set[DIRECTORY_ENTRY_SIZE + 28],
            entry_set[DIRECTORY_ENTRY_SIZE + 29],
            entry_set[DIRECTORY_ENTRY_SIZE + 30],
            entry_set[DIRECTORY_ENTRY_SIZE + 31],
        ]))
        .unwrap(),
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
    let (_zero_size_visited_count, zero_size_entries) = collect_dirents(&zero_size_parent_inode, 2);
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
        u16::from_le_bytes([default_entry_set[4], default_entry_set[5]]),
        u16::from_le_bytes([zero_size_entry_set[4], zero_size_entry_set[5]])
    );
    assert_eq!(default_entry_set[DIRECTORY_ENTRY_SIZE], STREAM_EXTENSION_ENTRY_TYPE);
    assert_eq!(zero_size_entry_set[DIRECTORY_ENTRY_SIZE], STREAM_EXTENSION_ENTRY_TYPE);
    assert_eq!(default_entry_set[DIRECTORY_ENTRY_SIZE + 1], 0x03);
    assert_eq!(zero_size_entry_set[DIRECTORY_ENTRY_SIZE + 1], 0x01);
    assert_ne!(
        u32::from_le_bytes([
            default_entry_set[DIRECTORY_ENTRY_SIZE + 20],
            default_entry_set[DIRECTORY_ENTRY_SIZE + 21],
            default_entry_set[DIRECTORY_ENTRY_SIZE + 22],
            default_entry_set[DIRECTORY_ENTRY_SIZE + 23],
        ]),
        0
    );
    assert_eq!(
        u32::from_le_bytes([
            zero_size_entry_set[DIRECTORY_ENTRY_SIZE + 20],
            zero_size_entry_set[DIRECTORY_ENTRY_SIZE + 21],
            zero_size_entry_set[DIRECTORY_ENTRY_SIZE + 22],
            zero_size_entry_set[DIRECTORY_ENTRY_SIZE + 23],
        ]),
        0
    );
    assert_eq!(
        usize::try_from(u64::from_le_bytes([
            default_entry_set[DIRECTORY_ENTRY_SIZE + 24],
            default_entry_set[DIRECTORY_ENTRY_SIZE + 25],
            default_entry_set[DIRECTORY_ENTRY_SIZE + 26],
            default_entry_set[DIRECTORY_ENTRY_SIZE + 27],
            default_entry_set[DIRECTORY_ENTRY_SIZE + 28],
            default_entry_set[DIRECTORY_ENTRY_SIZE + 29],
            default_entry_set[DIRECTORY_ENTRY_SIZE + 30],
            default_entry_set[DIRECTORY_ENTRY_SIZE + 31],
        ]))
        .unwrap(),
        default_disk.root_cluster_size()
    );
    assert_eq!(
        u64::from_le_bytes([
            zero_size_entry_set[DIRECTORY_ENTRY_SIZE + 24],
            zero_size_entry_set[DIRECTORY_ENTRY_SIZE + 25],
            zero_size_entry_set[DIRECTORY_ENTRY_SIZE + 26],
            zero_size_entry_set[DIRECTORY_ENTRY_SIZE + 27],
            zero_size_entry_set[DIRECTORY_ENTRY_SIZE + 28],
            zero_size_entry_set[DIRECTORY_ENTRY_SIZE + 29],
            zero_size_entry_set[DIRECTORY_ENTRY_SIZE + 30],
            zero_size_entry_set[DIRECTORY_ENTRY_SIZE + 31],
        ]),
        0
    );
    assert_eq!(
        u16::from_le_bytes([default_entry_set[2], default_entry_set[3]]),
        entry_set_checksum(&default_entry_set, 2)
    );
    assert_eq!(
        u16::from_le_bytes([zero_size_entry_set[2], zero_size_entry_set[3]]),
        entry_set_checksum(&zero_size_entry_set, 2)
    );
}

#[ktest]
fn directory_entry_mutation_parent_growth_extends_directory_before_publication() {
    init_lookup_test_runtime();

    let disk = ExfatLookupTestDisk::new();
    let (_fs, _root_inode, parent_inode) = mount_create_parent(&disk, FsFlags::empty(), None);
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
fn directory_entry_mutation_unlink_invalidates_visibility_before_reclamation() {
    init_lookup_test_runtime();

    let disk = ExfatLookupTestDisk::new();
    disk.install_root_directory(ROOT_FILE_ENTRY_INDEX, TEST_PARENT_NAME, TEST_PARENT_CLUSTER);
    disk.install_directory_file(
        TEST_PARENT_CLUSTER,
        0,
        "Victim",
        TEST_REGULAR_FILE_CLUSTER,
        disk.root_cluster_size(),
    );
    let (_fs, root_inode) = mount_root(&disk, None);
    let parent_inode = root_inode.lookup(TEST_PARENT_NAME).unwrap();

    assert!(disk.is_cluster_allocated(TEST_REGULAR_FILE_CLUSTER));
    assert_eq!(parent_inode.lookup("victim").unwrap().type_(), InodeType::File);

    parent_inode.unlink("Victim").unwrap();

    let removed_entry_set = disk.read_directory_entries(TEST_PARENT_CLUSTER, 0, 3);
    let (_visited_count, remaining_entries) = collect_dirents(&parent_inode, 2);

    assert_eq!(lookup_error(&parent_inode, "Victim"), Errno::ENOENT);
    assert!(remaining_entries.is_empty());
    assert_entry_set_invalidated(&removed_entry_set);
    assert!(!disk.is_cluster_allocated(TEST_REGULAR_FILE_CLUSTER));
}

#[ktest]
fn directory_entry_mutation_rmdir_requires_live_empty_directory() {
    init_lookup_test_runtime();

    let disk = ExfatLookupTestDisk::new();
    disk.install_root_directory(ROOT_FILE_ENTRY_INDEX, TEST_PARENT_NAME, TEST_PARENT_CLUSTER);
    disk.install_directory_subdirectory(
        TEST_PARENT_CLUSTER,
        0,
        "VictimDir",
        TEST_CHILD_DIRECTORY_CLUSTER,
    );
    disk.install_directory_file(
        TEST_CHILD_DIRECTORY_CLUSTER,
        0,
        "LateLeaf",
        TEST_CHILD_FILE_CLUSTER,
        disk.root_cluster_size(),
    );
    let (_fs, root_inode) = mount_root(&disk, None);
    let parent_inode = root_inode.lookup(TEST_PARENT_NAME).unwrap();

    let error = parent_inode.rmdir("VictimDir").unwrap_err();

    assert_eq!(error.error(), Errno::ENOTEMPTY);
}

#[ktest]
fn directory_entry_mutation_delete_refusals_preserve_typed_boundaries() {
    init_lookup_test_runtime();

    let fractured_disk = ExfatLookupTestDisk::new();
    fractured_disk.install_root_fractured_entry_set(ROOT_FILE_ENTRY_INDEX, "Broken");
    let (_fs, fractured_root) = mount_root(&fractured_disk, None);
    let fractured_bytes_before = fractured_disk.read_root_entries(ROOT_FILE_ENTRY_INDEX, 3);

    let fractured_error = fractured_root.unlink("Broken").unwrap_err();

    assert_eq!(fractured_error.error(), Errno::EUCLEAN);
    assert_eq!(
        fractured_disk.read_root_entries(ROOT_FILE_ENTRY_INDEX, 3),
        fractured_bytes_before
    );
    assert_eq!(lookup_error(&fractured_root, "Broken"), Errno::EUCLEAN);

    let critical_disk = ExfatLookupTestDisk::new();
    critical_disk.install_root_unrecognized_critical_entry(ROOT_FILE_ENTRY_INDEX);
    let (_fs, critical_root) = mount_root(&critical_disk, None);
    let critical_bytes_before = critical_disk.read_root_entries(ROOT_FILE_ENTRY_INDEX, 2);

    let critical_error = critical_root.rmdir("Broken").unwrap_err();

    assert_eq!(critical_error.error(), Errno::EUCLEAN);
    assert_eq!(
        critical_disk.read_root_entries(ROOT_FILE_ENTRY_INDEX, 2),
        critical_bytes_before
    );
    assert_eq!(lookup_error(&critical_root, "Broken"), Errno::EUCLEAN);
}

#[ktest]
fn directory_entry_mutation_delete_failure_maintenance_preserves_namespace_first_ordering() {
    init_lookup_test_runtime();

    let disk = ExfatLookupTestDisk::new();
    disk.install_root_directory(ROOT_FILE_ENTRY_INDEX, TEST_PARENT_NAME, TEST_PARENT_CLUSTER);
    disk.install_directory_file(
        TEST_PARENT_CLUSTER,
        0,
        "Victim",
        TEST_REGULAR_FILE_CLUSTER,
        disk.root_cluster_size(),
    );
    let allocation_bitmap_offset =
        disk.allocation_bitmap_byte_offset_for_cluster(TEST_REGULAR_FILE_CLUSTER);
    let failing_disk =
        ExfatLookupToggleFailingWriteDisk::new(disk.clone(), allocation_bitmap_offset, 1);
    let block_device: Arc<dyn BlockDevice> = failing_disk.clone();
    let (_fs, root_inode) = mount_root_from_block_device(block_device, FsFlags::empty(), None);
    let parent_inode = root_inode.lookup(TEST_PARENT_NAME).unwrap();

    assert!(disk.is_cluster_allocated(TEST_REGULAR_FILE_CLUSTER));

    failing_disk.enable_failures();
    parent_inode.unlink("Victim").unwrap();

    let removed_entry_set = disk.read_directory_entries(TEST_PARENT_CLUSTER, 0, 3);

    assert_eq!(lookup_error(&parent_inode, "Victim"), Errno::ENOENT);
    assert_entry_set_invalidated(&removed_entry_set);
    assert!(disk.is_cluster_allocated(TEST_REGULAR_FILE_CLUSTER));
}

#[ktest]
fn directory_entry_mutation_rename_within_directory_rewrites_visibility_without_duplicate_namespace() {
    init_lookup_test_runtime();

    let disk = ExfatLookupTestDisk::new();
    disk.install_root_directory(
        ROOT_FILE_ENTRY_INDEX,
        RENAME_SOURCE_PARENT_NAME,
        RENAME_SOURCE_PARENT_CLUSTER,
    );
    disk.install_directory_file(
        RENAME_SOURCE_PARENT_CLUSTER,
        0,
        "ShortName",
        RENAME_SOURCE_FILE_CLUSTER,
        disk.root_cluster_size(),
    );
    disk.install_directory_file(
        RENAME_SOURCE_PARENT_CLUSTER,
        3,
        "Neighbor",
        RENAME_TARGET_FILE_CLUSTER,
        disk.root_cluster_size(),
    );
    let (_fs, root_inode) = mount_root(&disk, None);
    let source_parent = root_inode.lookup(RENAME_SOURCE_PARENT_NAME).unwrap();

    source_parent
        .rename("ShortName", &source_parent, "VeryLongRenameName")
        .unwrap();

    let renamed = source_parent.lookup("VeryLongRenameName").unwrap();
    let (_visited_count, entries) = collect_dirents(&source_parent, 2);
    let invalidated_source_entry_set = disk.read_directory_entries(RENAME_SOURCE_PARENT_CLUSTER, 0, 3);
    let renamed_entry_set = disk.read_directory_entries(
        RENAME_SOURCE_PARENT_CLUSTER,
        entry_index_from_ino(renamed.ino()),
        4,
    );

    assert_eq!(lookup_error(&source_parent, "ShortName"), Errno::ENOENT);
    assert_eq!(visible_name_count(&entries, "VeryLongRenameName"), 1);
    assert_eq!(visible_name_count(&entries, "ShortName"), 0);
    assert_eq!(visible_name_count(&entries, "Neighbor"), 1);
    assert_eq!(entries.len(), 2);
    assert_entry_set_invalidated(&invalidated_source_entry_set);
    assert_eq!(renamed_entry_set[0], FILE_DIRECTORY_ENTRY_TYPE);
    assert_eq!(usize::from(renamed_entry_set[1]), 3);
    assert_eq!(
        u16::from_le_bytes([renamed_entry_set[2], renamed_entry_set[3]]),
        entry_set_checksum(&renamed_entry_set, 3)
    );
    assert_eq!(
        decode_entry_name(&renamed_entry_set),
        "VeryLongRenameName".encode_utf16().collect::<Vec<_>>()
    );
}

#[ktest]
fn directory_entry_mutation_rename_across_directories_publishes_destination_before_source_invalidation() {
    init_lookup_test_runtime();

    let disk = ExfatLookupTestDisk::new();
    disk.install_root_directory(
        ROOT_FILE_ENTRY_INDEX,
        RENAME_TARGET_PARENT_NAME,
        RENAME_TARGET_PARENT_CLUSTER,
    );
    disk.install_root_file(ROOT_SECOND_FILE_ENTRY_INDEX, "MoveMe");
    let failing_disk = ExfatLookupToggleFailingWriteDisk::new(
        disk.clone(),
        disk.root_directory_offset(),
        disk.root_cluster_size(),
    );
    let block_device: Arc<dyn BlockDevice> = failing_disk.clone();
    let (_fs, root_inode) = mount_root_from_block_device(block_device, FsFlags::empty(), None);
    let target_parent = root_inode.lookup(RENAME_TARGET_PARENT_NAME).unwrap();

    failing_disk.enable_failures();
    let error = root_inode
        .rename("MoveMe", &target_parent, "Moved")
        .unwrap_err();

    let (_root_visited_count, root_entries) = collect_dirents(&root_inode, 2);
    let (_target_visited_count, target_entries) = collect_dirents(&target_parent, 2);
    let target_entry_set = disk.read_directory_entries(RENAME_TARGET_PARENT_CLUSTER, 0, 3);

    assert_eq!(error.error(), Errno::EIO);
    assert!(root_inode.lookup("MoveMe").is_ok());
    assert!(target_parent.lookup("Moved").is_ok());
    assert_eq!(visible_name_count(&root_entries, "MoveMe"), 1);
    assert_eq!(visible_name_count(&target_entries, "Moved"), 1);
    assert_eq!(target_entry_set[0], FILE_DIRECTORY_ENTRY_TYPE);
    assert_eq!(
        decode_entry_name(&target_entry_set),
        "Moved".encode_utf16().collect::<Vec<_>>()
    );
}

#[ktest]
fn directory_entry_mutation_rename_directory_target_requires_live_empty_directory() {
    init_lookup_test_runtime();

    let disk = ExfatLookupTestDisk::new();
    let (_fs, _root_inode, source_parent, target_parent) = mount_rename_parent_pair(&disk);
    disk.install_directory_subdirectory(
        RENAME_SOURCE_PARENT_CLUSTER,
        0,
        "MoveDir",
        RENAME_SOURCE_DIRECTORY_CLUSTER,
    );
    disk.install_directory_subdirectory(
        RENAME_TARGET_PARENT_CLUSTER,
        0,
        "Occupied",
        RENAME_TARGET_DIRECTORY_CLUSTER,
    );
    disk.install_directory_file(
        RENAME_TARGET_DIRECTORY_CLUSTER,
        0,
        "Leaf",
        RENAME_TARGET_DIRECTORY_CHILD_CLUSTER,
        disk.root_cluster_size(),
    );

    let source_bytes_before = disk.read_cluster(RENAME_SOURCE_PARENT_CLUSTER);
    let target_bytes_before = disk.read_cluster(RENAME_TARGET_PARENT_CLUSTER);

    let error = source_parent
        .rename("MoveDir", &target_parent, "Occupied")
        .unwrap_err();

    assert_eq!(error.error(), Errno::ENOTEMPTY);
    assert!(source_parent.lookup("MoveDir").is_ok());
    assert!(target_parent.lookup("Occupied").is_ok());
    assert_directory_unchanged(
        &disk,
        &source_parent,
        RENAME_SOURCE_PARENT_CLUSTER,
        &source_bytes_before,
        &["MoveDir"],
    );
    assert_directory_unchanged(
        &disk,
        &target_parent,
        RENAME_TARGET_PARENT_CLUSTER,
        &target_bytes_before,
        &["Occupied"],
    );
}

#[ktest]
fn directory_entry_mutation_rename_refusals_preserve_typed_boundaries() {
    init_lookup_test_runtime();

    let fractured_source_disk = ExfatLookupTestDisk::new();
    fractured_source_disk.install_root_fractured_entry_set(ROOT_FILE_ENTRY_INDEX, "Broken");
    let (_fractured_source_fs, fractured_source_root) = mount_root(&fractured_source_disk, None);
    let fractured_source_bytes_before = fractured_source_disk.read_root_entries(ROOT_FILE_ENTRY_INDEX, 3);

    let fractured_source_error = fractured_source_root
        .rename("Broken", &fractured_source_root, "Renamed")
        .unwrap_err();

    assert_eq!(fractured_source_error.error(), Errno::EUCLEAN);
    assert_eq!(
        fractured_source_disk.read_root_entries(ROOT_FILE_ENTRY_INDEX, 3),
        fractured_source_bytes_before
    );
    assert_eq!(lookup_error(&fractured_source_root, "Broken"), Errno::EUCLEAN);

    let fractured_target_disk = ExfatLookupTestDisk::new();
    fractured_target_disk.install_root_file(ROOT_FILE_ENTRY_INDEX, "Source");
    fractured_target_disk.install_root_fractured_entry_set(ROOT_SECOND_FILE_ENTRY_INDEX, "Target");
    let (_fractured_target_fs, fractured_target_root) = mount_root(&fractured_target_disk, None);
    let fractured_target_bytes_before = fractured_target_disk.read_root_entries(ROOT_FILE_ENTRY_INDEX, 6);

    let fractured_target_error = fractured_target_root
        .rename("Source", &fractured_target_root, "Target")
        .unwrap_err();

    assert_eq!(fractured_target_error.error(), Errno::EUCLEAN);
    assert_eq!(
        fractured_target_disk.read_root_entries(ROOT_FILE_ENTRY_INDEX, 6),
        fractured_target_bytes_before
    );
    assert!(fractured_target_root.lookup("Source").is_ok());
    assert_eq!(lookup_error(&fractured_target_root, "Target"), Errno::EUCLEAN);
}

#[ktest]
fn directory_entry_mutation_rename_failure_maintenance_preserves_destination_first_ordering() {
    init_lookup_test_runtime();

    let disk = ExfatLookupTestDisk::new();
    disk.install_root_directory(
        ROOT_FILE_ENTRY_INDEX,
        RENAME_TARGET_PARENT_NAME,
        RENAME_TARGET_PARENT_CLUSTER,
    );
    disk.install_root_file(ROOT_SECOND_FILE_ENTRY_INDEX, "MoveMe");
    disk.install_directory_file(
        RENAME_TARGET_PARENT_CLUSTER,
        0,
        "ReplaceMe",
        RENAME_TARGET_FILE_CLUSTER,
        disk.root_cluster_size(),
    );
    let failing_disk = ExfatLookupToggleFailingWriteDisk::new(
        disk.clone(),
        disk.allocation_bitmap_byte_offset_for_cluster(RENAME_TARGET_FILE_CLUSTER),
        1,
    );
    let block_device: Arc<dyn BlockDevice> = failing_disk.clone();
    let (_fs, root_inode) = mount_root_from_block_device(block_device, FsFlags::empty(), None);
    let target_parent = root_inode.lookup(RENAME_TARGET_PARENT_NAME).unwrap();

    failing_disk.enable_failures();
    root_inode
        .rename("MoveMe", &target_parent, "ReplaceMe")
        .unwrap();

    let (_root_visited_count, root_entries) = collect_dirents(&root_inode, 2);
    let (_target_visited_count, target_entries) = collect_dirents(&target_parent, 2);

    assert_eq!(lookup_error(&root_inode, "MoveMe"), Errno::ENOENT);
    assert!(target_parent.lookup("ReplaceMe").is_ok());
    assert_eq!(visible_name_count(&root_entries, "MoveMe"), 0);
    assert_eq!(visible_name_count(&target_entries, "ReplaceMe"), 1);
    assert!(disk.is_cluster_allocated(RENAME_TARGET_FILE_CLUSTER));
}

#[ktest]
fn directory_entry_mutation_integration_success_path_create_rename_unlink_rmdir() {
    init_lookup_test_runtime();

    let disk = ExfatLookupTestDisk::new();
    let (_fs, _root_inode, source_parent, target_parent) =
        mount_rename_parent_pair_with_flags(&disk, FsFlags::empty(), Some("zero_size_dir"));

    let created_file = source_parent
        .create("MoveMe", InodeType::File, InodeMode::all())
        .unwrap();
    let created_directory = source_parent
        .create("EmptyDir", InodeType::Dir, InodeMode::all())
        .unwrap();
    let (_source_before_visited_count, source_before_entries) = collect_dirents(&source_parent, 2);
    let empty_dir_entry_index = entry_index_from_ino(created_directory.ino());
    let empty_dir_entry_set =
        disk.read_directory_entries(RENAME_SOURCE_PARENT_CLUSTER, empty_dir_entry_index, 3);

    assert_eq!(created_file.type_(), InodeType::File);
    assert_eq!(created_directory.type_(), InodeType::Dir);
    assert_eq!(created_directory.size(), 0);
    assert_eq!(entry_names(&source_before_entries), vec!["MoveMe", "EmptyDir"]);
    assert_eq!(
        u32::from_le_bytes([
            empty_dir_entry_set[DIRECTORY_ENTRY_SIZE + 20],
            empty_dir_entry_set[DIRECTORY_ENTRY_SIZE + 21],
            empty_dir_entry_set[DIRECTORY_ENTRY_SIZE + 22],
            empty_dir_entry_set[DIRECTORY_ENTRY_SIZE + 23],
        ]),
        0
    );
    assert_eq!(
        u64::from_le_bytes([
            empty_dir_entry_set[DIRECTORY_ENTRY_SIZE + 24],
            empty_dir_entry_set[DIRECTORY_ENTRY_SIZE + 25],
            empty_dir_entry_set[DIRECTORY_ENTRY_SIZE + 26],
            empty_dir_entry_set[DIRECTORY_ENTRY_SIZE + 27],
            empty_dir_entry_set[DIRECTORY_ENTRY_SIZE + 28],
            empty_dir_entry_set[DIRECTORY_ENTRY_SIZE + 29],
            empty_dir_entry_set[DIRECTORY_ENTRY_SIZE + 30],
            empty_dir_entry_set[DIRECTORY_ENTRY_SIZE + 31],
        ]),
        0
    );

    source_parent
        .rename("MoveMe", &target_parent, "MovedFile")
        .unwrap();

    let moved_lookup = target_parent.lookup("MovedFile").unwrap();
    let (_source_after_rename_visited_count, source_after_rename_entries) =
        collect_dirents(&source_parent, 2);
    let (_target_after_rename_visited_count, target_after_rename_entries) =
        collect_dirents(&target_parent, 2);

    assert_eq!(lookup_error(&source_parent, "MoveMe"), Errno::ENOENT);
    assert_eq!(moved_lookup.type_(), InodeType::File);
    assert_eq!(entry_names(&source_after_rename_entries), vec!["EmptyDir"]);
    assert_eq!(entry_names(&target_after_rename_entries), vec!["MovedFile"]);

    target_parent.unlink("MovedFile").unwrap();
    source_parent.rmdir("EmptyDir").unwrap();

    let (_source_final_visited_count, source_final_entries) = collect_dirents(&source_parent, 2);
    let (_target_final_visited_count, target_final_entries) = collect_dirents(&target_parent, 2);

    assert_eq!(lookup_error(&source_parent, "EmptyDir"), Errno::ENOENT);
    assert_eq!(lookup_error(&target_parent, "MovedFile"), Errno::ENOENT);
    assert!(source_final_entries.is_empty());
    assert!(target_final_entries.is_empty());
}

#[ktest]
fn directory_entry_mutation_integration_failure_maintenance_preserves_namespace_and_typed_boundaries()
{
    init_lookup_test_runtime();

    let rename_failure_disk = ExfatLookupTestDisk::new();
    rename_failure_disk.install_root_directory(
        ROOT_FILE_ENTRY_INDEX,
        RENAME_TARGET_PARENT_NAME,
        RENAME_TARGET_PARENT_CLUSTER,
    );
    rename_failure_disk.install_root_file(ROOT_SECOND_FILE_ENTRY_INDEX, "MoveMe");
    let rename_failure_wrapper = ExfatLookupToggleFailingWriteDisk::new(
        rename_failure_disk.clone(),
        rename_failure_disk.root_directory_offset(),
        rename_failure_disk.root_cluster_size(),
    );
    let rename_failure_device: Arc<dyn BlockDevice> = rename_failure_wrapper.clone();
    let (_rename_failure_fs, rename_failure_root) =
        mount_root_from_block_device(rename_failure_device, FsFlags::empty(), None);
    let rename_failure_target = rename_failure_root.lookup(RENAME_TARGET_PARENT_NAME).unwrap();

    rename_failure_wrapper.enable_failures();
    let rename_failure_error = rename_failure_root
        .rename("MoveMe", &rename_failure_target, "Moved")
        .unwrap_err();
    let (_rename_failure_root_visited_count, rename_failure_root_entries) =
        collect_dirents(&rename_failure_root, 2);
    let (_rename_failure_target_visited_count, rename_failure_target_entries) =
        collect_dirents(&rename_failure_target, 2);

    assert_eq!(rename_failure_error.error(), Errno::EIO);
    assert!(rename_failure_root.lookup("MoveMe").is_ok());
    assert!(rename_failure_target.lookup("Moved").is_ok());
    assert_eq!(visible_name_count(&rename_failure_root_entries, "MoveMe"), 1);
    assert_eq!(visible_name_count(&rename_failure_target_entries, "Moved"), 1);

    let unlink_failure_disk = ExfatLookupTestDisk::new();
    unlink_failure_disk.install_root_directory(
        ROOT_FILE_ENTRY_INDEX,
        TEST_PARENT_NAME,
        TEST_PARENT_CLUSTER,
    );
    unlink_failure_disk.install_directory_file(
        TEST_PARENT_CLUSTER,
        0,
        "Victim",
        TEST_REGULAR_FILE_CLUSTER,
        unlink_failure_disk.root_cluster_size(),
    );
    let unlink_failure_wrapper = ExfatLookupToggleFailingWriteDisk::new(
        unlink_failure_disk.clone(),
        unlink_failure_disk.allocation_bitmap_byte_offset_for_cluster(TEST_REGULAR_FILE_CLUSTER),
        1,
    );
    let unlink_failure_device: Arc<dyn BlockDevice> = unlink_failure_wrapper.clone();
    let (_unlink_failure_fs, unlink_failure_root) =
        mount_root_from_block_device(unlink_failure_device, FsFlags::empty(), None);
    let unlink_failure_parent = unlink_failure_root.lookup(TEST_PARENT_NAME).unwrap();

    unlink_failure_wrapper.enable_failures();
    unlink_failure_parent.unlink("Victim").unwrap();

    let removed_entry_set = unlink_failure_disk.read_directory_entries(TEST_PARENT_CLUSTER, 0, 3);
    assert_eq!(lookup_error(&unlink_failure_parent, "Victim"), Errno::ENOENT);
    assert_entry_set_invalidated(&removed_entry_set);
    assert!(unlink_failure_disk.is_cluster_allocated(TEST_REGULAR_FILE_CLUSTER));

    let non_empty_directory_disk = ExfatLookupTestDisk::new();
    let (_non_empty_directory_fs, _non_empty_directory_root, source_parent, target_parent) =
        mount_rename_parent_pair(&non_empty_directory_disk);
    non_empty_directory_disk.install_directory_subdirectory(
        RENAME_SOURCE_PARENT_CLUSTER,
        0,
        "MoveDir",
        RENAME_SOURCE_DIRECTORY_CLUSTER,
    );
    non_empty_directory_disk.install_directory_subdirectory(
        RENAME_TARGET_PARENT_CLUSTER,
        0,
        "Occupied",
        RENAME_TARGET_DIRECTORY_CLUSTER,
    );
    non_empty_directory_disk.install_directory_file(
        RENAME_TARGET_DIRECTORY_CLUSTER,
        0,
        "Leaf",
        RENAME_TARGET_DIRECTORY_CHILD_CLUSTER,
        non_empty_directory_disk.root_cluster_size(),
    );
    let source_bytes_before = non_empty_directory_disk.read_cluster(RENAME_SOURCE_PARENT_CLUSTER);
    let target_bytes_before = non_empty_directory_disk.read_cluster(RENAME_TARGET_PARENT_CLUSTER);

    let non_empty_directory_error = source_parent
        .rename("MoveDir", &target_parent, "Occupied")
        .unwrap_err();

    assert_eq!(non_empty_directory_error.error(), Errno::ENOTEMPTY);
    assert!(source_parent.lookup("MoveDir").is_ok());
    assert!(target_parent.lookup("Occupied").is_ok());
    assert_directory_unchanged(
        &non_empty_directory_disk,
        &source_parent,
        RENAME_SOURCE_PARENT_CLUSTER,
        &source_bytes_before,
        &["MoveDir"],
    );
    assert_directory_unchanged(
        &non_empty_directory_disk,
        &target_parent,
        RENAME_TARGET_PARENT_CLUSTER,
        &target_bytes_before,
        &["Occupied"],
    );

    let typed_invalid_source_disk = ExfatLookupTestDisk::new();
    typed_invalid_source_disk.install_root_fractured_entry_set(ROOT_FILE_ENTRY_INDEX, "Broken");
    let (_typed_invalid_source_fs, typed_invalid_source_root) =
        mount_root(&typed_invalid_source_disk, None);
    let typed_invalid_source_bytes =
        typed_invalid_source_disk.read_root_entries(ROOT_FILE_ENTRY_INDEX, 3);

    let typed_invalid_source_error = typed_invalid_source_root
        .rename("Broken", &typed_invalid_source_root, "Renamed")
        .unwrap_err();

    assert_eq!(typed_invalid_source_error.error(), Errno::EUCLEAN);
    assert_eq!(
        typed_invalid_source_disk.read_root_entries(ROOT_FILE_ENTRY_INDEX, 3),
        typed_invalid_source_bytes
    );
    assert_eq!(lookup_error(&typed_invalid_source_root, "Broken"), Errno::EUCLEAN);

    let typed_invalid_target_disk = ExfatLookupTestDisk::new();
    typed_invalid_target_disk.install_root_unrecognized_critical_entry(ROOT_FILE_ENTRY_INDEX);
    let (_typed_invalid_target_fs, typed_invalid_target_root) =
        mount_root(&typed_invalid_target_disk, None);
    let typed_invalid_target_bytes =
        typed_invalid_target_disk.read_root_entries(ROOT_FILE_ENTRY_INDEX, 2);

    let typed_invalid_target_error = typed_invalid_target_root.rmdir("Broken").unwrap_err();

    assert_eq!(typed_invalid_target_error.error(), Errno::EUCLEAN);
    assert_eq!(
        typed_invalid_target_disk.read_root_entries(ROOT_FILE_ENTRY_INDEX, 2),
        typed_invalid_target_bytes
    );
    assert_eq!(lookup_error(&typed_invalid_target_root, "Broken"), Errno::EUCLEAN);
}

#[ktest]
fn directory_entry_mutation_integration_concurrency_linearizes_cross_directory_rename_and_competing_mutations(
) {
    init_lookup_test_runtime();

    let disk = ExfatLookupTestDisk::new();
    let (_fs, _root_inode, source_parent, target_parent) = mount_rename_parent_pair(&disk);
    let busy_directory = target_parent
        .create("BusyDir", InodeType::Dir, InodeMode::all())
        .unwrap();
    busy_directory
        .create("Leaf", InodeType::File, InodeMode::all())
        .unwrap();
    target_parent
        .create("MovedAcross", InodeType::File, InodeMode::all())
        .unwrap();
    source_parent
        .create("MoveMe", InodeType::File, InodeMode::all())
        .unwrap();

    let fill_count = disk
        .root_directory_entry_capacity()
        .checked_div(3)
        .and_then(|entry_sets| entry_sets.checked_sub(2))
        .unwrap();
    for fill_index in 0..fill_count {
        let fill_name = format!("Fill{:02}", fill_index);
        target_parent
            .create(&fill_name, InodeType::File, InodeMode::all())
            .unwrap();
    }

    let target_size_before_growth = target_parent.size();
    let ready_count = Arc::new(AtomicUsize::new(0));
    let rename_result = Arc::new(Mutex::new(None));
    let create_result = Arc::new(Mutex::new(None));
    let rmdir_result = Arc::new(Mutex::new(None));

    let rename_thread = {
        let ready_count = ready_count.clone();
        let rename_result = rename_result.clone();
        let source_parent = source_parent.clone();
        let target_parent = target_parent.clone();

        ThreadOptions::new(move || {
            wait_for_concurrent_start(&ready_count, 3);
            *rename_result.lock() = Some(
                source_parent
                    .rename("MoveMe", &target_parent, "MovedAcross")
                    .map_err(|error| error.error()),
            );
        })
        .spawn()
    };

    let create_thread = {
        let ready_count = ready_count.clone();
        let create_result = create_result.clone();
        let target_parent = target_parent.clone();

        ThreadOptions::new(move || {
            wait_for_concurrent_start(&ready_count, 3);
            *create_result.lock() = Some(
                target_parent
                    .create("GrowthFile", InodeType::File, InodeMode::all())
                    .map(|created| created.ino())
                    .map_err(|error| error.error()),
            );
        })
        .spawn()
    };

    let rmdir_thread = {
        let ready_count = ready_count.clone();
        let rmdir_result = rmdir_result.clone();
        let target_parent = target_parent.clone();

        ThreadOptions::new(move || {
            wait_for_concurrent_start(&ready_count, 3);
            *rmdir_result.lock() = Some(target_parent.rmdir("BusyDir").map_err(|error| error.error()));
        })
        .spawn()
    };

    rename_thread.join();
    create_thread.join();
    rmdir_thread.join();

    assert_eq!(*rename_result.lock(), Some(Ok(())));
    assert!(matches!(*create_result.lock(), Some(Ok(_))));
    assert_eq!(*rmdir_result.lock(), Some(Err(Errno::ENOTEMPTY)));

    let moved_lookup = target_parent.lookup("MovedAcross").unwrap();
    let growth_lookup = target_parent.lookup("GrowthFile").unwrap();
    let busy_lookup = target_parent.lookup("BusyDir").unwrap();
    let (_source_visited_count, source_entries) = collect_dirents(&source_parent, 2);
    let (_target_visited_count, target_entries) = collect_dirents(&target_parent, 2);
    let (_busy_visited_count, busy_entries) = collect_dirents(&busy_lookup, 2);

    assert_eq!(lookup_error(&source_parent, "MoveMe"), Errno::ENOENT);
    assert_eq!(moved_lookup.type_(), InodeType::File);
    assert_eq!(growth_lookup.type_(), InodeType::File);
    assert_eq!(busy_lookup.type_(), InodeType::Dir);
    assert_eq!(target_parent.size(), target_size_before_growth + disk.root_cluster_size());
    assert_eq!(visible_name_count(&source_entries, "MoveMe"), 0);
    assert_eq!(visible_name_count(&target_entries, "MovedAcross"), 1);
    assert_eq!(visible_name_count(&target_entries, "GrowthFile"), 1);
    assert_eq!(visible_name_count(&target_entries, "BusyDir"), 1);
    assert_eq!(entry_names(&busy_entries), vec!["Leaf"]);
}

#[ktest]
fn file_content_mapping_cached_io_read_at_reads_regular_file_bytes() {
    init_lookup_test_runtime();

    let disk = ExfatLookupTestDisk::new();
    let expected = b"exfat read path";
    disk.install_root_file_with_contents(
        ROOT_FILE_ENTRY_INDEX,
        "ReadFile",
        TEST_REGULAR_FILE_CLUSTER,
        expected,
    );

    let (_fs, root_inode) = mount_root(&disk, None);
    let file_inode = root_inode.lookup("ReadFile").unwrap();
    let mut buffer = [0u8; 32];

    let read_len = file_inode.read_bytes_at(0, &mut buffer).unwrap();

    assert_eq!(read_len, expected.len());
    assert_eq!(&buffer[..read_len], expected);
}

#[ktest]
fn file_content_mapping_cached_io_read_at_rejects_directory_before_anomaly_gate() {
    init_lookup_test_runtime();

    let disk = ExfatLookupTestDisk::new();
    disk.install_root_directory(ROOT_FILE_ENTRY_INDEX, "DirOnly", TEST_CHILD_DIRECTORY_CLUSTER);
    disk.set_volume_flags(READ_AT_TEST_VOLUME_FLAGS);

    let (_fs, root_inode) = mount_root(&disk, None);
    let directory_inode = root_inode.lookup("DirOnly").unwrap();
    let mut buffer = [0u8; 8];

    let error = directory_inode.read_bytes_at(0, &mut buffer).unwrap_err();

    assert_eq!(error.error(), Errno::EISDIR);
}

#[ktest]
fn file_content_mapping_cached_io_read_at_fast_fails_on_imported_mount_anomaly() {
    init_lookup_test_runtime();

    let disk = ExfatLookupTestDisk::new();
    disk.install_root_file_with_contents(
        ROOT_FILE_ENTRY_INDEX,
        "AnomalousFile",
        TEST_REGULAR_FILE_CLUSTER,
        b"visible bytes",
    );
    disk.set_volume_flags(READ_AT_TEST_VOLUME_FLAGS);

    let (_fs, root_inode) = mount_root(&disk, None);
    let file_inode = root_inode.lookup("AnomalousFile").unwrap();
    let mut buffer = [0xA5; 16];

    let error = file_inode.read_bytes_at(0, &mut buffer).unwrap_err();

    assert_eq!(error.error(), Errno::EIO);
    assert_eq!(buffer, [0xA5; 16]);
}
