// SPDX-License-Identifier: MPL-2.0

use core::sync::atomic::{AtomicBool, Ordering};

use super::{
    assert_observed_bios, collect_dirents, entry_names, init_lookup_test_runtime,
    install_root_file_with_cluster_contents, lookup_exfat_inode, mount_root,
    mount_root_from_block_device, patterned_bytes, published_lookup_state, published_page_count,
    read_cache_page_bytes, wait_for_flag, Arc, BioType, BlockDevice, BootRegion, CachePage,
    ExfatInodeStream, ExfatLookupTestDisk, ExfatLookupToggleFailingReadDisk, FsFlags, Inode,
    InodeType, PageState, Thread, ThreadOptions, Vec, ROOT_FILE_ENTRY_INDEX,
    ROOT_SECOND_FILE_ENTRY_INDEX, ROOT_THIRD_FILE_ENTRY_INDEX, TEST_CONTIGUOUS_SECOND_CLUSTER,
    TEST_FRAGMENTED_FIRST_CLUSTER, TEST_FRAGMENTED_SECOND_CLUSTER, TEST_REGULAR_FILE_CLUSTER,
};

const PARTIAL_FILE_FIRST_CLUSTER: u32 = 24;
const PARTIAL_FILE_SECOND_CLUSTER: u32 = 25;
const PARTIAL_FILE_ENTRY_INDEX: usize = ROOT_THIRD_FILE_ENTRY_INDEX;
const SIDECAR_DIRECTORY_CLUSTER: u32 = 26;
const SIDECAR_DIRECTORY_ENTRY_INDEX: usize = ROOT_THIRD_FILE_ENTRY_INDEX + 3;

fn patterned_bytes_with_seed(len: usize, seed: u8) -> Vec<u8> {
    patterned_bytes(len)
        .into_iter()
        .map(|byte| byte.wrapping_add(seed))
        .collect()
}

fn expected_disk_offset(
    boot_region: &BootRegion,
    clusters: &[u32],
    cluster_size: usize,
    offset: usize,
) -> usize {
    let cluster = clusters[offset / cluster_size];
    boot_region.cluster_offset(cluster).unwrap() + offset % cluster_size
}

fn regular_file_state(inode: &Arc<dyn Inode>) -> (ExfatInodeStream, usize, usize) {
    let exfat_inode = lookup_exfat_inode(inode);
    (
        *exfat_inode.stream.read(),
        inode.size(),
        published_page_count(inode),
    )
}

fn assert_same_regular_file_state(
    before: (ExfatInodeStream, usize, usize),
    after: (ExfatInodeStream, usize, usize),
) {
    assert!(before.0 == after.0);
    assert_eq!(before.1, after.1);
    assert_eq!(before.2, after.2);
}

fn second_cluster_page_idx(cluster_size: usize) -> usize {
    cluster_size / PAGE_SIZE
}

