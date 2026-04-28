// SPDX-License-Identifier: MPL-2.0

use core::sync::atomic::{AtomicBool, Ordering};

use ostd::prelude::ktest;

use super::{
    super::super::{
        fs::{ExfatFs, ExfatMountOptions},
        volume::{
            VolumeAdminRequest, VolumeIdentityQuery, VolumeIdentityUpdate,
        },
    },
    ExfatRefactorMemoryDisk, ExfatRefactorToggleFailingWriteDisk,
    assert_request_errno_and_state_stable, assert_volume_admin_state_matches,
    capture_volume_admin_state, default_mount_options, expect_identity_label, find_directory_entry,
    handle_admin_request, init_mount_volume_state_test_runtime, mount_block_device, mounted_fs,
    next_volume_label, ordinary_root_namespace_entries, query_identity, read_root_directory_bytes,
};
use crate::{
    fs::vfs::file_system::FsFlags,
    prelude::*,
    process::credentials::capabilities::CapSet,
    thread::{Thread, kernel_thread::ThreadOptions},
};

const OVERSIZED_VOLUME_LABEL: &str = "0123456789AB";
const VOLUME_LABEL_ENTRY_TYPE: u8 = 0x83;

#[derive(Clone, Debug, Eq, PartialEq)]
enum VolumeAdminOutcome {
    Errno(Errno),
    Success,
}

fn classify_admin_call(result: Result<()>) -> VolumeAdminOutcome {
    match result {
        Ok(()) => VolumeAdminOutcome::Success,
        Err(error) => VolumeAdminOutcome::Errno(error.error()),
    }
}

fn repeated_label_query(fs: &Arc<ExfatFs>, expected_label: Option<&str>) {
    for _ in 0..4 {
        let label = expect_identity_label(query_identity(fs, VolumeIdentityQuery::Label).unwrap());
        assert_eq!(label.as_deref(), expected_label);
    }
}

#[ktest]
fn volume_admin_identity_integration_query_update_requery_then_trim_preserves_admin_boundary_state()
{
    init_mount_volume_state_test_runtime();

    let disk = ExfatRefactorMemoryDisk::new();
    let ordinary_entries_before =
        ordinary_root_namespace_entries(&read_root_directory_bytes(&disk));
    let (fs, _, _, _) = mounted_fs(&disk, default_mount_options());

    let initial_label =
        expect_identity_label(query_identity(&fs, VolumeIdentityQuery::Label).unwrap());
    let updated_label = next_volume_label(initial_label.as_deref());

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

    assert_request_errno_and_state_stable(
        &fs,
        CapSet::SYS_ADMIN,
        VolumeAdminRequest::TrimFreeSpace,
        Errno::EOPNOTSUPP,
    );
    assert_eq!(
        ordinary_root_namespace_entries(&read_root_directory_bytes(&disk)),
        ordinary_entries_before
    );
}

#[ktest]
fn volume_admin_identity_integration_failure_classes_preserve_identity_and_allocator_state() {
    init_mount_volume_state_test_runtime();

    let disk = ExfatRefactorMemoryDisk::new();
    let (fs, _, _, _) = mounted_fs(&disk, default_mount_options());

    assert_eq!(
        query_identity(&fs, VolumeIdentityQuery::Guid)
            .unwrap_err()
            .error(),
        Errno::EOPNOTSUPP
    );
    assert_request_errno_and_state_stable(
        &fs,
        CapSet::SYS_ADMIN,
        VolumeAdminRequest::UpdateIdentity(VolumeIdentityUpdate::Guid(None)),
        Errno::EOPNOTSUPP,
    );
    assert_request_errno_and_state_stable(
        &fs,
        CapSet::empty(),
        VolumeAdminRequest::UpdateIdentity(VolumeIdentityUpdate::Label(Some("VOLADMIN02".into()))),
        Errno::EPERM,
    );
    assert_request_errno_and_state_stable(
        &fs,
        CapSet::SYS_ADMIN,
        VolumeAdminRequest::UpdateIdentity(VolumeIdentityUpdate::Label(Some(
            OVERSIZED_VOLUME_LABEL.into(),
        ))),
        Errno::EINVAL,
    );

    let read_only_disk = ExfatRefactorMemoryDisk::new();
    let read_only_options = ExfatMountOptions {
        fs_flags: FsFlags::RDONLY,
        ..default_mount_options()
    };
    let (read_only_fs, _, _, _) = mounted_fs(&read_only_disk, read_only_options);
    assert_request_errno_and_state_stable(
        &read_only_fs,
        CapSet::SYS_ADMIN,
        VolumeAdminRequest::TrimFreeSpace,
        Errno::EROFS,
    );

    let write_failure_disk = ExfatRefactorMemoryDisk::new();
    let label_entry = find_directory_entry(&write_failure_disk, VOLUME_LABEL_ENTRY_TYPE);
    let failing_disk = ExfatRefactorToggleFailingWriteDisk::new(
        write_failure_disk.clone(),
        label_entry.offset,
        32,
    );
    let block_device: Arc<dyn BlockDevice> = failing_disk.clone();
    let (failing_fs, _, _, _) = mount_block_device(&block_device, default_mount_options()).unwrap();
    let before_failure = capture_volume_admin_state(&failing_fs);
    let next_label = next_volume_label(
        expect_identity_label(query_identity(&failing_fs, VolumeIdentityQuery::Label).unwrap())
            .as_deref(),
    );

    failing_disk.enable_failures();
    let error = handle_admin_request(
        &failing_fs,
        CapSet::SYS_ADMIN,
        VolumeAdminRequest::UpdateIdentity(VolumeIdentityUpdate::Label(Some(next_label.into()))),
    )
    .unwrap_err();
    assert_eq!(error.error(), Errno::EIO);
    assert_volume_admin_state_matches(&failing_fs, &before_failure);
}

