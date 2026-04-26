// SPDX-License-Identifier: MPL-2.0

use alloc::{sync::Arc, vec, vec::Vec};
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use aster_block::{BlockDevice, SECTOR_SIZE};
use ostd::mm::PAGE_SIZE;

use super::*;
use crate::thread::{Thread, kernel_thread::ThreadOptions};

struct FileSnapshot {
    entry_set: Vec<u8>,
    metadata: Metadata,
    page_count: usize,
    visible_bytes: Vec<u8>,
}

fn install_root_file_with_cluster_contents(
    disk: &Arc<ExfatLookupTestDisk>,
    entry_index: usize,
    name: &str,
    clusters: &[u32],
    data_length: usize,
    valid_data_length: usize,
    no_fat_chain: bool,
    contents: &[u8],
) {
    assert_eq!(contents.len(), data_length);
    disk.install_root_file_with_cluster_chain(
        entry_index,
        name,
        clusters[0],
        data_length,
        valid_data_length,
        no_fat_chain,
        clusters,
    );

    let cluster_size = disk.root_cluster_size();
    for (cluster_index, cluster) in clusters.iter().enumerate() {
        let start = cluster_index * cluster_size;
        if start >= contents.len() {
            break;
        }
        let end = (start + cluster_size).min(contents.len());
        disk.write_cluster_prefix(*cluster, &contents[start..end]);
    }

    if !no_fat_chain {
        for cluster_pair in clusters.windows(2) {
            disk.set_fat_chain_step(cluster_pair[0], cluster_pair[1]);
        }
        disk.terminate_fat_chain(*clusters.last().unwrap());
    }
}

fn visible_file_bytes(inode: &Arc<dyn Inode>) -> Vec<u8> {
    let mut bytes = vec![0; inode.size()];
    let read_len = inode.read_bytes_at(0, &mut bytes).unwrap();
    assert_eq!(read_len, bytes.len());
    bytes
}

fn snapshot_regular_file(
    disk: &Arc<ExfatLookupTestDisk>,
    inode: &Arc<dyn Inode>,
    entry_index: usize,
) -> FileSnapshot {
    FileSnapshot {
        entry_set: root_entry_set(disk, entry_index),
        metadata: inode.metadata(),
        page_count: published_page_count(inode),
        visible_bytes: visible_file_bytes(inode),
    }
}

fn assert_same_snapshot(actual: FileSnapshot, expected: &FileSnapshot) {
    assert_eq!(actual.entry_set, expected.entry_set);
    assert_metadata_unchanged(actual.metadata, expected.metadata);
    assert_eq!(actual.page_count, expected.page_count);
    assert_eq!(actual.visible_bytes, expected.visible_bytes);
}

fn wait_for_flag(flag: &AtomicBool) {
    while !flag.load(Ordering::Relaxed) {
        Thread::yield_now();
    }
}

