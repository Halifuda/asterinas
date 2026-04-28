// SPDX-License-Identifier: MPL-2.0

use alloc::{ffi::CString, format, string::String, sync::Arc, vec, vec::Vec};
use core::sync::atomic::{AtomicUsize, Ordering};

use aster_block::{
    BlockDevice, SECTOR_SIZE,
    bio::{BioStatus, BioType},
};
use ostd::{
    mm::{PAGE_SIZE, VmIo, VmReader},
    prelude::ktest,
};

use super::{
    super::{
        direntry::entry_set_checksum,
        fs::{ExfatFs, ExfatFsType},
        test_support::{
            disk::{
                ExfatLookupFlushControlDisk, ExfatLookupTestDisk, ExfatLookupToggleFailingReadDisk,
                ExfatLookupToggleFailingWriteDisk, ExfatLookupWriteControlDisk, ObservedBio,
            },
            inode_fixtures::{
                CapturedDirent, RejectingDirentVisitor, assert_flush_only,
                assert_metadata_unchanged, assert_observed_bios,
                assert_sync_writeback_before_device_sync, collect_dirents, decode_entry_name,
                dirty_regular_file_first_page, entry_index_from_ino, entry_names, entry_offsets,
                init_lookup_test_runtime, lookup_error, lookup_exfat_inode, mount_root,
                mount_root_from_block_device, mount_root_with_flags, next_stream_cluster,
                patterned_bytes, published_lookup_state, published_page_count,
                read_cache_page_bytes, root_entry_set, stream_first_cluster,
                stream_has_no_fat_chain, stream_lengths, visible_name_count,
                wait_for_concurrent_start, write_bytes_append,
            },
            integration_fixtures::{
                install_root_file_with_cluster_contents, wait_for_blocked_flush, wait_for_flag,
            },
            metadata_helpers::{
                assert_bytes_unchanged_except, assert_valid_entry_set_checksum,
                set_directory_entry_metadata, set_regular_file_entry_metadata,
            },
            timestamp::{
                encode_exfat_date, encode_exfat_date_only, encode_exfat_date_time,
                encode_valid_utc_offset_byte, expected_timestamp,
            },
        },
    },
    *,
};
use crate::{
    fs::{
        file::StatusFlags,
        vfs::{
            file_system::{FileSystem, FsFlags},
            inode::FallocMode,
            page_cache::{CachePage, CachePageExt, PageCacheBackend, PageState},
            registry::FsType,
        },
    },
    thread::{Thread, kernel_thread::ThreadOptions},
    vm::vmo::CommitFlags,
};

const DIRECTORY_ENTRY_SIZE: usize = 32;
const FILE_ATTRIBUTE_DIRECTORY: u16 = 0x0010;
const FILE_ATTRIBUTE_REGULAR: u16 = 0x0020;
const FILE_DIRECTORY_ENTRY_TYPE: u8 = 0x85;
const FILE_NAME_ENTRY_TYPE: u8 = 0xC1;
const FAT_END_OF_CHAIN: u32 = 0xFFFF_FFFF;
const ROOT_FILE_ENTRY_INDEX: usize = 4;
const ROOT_SECOND_FILE_ENTRY_INDEX: usize = ROOT_FILE_ENTRY_INDEX + 3;
const ROOT_THIRD_FILE_ENTRY_INDEX: usize = ROOT_FILE_ENTRY_INDEX + 6;
const STREAM_EXTENSION_ENTRY_TYPE: u8 = 0xC0;
const STREAM_FIRST_CLUSTER_OFFSET: usize = DIRECTORY_ENTRY_SIZE + 20;
const STREAM_GENERAL_FLAGS_OFFSET: usize = DIRECTORY_ENTRY_SIZE + 1;
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
const TEST_CONTIGUOUS_SECOND_CLUSTER: u32 = TEST_REGULAR_FILE_CLUSTER + 1;
const TEST_FRAGMENTED_FIRST_CLUSTER: u32 = 18;
const TEST_FRAGMENTED_SECOND_CLUSTER: u32 = 21;
const TEST_FRAGMENTED_THIRD_CLUSTER: u32 = 22;
const FILE_ATTRIBUTES_OFFSET: usize = 4;
const STREAM_DATA_LENGTH_OFFSET: usize = DIRECTORY_ENTRY_SIZE + 24;
const STREAM_VALID_DATA_LENGTH_OFFSET: usize = DIRECTORY_ENTRY_SIZE + 8;

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
    assert_eq!(
        entry_set[DIRECTORY_ENTRY_SIZE],
        STREAM_EXTENSION_ENTRY_TYPE & !0x80
    );
    assert_eq!(
        entry_set[DIRECTORY_ENTRY_SIZE * 2],
        FILE_NAME_ENTRY_TYPE & !0x80
    );
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

#[path = "lookup_resolution.rs"]
mod lookup_resolution;

#[path = "readdir_visibility.rs"]
mod readdir_visibility;

#[path = "lookup_integration.rs"]
mod lookup_integration;

#[path = "file_meta_projection.rs"]
mod file_meta_projection;

#[path = "dir_meta_projection.rs"]
mod dir_meta_projection;

#[path = "file_meta_timestamp.rs"]
mod file_meta_timestamp;

#[path = "dir_meta_timestamp.rs"]
mod dir_meta_timestamp;

#[path = "dir_meta_refresh.rs"]
mod dir_meta_refresh;

#[path = "dir_meta_integration.rs"]
mod dir_meta_integration;

#[path = "file_meta_integration.rs"]
mod file_meta_integration;

#[path = "cached_io_integration.rs"]
mod cached_io_integration;

#[path = "file_mutation_integration.rs"]
mod file_mutation_integration;

#[path = "file_sync_integration.rs"]
mod file_sync_integration;

#[path = "entry_field_update.rs"]
mod entry_field_update;

#[ktest]
fn directory_lookup_and_identity_integration_success_path_coheres_lookup_and_readdir() {
    lookup_integration::directory_lookup_and_identity_integration_success_path_coheres_lookup_and_readdir();
}

#[ktest]
fn directory_lookup_and_identity_integration_failure_path_preserves_typed_boundaries() {
    lookup_integration::directory_lookup_and_identity_integration_failure_path_preserves_typed_boundaries();
}

#[ktest]
fn directory_lookup_and_identity_integration_repeated_calls_stay_stable() {
    lookup_integration::directory_lookup_and_identity_integration_repeated_calls_stay_stable();
}

#[ktest]
fn file_content_mapping_cached_io_integration_success_path_coheres_read_mapping_and_page_cache() {
    cached_io_integration::file_content_mapping_cached_io_integration_success_path_coheres_read_mapping_and_page_cache();
}

