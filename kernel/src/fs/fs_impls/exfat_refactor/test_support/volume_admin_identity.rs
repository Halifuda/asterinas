// SPDX-License-Identifier: MPL-2.0

use alloc::vec::Vec;

use ostd::prelude::ktest;

use super::{
    super::super::{
        fs::{ExfatFs, ExfatMountOptions, FreeSpaceSnapshot},
        volume::{
            self, VolumeAdminRequest, VolumeIdentityEntries, VolumeIdentityQuery,
            VolumeIdentityUpdate,
        },
    },
    END_OF_DIRECTORY_ENTRY_TYPE, ExfatRefactorMemoryDisk, ExfatRefactorToggleFailingWriteDisk,
    assert_same_super_block, cluster_offset, default_mount_options, find_directory_entry,
    init_mount_volume_state_test_runtime, mount_block_device, mounted_fs, next_cluster,
};
use crate::{
    fs::vfs::file_system::{FsFlags, SuperBlock},
    prelude::*,
    process::credentials::capabilities::CapSet,
};

const VOLUME_GUID_ENTRY_TYPE: u8 = 0xA0;
const VOLUME_LABEL_ENTRY_TYPE: u8 = 0x83;

#[derive(Clone)]
pub(super) struct VolumeAdminStateSnapshot {
    flags: FsFlags,
    forced_shutdown: bool,
    free_space: FreeSpaceSnapshot,
    label: Option<String>,
    options: ExfatMountOptions,
    super_block: SuperBlock,
}

pub(super) fn query_identity(
    fs: &Arc<ExfatFs>,
    query: VolumeIdentityQuery,
) -> Result<VolumeIdentityEntries> {
    volume::query_volume_identity(fs, query)
}

pub(super) fn handle_admin_request(
    fs: &Arc<ExfatFs>,
    effective_capset: CapSet,
    request: VolumeAdminRequest,
) -> Result<()> {
    if !effective_capset.contains(CapSet::SYS_ADMIN) {
        return_errno_with_message!(
            Errno::EPERM,
            "exFAT volume administration requires SYS_ADMIN"
        );
    }
    match request {
        VolumeAdminRequest::ForceShutdown => {
            volume::admit_forced_shutdown(fs).map_err(Error::from)
        }
        VolumeAdminRequest::TrimFreeSpace => volume::administrative_trim_free_space(fs),
        VolumeAdminRequest::UpdateIdentity(update) => volume::update_volume_identity(fs, update),
    }
}

pub(super) fn capture_volume_admin_state(fs: &Arc<ExfatFs>) -> VolumeAdminStateSnapshot {
    let (flags, forced_shutdown, options) = {
        let state = fs.state.read();
        let publication = state.as_ref().unwrap();
        (
            publication.flags,
            publication.forced_shutdown,
            publication.options.clone(),
        )
    };
    VolumeAdminStateSnapshot {
        flags,
        forced_shutdown,
        free_space: fs.cached_free_space_snapshot().unwrap(),
        label: expect_identity_label(query_identity(fs, VolumeIdentityQuery::Label).unwrap()),
        options,
        super_block: fs.sb(),
    }
}

pub(super) fn assert_volume_admin_state_matches(
    fs: &Arc<ExfatFs>,
    expected: &VolumeAdminStateSnapshot,
) {
    let actual = capture_volume_admin_state(fs);
    assert_eq!(actual.flags, expected.flags);
    assert_eq!(actual.forced_shutdown, expected.forced_shutdown);
    assert_eq!(actual.free_space, expected.free_space);
    assert_eq!(actual.label, expected.label);
    assert_eq!(actual.options, expected.options);
    assert_same_super_block(&actual.super_block, &expected.super_block);
}

pub(super) fn assert_request_errno_and_state_stable(
    fs: &Arc<ExfatFs>,
    effective_capset: CapSet,
    request: VolumeAdminRequest,
    expected_errno: Errno,
) {
    let before = capture_volume_admin_state(fs);
    let error = handle_admin_request(fs, effective_capset, request).unwrap_err();
    assert_eq!(error.error(), expected_errno);
    assert_volume_admin_state_matches(fs, &before);
}