pub(super) fn file_content_mutation_integration_success_path_write_append_grow_shrink_readback() {
    init_lookup_test_runtime();

    const SUCCESS_WRITE_CLUSTER: u32 = 30;
    const SUCCESS_GAP_CLUSTER: u32 = 31;
    const SUCCESS_APPEND_CLUSTER: u32 = 33;
    const SUCCESS_FRAGMENT_CLUSTER: u32 = 40;
    const SUCCESS_FRAGMENT_BLOCKER_CLUSTER: u32 = SUCCESS_FRAGMENT_CLUSTER + 1;
    const SUCCESS_SHRINK_FIRST_CLUSTER: u32 = 44;
    const SUCCESS_SHRINK_SECOND_CLUSTER: u32 = 47;
    const SUCCESS_SHRINK_THIRD_CLUSTER: u32 = 48;
    const SUCCESS_FRAGMENT_ENTRY_INDEX: usize = ROOT_THIRD_FILE_ENTRY_INDEX + 3;
    const SUCCESS_FRAGMENT_BLOCKER_ENTRY_INDEX: usize = SUCCESS_FRAGMENT_ENTRY_INDEX + 3;
    const SUCCESS_SHRINK_ENTRY_INDEX: usize = SUCCESS_FRAGMENT_BLOCKER_ENTRY_INDEX + 3;

    let disk = ExfatLookupTestDisk::new();
    let cluster_size = disk.root_cluster_size();
    let append_initial_bytes = vec![b'A'; cluster_size];
    let fragment_initial_bytes = vec![b'Q'; cluster_size];
    let shrink_first_cluster_bytes = vec![b'A'; cluster_size];
    let shrink_second_cluster_bytes = vec![b'B'; cluster_size];
    let shrink_third_cluster_bytes = vec![b'C'; cluster_size];

    disk.install_root_file_with_contents(
        ROOT_FILE_ENTRY_INDEX,
        "WriteFile",
        SUCCESS_WRITE_CLUSTER,
        b"abcdefgh",
    );
    disk.install_root_file_with_cluster_chain(
        ROOT_SECOND_FILE_ENTRY_INDEX,
        "GapWrite",
        SUCCESS_GAP_CLUSTER,
        16,
        4,
        true,
        &[SUCCESS_GAP_CLUSTER],
    );
    disk.write_cluster_prefix(SUCCESS_GAP_CLUSTER, &[b'D', b'A', b'T', b'A']);
    disk.install_root_file_with_cluster_chain(
        ROOT_THIRD_FILE_ENTRY_INDEX,
        "AppendGrow",
        SUCCESS_APPEND_CLUSTER,
        cluster_size,
        cluster_size,
        true,
        &[SUCCESS_APPEND_CLUSTER],
    );
    disk.write_cluster_prefix(SUCCESS_APPEND_CLUSTER, &append_initial_bytes);
    disk.install_root_file_with_cluster_chain(
        SUCCESS_FRAGMENT_ENTRY_INDEX,
        "FragmentGrow",
        SUCCESS_FRAGMENT_CLUSTER,
        cluster_size,
        cluster_size,
        true,
        &[SUCCESS_FRAGMENT_CLUSTER],
    );
    disk.write_cluster_prefix(SUCCESS_FRAGMENT_CLUSTER, &fragment_initial_bytes);
    disk.install_root_file_with_contents(
        SUCCESS_FRAGMENT_BLOCKER_ENTRY_INDEX,
        "Blocker",
        SUCCESS_FRAGMENT_BLOCKER_CLUSTER,
        b"busy",
    );
    install_root_file_with_cluster_contents(
        &disk,
        SUCCESS_SHRINK_ENTRY_INDEX,
        "ShrinkFile",
        &[
            SUCCESS_SHRINK_FIRST_CLUSTER,
            SUCCESS_SHRINK_SECOND_CLUSTER,
            SUCCESS_SHRINK_THIRD_CLUSTER,
        ],
        cluster_size * 3,
        cluster_size * 3,
        false,
        &[
            shrink_first_cluster_bytes.as_slice(),
            shrink_second_cluster_bytes.as_slice(),
            shrink_third_cluster_bytes.as_slice(),
        ]
        .concat(),
    );

    let (_fs, root_inode) = mount_root(&disk, None);
    let write_inode = root_inode.lookup("WriteFile").unwrap();
    let gap_inode = root_inode.lookup("GapWrite").unwrap();
    let append_inode = root_inode.lookup("AppendGrow").unwrap();
    let fragment_inode = root_inode.lookup("FragmentGrow").unwrap();
    let shrink_inode = root_inode.lookup("ShrinkFile").unwrap();
    let shrink_entry_set_before = root_entry_set(&disk, SUCCESS_SHRINK_ENTRY_INDEX);

    assert_eq!(write_inode.write_bytes_at(2, b"XYZ").unwrap(), 3);
    assert_eq!(gap_inode.write_bytes_at(8, b"END").unwrap(), 3);
    assert_eq!(write_bytes_append(&append_inode, b"TAIL!").unwrap(), 5);
    assert_eq!(fragment_inode.write_bytes_at(cluster_size, b"tail").unwrap(), 4);

    let fragmented_entry_set = root_entry_set(&disk, SUCCESS_FRAGMENT_ENTRY_INDEX);
    let fragmented_cluster = disk.fat_chain_step(SUCCESS_FRAGMENT_CLUSTER);
    assert!(!stream_has_no_fat_chain(&fragmented_entry_set));
    assert_eq!(
        stream_first_cluster(&fragmented_entry_set),
        SUCCESS_FRAGMENT_CLUSTER
    );
    assert_ne!(fragmented_cluster, SUCCESS_FRAGMENT_BLOCKER_CLUSTER);
    assert_eq!(disk.fat_chain_step(fragmented_cluster), FAT_END_OF_CHAIN);
    assert!(disk.is_cluster_allocated(fragmented_cluster));

    let shrink_size = cluster_size + 2;
    shrink_inode.resize(shrink_size).unwrap();

    let mut write_bytes = [0u8; 8];
    let mut gap_bytes = [0xCC; 16];
    let mut append_bytes = vec![0xCC; cluster_size + 5];
    let mut shrink_visible_bytes = vec![0xCC; shrink_size];
    let shrunk_entry_set = root_entry_set(&disk, SUCCESS_SHRINK_ENTRY_INDEX);
    let mut eof_bytes = [0xDD; 4];

    assert_eq!(write_inode.read_bytes_at(0, &mut write_bytes).unwrap(), 8);
    assert_eq!(&write_bytes, b"abXYZfgh");
    assert_eq!(gap_inode.read_bytes_at(0, &mut gap_bytes).unwrap(), 16);
    assert_eq!(&gap_bytes[..4], b"DATA");
    assert_eq!(&gap_bytes[4..8], &[0; 4]);
    assert_eq!(&gap_bytes[8..11], b"END");
    assert_eq!(&gap_bytes[11..], &[0; 5]);
    assert_eq!(
        append_inode.read_bytes_at(0, &mut append_bytes).unwrap(),
        cluster_size + 5
    );
    assert_eq!(&append_bytes[..cluster_size], append_initial_bytes.as_slice());
    assert_eq!(&append_bytes[cluster_size..], b"TAIL!");
    assert_eq!(
        shrink_inode
            .read_bytes_at(0, &mut shrink_visible_bytes)
            .unwrap(),
        shrink_size
    );
    assert_eq!(
        &shrink_visible_bytes[..cluster_size],
        shrink_first_cluster_bytes.as_slice()
    );
    assert_eq!(
        &shrink_visible_bytes[cluster_size..],
        &shrink_second_cluster_bytes[..2]
    );
    assert_eq!(stream_lengths(&shrunk_entry_set), (shrink_size as u64, shrink_size as u64));
    assert_eq!(
        stream_first_cluster(&shrunk_entry_set),
        SUCCESS_SHRINK_FIRST_CLUSTER
    );
    assert!(!stream_has_no_fat_chain(&shrink_entry_set_before));
    assert!(!stream_has_no_fat_chain(&shrunk_entry_set));
    assert_eq!(shrink_inode.size(), shrink_size);
    assert_eq!(
        append_inode.metadata().nr_sectors_allocated,
        2 * cluster_size / SECTOR_SIZE
    );
    assert_eq!(
        shrink_inode.metadata().nr_sectors_allocated,
        2 * cluster_size / SECTOR_SIZE
    );
    assert_eq!(
        &shrunk_entry_set[FILE_ATTRIBUTES_OFFSET..FILE_ATTRIBUTES_OFFSET + 2],
        &shrink_entry_set_before[FILE_ATTRIBUTES_OFFSET..FILE_ATTRIBUTES_OFFSET + 2]
    );
    assert!(disk.is_cluster_allocated(SUCCESS_APPEND_CLUSTER));
    assert!(disk.is_cluster_allocated(SUCCESS_FRAGMENT_BLOCKER_CLUSTER));
    assert!(!disk.is_cluster_allocated(SUCCESS_SHRINK_THIRD_CLUSTER));
    assert_eq!(published_page_count(&shrink_inode), shrink_size.div_ceil(PAGE_SIZE));
    assert_eq!(shrink_inode.read_bytes_at(shrink_size, &mut eof_bytes).unwrap(), 0);
    assert_eq!(eof_bytes, [0xDD; 4]);
}

