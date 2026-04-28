// SPDX-License-Identifier: MPL-2.0

use alloc::{ffi::CString, string::String, sync::Arc, vec, vec::Vec};
use core::sync::atomic::{AtomicUsize, Ordering};

use aster_block::{bio::BioType, BlockDevice};
use ostd::mm::{VmIo, VmReader, PAGE_SIZE};

use super::{
    super::{
        boot::BootRegion,
        fs::{ExfatFs, ExfatFsType},
        inode::ExfatInode,
    },
    disk::{ExfatLookupTestDisk, ObservedBio},
};
use crate::{
    fs::{
        file::{InodeType, StatusFlags},
        utils::DirentVisitor,
        vfs::{
            file_system::{FileSystem, FsFlags},
            inode::{Inode, Metadata},
            page_cache::{CachePage, CachePageExt, PageState},
        },
    },
    prelude::*,
    thread::Thread,
    vm::vmo::CommitFlags,
};

const DIRECTORY_ENTRY_SIZE: usize = 32;
const FILE_NAME_ENTRY_TYPE: u8 = 0xC1;
const STREAM_DATA_LENGTH_OFFSET: usize = DIRECTORY_ENTRY_SIZE + 24;
const STREAM_FIRST_CLUSTER_OFFSET: usize = DIRECTORY_ENTRY_SIZE + 20;
const STREAM_GENERAL_FLAGS_OFFSET: usize = DIRECTORY_ENTRY_SIZE + 1;
const STREAM_VALID_DATA_LENGTH_OFFSET: usize = DIRECTORY_ENTRY_SIZE + 8;

#[derive(Debug, Eq, PartialEq)]
pub(in super::super) struct CapturedDirent {
    pub(in super::super) name: String,
    pub(in super::super) ino: u64,
    pub(in super::super) inode_type: InodeType,
    pub(in super::super) offset: usize,
}

impl DirentVisitor for Vec<CapturedDirent> {
    fn visit(&mut self, name: &str, ino: u64, inode_type: InodeType, offset: usize) -> Result<()> {
        self.push(CapturedDirent {
            name: name.into(),
            ino,
            inode_type,
            offset,
        });
        Ok(())
    }
}