#[ktest]
fn file_metadata_projection_and_update_projection_substrate_projects_regular_file_snapshot_from_entry_set_and_stream_state()
 {
    file_meta_projection::file_metadata_projection_and_update_projection_substrate_projects_regular_file_snapshot_from_entry_set_and_stream_state();
}

#[ktest]
fn file_metadata_projection_and_update_projection_substrate_rejects_invalid_timestamp_layout_without_disturbing_neighbor_lookups()
 {
    file_meta_projection::file_metadata_projection_and_update_projection_substrate_rejects_invalid_timestamp_layout_without_disturbing_neighbor_lookups();
}

#[ktest]
fn directory_metadata_projection_and_update_projection_substrate_projects_ordinary_directory_from_validated_self_entry_set()
 {
    dir_meta_projection::directory_metadata_projection_and_update_projection_substrate_projects_ordinary_directory_from_validated_self_entry_set();
}

#[ktest]
fn directory_metadata_projection_and_update_projection_substrate_keeps_root_projection_synthetic_without_self_entry_fabrication()
 {
    dir_meta_projection::directory_metadata_projection_and_update_projection_substrate_keeps_root_projection_synthetic_without_self_entry_fabrication();
}

#[ktest]
fn directory_metadata_projection_and_update_projection_substrate_rejects_broken_ordinary_self_entry_sets_through_result_getters()
 {
    dir_meta_projection::directory_metadata_projection_and_update_projection_substrate_rejects_broken_ordinary_self_entry_sets_through_result_getters();
}

#[ktest]
fn file_metadata_projection_and_update_policy_and_timestamp_mutation_updates_durable_read_only_projection_and_metadata_only_dirty_state()
 {
    file_meta_timestamp::file_metadata_projection_and_update_policy_and_timestamp_mutation_updates_durable_read_only_projection_and_metadata_only_dirty_state();
}

#[ktest]
fn file_metadata_projection_and_update_policy_and_timestamp_mutation_owner_group_confirm_projection_and_refuse_escape()
 {
    file_meta_timestamp::file_metadata_projection_and_update_policy_and_timestamp_mutation_owner_group_confirm_projection_and_refuse_escape();
}

#[ktest]
fn file_metadata_projection_and_update_policy_and_timestamp_mutation_rewrites_only_owned_timestamp_families()
 {
    file_meta_timestamp::file_metadata_projection_and_update_policy_and_timestamp_mutation_rewrites_only_owned_timestamp_families();
}

#[ktest]
fn file_metadata_projection_and_update_policy_and_timestamp_mutation_treats_ctime_as_synthetic_only()
 {
    file_meta_timestamp::file_metadata_projection_and_update_policy_and_timestamp_mutation_treats_ctime_as_synthetic_only();
}

#[ktest]
fn file_metadata_projection_and_update_policy_and_timestamp_mutation_policy_denial_and_io_failure_preserve_last_good_state()
 {
    file_meta_timestamp::file_metadata_projection_and_update_policy_and_timestamp_mutation_policy_denial_and_io_failure_preserve_last_good_state();
}

#[ktest]
fn directory_metadata_projection_and_update_policy_and_timestamp_mutation_updates_only_dos_read_only_for_ordinary_directories()
 {
    dir_meta_timestamp::directory_metadata_projection_and_update_policy_and_timestamp_mutation_updates_only_dos_read_only_for_ordinary_directories();
}

#[ktest]
fn directory_metadata_projection_and_update_policy_and_timestamp_mutation_owner_group_follow_mount_envelope()
 {
    dir_meta_timestamp::directory_metadata_projection_and_update_policy_and_timestamp_mutation_owner_group_follow_mount_envelope();
}

#[ktest]
fn directory_metadata_projection_and_update_policy_and_timestamp_mutation_rewrites_only_directory_timestamp_families()
 {
    dir_meta_timestamp::directory_metadata_projection_and_update_policy_and_timestamp_mutation_rewrites_only_directory_timestamp_families();
}

#[ktest]
fn directory_metadata_projection_and_update_policy_and_timestamp_mutation_root_and_ctime_requests_stay_bounded()
 {
    dir_meta_timestamp::directory_metadata_projection_and_update_policy_and_timestamp_mutation_root_and_ctime_requests_stay_bounded();
}

#[ktest]
fn directory_metadata_projection_and_update_policy_and_timestamp_mutation_denials_and_failures_preserve_last_good_state()
 {
    dir_meta_timestamp::directory_metadata_projection_and_update_policy_and_timestamp_mutation_denials_and_failures_preserve_last_good_state();
}

#[ktest]
fn directory_metadata_projection_and_update_namespace_refresh_create_and_mkdir_refresh_parent_timestamp()
 {
    dir_meta_refresh::directory_metadata_projection_and_update_namespace_refresh_create_and_mkdir_refresh_parent_timestamp();
}

#[ktest]
fn directory_metadata_projection_and_update_namespace_refresh_unlink_and_rmdir_refresh_parent_timestamp()
 {
    dir_meta_refresh::directory_metadata_projection_and_update_namespace_refresh_unlink_and_rmdir_refresh_parent_timestamp();
}

#[ktest]
fn directory_metadata_projection_and_update_namespace_refresh_rename_refreshes_affected_directories()
 {
    dir_meta_refresh::directory_metadata_projection_and_update_namespace_refresh_rename_refreshes_affected_directories();
}

#[ktest]
fn directory_metadata_projection_and_update_namespace_refresh_failure_preserves_last_good_state() {
    dir_meta_refresh::directory_metadata_projection_and_update_namespace_refresh_failure_preserves_last_good_state();
}

#[ktest]
fn directory_metadata_projection_and_update_integration_namespace_mutation_sequence_preserves_projection_and_durable_self_entry_sets()
 {
    dir_meta_integration::directory_metadata_projection_and_update_integration_namespace_mutation_sequence_preserves_projection_and_durable_self_entry_sets();
}

#[ktest]
fn directory_metadata_projection_and_update_integration_failure_maintenance_preserves_last_good_directory_metadata_publication()
 {
    dir_meta_integration::directory_metadata_projection_and_update_integration_failure_maintenance_preserves_last_good_directory_metadata_publication();
}

#[ktest]
fn directory_metadata_projection_and_update_integration_concurrency_observes_only_pre_or_post_projection_views()
 {
    dir_meta_integration::directory_metadata_projection_and_update_integration_concurrency_observes_only_pre_or_post_projection_views();
}

#[ktest]
fn file_metadata_projection_update_integration_success_path_live_and_reread_projection_agree_after_sync()
 {
    file_meta_integration::file_metadata_projection_update_integration_success_path_live_and_reread_projection_agree_after_sync();
}

#[ktest]
fn file_metadata_projection_update_integration_failure_maintenance_preserves_state_and_retry() {
    file_meta_integration::file_metadata_projection_update_integration_failure_maintenance_preserves_state_and_retry();
}