pub(super) fn file_content_mapping_cached_io_integration_success_path_coheres_read_mapping_and_page_cache(
) {
    init_lookup_test_runtime();

    let disk = ExfatLookupTestDisk::new();
    let cluster_size = disk.root_cluster_size();

    let contiguous_len = cluster_size + SECTOR_SIZE;
    let contiguous_clusters = vec![TEST_REGULAR_FILE_CLUSTER, TEST_CONTIGUOUS_SECOND_CLUSTER];
    let contiguous_bytes = patterned_bytes_with_seed(contiguous_len, 0x11);
    install_root_file_with_cluster_contents(
        &disk,
        ROOT_FILE_ENTRY_INDEX,
        "ContiguousFile",
        &contiguous_clusters,
        contiguous_len,
        contiguous_len,
        true,
        &contiguous_bytes,
    );

    let fragmented_len = PAGE_SIZE + cluster_size;
    let fragmented_cluster_count = fragmented_len.div_ceil(cluster_size);
    let fragmented_clusters: Vec<u32> = (0..fragmented_cluster_count)
        .map(|index| TEST_FRAGMENTED_FIRST_CLUSTER + u32::try_from(index * 3).unwrap())
        .collect();
    let fragmented_bytes = patterned_bytes_with_seed(fragmented_len, 0x37);
    install_root_file_with_cluster_contents(
        &disk,
        ROOT_SECOND_FILE_ENTRY_INDEX,
        "FragmentedFile",
        &fragmented_clusters,
        fragmented_len,
        fragmented_len,
        false,
        &fragmented_bytes,
    );

    let partial_len = cluster_size + SECTOR_SIZE;
    let partial_valid_len = SECTOR_SIZE;
    let mut partial_bytes = vec![0xD3; partial_len];
    let partial_prefix = patterned_bytes_with_seed(partial_valid_len, 0x59);
    partial_bytes[..partial_valid_len].copy_from_slice(&partial_prefix);
    install_root_file_with_cluster_contents(
        &disk,
        PARTIAL_FILE_ENTRY_INDEX,
        "PartialFile",
        &[PARTIAL_FILE_FIRST_CLUSTER, PARTIAL_FILE_SECOND_CLUSTER],
        partial_len,
        partial_valid_len,
        true,
        &partial_bytes,
    );

    disk.install_root_directory(
        SIDECAR_DIRECTORY_ENTRY_INDEX,
        "DirSidecar",
        SIDECAR_DIRECTORY_CLUSTER,
    );

    let (_fs, root_inode) = mount_root(&disk, None);
    let (_visible_count, root_entries) = collect_dirents(&root_inode, 2);
    let sidecar_directory = root_inode.lookup("DirSidecar").unwrap();

    assert_eq!(
        entry_names(&root_entries),
        vec![
            "ContiguousFile",
            "FragmentedFile",
            "PartialFile",
            "DirSidecar"
        ]
    );
    assert_eq!(sidecar_directory.type_(), InodeType::Dir);

    let contiguous_inode = root_inode.lookup("contiguousfile").unwrap();
    let contiguous_state_before = regular_file_state(&contiguous_inode);
    let contiguous_exfat_inode = lookup_exfat_inode(&contiguous_inode);
    let (contiguous_block_device, contiguous_boot_region) =
        published_lookup_state(&contiguous_inode);
    let mut contiguous_buffer = vec![0; contiguous_len];

    let contiguous_read_len = contiguous_inode
        .read_bytes_at(0, &mut contiguous_buffer)
        .unwrap();
    let contiguous_second_offset = cluster_size;
    let contiguous_mapping = contiguous_exfat_inode
        .map_regular_file_logical_offset(
            &contiguous_block_device,
            &contiguous_boot_region,
            contiguous_second_offset,
        )
        .unwrap();
    let contiguous_page = CachePage::alloc_uninit().unwrap();
    let contiguous_waiter = contiguous_exfat_inode
        .read_page_async(0, &contiguous_page)
        .unwrap();

    assert_eq!(contiguous_read_len, contiguous_len);
    assert_eq!(contiguous_buffer, contiguous_bytes);
    assert_eq!(
        contiguous_mapping,
        Some(expected_disk_offset(
            &contiguous_boot_region,
            &contiguous_clusters,
            cluster_size,
            contiguous_second_offset,
        ))
    );
    assert_eq!(contiguous_waiter.wait(), Some(BioStatus::Complete));
    assert_eq!(
        &read_cache_page_bytes(&contiguous_page)[..PAGE_SIZE.min(contiguous_bytes.len())],
        &contiguous_bytes[..PAGE_SIZE.min(contiguous_bytes.len())]
    );
    assert_same_regular_file_state(
        contiguous_state_before,
        regular_file_state(&contiguous_inode),
    );

    let fragmented_inode = root_inode.lookup("FragmentedFile").unwrap();
    let fragmented_state_before = regular_file_state(&fragmented_inode);
    let fragmented_exfat_inode = lookup_exfat_inode(&fragmented_inode);
    let (fragmented_block_device, fragmented_boot_region) =
        published_lookup_state(&fragmented_inode);
    let fragmented_probe_offset = PAGE_SIZE;
    let mut fragmented_buffer = vec![0; fragmented_len];
    let fragmented_read_len = fragmented_inode
        .read_bytes_at(0, &mut fragmented_buffer)
        .unwrap();
    let fragmented_mapping = fragmented_exfat_inode
        .map_regular_file_logical_offset(
            &fragmented_block_device,
            &fragmented_boot_region,
            fragmented_probe_offset,
        )
        .unwrap();
    let fragmented_write_page_idx = fragmented_probe_offset / PAGE_SIZE;
    let fragmented_page = CachePage::alloc_zero(PageState::UpToDate).unwrap();
    let fragmented_page_bytes = patterned_bytes_with_seed(PAGE_SIZE, 0x7A);
    fragmented_page
        .write_bytes(0, &fragmented_page_bytes)
        .unwrap();
    let _ = disk.take_observed_bios();
    let fragmented_waiter = fragmented_exfat_inode
        .write_page_async(fragmented_write_page_idx, &fragmented_page)
        .unwrap();

    assert_eq!(fragmented_read_len, fragmented_len);
    assert_eq!(fragmented_buffer, fragmented_bytes);
    assert_eq!(
        fragmented_mapping,
        Some(expected_disk_offset(
            &fragmented_boot_region,
            &fragmented_clusters,
            cluster_size,
            fragmented_probe_offset,
        ))
    );
    assert_eq!(fragmented_waiter.wait(), Some(BioStatus::Complete));
    assert!(!disk.take_observed_bios().is_empty());
    assert_same_regular_file_state(
        fragmented_state_before,
        regular_file_state(&fragmented_inode),
    );

    let partial_inode = root_inode.lookup("PartialFile").unwrap();
    let partial_state_before = regular_file_state(&partial_inode);
    let partial_exfat_inode = lookup_exfat_inode(&partial_inode);
    let mut partial_buffer = vec![0xEE; partial_len];
    let partial_read_len = partial_inode.read_bytes_at(0, &mut partial_buffer).unwrap();
    let partial_page = CachePage::alloc_uninit().unwrap();
    let partial_waiter = partial_exfat_inode
        .read_page_async(0, &partial_page)
        .unwrap();
    let partial_page_bytes = read_cache_page_bytes(&partial_page);

    assert_eq!(partial_read_len, partial_len);
    assert_eq!(
        &partial_buffer[..partial_valid_len],
        partial_prefix.as_slice()
    );
    assert!(partial_buffer[partial_valid_len..]
        .iter()
        .all(|byte| *byte == 0));
    assert_eq!(partial_waiter.wait(), Some(BioStatus::Complete));
    assert_eq!(
        &partial_page_bytes[..partial_valid_len],
        partial_prefix.as_slice()
    );
    assert!(partial_page_bytes[partial_valid_len..]
        .iter()
        .all(|byte| *byte == 0));
    assert_eq!(
        published_page_count(&partial_inode),
        partial_len.div_ceil(PAGE_SIZE)
    );
    assert_same_regular_file_state(partial_state_before, regular_file_state(&partial_inode));
}