pub(super) fn file_content_mutation_integration_failure_maintenance_preserves_safe_visibility() {
    init_lookup_test_runtime();

    let allocation_disk = ExfatLookupTestDisk::new();
    let cluster_size = allocation_disk.root_cluster_size();
    let allocation_bytes = vec![b'A'; cluster_size];
    allocation_disk.install_root_file_with_cluster_chain(
        ROOT_FILE_ENTRY_INDEX,
        "AllocFail",
        TEST_REGULAR_FILE_CLUSTER,
        cluster_size,
        cluster_size,
        true,
        &[TEST_REGULAR_FILE_CLUSTER],
    );
    allocation_disk.write_cluster_prefix(TEST_REGULAR_FILE_CLUSTER, &allocation_bytes);
    let allocation_fail_disk = ExfatLookupToggleFailingWriteDisk::new(
        allocation_disk.clone(),
        allocation_disk.allocation_bitmap_byte_offset_for_cluster(TEST_CONTIGUOUS_SECOND_CLUSTER),
        1,
    );
    let allocation_block_device: Arc<dyn BlockDevice> = allocation_fail_disk.clone();
    let (_fs, allocation_root) =
        mount_root_from_block_device(allocation_block_device, FsFlags::empty(), None);
    let allocation_inode = allocation_root.lookup("AllocFail").unwrap();
    let allocation_snapshot =
        snapshot_regular_file(&allocation_disk, &allocation_inode, ROOT_FILE_ENTRY_INDEX);

    allocation_fail_disk.enable_failures();
    let allocation_error = write_bytes_append(&allocation_inode, b"grow").unwrap_err();

    assert_eq!(allocation_error.error(), Errno::EIO);
    assert_same_snapshot(
        snapshot_regular_file(&allocation_disk, &allocation_inode, ROOT_FILE_ENTRY_INDEX),
        &allocation_snapshot,
    );

    let fat_disk = ExfatLookupTestDisk::new();
    fat_disk.install_root_file_with_cluster_chain(
        ROOT_FILE_ENTRY_INDEX,
        "FatFail",
        TEST_REGULAR_FILE_CLUSTER,
        cluster_size,
        cluster_size,
        true,
        &[TEST_REGULAR_FILE_CLUSTER],
    );
    fat_disk.write_cluster_prefix(TEST_REGULAR_FILE_CLUSTER, &allocation_bytes);
    fat_disk.install_root_file_with_contents(
        ROOT_SECOND_FILE_ENTRY_INDEX,
        "Busy",
        TEST_CONTIGUOUS_SECOND_CLUSTER,
        b"busy",
    );
    let fat_fail_disk = ExfatLookupToggleFailingWriteDisk::new(
        fat_disk.clone(),
        fat_disk.fat_entry_offset(TEST_REGULAR_FILE_CLUSTER),
        core::mem::size_of::<u32>(),
    );
    let fat_block_device: Arc<dyn BlockDevice> = fat_fail_disk.clone();
    let (_fs, fat_root) = mount_root_from_block_device(fat_block_device, FsFlags::empty(), None);
    let fat_inode = fat_root.lookup("FatFail").unwrap();
    let fat_snapshot = snapshot_regular_file(&fat_disk, &fat_inode, ROOT_FILE_ENTRY_INDEX);

    fat_fail_disk.enable_failures();
    let fat_error = write_bytes_append(&fat_inode, b"tail").unwrap_err();

    assert_eq!(fat_error.error(), Errno::EIO);
    assert_same_snapshot(
        snapshot_regular_file(&fat_disk, &fat_inode, ROOT_FILE_ENTRY_INDEX),
        &fat_snapshot,
    );

    let zero_fill_disk = ExfatLookupTestDisk::new();
    let mut zero_fill_cluster = vec![0xA5; cluster_size];
    zero_fill_cluster[..4].copy_from_slice(b"DATA");
    zero_fill_disk.install_root_file_with_cluster_chain(
        ROOT_FILE_ENTRY_INDEX,
        "ZeroFillFail",
        TEST_REGULAR_FILE_CLUSTER,
        cluster_size,
        4,
        true,
        &[TEST_REGULAR_FILE_CLUSTER],
    );
    zero_fill_disk.write_cluster_prefix(TEST_REGULAR_FILE_CLUSTER, &zero_fill_cluster);
    let zero_fill_fail_disk = ExfatLookupToggleFailingWriteDisk::new(
        zero_fill_disk.clone(),
        zero_fill_disk.cluster_offset(TEST_REGULAR_FILE_CLUSTER) + 4,
        1,
    );
    let zero_fill_block_device: Arc<dyn BlockDevice> = zero_fill_fail_disk.clone();
    let (_fs, zero_fill_root) =
        mount_root_from_block_device(zero_fill_block_device, FsFlags::empty(), None);
    let zero_fill_inode = zero_fill_root.lookup("ZeroFillFail").unwrap();
    let zero_fill_snapshot =
        snapshot_regular_file(&zero_fill_disk, &zero_fill_inode, ROOT_FILE_ENTRY_INDEX);

    zero_fill_fail_disk.enable_failures();
    let zero_fill_error = zero_fill_inode
        .write_bytes_at(cluster_size + 4, b"END")
        .unwrap_err();

    assert_eq!(zero_fill_error.error(), Errno::EIO);
    assert_same_snapshot(
        snapshot_regular_file(&zero_fill_disk, &zero_fill_inode, ROOT_FILE_ENTRY_INDEX),
        &zero_fill_snapshot,
    );

    let payload_disk = ExfatLookupTestDisk::new();
    payload_disk.install_root_file_with_cluster_chain(
        ROOT_FILE_ENTRY_INDEX,
        "PayloadFail",
        TEST_REGULAR_FILE_CLUSTER,
        cluster_size,
        cluster_size,
        true,
        &[TEST_REGULAR_FILE_CLUSTER],
    );
    payload_disk.write_cluster_prefix(TEST_REGULAR_FILE_CLUSTER, &allocation_bytes);
    let payload_fail_disk = ExfatLookupToggleFailingWriteDisk::new(
        payload_disk.clone(),
        payload_disk.cluster_offset(TEST_REGULAR_FILE_CLUSTER),
        1,
    );
    let payload_block_device: Arc<dyn BlockDevice> = payload_fail_disk.clone();
    let (_fs, payload_root) =
        mount_root_from_block_device(payload_block_device, FsFlags::empty(), None);
    let payload_inode = payload_root.lookup("PayloadFail").unwrap();
    let payload_snapshot =
        snapshot_regular_file(&payload_disk, &payload_inode, ROOT_FILE_ENTRY_INDEX);

    payload_fail_disk.enable_failures();
    let payload_error = payload_inode.write_bytes_at(0, b"boom").unwrap_err();

    assert_eq!(payload_error.error(), Errno::EIO);
    assert_same_snapshot(
        snapshot_regular_file(&payload_disk, &payload_inode, ROOT_FILE_ENTRY_INDEX),
        &payload_snapshot,
    );

    let publication_disk = ExfatLookupTestDisk::new();
    publication_disk.install_root_file_with_cluster_chain(
        ROOT_FILE_ENTRY_INDEX,
        "PublishFail",
        TEST_REGULAR_FILE_CLUSTER,
        cluster_size,
        cluster_size,
        true,
        &[TEST_REGULAR_FILE_CLUSTER],
    );
    publication_disk.write_cluster_prefix(TEST_REGULAR_FILE_CLUSTER, &allocation_bytes);
    let publication_fail_disk = ExfatLookupToggleFailingWriteDisk::new(
        publication_disk.clone(),
        publication_disk.root_directory_offset()
            + ROOT_FILE_ENTRY_INDEX * DIRECTORY_ENTRY_SIZE
            + DIRECTORY_ENTRY_SIZE,
        DIRECTORY_ENTRY_SIZE,
    );
    let publication_block_device: Arc<dyn BlockDevice> = publication_fail_disk.clone();
    let (_fs, publication_root) =
        mount_root_from_block_device(publication_block_device, FsFlags::empty(), None);
    let publication_inode = publication_root.lookup("PublishFail").unwrap();
    let publication_snapshot =
        snapshot_regular_file(&publication_disk, &publication_inode, ROOT_FILE_ENTRY_INDEX);

    publication_fail_disk.enable_failures();
    let publication_error = write_bytes_append(&publication_inode, b"boom").unwrap_err();

    assert_eq!(publication_error.error(), Errno::EIO);
    assert_same_snapshot(
        snapshot_regular_file(&publication_disk, &publication_inode, ROOT_FILE_ENTRY_INDEX),
        &publication_snapshot,
    );

    let shrink_disk = ExfatLookupTestDisk::new();
    let first_cluster_bytes = vec![b'A'; cluster_size];
    let second_cluster_bytes = vec![b'B'; cluster_size];
    let third_cluster_bytes = vec![b'C'; cluster_size];
    install_root_file_with_cluster_contents(
        &shrink_disk,
        ROOT_FILE_ENTRY_INDEX,
        "ShrinkFreeFail",
        &[
            TEST_FRAGMENTED_FIRST_CLUSTER,
            TEST_FRAGMENTED_SECOND_CLUSTER,
            TEST_FRAGMENTED_THIRD_CLUSTER,
        ],
        cluster_size * 3,
        cluster_size * 3,
        false,
        &[
            first_cluster_bytes.as_slice(),
            second_cluster_bytes.as_slice(),
            third_cluster_bytes.as_slice(),
        ]
        .concat(),
    );
    let shrink_fail_disk = ExfatLookupToggleFailingWriteDisk::new(
        shrink_disk.clone(),
        shrink_disk.allocation_bitmap_byte_offset_for_cluster(TEST_FRAGMENTED_THIRD_CLUSTER),
        1,
    );
    let shrink_block_device: Arc<dyn BlockDevice> = shrink_fail_disk.clone();
    let (_fs, shrink_root) =
        mount_root_from_block_device(shrink_block_device, FsFlags::empty(), None);
    let shrink_inode = shrink_root.lookup("ShrinkFreeFail").unwrap();
    let shrink_size = cluster_size + 2;

    shrink_fail_disk.enable_failures();
    let shrink_error = shrink_inode.resize(shrink_size).unwrap_err();

    let shrink_entry_set = root_entry_set(&shrink_disk, ROOT_FILE_ENTRY_INDEX);
    let mut visible_bytes = vec![0xCC; shrink_size];
    let mut eof_bytes = [0xDD; 4];
    let read_len = shrink_inode.read_bytes_at(0, &mut visible_bytes).unwrap();

    assert_eq!(shrink_error.error(), Errno::EIO);
    assert_eq!(shrink_inode.size(), shrink_size);
    assert_eq!(read_len, shrink_size);
    assert_eq!(&visible_bytes[..cluster_size], first_cluster_bytes.as_slice());
    assert_eq!(&visible_bytes[cluster_size..read_len], &second_cluster_bytes[..2]);
    assert_eq!(stream_lengths(&shrink_entry_set), (shrink_size as u64, shrink_size as u64));
    assert_eq!(
        stream_first_cluster(&shrink_entry_set),
        TEST_FRAGMENTED_FIRST_CLUSTER
    );
    assert!(!stream_has_no_fat_chain(&shrink_entry_set));
    assert_eq!(
        shrink_disk.fat_chain_step(TEST_FRAGMENTED_FIRST_CLUSTER),
        TEST_FRAGMENTED_SECOND_CLUSTER
    );
    assert_eq!(
        shrink_disk.fat_chain_step(TEST_FRAGMENTED_SECOND_CLUSTER),
        FAT_END_OF_CHAIN
    );
    assert!(shrink_disk.is_cluster_allocated(TEST_FRAGMENTED_THIRD_CLUSTER));
    assert_eq!(
        shrink_inode.metadata().nr_sectors_allocated,
        2 * cluster_size / SECTOR_SIZE
    );
    assert_eq!(shrink_inode.read_bytes_at(shrink_size, &mut eof_bytes).unwrap(), 0);
    assert_eq!(eof_bytes, [0xDD; 4]);
}

