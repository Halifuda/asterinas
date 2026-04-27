// SPDX-License-Identifier: MPL-2.0

use super::*;

fn wait_for_blocked_flush(flush_control_disk: &ExfatLookupFlushControlDisk) {
    while !flush_control_disk.flush_started() {
        Thread::yield_now();
    }
}

fn assert_metadata_only_device_sync(observed_bios: &[ObservedBio]) {
    assert!(
        !observed_bios.is_empty(),
        "expected metadata-only device sync BIOs, got none"
    );
    assert_eq!(
        observed_bios.last().map(|bio| bio.type_),
        Some(BioType::Flush),
        "expected metadata-only device sync to end with a flush BIO, got {observed_bios:?}"
    );
    assert!(
        observed_bios.iter().all(|bio| bio.type_ != BioType::Write),
        "expected metadata-only device sync without write BIOs, got {observed_bios:?}"
    );
}

pub(super) fn file_sync_and_persistence_integration_success_path_sync_data_then_sync_all_preserve_ordering_and_scope_boundary()
 {
    init_lookup_test_runtime();

    let disk = ExfatLookupTestDisk::new();
    disk.install_root_file_with_contents(
        ROOT_FILE_ENTRY_INDEX,
        "IntegratedSync",
        TEST_REGULAR_FILE_CLUSTER,
        b"abcdefgh",
    );

    let (_fs, root_inode) = mount_root(&disk, None);
    let file_inode = root_inode.lookup("IntegratedSync").unwrap();
    let expected_bytes = b"ABCDefghTAIL";

    assert_eq!(file_inode.write_bytes_at(8, b"TAIL").unwrap(), 4);
    dirty_regular_file_first_page(&file_inode, expected_bytes);
    let _ = disk.take_observed_bios();

    file_inode.sync_data().unwrap();

    let sync_data_bios = disk.take_observed_bios();
    assert_sync_writeback_before_device_sync(&sync_data_bios);
    assert_eq!(file_inode.size(), expected_bytes.len());
    assert_eq!(
        stream_lengths(&root_entry_set(&disk, ROOT_FILE_ENTRY_INDEX)),
        (expected_bytes.len() as u64, expected_bytes.len() as u64)
    );
    assert_eq!(
        &disk.read_cluster(TEST_REGULAR_FILE_CLUSTER)[..expected_bytes.len()],
        expected_bytes
    );

    file_inode.sync_all().unwrap();
    assert_metadata_only_device_sync(&disk.take_observed_bios());

    file_inode.sync_all().unwrap();
    assert!(disk.take_observed_bios().is_empty());
}

pub(super) fn file_sync_and_persistence_integration_failure_maintenance_device_stage_retry_preserves_dirty_window()
 {
    init_lookup_test_runtime();

    let disk = ExfatLookupTestDisk::new();
    disk.install_root_file_with_contents(
        ROOT_FILE_ENTRY_INDEX,
        "RetryingIntegration",
        TEST_REGULAR_FILE_CLUSTER,
        b"abcdefgh",
    );
    let flush_control_disk = ExfatLookupFlushControlDisk::new(disk.clone());
    let block_device: Arc<dyn BlockDevice> = flush_control_disk.clone();
    let (_fs, root_inode) = mount_root_from_block_device(block_device, FsFlags::empty(), None);
    let file_inode = root_inode.lookup("RetryingIntegration").unwrap();
    let expected_bytes = b"ABCDefghTAIL";

    assert_eq!(file_inode.write_bytes_at(8, b"TAIL").unwrap(), 4);
    dirty_regular_file_first_page(&file_inode, expected_bytes);
    let _ = disk.take_observed_bios();

    flush_control_disk.enable_blocking_flush();
    let sync_result = Arc::new(Mutex::new(None));
    let sync_thread = {
        let file_inode = file_inode.clone();
        let sync_result = sync_result.clone();
        ThreadOptions::new(move || {
            *sync_result.lock() = Some(file_inode.sync_data().map_err(|error| error.error()));
        })
        .spawn()
    };

    wait_for_blocked_flush(&flush_control_disk);
    flush_control_disk.enable_flush_failures();
    flush_control_disk.release_blocked_flush();
    sync_thread.join();

    assert_eq!(*sync_result.lock(), Some(Err(Errno::EIO)));
    let failed_bios = disk.take_observed_bios();
    assert_sync_writeback_before_device_sync(&failed_bios);
    assert_eq!(
        &disk.read_cluster(TEST_REGULAR_FILE_CLUSTER)[..expected_bytes.len()],
        expected_bytes
    );

    flush_control_disk.disable_flush_failures();
    file_inode.sync_data().unwrap();
    assert_metadata_only_device_sync(&disk.take_observed_bios());

    file_inode.sync_data().unwrap();
    assert!(disk.take_observed_bios().is_empty());

    file_inode.sync_all().unwrap();
    assert_metadata_only_device_sync(&disk.take_observed_bios());

    file_inode.sync_all().unwrap();
    assert!(disk.take_observed_bios().is_empty());
}

pub(super) fn file_sync_and_persistence_integration_repeated_calls_preserve_clean_stability_and_metadata_boundary()
 {
    file_sync_and_persistence_revalidation_metadata_scope_and_failure_maintenance_repeated_clean_sync_calls_do_not_manufacture_dirty_state();
}

pub(super) fn file_sync_and_persistence_integration_concurrency_blocked_sync_revalidates_later_dirty_work()
 {
    file_sync_and_persistence_revalidation_metadata_scope_and_failure_maintenance_later_dirty_work_remains_outstanding_after_blocked_sync_success();
}