pub(in super::super) struct RejectingDirentVisitor {
    pub(in super::super) entries: Vec<String>,
    pub(in super::super) reject_name: &'static str,
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

pub(in super::super) fn init_lookup_test_runtime() {
    crate::time::clocks::init_for_ktest();
}

pub(in super::super) fn collect_dirents(
    inode: &Arc<dyn Inode>,
    offset: usize,
) -> (usize, Vec<CapturedDirent>) {
    let mut entries = Vec::new();
    let visited_count = inode.readdir_at(offset, &mut entries).unwrap();
    (visited_count, entries)
}

pub(in super::super) fn entry_names(entries: &[CapturedDirent]) -> Vec<&str> {
    entries.iter().map(|entry| entry.name.as_str()).collect()
}

pub(in super::super) fn entry_offsets(entries: &[CapturedDirent]) -> Vec<usize> {
    entries.iter().map(|entry| entry.offset).collect()
}

pub(in super::super) fn lookup_error(inode: &Arc<dyn Inode>, name: &str) -> Errno {
    inode.lookup(name).unwrap_err().error()
}

pub(in super::super) fn mount_root(
    disk: &Arc<ExfatLookupTestDisk>,
    options: Option<&str>,
) -> (Arc<dyn FileSystem>, Arc<dyn Inode>) {
    mount_root_with_flags(disk, FsFlags::empty(), options)
}

pub(in super::super) fn mount_root_with_flags(
    disk: &Arc<ExfatLookupTestDisk>,
    flags: FsFlags,
    options: Option<&str>,
) -> (Arc<dyn FileSystem>, Arc<dyn Inode>) {
    let block_device: Arc<dyn BlockDevice> = disk.as_block_device();
    mount_root_from_block_device(block_device, flags, options)
}

pub(in super::super) fn mount_root_from_block_device(
    block_device: Arc<dyn BlockDevice>,
    flags: FsFlags,
    options: Option<&str>,
) -> (Arc<dyn FileSystem>, Arc<dyn Inode>) {
    let args = options.map(|mount_options| CString::new(mount_options).unwrap());
    let fs = ExfatFsType.create(flags, args, Some(block_device)).unwrap();
    let root_inode = fs.root_inode();
    (fs, root_inode)
}

pub(in super::super) fn lookup_exfat_inode(inode: &Arc<dyn Inode>) -> &ExfatInode {
    inode.downcast_ref::<ExfatInode>().unwrap()
}

pub(in super::super) fn published_lookup_state(
    inode: &Arc<dyn Inode>,
) -> (Arc<dyn BlockDevice>, BootRegion) {
    let fs = inode.fs();
    let exfat_fs = fs.downcast_ref::<ExfatFs>().unwrap();
    let (block_device, boot_region, _, _, _) = exfat_fs.published_lookup_state().unwrap();
    (block_device, boot_region)
}

pub(in super::super) fn published_page_count(inode: &Arc<dyn Inode>) -> usize {
    inode.size().div_ceil(PAGE_SIZE)
}

pub(in super::super) fn patterned_bytes(len: usize) -> Vec<u8> {
    (0..len)
        .map(|index| u8::try_from(index % 251).unwrap())
        .collect()
}

pub(in super::super) fn read_cache_page_bytes(page: &CachePage) -> Vec<u8> {
    let mut bytes = vec![0; PAGE_SIZE];
    page.read_bytes(0, &mut bytes).unwrap();
    bytes
}

pub(in super::super) fn dirty_regular_file_first_page(file_inode: &Arc<dyn Inode>, bytes: &[u8]) {
    let page_cache = file_inode.page_cache().unwrap();
    let page = page_cache
        .commit_on(0, CommitFlags::WILL_OVERWRITE)
        .unwrap();
    let page: ostd::mm::Frame<dyn ostd::mm::frame::meta::AnyFrameMeta> = page.into();
    let mut page = CachePage::try_from(page).unwrap();
    page.write_bytes(0, bytes).unwrap();
    page.store_state(PageState::Dirty);
}

pub(in super::super) fn assert_observed_bios(
    observed_bios: &[ObservedBio],
    expected_type: BioType,
    expected_ranges: &[(usize, usize)],
) {
    assert_eq!(observed_bios.len(), expected_ranges.len());

    for (observed_bio, (expected_start, expected_len)) in
        observed_bios.iter().zip(expected_ranges.iter().copied())
    {
        assert_eq!(observed_bio.type_, expected_type);
        assert_eq!(
            observed_bio.byte_range,
            expected_start..expected_start + expected_len
        );
        assert_eq!(observed_bio.segment_lengths, vec![expected_len]);
    }
}

pub(in super::super) fn assert_sync_writeback_before_device_sync(observed_bios: &[ObservedBio]) {
    let write_index = observed_bios
        .iter()
        .position(|bio| bio.type_ == BioType::Write)
        .unwrap_or_else(|| {
            panic!("expected writeback BIO before device sync, got {observed_bios:?}")
        });
    let flush_index = observed_bios
        .iter()
        .position(|bio| bio.type_ == BioType::Flush)
        .unwrap_or_else(|| {
            panic!("expected device-sync flush BIO after writeback, got {observed_bios:?}")
        });

    assert!(write_index < flush_index);
    assert!(observed_bios[..flush_index]
        .iter()
        .all(|bio| bio.type_ == BioType::Write));
    assert!(observed_bios[flush_index..]
        .iter()
        .all(|bio| bio.type_ == BioType::Flush));
}

pub(in super::super) fn assert_flush_only(observed_bios: &[ObservedBio]) {
    assert!(
        !observed_bios.is_empty(),
        "expected a device-sync flush BIO, got no block I/O"
    );
    assert!(
        observed_bios.iter().all(|bio| bio.type_ == BioType::Flush),
        "expected only device-sync flush BIOs, got {observed_bios:?}"
    );
}

pub(in super::super) fn decode_entry_name(entry_set: &[u8]) -> Vec<u16> {
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

pub(in super::super) fn entry_index_from_ino(ino: u64) -> usize {
    usize::try_from(ino & u64::from(u32::MAX)).unwrap()
}

pub(in super::super) fn root_entry_set(
    disk: &Arc<ExfatLookupTestDisk>,
    entry_index: usize,
) -> Vec<u8> {
    let secondary_count = usize::from(disk.read_root_entries(entry_index, 1)[1]);
    disk.read_root_entries(entry_index, secondary_count + 1)
}

pub(in super::super) fn stream_lengths(entry_set: &[u8]) -> (u64, u64) {
    let valid_data_length = u64::from_le_bytes(
        entry_set[STREAM_VALID_DATA_LENGTH_OFFSET..STREAM_VALID_DATA_LENGTH_OFFSET + 8]
            .try_into()
            .unwrap(),
    );
    let data_length = u64::from_le_bytes(
        entry_set[STREAM_DATA_LENGTH_OFFSET..STREAM_DATA_LENGTH_OFFSET + 8]
            .try_into()
            .unwrap(),
    );
    (valid_data_length, data_length)
}

pub(in super::super) fn stream_first_cluster(entry_set: &[u8]) -> u32 {
    u32::from_le_bytes(
        entry_set[STREAM_FIRST_CLUSTER_OFFSET..STREAM_FIRST_CLUSTER_OFFSET + 4]
            .try_into()
            .unwrap(),
    )
}

pub(in super::super) fn stream_has_no_fat_chain(entry_set: &[u8]) -> bool {
    entry_set[STREAM_GENERAL_FLAGS_OFFSET] & 0x02 != 0
}

pub(in super::super) fn write_bytes_append(inode: &Arc<dyn Inode>, buf: &[u8]) -> Result<usize> {
    let mut reader = VmReader::from(buf).to_fallible();
    inode.write_at(0, &mut reader, StatusFlags::O_APPEND)
}

pub(in super::super) fn next_stream_cluster(
    disk: &Arc<ExfatLookupTestDisk>,
    entry_set: &[u8],
) -> u32 {
    let first_cluster = stream_first_cluster(entry_set);
    if stream_has_no_fat_chain(entry_set) {
        first_cluster + 1
    } else {
        disk.fat_chain_step(first_cluster)
    }
}

pub(in super::super) fn assert_metadata_unchanged(actual: Metadata, expected: Metadata) {
    assert_eq!(actual.ino, expected.ino);
    assert_eq!(actual.size, expected.size);
    assert_eq!(actual.optimal_block_size, expected.optimal_block_size);
    assert_eq!(actual.nr_sectors_allocated, expected.nr_sectors_allocated);
    assert_eq!(actual.last_access_at, expected.last_access_at);
    assert_eq!(actual.last_modify_at, expected.last_modify_at);
    assert_eq!(actual.last_meta_change_at, expected.last_meta_change_at);
    assert_eq!(actual.type_, expected.type_);
    assert_eq!(actual.mode, expected.mode);
    assert_eq!(actual.nr_hard_links, expected.nr_hard_links);
    assert_eq!(actual.uid, expected.uid);
    assert_eq!(actual.gid, expected.gid);
    assert_eq!(actual.container_dev_id, expected.container_dev_id);
    assert_eq!(actual.self_dev_id, expected.self_dev_id);
}

pub(in super::super) fn visible_name_count(
    entries: &[CapturedDirent],
    expected_name: &str,
) -> usize {
    entries
        .iter()
        .filter(|entry| entry.name == expected_name)
        .count()
}

pub(in super::super) fn wait_for_concurrent_start(
    ready_count: &AtomicUsize,
    participant_count: usize,
) {
    ready_count.fetch_add(1, Ordering::Relaxed);
    while ready_count.load(Ordering::Relaxed) < participant_count {
        Thread::yield_now();
    }
}