pub(super) fn expect_identity_label(entries: VolumeIdentityEntries) -> Option<String> {
    let VolumeIdentityEntries { guid, label } = entries;
    assert_eq!(guid, None);
    label
}

pub(super) fn next_volume_label(current_label: Option<&str>) -> &'static str {
    match current_label {
        Some("VOLADMIN01") => "VOLADMIN02",
        _ => "VOLADMIN01",
    }
}

pub(super) fn read_root_directory_bytes(disk: &Arc<ExfatRefactorMemoryDisk>) -> Vec<u8> {
    let validated_mount =
        crate::fs::fs_impls::exfat_refactor::test_support::load_validated_mount(disk.as_ref())
            .unwrap();
    let boot_region = validated_mount.boot_region;
    let mut current_cluster = boot_region.root_dir_cluster;
    let mut directory_bytes = Vec::new();
    let mut visited_clusters = BTreeSet::new();

    loop {
        assert!(visited_clusters.insert(current_cluster));
        let current_cluster_offset = cluster_offset(&boot_region, current_cluster);
        directory_bytes.extend(disk.read_bytes(current_cluster_offset, boot_region.cluster_size));

        let Some(next_cluster) = next_cluster(disk, &boot_region, current_cluster) else {
            return directory_bytes;
        };
        current_cluster = next_cluster;
    }
}

pub(super) fn ordinary_root_namespace_entries(
    directory_bytes: &[u8],
) -> Vec<(usize, usize, Vec<u16>)> {
    let mut ordinary_entries = Vec::new();
    let mut entry_index = 0;

    loop {
        match super::super::direntry::scan_directory_entry(true, directory_bytes, entry_index)
            .unwrap()
        {
            super::super::direntry::ScannedDirectoryEntry::Anomaly { kind, slot_range } => {
                panic!(
                    "unexpected root anomaly {:?} at slot {}",
                    kind,
                    slot_range.first_entry_index()
                );
            }
            super::super::direntry::ScannedDirectoryEntry::EndOfDirectory { .. } => {
                return ordinary_entries;
            }
            super::super::direntry::ScannedDirectoryEntry::File(entry_set) => {
                let slot_range = entry_set.slot_range();
                ordinary_entries.push((
                    slot_range.first_entry_index(),
                    slot_range.entry_count(),
                    entry_set.name().unwrap(),
                ));
                entry_index = slot_range.next_entry_index().unwrap();
            }
            super::super::direntry::ScannedDirectoryEntry::Vacant(slot_range) => {
                entry_index = slot_range.next_entry_index().unwrap();
            }
        }
    }
}

fn scanned_entry_start_index(
    scanned_entry: super::super::direntry::ScannedDirectoryEntry<'_>,
) -> usize {
    match scanned_entry {
        super::super::direntry::ScannedDirectoryEntry::Anomaly { slot_range, .. } => {
            slot_range.first_entry_index()
        }
        super::super::direntry::ScannedDirectoryEntry::EndOfDirectory { entry_index } => {
            entry_index
        }
        super::super::direntry::ScannedDirectoryEntry::File(entry_set) => {
            entry_set.slot_range().first_entry_index()
        }
        super::super::direntry::ScannedDirectoryEntry::Vacant(slot_range) => {
            slot_range.first_entry_index()
        }
    }
}

fn find_root_entry_index(directory_bytes: &[u8], entry_type: u8) -> Option<usize> {
    for (entry_index, entry) in directory_bytes.chunks_exact(32).enumerate() {
        if entry[0] == entry_type {
            return Some(entry_index);
        }
        if entry[0] == END_OF_DIRECTORY_ENTRY_TYPE {
            return None;
        }
    }
    None
}