pub(super) fn file_content_mapping_cached_io_integration_failure_maintenance_preserves_stream_state_and_page_visibility(
) {
    init_lookup_test_runtime();

    let broken_disk = ExfatLookupTestDisk::new();
    let cluster_size = broken_disk.root_cluster_size();
    let broken_len = cluster_size + SECTOR_SIZE;
    let broken_clusters = [
        TEST_FRAGMENTED_FIRST_CLUSTER,
        TEST_FRAGMENTED_SECOND_CLUSTER,
    ];
    let broken_bytes = patterned_bytes_with_seed(broken_len, 0x23);
    install_root_file_with_cluster_contents(
        &broken_disk,
        ROOT_FILE_ENTRY_INDEX,
        "BrokenChain",
        &broken_clusters,
        broken_len,
        broken_len,
        false,
        &broken_bytes,
    );
    broken_disk.terminate_fat_chain(broken_clusters[0]);

    let (_fs, broken_root) = mount_root(&broken_disk, None);
    let broken_inode = broken_root.lookup("BrokenChain").unwrap();
    let broken_state_before = regular_file_state(&broken_inode);
    let broken_exfat_inode = lookup_exfat_inode(&broken_inode);
    let (broken_block_device, broken_boot_region) = published_lookup_state(&broken_inode);
    let broken_probe_offset = cluster_size;
    let mut broken_tail = [0u8; SECTOR_SIZE];
    let broken_mapping_error = broken_exfat_inode
        .map_regular_file_logical_offset(
            &broken_block_device,
            &broken_boot_region,
            broken_probe_offset,
        )
        .unwrap_err();
    let broken_read_error = broken_inode
        .read_bytes_at(broken_probe_offset, &mut broken_tail)
        .unwrap_err();
    let broken_page = CachePage::alloc_uninit().unwrap();
    let broken_page_error = broken_exfat_inode
        .read_page_async(second_cluster_page_idx(cluster_size), &broken_page)
        .unwrap_err();
    let mut intact_prefix = [0u8; SECTOR_SIZE];
    let intact_len = broken_inode.read_bytes_at(0, &mut intact_prefix).unwrap();

    assert_eq!(broken_mapping_error.error(), Errno::EIO);
    assert_eq!(broken_read_error.error(), Errno::EIO);
    assert_eq!(broken_page_error.error(), Errno::EIO);
    assert_eq!(intact_len, SECTOR_SIZE);
    assert_eq!(intact_prefix.as_slice(), &broken_bytes[..SECTOR_SIZE]);
    assert_eq!(
        broken_exfat_inode
            .map_regular_file_logical_offset(&broken_block_device, &broken_boot_region, 0)
            .unwrap(),
        Some(expected_disk_offset(
            &broken_boot_region,
            &broken_clusters,
            cluster_size,
            0,
        ))
    );
    assert_same_regular_file_state(broken_state_before, regular_file_state(&broken_inode));

    let failing_disk = ExfatLookupTestDisk::new();
    let failing_bytes = patterned_bytes_with_seed(SECTOR_SIZE, 0x41);
    failing_disk.install_root_file_with_contents(
        ROOT_FILE_ENTRY_INDEX,
        "FailingPage",
        TEST_REGULAR_FILE_CLUSTER,
        &failing_bytes,
    );
    let failing_device = ExfatLookupToggleFailingReadDisk::new(
        failing_disk.clone(),
        failing_disk.cluster_offset(TEST_REGULAR_FILE_CLUSTER),
        SECTOR_SIZE,
    );
    let failing_block_device: Arc<dyn BlockDevice> = failing_device.clone();
    let (_fs, failing_root) =
        mount_root_from_block_device(failing_block_device, FsFlags::empty(), None);
    let failing_inode = failing_root.lookup("FailingPage").unwrap();
    let failing_state_before = regular_file_state(&failing_inode);
    let failing_exfat_inode = lookup_exfat_inode(&failing_inode);
    let failing_page = CachePage::alloc_uninit().unwrap();

    failing_device.enable_failures();
    let failing_waiter = failing_exfat_inode
        .read_page_async(0, &failing_page)
        .unwrap();

    assert!(failing_waiter.wait().is_none());
    assert_eq!(failing_waiter.status(0), BioStatus::IoError);
    assert_eq!(failing_page.load_state(), PageState::Uninit);
    assert_same_regular_file_state(failing_state_before, regular_file_state(&failing_inode));
}