#[ktest]
fn file_metadata_projection_update_integration_repeated_calls_keep_metadata_stable() {
    file_meta_integration::file_metadata_projection_update_integration_repeated_calls_keep_metadata_stable();
}

#[ktest]
fn file_metadata_projection_update_integration_concurrency_serializes_metadata_and_content_updates()
{
    file_meta_integration::file_metadata_projection_update_integration_concurrency_serializes_metadata_and_content_updates();
}

#[ktest]
fn file_content_mapping_cached_io_integration_failure_maintenance_preserves_stream_state_and_page_visibility()
 {
    cached_io_integration::file_content_mapping_cached_io_integration_failure_maintenance_preserves_stream_state_and_page_visibility();
}

#[ktest]
fn file_content_mapping_cached_io_integration_repeated_calls_stay_stable_across_cache_and_mapping()
{
    cached_io_integration::file_content_mapping_cached_io_integration_repeated_calls_stay_stable_across_cache_and_mapping();
}

#[ktest]
fn file_content_mapping_cached_io_integration_concurrency_serializes_mapping_against_truncate_boundary()
 {
    cached_io_integration::file_content_mapping_cached_io_integration_concurrency_serializes_mapping_against_truncate_boundary();
}

#[ktest]
fn file_content_mutation_integration_success_path_write_append_grow_shrink_readback() {
    file_mutation_integration::file_content_mutation_integration_success_path_write_append_grow_shrink_readback();
}

#[ktest]
fn file_content_mutation_integration_failure_maintenance_preserves_safe_visibility() {
    file_mutation_integration::file_content_mutation_integration_failure_maintenance_preserves_safe_visibility();
}

#[ktest]
fn file_content_mutation_integration_repeated_calls_keep_state_stable() {
    file_mutation_integration::file_content_mutation_integration_repeated_calls_keep_state_stable();
}

#[ktest]
fn file_content_mutation_integration_concurrency_serializes_mutation_and_observation() {
    file_mutation_integration::file_content_mutation_integration_concurrency_serializes_mutation_and_observation();
}

#[ktest]
fn file_sync_and_persistence_integration_success_path_sync_data_then_sync_all_preserve_ordering_and_scope_boundary()
 {
    file_sync_integration::file_sync_and_persistence_integration_success_path_sync_data_then_sync_all_preserve_ordering_and_scope_boundary();
}

#[ktest]
fn file_sync_and_persistence_integration_failure_maintenance_device_stage_retry_preserves_dirty_window()
 {
    file_sync_integration::file_sync_and_persistence_integration_failure_maintenance_device_stage_retry_preserves_dirty_window();
}

#[ktest]
fn file_sync_and_persistence_integration_repeated_calls_preserve_clean_stability_and_metadata_boundary()
 {
    file_sync_integration::file_sync_and_persistence_integration_repeated_calls_preserve_clean_stability_and_metadata_boundary();
}

#[ktest]
fn file_sync_and_persistence_integration_concurrency_blocked_sync_revalidates_later_dirty_work() {
    file_sync_integration::file_sync_and_persistence_integration_concurrency_blocked_sync_revalidates_later_dirty_work();
}

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
    assert_eq!(
        u16::from_le_bytes([entry_set[4], entry_set[5]]),
        FILE_ATTRIBUTE_REGULAR
    );
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
    assert_eq!(
        decode_entry_name(&entry_set),
        "CreateFile".encode_utf16().collect::<Vec<_>>()
    );
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
    assert_eq!(
        decode_entry_name(&entry_set),
        "CreateDir".encode_utf16().collect::<Vec<_>>()
    );
    assert!(
        disk.read_cluster(first_cluster)
            .iter()
            .all(|byte| *byte == 0)
    );
}