#[ktest]
fn volume_admin_identity_query_update_round_trips_lossless_label_and_preserves_root_namespace() {
    init_mount_volume_state_test_runtime();

    let disk = ExfatRefactorMemoryDisk::new();
    let ordinary_entries_before =
        ordinary_root_namespace_entries(&read_root_directory_bytes(&disk));
    let (fs, _, _, _) = mounted_fs(&disk, default_mount_options());
    let initial_label =
        expect_identity_label(query_identity(&fs, VolumeIdentityQuery::Label).unwrap());
    let updated_label = next_volume_label(initial_label.as_deref());

    assert_eq!(
        initial_label,
        expect_identity_label(query_identity(&fs, VolumeIdentityQuery::Label).unwrap())
    );
    handle_admin_request(
        &fs,
        CapSet::SYS_ADMIN,
        VolumeAdminRequest::UpdateIdentity(VolumeIdentityUpdate::Label(Some(updated_label.into()))),
    )
    .unwrap();
    assert_eq!(
        expect_identity_label(query_identity(&fs, VolumeIdentityQuery::Label).unwrap(),),
        Some(updated_label.into())
    );

    let root_directory_bytes = read_root_directory_bytes(&disk);
    let ordinary_entries_after = ordinary_root_namespace_entries(&root_directory_bytes);
    assert_eq!(ordinary_entries_after, ordinary_entries_before);

    let volume_label_index = find_root_entry_index(&root_directory_bytes, VOLUME_LABEL_ENTRY_TYPE)
        .expect("expected volume-label administrative entry");
    let skipped_label_entry = super::super::direntry::scan_directory_entry(
        true,
        &root_directory_bytes,
        volume_label_index,
    )
    .unwrap();
    assert!(scanned_entry_start_index(skipped_label_entry) > volume_label_index);

    if let Some(volume_guid_index) =
        find_root_entry_index(&root_directory_bytes, VOLUME_GUID_ENTRY_TYPE)
    {
        let skipped_guid_entry = super::super::direntry::scan_directory_entry(
            true,
            &root_directory_bytes,
            volume_guid_index,
        )
        .unwrap();
        assert!(scanned_entry_start_index(skipped_guid_entry) > volume_guid_index);
    }
}

#[ktest]
fn volume_admin_identity_unsupported_guid_and_trim_requests_preserve_state() {
    init_mount_volume_state_test_runtime();

    let disk = ExfatRefactorMemoryDisk::new();
    let (fs, _, _, _) = mounted_fs(
        &disk,
        ExfatMountOptions {
            discard: true,
            ..default_mount_options()
        },
    );

    assert_request_errno_and_state_stable(
        &fs,
        CapSet::SYS_ADMIN,
        VolumeAdminRequest::TrimFreeSpace,
        Errno::EOPNOTSUPP,
    );
    assert_eq!(
        query_identity(&fs, VolumeIdentityQuery::Guid)
            .unwrap_err()
            .error(),
        Errno::EOPNOTSUPP
    );
    assert_eq!(
        query_identity(&fs, VolumeIdentityQuery::LabelAndGuid)
            .unwrap_err()
            .error(),
        Errno::EOPNOTSUPP,
    );
    assert_request_errno_and_state_stable(
        &fs,
        CapSet::SYS_ADMIN,
        VolumeAdminRequest::UpdateIdentity(VolumeIdentityUpdate::Guid(Some([0xAB; 16]))),
        Errno::EOPNOTSUPP,
    );
    assert_request_errno_and_state_stable(
        &fs,
        CapSet::SYS_ADMIN,
        VolumeAdminRequest::UpdateIdentity(VolumeIdentityUpdate::LabelAndGuid {
            guid: Some([0xCD; 16]),
            label: Some(String::from("VOLADMIN01")),
        }),
        Errno::EOPNOTSUPP,
    );
    assert_request_errno_and_state_stable(
        &fs,
        CapSet::SYS_ADMIN,
        VolumeAdminRequest::TrimFreeSpace,
        Errno::EOPNOTSUPP,
    );
}