pub(super) fn file_content_mutation_integration_repeated_calls_keep_state_stable() {
    init_lookup_test_runtime();

    let disk = ExfatLookupTestDisk::new();
    let cluster_size = disk.root_cluster_size();
    let initial_bytes = vec![b'R'; cluster_size];
    disk.install_root_file_with_cluster_chain(
        ROOT_FILE_ENTRY_INDEX,
        "RepeatedMutation",
        TEST_REGULAR_FILE_CLUSTER,
        cluster_size,
        cluster_size,
        true,
        &[TEST_REGULAR_FILE_CLUSTER],
    );
    disk.write_cluster_prefix(TEST_REGULAR_FILE_CLUSTER, &initial_bytes);

    let (_fs, root_inode) = mount_root(&disk, None);
    let file_inode = root_inode.lookup("RepeatedMutation").unwrap();

    assert_eq!(write_bytes_append(&file_inode, b"GROW").unwrap(), 4);
    let snapshot = snapshot_regular_file(&disk, &file_inode, ROOT_FILE_ENTRY_INDEX);

    assert_eq!(file_inode.write_bytes_at(3, b"").unwrap(), 0);
    assert_eq!(write_bytes_append(&file_inode, b"").unwrap(), 0);
    file_inode.resize(file_inode.size()).unwrap();
    file_inode.resize(file_inode.size()).unwrap();

    for _ in 0..2 {
        for mode in [
            FallocMode::Allocate,
            FallocMode::AllocateKeepSize,
            FallocMode::AllocateUnshareRange,
            FallocMode::PunchHoleKeepSize,
            FallocMode::ZeroRange,
            FallocMode::ZeroRangeKeepSize,
            FallocMode::CollapseRange,
            FallocMode::InsertRange,
        ] {
            let error = file_inode.fallocate(mode, 0, 4).unwrap_err();
            assert_eq!(error.error(), Errno::EOPNOTSUPP);
        }
    }

    assert_same_snapshot(
        snapshot_regular_file(&disk, &file_inode, ROOT_FILE_ENTRY_INDEX),
        &snapshot,
    );
}