pub(super) fn file_content_mapping_cached_io_integration_repeated_calls_stay_stable_across_cache_and_mapping(
) {
    init_lookup_test_runtime();

    let disk = ExfatLookupTestDisk::new();
    let cluster_size = disk.root_cluster_size();
    let repeated_len = PAGE_SIZE + SECTOR_SIZE;
    let repeated_cluster_count = repeated_len.div_ceil(cluster_size);
    let repeated_clusters: Vec<u32> = (0..repeated_cluster_count)
        .map(|index| TEST_REGULAR_FILE_CLUSTER + u32::try_from(index).unwrap())
        .collect();
    let repeated_bytes = patterned_bytes_with_seed(repeated_len, 0x6C);
    install_root_file_with_cluster_contents(
        &disk,
        ROOT_FILE_ENTRY_INDEX,
        "RepeatedFile",
        &repeated_clusters,
        repeated_len,
        repeated_len,
        true,
        &repeated_bytes,
    );

    let (_fs, root_inode) = mount_root(&disk, None);
    let repeated_inode = root_inode.lookup("RepeatedFile").unwrap();
    let repeated_state_before = regular_file_state(&repeated_inode);
    let repeated_exfat_inode = lookup_exfat_inode(&repeated_inode);
    let (repeated_block_device, repeated_boot_region) = published_lookup_state(&repeated_inode);
    let repeated_probe_offset = PAGE_SIZE;
    let mut first_read = vec![0; repeated_len];
    let mut second_read = vec![0; repeated_len];
    let page_cache = repeated_inode.page_cache().unwrap();
    let cached_len = PAGE_SIZE.min(repeated_len);
    let mut first_cached = vec![0; cached_len];
    let mut second_cached = vec![0; cached_len];

    let first_read_len = repeated_inode.read_bytes_at(0, &mut first_read).unwrap();
    let first_mapping = repeated_exfat_inode
        .map_regular_file_logical_offset(
            &repeated_block_device,
            &repeated_boot_region,
            repeated_probe_offset,
        )
        .unwrap();
    let _ = disk.take_observed_bios();
    page_cache.read_bytes(0, &mut first_cached).unwrap();
    let first_cached_bios = disk.take_observed_bios();

    let second_read_len = repeated_inode.read_bytes_at(0, &mut second_read).unwrap();
    let second_mapping = repeated_exfat_inode
        .map_regular_file_logical_offset(
            &repeated_block_device,
            &repeated_boot_region,
            repeated_probe_offset,
        )
        .unwrap();
    page_cache.read_bytes(0, &mut second_cached).unwrap();
    let second_cached_bios = disk.take_observed_bios();

    assert_eq!(first_read_len, repeated_len);
    assert_eq!(second_read_len, repeated_len);
    assert_eq!(first_read, repeated_bytes);
    assert_eq!(second_read, repeated_bytes);
    assert_eq!(
        first_mapping,
        Some(expected_disk_offset(
            &repeated_boot_region,
            &repeated_clusters,
            cluster_size,
            repeated_probe_offset,
        ))
    );
    assert_eq!(second_mapping, first_mapping);
    assert_eq!(first_cached, repeated_bytes[..cached_len]);
    assert_eq!(second_cached, first_cached);
    assert!(!first_cached_bios.is_empty());
    assert!(second_cached_bios
        .iter()
        .all(|observed_bio| observed_bio.type_ == BioType::Read));
    assert_same_regular_file_state(repeated_state_before, regular_file_state(&repeated_inode));
}