#[ktest]
fn volume_admin_identity_permission_and_read_only_refusals_preserve_state() {
    init_mount_volume_state_test_runtime();

    let writable_disk = ExfatRefactorMemoryDisk::new();
    let (writable_fs, _, _, _) = mounted_fs(&writable_disk, default_mount_options());
    let attempted_label = next_volume_label(
        expect_identity_label(query_identity(&writable_fs, VolumeIdentityQuery::Label).unwrap())
            .as_deref(),
    );

    assert_request_errno_and_state_stable(
        &writable_fs,
        CapSet::empty(),
        VolumeAdminRequest::UpdateIdentity(VolumeIdentityUpdate::Label(Some(
            attempted_label.into(),
        ))),
        Errno::EPERM,
    );

    let read_only_disk = ExfatRefactorMemoryDisk::new();
    let (read_only_fs, _, _, _) = mounted_fs(
        &read_only_disk,
        ExfatMountOptions {
            discard: true,
            fs_flags: FsFlags::RDONLY,
            ..default_mount_options()
        },
    );

    assert_request_errno_and_state_stable(
        &read_only_fs,
        CapSet::SYS_ADMIN,
        VolumeAdminRequest::UpdateIdentity(VolumeIdentityUpdate::Label(Some(String::from(
            "VOLADMIN01",
        )))),
        Errno::EROFS,
    );
    assert_request_errno_and_state_stable(
        &read_only_fs,
        CapSet::SYS_ADMIN,
        VolumeAdminRequest::TrimFreeSpace,
        Errno::EROFS,
    );
}

#[ktest]
fn volume_admin_identity_oversized_label_update_preserves_prior_identity() {
    init_mount_volume_state_test_runtime();

    let disk = ExfatRefactorMemoryDisk::new();
    let (fs, _, _, _) = mounted_fs(&disk, default_mount_options());

    assert_request_errno_and_state_stable(
        &fs,
        CapSet::SYS_ADMIN,
        VolumeAdminRequest::UpdateIdentity(VolumeIdentityUpdate::Label(Some(String::from(
            "VOLADMIN0123",
        )))),
        Errno::EINVAL,
    );
}

#[ktest]
fn volume_admin_identity_forced_shutdown_fast_fails_follow_on_trim_and_mutation() {
    init_mount_volume_state_test_runtime();

    let disk = ExfatRefactorMemoryDisk::new();
    let (fs, _, _, _) = mounted_fs(
        &disk,
        ExfatMountOptions {
            discard: true,
            ..default_mount_options()
        },
    );

    handle_admin_request(&fs, CapSet::SYS_ADMIN, VolumeAdminRequest::ForceShutdown).unwrap();
    let post_shutdown_state = capture_volume_admin_state(&fs);
    assert!(post_shutdown_state.forced_shutdown);

    let requested_label = next_volume_label(post_shutdown_state.label.as_deref());
    let update_error = handle_admin_request(
        &fs,
        CapSet::SYS_ADMIN,
        VolumeAdminRequest::UpdateIdentity(VolumeIdentityUpdate::Label(Some(
            requested_label.into(),
        ))),
    )
    .unwrap_err();
    assert_eq!(update_error.error(), Errno::EIO);
    assert_volume_admin_state_matches(&fs, &post_shutdown_state);

    let trim_error =
        handle_admin_request(&fs, CapSet::SYS_ADMIN, VolumeAdminRequest::TrimFreeSpace)
            .unwrap_err();
    assert_eq!(trim_error.error(), Errno::EIO);
    assert_volume_admin_state_matches(&fs, &post_shutdown_state);
}

#[ktest]
fn volume_admin_identity_write_failure_preserves_prior_label_and_mount_state() {
    init_mount_volume_state_test_runtime();

    let disk = ExfatRefactorMemoryDisk::new();
    let volume_label_entry = find_directory_entry(&disk, VOLUME_LABEL_ENTRY_TYPE);
    let failing_disk =
        ExfatRefactorToggleFailingWriteDisk::new(disk.clone(), volume_label_entry.offset, 32);
    let block_device: Arc<dyn BlockDevice> = failing_disk.clone();
    let (fs, _, _, _) = mount_block_device(&block_device, default_mount_options()).unwrap();
    let before = capture_volume_admin_state(&fs);
    let requested_label = next_volume_label(before.label.as_deref());

    failing_disk.enable_failures();

    let error = handle_admin_request(
        &fs,
        CapSet::SYS_ADMIN,
        VolumeAdminRequest::UpdateIdentity(VolumeIdentityUpdate::Label(Some(
            requested_label.into(),
        ))),
    )
    .unwrap_err();
    assert_eq!(error.error(), Errno::EIO);
    assert_volume_admin_state_matches(&fs, &before);
    assert_eq!(
        expect_identity_label(query_identity(&fs, VolumeIdentityQuery::Label).unwrap(),),
        before.label
    );
}