pub(super) fn file_content_mutation_integration_concurrency_serializes_mutation_and_observation() {
    init_lookup_test_runtime();

    let disk = ExfatLookupTestDisk::new();
    let cluster_size = disk.root_cluster_size();
    let initial_bytes = vec![b'A'; cluster_size];
    let resize_first_cluster_bytes = vec![b'A'; cluster_size];
    let resize_second_cluster_bytes = vec![b'B'; cluster_size];
    let resize_third_cluster_bytes = vec![b'C'; cluster_size];
    const DIRECT_FILE_CLUSTER: u32 = 50;
    disk.install_root_file_with_cluster_chain(
        ROOT_FILE_ENTRY_INDEX,
        "ConcurrentFile",
        TEST_REGULAR_FILE_CLUSTER,
        cluster_size,
        cluster_size,
        true,
        &[TEST_REGULAR_FILE_CLUSTER],
    );
    disk.write_cluster_prefix(TEST_REGULAR_FILE_CLUSTER, &initial_bytes);
    install_root_file_with_cluster_contents(
        &disk,
        ROOT_SECOND_FILE_ENTRY_INDEX,
        "ResizeFile",
        &[
            TEST_FRAGMENTED_FIRST_CLUSTER,
            TEST_FRAGMENTED_SECOND_CLUSTER,
            TEST_FRAGMENTED_THIRD_CLUSTER,
        ],
        cluster_size * 3,
        cluster_size * 3,
        false,
        &[
            resize_first_cluster_bytes.as_slice(),
            resize_second_cluster_bytes.as_slice(),
            resize_third_cluster_bytes.as_slice(),
        ]
        .concat(),
    );
    disk.install_root_file_with_cluster_chain(
        ROOT_THIRD_FILE_ENTRY_INDEX,
        "DirectFile",
        DIRECT_FILE_CLUSTER,
        cluster_size,
        cluster_size,
        true,
        &[DIRECT_FILE_CLUSTER],
    );
    disk.write_cluster_prefix(DIRECT_FILE_CLUSTER, &initial_bytes);

    let (_fs, root_inode) = mount_root(&disk, None);
    let file_inode = root_inode.lookup("ConcurrentFile").unwrap();
    let resize_inode = root_inode.lookup("ResizeFile").unwrap();
    let direct_inode = root_inode.lookup("DirectFile").unwrap();
    let (block_device, boot_region) = published_lookup_state(&file_inode);

    let observer_ready = Arc::new(AtomicBool::new(false));
    let release_observer = Arc::new(AtomicBool::new(false));
    let append_started = Arc::new(AtomicBool::new(false));
    let append_completed = Arc::new(AtomicBool::new(false));

    let observer_thread = {
        let observer_ready = observer_ready.clone();
        let release_observer = release_observer.clone();
        let file_inode = file_inode.clone();

        ThreadOptions::new(move || {
            let exfat_inode = lookup_exfat_inode(&file_inode);
            let _observer_guard = exfat_inode.admission.read();
            observer_ready.store(true, Ordering::Relaxed);
            while !release_observer.load(Ordering::Relaxed) {
                Thread::yield_now();
            }
        })
        .spawn()
    };

    wait_for_flag(&observer_ready);
    let mut prefix = [0u8; 4];
    assert_eq!(file_inode.read_bytes_at(0, &mut prefix).unwrap(), prefix.len());
    assert_eq!(&prefix, &initial_bytes[..4]);
    assert_eq!(
        lookup_exfat_inode(&file_inode)
            .map_regular_file_logical_offset(&block_device, &boot_region, 0)
            .unwrap(),
        Some(boot_region.cluster_offset(TEST_REGULAR_FILE_CLUSTER).unwrap())
    );

    let append_result = Arc::new(spin::Mutex::new(None));
    {
        let append_started = append_started.clone();
        let append_completed = append_completed.clone();
        let file_inode = file_inode.clone();
        let append_result_for_thread = append_result.clone();

        ThreadOptions::new(move || {
            append_started.store(true, Ordering::Relaxed);
            *append_result_for_thread.lock() =
                Some(write_bytes_append(&file_inode, b"tail").map_err(|error| error.error()));
            append_completed.store(true, Ordering::Relaxed);
        })
        .spawn();
    }

    wait_for_flag(&append_started);
    for _ in 0..64 {
        Thread::yield_now();
    }
    assert!(!append_completed.load(Ordering::Relaxed));

    release_observer.store(true, Ordering::Relaxed);
    observer_thread.join();
    wait_for_flag(&append_completed);
    assert_eq!(*append_result.lock(), Some(Ok(4)));
    let appended_entry_set = root_entry_set(&disk, ROOT_FILE_ENTRY_INDEX);
    let appended_cluster = next_stream_cluster(&disk, &appended_entry_set);
    assert_eq!(file_inode.size(), cluster_size + 4);
    assert_eq!(
        visible_file_bytes(&file_inode),
        [initial_bytes.as_slice(), b"tail"].concat()
    );

    let resize_observer_ready = Arc::new(AtomicBool::new(false));
    let release_resize_observer = Arc::new(AtomicBool::new(false));
    let resize_started = Arc::new(AtomicBool::new(false));
    let resize_completed = Arc::new(AtomicBool::new(false));

    let resize_observer = {
        let resize_observer_ready = resize_observer_ready.clone();
        let release_resize_observer = release_resize_observer.clone();
        let resize_inode = resize_inode.clone();

        ThreadOptions::new(move || {
            let exfat_inode = lookup_exfat_inode(&resize_inode);
            let _observer_guard = exfat_inode.admission.read();
            resize_observer_ready.store(true, Ordering::Relaxed);
            while !release_resize_observer.load(Ordering::Relaxed) {
                Thread::yield_now();
            }
        })
        .spawn()
    };

    wait_for_flag(&resize_observer_ready);
    let mut bytes = vec![0xCC; cluster_size * 3];
    assert_eq!(
        resize_inode.read_bytes_at(0, &mut bytes).unwrap(),
        cluster_size * 3
    );
    assert_eq!(&bytes[..cluster_size], resize_first_cluster_bytes.as_slice());
    assert_eq!(
        &bytes[cluster_size..cluster_size * 2],
        resize_second_cluster_bytes.as_slice()
    );
    assert_eq!(
        &bytes[cluster_size * 2..],
        resize_third_cluster_bytes.as_slice()
    );
    assert_eq!(
        lookup_exfat_inode(&resize_inode)
            .map_regular_file_logical_offset(&block_device, &boot_region, cluster_size)
            .unwrap(),
        Some(boot_region.cluster_offset(TEST_FRAGMENTED_SECOND_CLUSTER).unwrap())
    );

    let resize_result = Arc::new(spin::Mutex::new(None));
    {
        let resize_started = resize_started.clone();
        let resize_completed = resize_completed.clone();
        let resize_inode = resize_inode.clone();
        let resize_result_for_thread = resize_result.clone();

        ThreadOptions::new(move || {
            resize_started.store(true, Ordering::Relaxed);
            *resize_result_for_thread.lock() =
                Some(
                    resize_inode
                        .resize(cluster_size + 2)
                        .map_err(|error| error.error()),
                );
            resize_completed.store(true, Ordering::Relaxed);
        })
        .spawn();
    }

    wait_for_flag(&resize_started);
    for _ in 0..64 {
        Thread::yield_now();
    }
    assert!(!resize_completed.load(Ordering::Relaxed));

    release_resize_observer.store(true, Ordering::Relaxed);
    resize_observer.join();
    wait_for_flag(&resize_completed);
    assert_eq!(*resize_result.lock(), Some(Ok(())));
    assert_eq!(resize_inode.size(), cluster_size + 2);
    assert_eq!(
        visible_file_bytes(&resize_inode),
        [
            resize_first_cluster_bytes.as_slice(),
            &resize_second_cluster_bytes[..2],
        ]
        .concat()
    );
    let mut eof_bytes = [0xDD; 4];
    assert_eq!(
        resize_inode
            .read_bytes_at(cluster_size + 2, &mut eof_bytes)
            .unwrap(),
        0
    );
    assert_eq!(eof_bytes, [0xDD; 4]);

    let ready_count = Arc::new(AtomicUsize::new(0));
    let append_after_shrink_result = Arc::new(spin::Mutex::new(None));
    let append_after_shrink = {
        let ready_count = ready_count.clone();
        let direct_inode = direct_inode.clone();
        let append_after_shrink_result = append_after_shrink_result.clone();

        ThreadOptions::new(move || {
            wait_for_concurrent_start(&ready_count, 2);
            *append_after_shrink_result.lock() =
                Some(write_bytes_append(&direct_inode, b"OK").map_err(|error| error.error()));
        })
        .spawn()
    };
    let direct_probe_result = Arc::new(spin::Mutex::new(None));
    let direct_probe_thread = {
        let ready_count = ready_count.clone();
        let direct_probe_result = direct_probe_result.clone();
        let direct_inode = direct_inode.clone();

        ThreadOptions::new(move || {
            wait_for_concurrent_start(&ready_count, 2);
            let error = direct_inode.write_bytes_direct_at(1, b"BAD!").unwrap_err();
            *direct_probe_result.lock() = Some(error.error());
        })
        .spawn()
    };

    append_after_shrink.join();
    direct_probe_thread.join();

    assert_eq!(*append_after_shrink_result.lock(), Some(Ok(2)));
    assert_eq!(*direct_probe_result.lock(), Some(Errno::EINVAL));
    assert_eq!(direct_inode.size(), cluster_size + 2);
    let final_entry_set = root_entry_set(&disk, ROOT_THIRD_FILE_ENTRY_INDEX);
    let final_appended_cluster = next_stream_cluster(&disk, &final_entry_set);
    assert_eq!(
        visible_file_bytes(&direct_inode),
        [vec![b'A'; cluster_size].as_slice(), b"OK"].concat()
    );
    assert_eq!(
        lookup_exfat_inode(&direct_inode)
            .map_regular_file_logical_offset(&block_device, &boot_region, cluster_size)
            .unwrap(),
        Some(boot_region.cluster_offset(final_appended_cluster).unwrap())
    );
}