pub(super) fn file_content_mapping_cached_io_integration_concurrency_serializes_mapping_against_truncate_boundary(
) {
    init_lookup_test_runtime();

    let disk = ExfatLookupTestDisk::new();
    let cluster_size = disk.root_cluster_size();
    let serialized_len = PAGE_SIZE + cluster_size;
    let serialized_cluster_count = serialized_len.div_ceil(cluster_size);
    let serialized_clusters: Vec<u32> = (0..serialized_cluster_count)
        .map(|index| TEST_REGULAR_FILE_CLUSTER + u32::try_from(index).unwrap())
        .collect();
    let serialized_bytes = patterned_bytes_with_seed(serialized_len, 0x4E);
    install_root_file_with_cluster_contents(
        &disk,
        ROOT_FILE_ENTRY_INDEX,
        "SerializedFile",
        &serialized_clusters,
        serialized_len,
        serialized_len,
        true,
        &serialized_bytes,
    );

    let (_fs, root_inode) = mount_root(&disk, None);
    let serialized_inode = root_inode.lookup("SerializedFile").unwrap();
    let serialized_state_before = regular_file_state(&serialized_inode);
    let (serialized_block_device, serialized_boot_region) =
        published_lookup_state(&serialized_inode);
    let probe_offset = cluster_size;
    let expected_mapping = Some(expected_disk_offset(
        &serialized_boot_region,
        &serialized_clusters,
        cluster_size,
        probe_offset,
    ));

    let read_boundary_ready = Arc::new(AtomicBool::new(false));
    let release_read_boundary = Arc::new(AtomicBool::new(false));
    let writer_started = Arc::new(AtomicBool::new(false));
    let writer_acquired = Arc::new(AtomicBool::new(false));

    let read_holder = {
        let read_boundary_ready = read_boundary_ready.clone();
        let release_read_boundary = release_read_boundary.clone();
        let serialized_inode = serialized_inode.clone();
        let serialized_block_device = serialized_block_device.clone();

        ThreadOptions::new(move || {
            let exfat_inode = lookup_exfat_inode(&serialized_inode);
            let _mapping_guard = exfat_inode.admission.read();
            assert_eq!(
                exfat_inode
                    .map_regular_file_logical_offset(
                        &serialized_block_device,
                        &serialized_boot_region,
                        probe_offset,
                    )
                    .unwrap(),
                expected_mapping
            );
            let page = CachePage::alloc_uninit().unwrap();
            let waiter = exfat_inode.read_page_async(0, &page).unwrap();
            assert_eq!(waiter.wait(), Some(BioStatus::Complete));
            read_boundary_ready.store(true, Ordering::Relaxed);
            while !release_read_boundary.load(Ordering::Relaxed) {
                Thread::yield_now();
            }
        })
        .spawn()
    };

    wait_for_flag(&read_boundary_ready);

    let writer_thread = {
        let writer_started = writer_started.clone();
        let writer_acquired = writer_acquired.clone();
        let serialized_inode = serialized_inode.clone();

        ThreadOptions::new(move || {
            let exfat_inode = lookup_exfat_inode(&serialized_inode);
            writer_started.store(true, Ordering::Relaxed);
            let _mutation_guard = exfat_inode.admission.write();
            writer_acquired.store(true, Ordering::Relaxed);
        })
        .spawn()
    };

    wait_for_flag(&writer_started);
    for _ in 0..64 {
        Thread::yield_now();
    }
    assert!(!writer_acquired.load(Ordering::Relaxed));

    release_read_boundary.store(true, Ordering::Relaxed);
    read_holder.join();
    writer_thread.join();
    assert!(writer_acquired.load(Ordering::Relaxed));

    let mutation_guard = lookup_exfat_inode(&serialized_inode).admission.write();
    let mapping_started = Arc::new(AtomicBool::new(false));
    let mapping_completed = Arc::new(AtomicBool::new(false));
    let mapping_result = Arc::new(Mutex::new(None));
    let page_result = Arc::new(Mutex::new(None));

    let mapping_thread = {
        let mapping_started = mapping_started.clone();
        let mapping_completed = mapping_completed.clone();
        let mapping_result = mapping_result.clone();
        let page_result = page_result.clone();
        let serialized_inode = serialized_inode.clone();
        let serialized_block_device = serialized_block_device.clone();

        ThreadOptions::new(move || {
            let exfat_inode = lookup_exfat_inode(&serialized_inode);
            mapping_started.store(true, Ordering::Relaxed);
            *mapping_result.lock() = Some(
                exfat_inode
                    .map_regular_file_logical_offset(
                        &serialized_block_device,
                        &serialized_boot_region,
                        probe_offset,
                    )
                    .map_err(|error| error.error()),
            );
            let page = CachePage::alloc_uninit().unwrap();
            *page_result.lock() = Some(
                exfat_inode
                    .read_page_async(0, &page)
                    .map(|waiter| waiter.wait())
                    .map_err(|error| error.error()),
            );
            mapping_completed.store(true, Ordering::Relaxed);
        })
        .spawn()
    };

    wait_for_flag(&mapping_started);
    for _ in 0..64 {
        Thread::yield_now();
    }
    assert!(!mapping_completed.load(Ordering::Relaxed));

    drop(mutation_guard);
    mapping_thread.join();

    assert_eq!(*mapping_result.lock(), Some(Ok(expected_mapping)));
    assert_eq!(*page_result.lock(), Some(Ok(Some(BioStatus::Complete))));
    assert_same_regular_file_state(
        serialized_state_before,
        regular_file_state(&serialized_inode),
    );
}