#[ktest]
fn directory_entry_mutation_zero_size_dir_changes_only_newborn_shape() {
    entry_field_update::directory_entry_mutation_zero_size_dir_changes_only_newborn_shape();
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
    assert_eq!(
        parent_inode.lookup("victim").unwrap().type_(),
        InodeType::File
    );

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
fn directory_entry_mutation_rename_within_directory_rewrites_visibility_without_duplicate_namespace()
 {
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
    let invalidated_source_entry_set =
        disk.read_directory_entries(RENAME_SOURCE_PARENT_CLUSTER, 0, 3);
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
fn directory_entry_mutation_rename_across_directories_publishes_destination_before_source_invalidation()
 {
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
    let fractured_source_bytes_before =
        fractured_source_disk.read_root_entries(ROOT_FILE_ENTRY_INDEX, 3);

    let fractured_source_error = fractured_source_root
        .rename("Broken", &fractured_source_root, "Renamed")
        .unwrap_err();

    assert_eq!(fractured_source_error.error(), Errno::EUCLEAN);
    assert_eq!(
        fractured_source_disk.read_root_entries(ROOT_FILE_ENTRY_INDEX, 3),
        fractured_source_bytes_before
    );
    assert_eq!(
        lookup_error(&fractured_source_root, "Broken"),
        Errno::EUCLEAN
    );

    let fractured_target_disk = ExfatLookupTestDisk::new();
    fractured_target_disk.install_root_file(ROOT_FILE_ENTRY_INDEX, "Source");
    fractured_target_disk.install_root_fractured_entry_set(ROOT_SECOND_FILE_ENTRY_INDEX, "Target");
    let (_fractured_target_fs, fractured_target_root) = mount_root(&fractured_target_disk, None);
    let fractured_target_bytes_before =
        fractured_target_disk.read_root_entries(ROOT_FILE_ENTRY_INDEX, 6);

    let fractured_target_error = fractured_target_root
        .rename("Source", &fractured_target_root, "Target")
        .unwrap_err();

    assert_eq!(fractured_target_error.error(), Errno::EUCLEAN);
    assert_eq!(
        fractured_target_disk.read_root_entries(ROOT_FILE_ENTRY_INDEX, 6),
        fractured_target_bytes_before
    );
    assert!(fractured_target_root.lookup("Source").is_ok());
    assert_eq!(
        lookup_error(&fractured_target_root, "Target"),
        Errno::EUCLEAN
    );
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
    assert_eq!(
        entry_names(&source_before_entries),
        vec!["MoveMe", "EmptyDir"]
    );
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
    entry_field_update::directory_entry_mutation_integration_failure_maintenance_preserves_namespace_and_typed_boundaries();
}

#[ktest]
fn directory_entry_mutation_integration_concurrency_linearizes_cross_directory_rename_and_competing_mutations()
 {
    entry_field_update::directory_entry_mutation_integration_concurrency_linearizes_cross_directory_rename_and_competing_mutations();
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
fn file_content_mutation_write_boundary_write_at_updates_visible_bytes_without_growth() {
    init_lookup_test_runtime();

    let disk = ExfatLookupTestDisk::new();
    disk.install_root_file_with_contents(
        ROOT_FILE_ENTRY_INDEX,
        "WriteFile",
        TEST_REGULAR_FILE_CLUSTER,
        b"abcdefgh",
    );

    let (_fs, root_inode) = mount_root(&disk, None);
    let file_inode = root_inode.lookup("WriteFile").unwrap();
    let entry_set_before = root_entry_set(&disk, ROOT_FILE_ENTRY_INDEX);

    let written_len = file_inode.write_bytes_at(2, b"XYZ").unwrap();

    assert_eq!(written_len, 3);
    let mut visible_bytes = [0u8; 8];
    let read_len = file_inode.read_bytes_at(0, &mut visible_bytes).unwrap();
    assert_eq!(read_len, visible_bytes.len());
    assert_eq!(&visible_bytes, b"abXYZfgh");

    let entry_set_after = root_entry_set(&disk, ROOT_FILE_ENTRY_INDEX);
    let (valid_data_length, data_length) = stream_lengths(&entry_set_after);
    assert_eq!((valid_data_length, data_length), (8, 8));
    assert_eq!(
        &entry_set_after[FILE_ATTRIBUTES_OFFSET..FILE_ATTRIBUTES_OFFSET + 2],
        &entry_set_before[FILE_ATTRIBUTES_OFFSET..FILE_ATTRIBUTES_OFFSET + 2]
    );
}

#[ktest]
fn file_content_mutation_write_boundary_gap_write_zero_fills_exposed_range() {
    init_lookup_test_runtime();

    let disk = ExfatLookupTestDisk::new();
    let mut backing_bytes = [0xA5; 16];
    backing_bytes[..4].copy_from_slice(b"DATA");
    disk.install_root_file_with_cluster_chain(
        ROOT_FILE_ENTRY_INDEX,
        "GapWrite",
        TEST_REGULAR_FILE_CLUSTER,
        backing_bytes.len(),
        4,
        true,
        &[TEST_REGULAR_FILE_CLUSTER],
    );
    disk.write_cluster_prefix(TEST_REGULAR_FILE_CLUSTER, &backing_bytes);

    let (_fs, root_inode) = mount_root(&disk, None);
    let file_inode = root_inode.lookup("GapWrite").unwrap();

    let written_len = file_inode.write_bytes_at(8, b"END").unwrap();

    assert_eq!(written_len, 3);
    let mut visible_bytes = [0xCC; 16];
    let read_len = file_inode.read_bytes_at(0, &mut visible_bytes).unwrap();
    assert_eq!(read_len, visible_bytes.len());
    assert_eq!(&visible_bytes[..4], b"DATA");
    assert_eq!(&visible_bytes[4..8], &[0; 4]);
    assert_eq!(&visible_bytes[8..11], b"END");
    assert_eq!(&visible_bytes[11..], &[0; 5]);

    let cluster_bytes = disk.read_cluster(TEST_REGULAR_FILE_CLUSTER);
    assert_eq!(&cluster_bytes[..4], b"DATA");
    assert_eq!(&cluster_bytes[4..8], &[0; 4]);
    assert_eq!(&cluster_bytes[8..11], b"END");

    let entry_set_after = root_entry_set(&disk, ROOT_FILE_ENTRY_INDEX);
    let (valid_data_length, data_length) = stream_lengths(&entry_set_after);
    assert_eq!((valid_data_length, data_length), (11, 16));
}

#[ktest]
fn file_content_mutation_write_boundary_direct_write_rejects_misaligned_o_direct_without_side_effects()
 {
    init_lookup_test_runtime();

    let disk = ExfatLookupTestDisk::new();
    disk.install_root_file_with_contents(
        ROOT_FILE_ENTRY_INDEX,
        "DirectWrite",
        TEST_REGULAR_FILE_CLUSTER,
        b"aligned-data",
    );

    let (_fs, root_inode) = mount_root(&disk, None);
    let file_inode = root_inode.lookup("DirectWrite").unwrap();
    let entry_set_before = root_entry_set(&disk, ROOT_FILE_ENTRY_INDEX);
    let cluster_before = disk.read_cluster(TEST_REGULAR_FILE_CLUSTER);
    let metadata_before = file_inode.metadata();

    let error = file_inode.write_bytes_direct_at(1, b"BAD!").unwrap_err();

    assert_eq!(error.error(), Errno::EINVAL);
    assert_eq!(
        root_entry_set(&disk, ROOT_FILE_ENTRY_INDEX),
        entry_set_before
    );
    assert_eq!(disk.read_cluster(TEST_REGULAR_FILE_CLUSTER), cluster_before);
    assert_metadata_unchanged(file_inode.metadata(), metadata_before);
}

#[ktest]
fn file_content_mutation_write_boundary_fallocate_modes_return_eopnotsupp_without_side_effects() {
    init_lookup_test_runtime();

    let disk = ExfatLookupTestDisk::new();
    disk.install_root_file_with_contents(
        ROOT_FILE_ENTRY_INDEX,
        "FallocateFile",
        TEST_REGULAR_FILE_CLUSTER,
        b"stay-put",
    );

    let (_fs, root_inode) = mount_root(&disk, None);
    let file_inode = root_inode.lookup("FallocateFile").unwrap();
    let entry_set_before = root_entry_set(&disk, ROOT_FILE_ENTRY_INDEX);
    let cluster_before = disk.read_cluster(TEST_REGULAR_FILE_CLUSTER);
    let metadata_before = file_inode.metadata();

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

    assert_eq!(
        root_entry_set(&disk, ROOT_FILE_ENTRY_INDEX),
        entry_set_before
    );
    assert_eq!(disk.read_cluster(TEST_REGULAR_FILE_CLUSTER), cluster_before);
    assert_metadata_unchanged(file_inode.metadata(), metadata_before);
}

#[ktest]
fn file_content_mutation_write_boundary_write_at_rejects_directory_before_anomaly_gate() {
    init_lookup_test_runtime();

    let disk = ExfatLookupTestDisk::new();
    disk.install_root_directory(
        ROOT_FILE_ENTRY_INDEX,
        "WriteDir",
        TEST_CHILD_DIRECTORY_CLUSTER,
    );
    disk.set_volume_flags(READ_AT_TEST_VOLUME_FLAGS);

    let (_fs, root_inode) = mount_root(&disk, None);
    let directory_inode = root_inode.lookup("WriteDir").unwrap();
    let entry_set_before = root_entry_set(&disk, ROOT_FILE_ENTRY_INDEX);

    let error = directory_inode.write_bytes_at(0, b"dir").unwrap_err();

    assert_eq!(error.error(), Errno::EISDIR);
    assert_eq!(
        root_entry_set(&disk, ROOT_FILE_ENTRY_INDEX),
        entry_set_before
    );
}

#[ktest]
fn file_content_mutation_write_boundary_write_at_fast_fails_on_imported_mount_anomaly() {
    init_lookup_test_runtime();

    let disk = ExfatLookupTestDisk::new();
    disk.install_root_file_with_contents(
        ROOT_FILE_ENTRY_INDEX,
        "AnomalousWrite",
        TEST_REGULAR_FILE_CLUSTER,
        b"visible bytes",
    );
    disk.set_volume_flags(READ_AT_TEST_VOLUME_FLAGS);

    let (_fs, root_inode) = mount_root(&disk, None);
    let file_inode = root_inode.lookup("AnomalousWrite").unwrap();
    let entry_set_before = root_entry_set(&disk, ROOT_FILE_ENTRY_INDEX);
    let cluster_before = disk.read_cluster(TEST_REGULAR_FILE_CLUSTER);
    let metadata_before = file_inode.metadata();

    let error = file_inode.write_bytes_at(0, b"fail").unwrap_err();

    assert_eq!(error.error(), Errno::EIO);
    assert_eq!(
        root_entry_set(&disk, ROOT_FILE_ENTRY_INDEX),
        entry_set_before
    );
    assert_eq!(disk.read_cluster(TEST_REGULAR_FILE_CLUSTER), cluster_before);
    assert_metadata_unchanged(file_inode.metadata(), metadata_before);
}

#[ktest]
fn file_content_mutation_growth_shrink_and_allocation_topology_append_growth_allocates_and_publishes_new_eof()
 {
    file_mutation_integration::file_content_mutation_growth_shrink_and_allocation_topology_append_growth_allocates_and_publishes_new_eof();
}

#[ktest]
fn file_content_mutation_growth_shrink_and_allocation_topology_gap_growth_zero_fills_newly_allocated_range()
 {
    file_mutation_integration::file_content_mutation_growth_shrink_and_allocation_topology_gap_growth_zero_fills_newly_allocated_range();
}

#[ktest]
fn file_content_mutation_growth_shrink_and_allocation_topology_non_contiguous_growth_backfills_fat_links()
 {
    file_mutation_integration::file_content_mutation_growth_shrink_and_allocation_topology_non_contiguous_growth_backfills_fat_links();
}

#[ktest]
fn file_content_mutation_growth_shrink_and_allocation_topology_resize_shrink_releases_clusters_and_truncates_chain()
 {
    file_mutation_integration::file_content_mutation_growth_shrink_and_allocation_topology_resize_shrink_releases_clusters_and_truncates_chain();
}

#[ktest]
fn file_content_mutation_growth_shrink_and_allocation_topology_publication_failure_preserves_visible_eof()
 {
    file_mutation_integration::file_content_mutation_growth_shrink_and_allocation_topology_publication_failure_preserves_visible_eof();
}

#[ktest]
fn file_content_mapping_cached_io_read_at_clips_at_eof_and_returns_zero_at_eof() {
    init_lookup_test_runtime();

    let disk = ExfatLookupTestDisk::new();
    let expected = b"EOF boundary";
    disk.install_root_file_with_contents(
        ROOT_FILE_ENTRY_INDEX,
        "ClipFile",
        TEST_REGULAR_FILE_CLUSTER,
        expected,
    );

    let (_fs, root_inode) = mount_root(&disk, None);
    let file_inode = root_inode.lookup("ClipFile").unwrap();
    let mut clipped_buffer = [0xCC; 16];

    let clipped_len = file_inode
        .read_bytes_at(expected.len() - 3, &mut clipped_buffer)
        .unwrap();

    assert_eq!(clipped_len, 3);
    assert_eq!(
        &clipped_buffer[..clipped_len],
        &expected[expected.len() - 3..]
    );
    assert_eq!(clipped_buffer[clipped_len], 0xCC);

    let mut eof_buffer = [0xDD; 8];
    let eof_len = file_inode
        .read_bytes_at(expected.len(), &mut eof_buffer)
        .unwrap();

    assert_eq!(eof_len, 0);
    assert_eq!(eof_buffer, [0xDD; 8]);
}

#[ktest]
fn file_content_mapping_cached_io_read_at_zero_fills_valid_data_suffix() {
    init_lookup_test_runtime();

    const UNINITIALIZED_MEDIA: &[u8; 8] = b"TAILTAIL";

    let disk = ExfatLookupTestDisk::new();
    let initialized_prefix = b"initDATA";
    let mut on_disk_bytes = [0u8; 16];
    on_disk_bytes[..initialized_prefix.len()].copy_from_slice(initialized_prefix);
    on_disk_bytes[initialized_prefix.len()..].copy_from_slice(UNINITIALIZED_MEDIA);
    disk.install_root_file_with_contents(
        ROOT_FILE_ENTRY_INDEX,
        "ZeroTail",
        TEST_REGULAR_FILE_CLUSTER,
        &on_disk_bytes,
    );
    disk.set_root_stream_extension(
        ROOT_FILE_ENTRY_INDEX,
        TEST_REGULAR_FILE_CLUSTER,
        on_disk_bytes.len(),
        initialized_prefix.len(),
    );

    let (_fs, root_inode) = mount_root(&disk, None);
    let file_inode = root_inode.lookup("ZeroTail").unwrap();
    let mut buffer = [0xEE; 16];

    let read_len = file_inode.read_bytes_at(0, &mut buffer).unwrap();

    assert_eq!(read_len, on_disk_bytes.len());
    assert_eq!(&buffer[..initialized_prefix.len()], initialized_prefix);
    assert_eq!(
        &buffer[initialized_prefix.len()..read_len],
        &[0u8; UNINITIALIZED_MEDIA.len()]
    );
}

#[ktest]
fn file_content_mapping_cached_io_read_at_rejects_directory_before_anomaly_gate() {
    init_lookup_test_runtime();

    let disk = ExfatLookupTestDisk::new();
    disk.install_root_directory(
        ROOT_FILE_ENTRY_INDEX,
        "DirOnly",
        TEST_CHILD_DIRECTORY_CLUSTER,
    );
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

#[ktest]
fn file_content_mapping_cached_io_map_regular_file_logical_offset_maps_contiguous_nofatchain_offsets()
 {
    init_lookup_test_runtime();

    let disk = ExfatLookupTestDisk::new();
    let cluster_size = disk.root_cluster_size();
    disk.install_root_file_with_cluster_chain(
        ROOT_FILE_ENTRY_INDEX,
        "ContiguousMap",
        TEST_REGULAR_FILE_CLUSTER,
        cluster_size + 5,
        cluster_size + 5,
        true,
        &[TEST_REGULAR_FILE_CLUSTER, TEST_CONTIGUOUS_SECOND_CLUSTER],
    );
    disk.write_cluster_prefix(TEST_REGULAR_FILE_CLUSTER, b"contiguous-first");
    disk.write_cluster_prefix(TEST_CONTIGUOUS_SECOND_CLUSTER, b"contiguous-second");

    let (_fs, root_inode) = mount_root(&disk, None);
    let file_inode = root_inode.lookup("ContiguousMap").unwrap();
    let exfat_inode = lookup_exfat_inode(&file_inode);
    let (block_device, boot_region) = published_lookup_state(&file_inode);

    let first_offset = exfat_inode
        .map_regular_file_logical_offset(&block_device, &boot_region, 0)
        .unwrap();
    let second_cluster_offset = exfat_inode
        .map_regular_file_logical_offset(&block_device, &boot_region, cluster_size + 2)
        .unwrap();

    assert_eq!(
        first_offset,
        Some(
            boot_region
                .cluster_offset(TEST_REGULAR_FILE_CLUSTER)
                .unwrap()
        )
    );
    assert_eq!(
        second_cluster_offset,
        Some(
            boot_region
                .cluster_offset(TEST_CONTIGUOUS_SECOND_CLUSTER)
                .unwrap()
                + 2,
        )
    );
}

#[ktest]
fn file_content_mapping_cached_io_map_regular_file_logical_offset_follows_fragmented_fat_chain() {
    init_lookup_test_runtime();

    let disk = ExfatLookupTestDisk::new();
    let cluster_size = disk.root_cluster_size();
    disk.install_root_file_with_cluster_chain(
        ROOT_FILE_ENTRY_INDEX,
        "FragmentedMap",
        TEST_FRAGMENTED_FIRST_CLUSTER,
        cluster_size + 7,
        cluster_size + 7,
        false,
        &[
            TEST_FRAGMENTED_FIRST_CLUSTER,
            TEST_FRAGMENTED_SECOND_CLUSTER,
        ],
    );
    disk.set_fat_chain_step(
        TEST_FRAGMENTED_FIRST_CLUSTER,
        TEST_FRAGMENTED_SECOND_CLUSTER,
    );
    disk.terminate_fat_chain(TEST_FRAGMENTED_SECOND_CLUSTER);

    let (_fs, root_inode) = mount_root(&disk, None);
    let file_inode = root_inode.lookup("FragmentedMap").unwrap();
    let exfat_inode = lookup_exfat_inode(&file_inode);
    let (block_device, boot_region) = published_lookup_state(&file_inode);

    let second_cluster_offset = exfat_inode
        .map_regular_file_logical_offset(&block_device, &boot_region, cluster_size + 4)
        .unwrap();

    assert_eq!(
        second_cluster_offset,
        Some(
            boot_region
                .cluster_offset(TEST_FRAGMENTED_SECOND_CLUSTER)
                .unwrap()
                + 4,
        )
    );
}

#[ktest]
fn file_content_mapping_cached_io_page_count_and_cache_holder_follow_published_file_size() {
    init_lookup_test_runtime();

    let disk = ExfatLookupTestDisk::new();
    let cluster_size = disk.root_cluster_size();
    let data_length = PAGE_SIZE + 1;
    let cluster_count = data_length.div_ceil(cluster_size);
    let clusters: Vec<u32> = (0..cluster_count)
        .map(|index| TEST_REGULAR_FILE_CLUSTER + u32::try_from(index).unwrap())
        .collect();
    disk.install_root_file_with_cluster_chain(
        ROOT_FILE_ENTRY_INDEX,
        "PageCount",
        clusters[0],
        data_length,
        data_length,
        true,
        &clusters,
    );

    let (_fs, root_inode) = mount_root(&disk, None);
    let file_inode = root_inode.lookup("PageCount").unwrap();
    let page_cache = file_inode.page_cache().unwrap();

    assert_eq!(
        published_page_count(&file_inode),
        data_length.div_ceil(PAGE_SIZE)
    );
    assert_eq!(
        page_cache.size(),
        published_page_count(&file_inode) * PAGE_SIZE
    );
}

#[ktest]
fn file_content_mapping_cached_io_map_regular_file_logical_offset_returns_none_for_eof_and_uninitialized_offsets()
 {
    init_lookup_test_runtime();

    let disk = ExfatLookupTestDisk::new();
    let cluster_size = disk.root_cluster_size();
    let valid_data_length = cluster_size - 4;
    let data_length = cluster_size + 8;
    disk.install_root_file_with_cluster_chain(
        ROOT_FILE_ENTRY_INDEX,
        "SparseTailMap",
        TEST_REGULAR_FILE_CLUSTER,
        data_length,
        valid_data_length,
        true,
        &[TEST_REGULAR_FILE_CLUSTER, TEST_CONTIGUOUS_SECOND_CLUSTER],
    );

    let (_fs, root_inode) = mount_root(&disk, None);
    let file_inode = root_inode.lookup("SparseTailMap").unwrap();
    let exfat_inode = lookup_exfat_inode(&file_inode);
    let (block_device, boot_region) = published_lookup_state(&file_inode);

    let last_initialized = exfat_inode
        .map_regular_file_logical_offset(&block_device, &boot_region, valid_data_length - 1)
        .unwrap();
    let first_uninitialized = exfat_inode
        .map_regular_file_logical_offset(&block_device, &boot_region, valid_data_length)
        .unwrap();
    let eof = exfat_inode
        .map_regular_file_logical_offset(&block_device, &boot_region, data_length)
        .unwrap();

    assert_eq!(
        last_initialized,
        Some(
            boot_region
                .cluster_offset(TEST_REGULAR_FILE_CLUSTER)
                .unwrap()
                + valid_data_length
                - 1,
        )
    );
    assert_eq!(first_uninitialized, None);
    assert_eq!(eof, None);
}

#[ktest]
fn file_content_mapping_cached_io_mapping_surfaces_reject_directories() {
    init_lookup_test_runtime();

    let disk = ExfatLookupTestDisk::new();
    disk.install_root_directory(
        ROOT_FILE_ENTRY_INDEX,
        "MappingDir",
        TEST_CHILD_DIRECTORY_CLUSTER,
    );

    let (_fs, root_inode) = mount_root(&disk, None);
    let directory_inode = root_inode.lookup("MappingDir").unwrap();
    let exfat_inode = lookup_exfat_inode(&directory_inode);
    let (block_device, boot_region) = published_lookup_state(&directory_inode);

    let mapping_error = exfat_inode
        .map_regular_file_logical_offset(&block_device, &boot_region, 0)
        .unwrap_err();

    assert_eq!(mapping_error.error(), Errno::EISDIR);
    assert!(directory_inode.page_cache().is_none());
}

#[ktest]
fn file_content_mapping_cached_io_page_cache_backend_contiguous_read_returns_waiter_backed_success()
{
    init_lookup_test_runtime();

    let disk = ExfatLookupTestDisk::new();
    let contents = patterned_bytes(disk.root_cluster_size().min(SECTOR_SIZE * 2));
    disk.install_root_file_with_contents(
        ROOT_FILE_ENTRY_INDEX,
        "PageCacheRead",
        TEST_REGULAR_FILE_CLUSTER,
        &contents,
    );

    let (_fs, root_inode) = mount_root(&disk, None);
    let file_inode = root_inode.lookup("PageCacheRead").unwrap();
    let exfat_inode = lookup_exfat_inode(&file_inode);
    let (_block_device, boot_region) = published_lookup_state(&file_inode);

    assert!(file_inode.page_cache().is_some());
    let _ = disk.take_observed_bios();

    let page = CachePage::alloc_uninit().unwrap();
    let waiter = exfat_inode.read_page_async(0, &page).unwrap();

    assert!(waiter.nreqs() > 0);
    assert_eq!(waiter.wait(), Some(BioStatus::Complete));

    let page_bytes = read_cache_page_bytes(&page);
    assert_eq!(&page_bytes[..contents.len()], contents.as_slice());
    assert!(page_bytes[contents.len()..].iter().all(|byte| *byte == 0));
    assert_observed_bios(
        &disk.take_observed_bios(),
        BioType::Read,
        &[(
            boot_region
                .cluster_offset(TEST_REGULAR_FILE_CLUSTER)
                .unwrap(),
            contents.len(),
        )],
    );
}

#[ktest]
fn file_content_mapping_cached_io_page_cache_backend_fragmented_writeback_preserves_segmented_mapping()
 {
    cached_io_integration::file_content_mapping_cached_io_page_cache_backend_fragmented_writeback_preserves_segmented_mapping();
}

#[ktest]
fn file_content_mapping_cached_io_page_cache_backend_zero_fills_mid_page_uninitialized_suffix() {
    cached_io_integration::file_content_mapping_cached_io_page_cache_backend_zero_fills_mid_page_uninitialized_suffix();
}

#[ktest]
fn file_content_mapping_cached_io_page_cache_backend_rejects_directory_before_submission() {
    init_lookup_test_runtime();

    let disk = ExfatLookupTestDisk::new();
    disk.install_root_directory(
        ROOT_FILE_ENTRY_INDEX,
        "BackendDir",
        TEST_CHILD_DIRECTORY_CLUSTER,
    );

    let (_fs, root_inode) = mount_root(&disk, None);
    let directory_inode = root_inode.lookup("BackendDir").unwrap();
    let exfat_inode = lookup_exfat_inode(&directory_inode);
    let page = CachePage::alloc_zero(PageState::UpToDate).unwrap();

    assert!(directory_inode.page_cache().is_none());
    let _ = disk.take_observed_bios();

    let read_error = exfat_inode.read_page_async(0, &page).unwrap_err();
    let write_error = exfat_inode.write_page_async(0, &page).unwrap_err();

    assert_eq!(read_error.error(), Errno::EISDIR);
    assert_eq!(write_error.error(), Errno::EISDIR);
    assert!(disk.take_observed_bios().is_empty());
}

#[ktest]
fn file_content_mapping_cached_io_page_cache_backend_fast_fails_imported_anomaly_before_submission()
{
    init_lookup_test_runtime();

    let disk = ExfatLookupTestDisk::new();
    let contents = patterned_bytes(SECTOR_SIZE);
    disk.install_root_file_with_contents(
        ROOT_FILE_ENTRY_INDEX,
        "AnomalousPage",
        TEST_REGULAR_FILE_CLUSTER,
        &contents,
    );
    disk.set_volume_flags(READ_AT_TEST_VOLUME_FLAGS);

    let (_fs, root_inode) = mount_root(&disk, None);
    let file_inode = root_inode.lookup("AnomalousPage").unwrap();
    let exfat_inode = lookup_exfat_inode(&file_inode);
    let page = CachePage::alloc_uninit().unwrap();

    let _ = disk.take_observed_bios();
    let error = exfat_inode.read_page_async(0, &page).unwrap_err();

    assert_eq!(error.error(), Errno::EIO);
    assert!(disk.take_observed_bios().is_empty());
}

#[ktest]
fn file_sync_and_persistence_writeback_ordering_and_admission_boundary_sync_data_orders_file_writeback_before_device_sync()
 {
    init_lookup_test_runtime();

    let disk = ExfatLookupTestDisk::new();
    disk.install_root_file_with_contents(
        ROOT_FILE_ENTRY_INDEX,
        "SyncDataFile",
        TEST_REGULAR_FILE_CLUSTER,
        b"abcdefgh",
    );

    let (_fs, root_inode) = mount_root(&disk, None);
    let file_inode = root_inode.lookup("SyncDataFile").unwrap();

    dirty_regular_file_first_page(&file_inode, b"abXYZfgh");
    let _ = disk.take_observed_bios();

    file_inode.sync_data().unwrap();

    let observed_bios = disk.take_observed_bios();
    assert_sync_writeback_before_device_sync(&observed_bios);
    assert_eq!(
        &disk.read_cluster(TEST_REGULAR_FILE_CLUSTER)[..8],
        b"abXYZfgh"
    );
}

#[ktest]
fn file_sync_and_persistence_writeback_ordering_and_admission_boundary_sync_all_orders_file_writeback_before_device_sync()
 {
    init_lookup_test_runtime();

    let disk = ExfatLookupTestDisk::new();
    disk.install_root_file_with_contents(
        ROOT_FILE_ENTRY_INDEX,
        "SyncAllFile",
        TEST_REGULAR_FILE_CLUSTER,
        b"abcdefgh",
    );

    let (_fs, root_inode) = mount_root(&disk, None);
    let file_inode = root_inode.lookup("SyncAllFile").unwrap();

    dirty_regular_file_first_page(&file_inode, b"abcdWXYZ");
    let _ = disk.take_observed_bios();

    file_inode.sync_all().unwrap();

    let observed_bios = disk.take_observed_bios();
    assert_sync_writeback_before_device_sync(&observed_bios);
    assert_eq!(
        &disk.read_cluster(TEST_REGULAR_FILE_CLUSTER)[..8],
        b"abcdWXYZ"
    );
}

#[ktest]
fn file_sync_and_persistence_writeback_ordering_and_admission_boundary_sync_fast_fail_rejects_imported_mount_anomaly_before_writeback_or_device_sync()
 {
    init_lookup_test_runtime();

    let disk = ExfatLookupTestDisk::new();
    disk.install_root_file_with_contents(
        ROOT_FILE_ENTRY_INDEX,
        "SyncFail",
        TEST_REGULAR_FILE_CLUSTER,
        b"durable bytes",
    );
    disk.set_volume_flags(READ_AT_TEST_VOLUME_FLAGS);

    let (_fs, root_inode) = mount_root(&disk, None);
    let file_inode = root_inode.lookup("SyncFail").unwrap();
    let entry_set_before = root_entry_set(&disk, ROOT_FILE_ENTRY_INDEX);
    let cluster_before = disk.read_cluster(TEST_REGULAR_FILE_CLUSTER);
    let metadata_before = file_inode.metadata();

    let _ = disk.take_observed_bios();
    let error = file_inode.sync_data().unwrap_err();

    assert_eq!(error.error(), Errno::EIO);
    assert!(disk.take_observed_bios().is_empty());
    assert_eq!(
        root_entry_set(&disk, ROOT_FILE_ENTRY_INDEX),
        entry_set_before
    );
    assert_eq!(disk.read_cluster(TEST_REGULAR_FILE_CLUSTER), cluster_before);
    assert_metadata_unchanged(file_inode.metadata(), metadata_before);
}

#[ktest]
fn file_sync_and_persistence_revalidation_metadata_scope_and_failure_maintenance_sync_data_leaves_metadata_only_interval_for_sync_all()
 {
    init_lookup_test_runtime();

    let disk = ExfatLookupTestDisk::new();
    disk.install_root_file_with_contents(
        ROOT_FILE_ENTRY_INDEX,
        "ScopedSync",
        TEST_REGULAR_FILE_CLUSTER,
        b"abcdefgh",
    );

    let (_fs, root_inode) = mount_root(&disk, None);
    let file_inode = root_inode.lookup("ScopedSync").unwrap();

    assert_eq!(file_inode.write_bytes_at(0, b"ABCD").unwrap(), 4);
    let _ = disk.take_observed_bios();

    file_inode.sync_data().unwrap();
    assert_flush_only(&disk.take_observed_bios());

    file_inode.sync_data().unwrap();
    assert!(disk.take_observed_bios().is_empty());

    file_inode.sync_all().unwrap();
    assert_flush_only(&disk.take_observed_bios());

    file_inode.sync_all().unwrap();
    assert!(disk.take_observed_bios().is_empty());
}

#[ktest]
fn file_sync_and_persistence_revalidation_metadata_scope_and_failure_maintenance_repeated_clean_sync_calls_do_not_manufacture_dirty_state()
 {
    init_lookup_test_runtime();

    let disk = ExfatLookupTestDisk::new();
    disk.install_root_file_with_contents(
        ROOT_FILE_ENTRY_INDEX,
        "CleanSync",
        TEST_REGULAR_FILE_CLUSTER,
        b"still clean",
    );

    let (_fs, root_inode) = mount_root(&disk, None);
    let file_inode = root_inode.lookup("CleanSync").unwrap();
    assert!(file_inode.page_cache().is_some());

    let _ = disk.take_observed_bios();
    file_inode.sync_data().unwrap();
    file_inode.sync_data().unwrap();
    file_inode.sync_all().unwrap();
    file_inode.sync_all().unwrap();

    assert!(disk.take_observed_bios().is_empty());
}

#[ktest]
fn file_sync_and_persistence_revalidation_metadata_scope_and_failure_maintenance_device_stage_failure_leaves_dirty_window_retryable()
 {
    init_lookup_test_runtime();

    let disk = ExfatLookupTestDisk::new();
    disk.install_root_file_with_contents(
        ROOT_FILE_ENTRY_INDEX,
        "RetryableSync",
        TEST_REGULAR_FILE_CLUSTER,
        b"retryable",
    );
    let flush_control_disk = ExfatLookupFlushControlDisk::new(disk.clone());
    let block_device: Arc<dyn BlockDevice> = flush_control_disk.clone();
    let (_fs, root_inode) = mount_root_from_block_device(block_device, FsFlags::empty(), None);
    let file_inode = root_inode.lookup("RetryableSync").unwrap();

    assert_eq!(file_inode.write_bytes_at(0, b"RETY").unwrap(), 4);
    let _ = disk.take_observed_bios();

    flush_control_disk.enable_flush_failures();
    let error = file_inode.sync_data().unwrap_err();

    assert_eq!(error.error(), Errno::EIO);
    assert_flush_only(&disk.take_observed_bios());

    flush_control_disk.disable_flush_failures();
    file_inode.sync_data().unwrap();
    assert_flush_only(&disk.take_observed_bios());

    file_inode.sync_data().unwrap();
    assert!(disk.take_observed_bios().is_empty());
}

#[ktest]
fn file_sync_and_persistence_revalidation_metadata_scope_and_failure_maintenance_later_dirty_work_remains_outstanding_after_blocked_sync_success()
 {
    init_lookup_test_runtime();

    let disk = ExfatLookupTestDisk::new();
    disk.install_root_file_with_contents(
        ROOT_FILE_ENTRY_INDEX,
        "ConcurrentSync",
        TEST_REGULAR_FILE_CLUSTER,
        b"abcdefgh",
    );
    let flush_control_disk = ExfatLookupFlushControlDisk::new(disk.clone());
    let block_device: Arc<dyn BlockDevice> = flush_control_disk.clone();
    let (_fs, root_inode) = mount_root_from_block_device(block_device, FsFlags::empty(), None);
    let file_inode = root_inode.lookup("ConcurrentSync").unwrap();

    assert_eq!(file_inode.write_bytes_at(0, b"FIRST").unwrap(), 5);
    let _ = disk.take_observed_bios();

    flush_control_disk.enable_blocking_flush();
    let sync_result = Arc::new(Mutex::new(None));
    let writer_result = Arc::new(Mutex::new(None));

    let sync_thread = {
        let file_inode = file_inode.clone();
        let sync_result = sync_result.clone();
        ThreadOptions::new(move || {
            *sync_result.lock() = Some(file_inode.sync_data().map_err(|error| error.error()));
        })
        .spawn()
    };

    while !flush_control_disk.flush_started() {
        Thread::yield_now();
    }

    let writer_thread = {
        let file_inode = file_inode.clone();
        let writer_result = writer_result.clone();
        ThreadOptions::new(move || {
            *writer_result.lock() = Some(
                file_inode
                    .write_bytes_at(0, b"LATER")
                    .map_err(|error| error.error()),
            );
        })
        .spawn()
    };

    let mut writer_completed_while_flush_blocked = false;
    for _ in 0..10_000 {
        if writer_result.lock().is_some() {
            writer_completed_while_flush_blocked = true;
            break;
        }
        Thread::yield_now();
    }

    flush_control_disk.release_blocked_flush();
    sync_thread.join();
    writer_thread.join();

    assert!(writer_completed_while_flush_blocked);
    assert_eq!(*writer_result.lock(), Some(Ok(5)));
    assert_eq!(*sync_result.lock(), Some(Ok(())));
    assert_eq!(&disk.read_cluster(TEST_REGULAR_FILE_CLUSTER)[..5], b"LATER");

    let _ = disk.take_observed_bios();
    file_inode.sync_data().unwrap();
    assert_flush_only(&disk.take_observed_bios());

    file_inode.sync_data().unwrap();
    assert!(disk.take_observed_bios().is_empty());
}