#[ktest]
fn volume_admin_identity_integration_repeated_queries_stay_stable_for_present_and_unset_identity() {
    init_mount_volume_state_test_runtime();

    let disk = ExfatRefactorMemoryDisk::new();
    let ordinary_entries_before =
        ordinary_root_namespace_entries(&read_root_directory_bytes(&disk));
    let (fs, _, _, _) = mounted_fs(&disk, default_mount_options());
    let updated_label = next_volume_label(
        expect_identity_label(query_identity(&fs, VolumeIdentityQuery::Label).unwrap()).as_deref(),
    );

    handle_admin_request(
        &fs,
        CapSet::SYS_ADMIN,
        VolumeAdminRequest::UpdateIdentity(VolumeIdentityUpdate::Label(Some(updated_label.into()))),
    )
    .unwrap();
    repeated_label_query(&fs, Some(updated_label));

    handle_admin_request(
        &fs,
        CapSet::SYS_ADMIN,
        VolumeAdminRequest::UpdateIdentity(VolumeIdentityUpdate::Label(None)),
    )
    .unwrap();
    repeated_label_query(&fs, None);

    assert_eq!(
        ordinary_root_namespace_entries(&read_root_directory_bytes(&disk)),
        ordinary_entries_before
    );
}

#[ktest]
fn volume_admin_identity_integration_concurrent_update_shutdown_and_trim_linearize_without_deadlock()
 {
    init_mount_volume_state_test_runtime();

    let disk = ExfatRefactorMemoryDisk::new();
    let (fs, _, _, _) = mounted_fs(&disk, default_mount_options());
    let initial_label =
        expect_identity_label(query_identity(&fs, VolumeIdentityQuery::Label).unwrap());
    let updated_label = String::from(next_volume_label(initial_label.as_deref()));
    let start = Arc::new(AtomicBool::new(false));
    let mutation_outcome = Arc::new(Mutex::new(None));
    let shutdown_outcome = Arc::new(Mutex::new(None));
    let trim_outcome = Arc::new(Mutex::new(None));

    let mutation_thread = {
        let fs = fs.clone();
        let start = start.clone();
        let mutation_outcome = mutation_outcome.clone();
        let updated_label = updated_label.clone();

        ThreadOptions::new(move || {
            while !start.load(Ordering::Relaxed) {
                Thread::yield_now();
            }
            *mutation_outcome.lock() = Some(classify_admin_call(handle_admin_request(
                &fs,
                CapSet::SYS_ADMIN,
                VolumeAdminRequest::UpdateIdentity(VolumeIdentityUpdate::Label(Some(
                    updated_label,
                ))),
            )));
        })
        .spawn()
    };

    let shutdown_thread = {
        let fs = fs.clone();
        let start = start.clone();
        let shutdown_outcome = shutdown_outcome.clone();

        ThreadOptions::new(move || {
            while !start.load(Ordering::Relaxed) {
                Thread::yield_now();
            }
            *shutdown_outcome.lock() = Some(classify_admin_call(handle_admin_request(
                &fs,
                CapSet::SYS_ADMIN,
                VolumeAdminRequest::ForceShutdown,
            )));
        })
        .spawn()
    };

    let trim_thread = {
        let fs = fs.clone();
        let start = start.clone();
        let trim_outcome = trim_outcome.clone();

        ThreadOptions::new(move || {
            while !start.load(Ordering::Relaxed) {
                Thread::yield_now();
            }
            *trim_outcome.lock() = Some(classify_admin_call(handle_admin_request(
                &fs,
                CapSet::SYS_ADMIN,
                VolumeAdminRequest::TrimFreeSpace,
            )));
        })
        .spawn()
    };

    start.store(true, Ordering::Relaxed);
    mutation_thread.join();
    shutdown_thread.join();
    trim_thread.join();

    let mutation_outcome = mutation_outcome.lock().clone().unwrap();
    let shutdown_outcome = shutdown_outcome.lock().clone().unwrap();
    let trim_outcome = trim_outcome.lock().clone().unwrap();

    assert_eq!(shutdown_outcome, VolumeAdminOutcome::Success);
    assert!(matches!(
        mutation_outcome,
        VolumeAdminOutcome::Success | VolumeAdminOutcome::Errno(Errno::EIO)
    ));
    assert!(matches!(
        trim_outcome,
        VolumeAdminOutcome::Errno(Errno::EOPNOTSUPP) | VolumeAdminOutcome::Errno(Errno::EIO)
    ));
    assert!(fs.state.read().as_ref().unwrap().forced_shutdown);

    let queried_label =
        expect_identity_label(query_identity(&fs, VolumeIdentityQuery::Label).unwrap());
    match mutation_outcome {
        VolumeAdminOutcome::Success => {
            assert_eq!(queried_label.as_deref(), Some(updated_label.as_str()));
        }
        VolumeAdminOutcome::Errno(Errno::EIO) => {
            assert_eq!(queried_label, initial_label);
        }
        other => panic!("unexpected mutation outcome: {:?}", other),
    }

    assert_request_errno_and_state_stable(
        &fs,
        CapSet::SYS_ADMIN,
        VolumeAdminRequest::TrimFreeSpace,
        Errno::EIO,
    );
    assert_request_errno_and_state_stable(
        &fs,
        CapSet::SYS_ADMIN,
        VolumeAdminRequest::UpdateIdentity(VolumeIdentityUpdate::Label(Some(
            next_volume_label(queried_label.as_deref()).into(),
        ))),
        Errno::EIO,
    );
}
