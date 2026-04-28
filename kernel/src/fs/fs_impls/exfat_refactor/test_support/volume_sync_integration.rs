// SPDX-License-Identifier: MPL-2.0

use alloc::{sync::Arc, vec::Vec};
use core::{
    fmt,
    sync::atomic::{AtomicBool, Ordering},
};

use aster_block::{
    BlockDevice, BlockDeviceMeta,
    bio::{BioEnqueueError, BioStatus, BioType, SubmittedBio},
};
use device_id::DeviceId;
use ostd::mm::io::util::HasVmReaderWriter;
use spin::Mutex;

use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ObservedMountState {
    clear_to_zero: bool,
    flags: FsFlags,
    forced_shutdown: bool,
    media_failure: bool,
    volume_dirty: bool,
}

struct ExfatRefactorFlushControlDisk {
    block_flush: AtomicBool,
    flush_started: AtomicBool,
    inner: Arc<ExfatRefactorMemoryDisk>,
}

impl ExfatRefactorFlushControlDisk {
    fn new(inner: Arc<ExfatRefactorMemoryDisk>) -> Arc<Self> {
        Arc::new(Self {
            block_flush: AtomicBool::new(false),
            flush_started: AtomicBool::new(false),
            inner,
        })
    }

    fn enable_blocking_flush(&self) {
        self.flush_started.store(false, Ordering::Relaxed);
        self.block_flush.store(true, Ordering::Relaxed);
    }

    fn flush_started(&self) -> bool {
        self.flush_started.load(Ordering::Relaxed)
    }

    fn release_blocked_flush(&self) {
        self.block_flush.store(false, Ordering::Relaxed);
    }
}

impl fmt::Debug for ExfatRefactorFlushControlDisk {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ExfatRefactorFlushControlDisk")
            .field("block_flush", &self.block_flush.load(Ordering::Relaxed))
            .field("flush_started", &self.flush_started.load(Ordering::Relaxed))
            .field("sectors_count", &self.inner.sectors_count())
            .finish()
    }
}

impl BlockDevice for ExfatRefactorFlushControlDisk {
    fn enqueue(&self, bio: SubmittedBio) -> core::result::Result<(), BioEnqueueError> {
        let bio_type = bio.type_();
        if bio_type == BioType::Flush {
            self.flush_started.store(true, Ordering::Relaxed);
            while self.block_flush.load(Ordering::Relaxed) {
                Thread::yield_now();
            }
            bio.complete(BioStatus::Complete);
            return Ok(());
        }

        let mut current_offset = bio.sid_range().start.to_offset();
        for segment in bio.segments() {
            let size = match bio_type {
                BioType::Read => segment
                    .inner_dma_slice()
                    .writer()
                    .unwrap()
                    .write(self.inner.blocks.reader().skip(current_offset)),
                BioType::Write => self
                    .inner
                    .blocks
                    .writer()
                    .skip(current_offset)
                    .write(&mut segment.inner_dma_slice().reader().unwrap()),
                _ => 0,
            };
            current_offset += size;
        }
        bio.complete(BioStatus::Complete);
        Ok(())
    }

    fn metadata(&self) -> BlockDeviceMeta {
        self.inner.metadata()
    }

    fn name(&self) -> &str {
        "exfat-refactor-flush-control-test"
    }

    fn id(&self) -> DeviceId {
        DeviceId::null()
    }
}

fn expected_mount_state(
    expected_flags: FsFlags,
    expected_volume_flags: u16,
    forced_shutdown: bool,
) -> ObservedMountState {
    ObservedMountState {
        clear_to_zero: expected_volume_flags & 0x0008 != 0,
        flags: expected_flags,
        forced_shutdown,
        media_failure: expected_volume_flags & 0x0004 != 0,
        volume_dirty: expected_volume_flags & 0x0002 != 0,
    }
}

fn observed_mount_state(fs: &Arc<ExfatFs>) -> ObservedMountState {
    let state = fs.state.read();
    let publication = state.as_ref().unwrap();
    ObservedMountState {
        clear_to_zero: publication.anomaly.clear_to_zero,
        flags: publication.flags,
        forced_shutdown: publication.forced_shutdown,
        media_failure: publication.anomaly.media_failure,
        volume_dirty: publication.anomaly.volume_dirty,
    }
}

fn assert_mount_surface_stable(
    fs: &Arc<ExfatFs>,
    root_inode: &Arc<dyn Inode>,
    super_block: &SuperBlock,
) {
    assert!(Arc::ptr_eq(root_inode, &fs.root_inode()));
    assert_same_super_block(super_block, &fs.sb());
}

#[ktest]
fn filesystem_sync_and_volume_state_integration_success_path_preserves_root_publication_and_clears_dirty_only_after_sync()
 {
    init_mount_volume_state_test_runtime();

    let disk = ExfatRefactorMemoryDisk::new();
    disk.write_bytes(106, &CLEAN_TEST_VOLUME_FLAGS.to_le_bytes());
    let (fs, root_inode, super_block, flags) = mounted_fs(&disk, default_mount_options());

    assert_eq!(flags, FsFlags::empty());
    assert_eq!(
        observed_mount_state(&fs),
        expected_mount_state(FsFlags::empty(), CLEAN_TEST_VOLUME_FLAGS, false)
    );
    assert_eq!(boot_volume_flags(&disk), CLEAN_TEST_VOLUME_FLAGS);

    let (state_guard, _, _, anomaly, _, _) = fs.admitted_mutation_state().unwrap();
    assert!(anomaly.volume_dirty);
    assert!(anomaly.media_failure);
    assert!(anomaly.clear_to_zero);
    assert_eq!(boot_volume_flags(&disk), TEST_VOLUME_FLAGS);
    drop(state_guard);

    assert_eq!(
        observed_mount_state(&fs),
        expected_mount_state(FsFlags::empty(), TEST_VOLUME_FLAGS, false)
    );

    fs.sync().unwrap();

    assert_eq!(boot_volume_flags(&disk), CLEAN_TEST_VOLUME_FLAGS);
    assert_eq!(
        observed_mount_state(&fs),
        expected_mount_state(FsFlags::empty(), CLEAN_TEST_VOLUME_FLAGS, false)
    );
    assert_mount_surface_stable(&fs, &root_inode, &super_block);
}

#[ktest]
fn filesystem_sync_and_volume_state_integration_failure_maintenance_preserves_conservative_state_across_sync_failure_and_shutdown_suppression()
 {
    init_mount_volume_state_test_runtime();

    let sync_failure_disk = ExfatRefactorMemoryDisk::new();
    sync_failure_disk.write_bytes(106, &TEST_VOLUME_FLAGS.to_le_bytes());
    let failing_flush_disk =
        ExfatRefactorCountedFailingFlushDisk::new(sync_failure_disk.clone(), 2);
    let block_device: Arc<dyn BlockDevice> = failing_flush_disk;
    let (sync_failure_fs, root_inode, super_block, flags) =
        mount_block_device(&block_device, default_mount_options()).unwrap();

    assert_eq!(flags, FsFlags::empty());
    assert_eq!(
        observed_mount_state(&sync_failure_fs),
        expected_mount_state(FsFlags::empty(), TEST_VOLUME_FLAGS, false)
    );

    let sync_error = sync_failure_fs.sync().unwrap_err();
    assert_eq!(sync_error.error(), Errno::EIO);
    assert_eq!(
        boot_volume_flags(&sync_failure_disk),
        CLEAN_TEST_VOLUME_FLAGS
    );
    assert_eq!(
        observed_mount_state(&sync_failure_fs),
        expected_mount_state(FsFlags::empty(), TEST_VOLUME_FLAGS, false)
    );
    assert_mount_surface_stable(&sync_failure_fs, &root_inode, &super_block);

    let shutdown_disk = ExfatRefactorMemoryDisk::new();
    shutdown_disk.write_bytes(106, &TEST_VOLUME_FLAGS.to_le_bytes());
    let shutdown_options = ExfatMountOptions {
        discard: true,
        ..default_mount_options()
    };
    let (shutdown_fs, shutdown_root_inode, shutdown_super_block, shutdown_flags) =
        mounted_fs(&shutdown_disk, shutdown_options);

    assert_eq!(shutdown_flags, FsFlags::empty());
    shutdown_fs.admit_forced_shutdown().unwrap();

    assert_eq!(
        shutdown_fs.admitted_mutation_state().err(),
        Some(MountVolumeStateError::DeviceIo)
    );
    assert_eq!(
        shutdown_fs
            .administrative_trim_free_space()
            .unwrap_err()
            .error(),
        Errno::EIO
    );
    assert_eq!(shutdown_fs.sync().unwrap_err().error(), Errno::EIO);
    assert_eq!(boot_volume_flags(&shutdown_disk), TEST_VOLUME_FLAGS);
    assert_eq!(
        observed_mount_state(&shutdown_fs),
        expected_mount_state(FsFlags::empty(), TEST_VOLUME_FLAGS, true)
    );
    assert_mount_surface_stable(&shutdown_fs, &shutdown_root_inode, &shutdown_super_block);
}

#[ktest]
fn filesystem_sync_and_volume_state_integration_repeated_sync_and_observation_remain_stable_without_new_dirty_work()
 {
    init_mount_volume_state_test_runtime();

    let quiesced_disk = ExfatRefactorMemoryDisk::new();
    quiesced_disk.write_bytes(106, &0u16.to_le_bytes());
    let (quiesced_fs, quiesced_root_inode, quiesced_super_block, quiesced_flags) =
        mounted_fs(&quiesced_disk, default_mount_options());

    assert_eq!(quiesced_flags, FsFlags::empty());
    for _ in 0..3 {
        quiesced_fs.sync().unwrap();
        assert_eq!(boot_volume_flags(&quiesced_disk), 0);
        assert_eq!(
            observed_mount_state(&quiesced_fs),
            expected_mount_state(FsFlags::empty(), 0, false)
        );
        assert_mount_surface_stable(&quiesced_fs, &quiesced_root_inode, &quiesced_super_block);
    }

    let overlay_disk = ExfatRefactorMemoryDisk::new();
    overlay_disk.write_bytes(106, &CLEAN_TEST_VOLUME_FLAGS.to_le_bytes());
    let (overlay_fs, overlay_root_inode, overlay_super_block, overlay_flags) =
        mounted_fs(&overlay_disk, default_mount_options());

    assert_eq!(overlay_flags, FsFlags::empty());
    for _ in 0..3 {
        overlay_fs.sync().unwrap();
        assert_eq!(boot_volume_flags(&overlay_disk), CLEAN_TEST_VOLUME_FLAGS);
        assert_eq!(
            observed_mount_state(&overlay_fs),
            expected_mount_state(FsFlags::empty(), CLEAN_TEST_VOLUME_FLAGS, false)
        );
        assert_mount_surface_stable(&overlay_fs, &overlay_root_inode, &overlay_super_block);
    }
}

#[ktest]
fn filesystem_sync_and_volume_state_integration_concurrent_sync_observation_and_shutdown_linearize_without_deadlock()
 {
    init_mount_volume_state_test_runtime();

    let disk = ExfatRefactorMemoryDisk::new();
    disk.write_bytes(106, &TEST_VOLUME_FLAGS.to_le_bytes());
    let flush_control_disk = ExfatRefactorFlushControlDisk::new(disk.clone());
    let block_device: Arc<dyn BlockDevice> = flush_control_disk.clone();
    let (fs, root_inode, super_block, flags) =
        mount_block_device(&block_device, default_mount_options()).unwrap();

    assert_eq!(flags, FsFlags::empty());
    assert_eq!(
        observed_mount_state(&fs),
        expected_mount_state(FsFlags::empty(), TEST_VOLUME_FLAGS, false)
    );

    flush_control_disk.enable_blocking_flush();

    let sync_result = Arc::new(Mutex::new(None));
    let observer_started = Arc::new(AtomicBool::new(false));
    let observer_done = Arc::new(AtomicBool::new(false));
    let shutdown_started = Arc::new(AtomicBool::new(false));
    let shutdown_done = Arc::new(AtomicBool::new(false));
    let observed_states = Arc::new(Mutex::new(Vec::new()));

    let sync_thread = {
        let fs = fs.clone();
        let sync_result = sync_result.clone();
        ThreadOptions::new(move || {
            *sync_result.lock() = Some(fs.sync().map_err(|error| error.error()));
        })
        .spawn()
    };

    for _ in 0..10_000 {
        if flush_control_disk.flush_started() {
            break;
        }
        Thread::yield_now();
    }
    assert!(flush_control_disk.flush_started());

    let observer_thread = {
        let fs = fs.clone();
        let observer_done = observer_done.clone();
        let observer_started = observer_started.clone();
        let observed_states = observed_states.clone();
        let shutdown_done = shutdown_done.clone();
        let root_ino = root_inode.ino();
        let super_block = super_block.clone();

        ThreadOptions::new(move || {
            observer_started.store(true, Ordering::Relaxed);
            let mut saw_shutdown_state = false;
            for _ in 0..2_048 {
                let observed_state = observed_mount_state(&fs);
                assert_eq!(fs.root_inode().ino(), root_ino);
                assert_same_super_block(&super_block, &fs.sb());
                saw_shutdown_state |= observed_state.forced_shutdown;
                observed_states.lock().push(observed_state);
                if shutdown_done.load(Ordering::Relaxed) && saw_shutdown_state {
                    break;
                }
                Thread::yield_now();
            }
            observer_done.store(true, Ordering::Relaxed);
        })
        .spawn()
    };

    let shutdown_thread = {
        let fs = fs.clone();
        let shutdown_done = shutdown_done.clone();
        let shutdown_started = shutdown_started.clone();
        ThreadOptions::new(move || {
            shutdown_started.store(true, Ordering::Relaxed);
            fs.admit_forced_shutdown().unwrap();
            shutdown_done.store(true, Ordering::Relaxed);
        })
        .spawn()
    };

    for _ in 0..10_000 {
        if observer_started.load(Ordering::Relaxed) && shutdown_started.load(Ordering::Relaxed) {
            break;
        }
        Thread::yield_now();
    }
    assert!(observer_started.load(Ordering::Relaxed));
    assert!(shutdown_started.load(Ordering::Relaxed));

    for _ in 0..512 {
        Thread::yield_now();
    }
    assert!(!observer_done.load(Ordering::Relaxed));
    assert!(!shutdown_done.load(Ordering::Relaxed));

    flush_control_disk.release_blocked_flush();

    sync_thread.join();
    observer_thread.join();
    shutdown_thread.join();

    assert_eq!(*sync_result.lock(), Some(Ok(())));
    assert_eq!(fs.sync().unwrap_err().error(), Errno::EIO);
    assert_eq!(boot_volume_flags(&disk), CLEAN_TEST_VOLUME_FLAGS);
    assert_eq!(
        observed_mount_state(&fs),
        expected_mount_state(FsFlags::empty(), CLEAN_TEST_VOLUME_FLAGS, true)
    );
    assert_mount_surface_stable(&fs, &root_inode, &super_block);

    let expected_clean_state =
        expected_mount_state(FsFlags::empty(), CLEAN_TEST_VOLUME_FLAGS, false);
    let expected_shutdown_state =
        expected_mount_state(FsFlags::empty(), CLEAN_TEST_VOLUME_FLAGS, true);
    let observed_states = observed_states.lock();
    assert!(!observed_states.is_empty());
    assert!(observed_states.iter().all(|observed_state| {
        *observed_state == expected_clean_state || *observed_state == expected_shutdown_state
    }));
    assert!(observed_states.contains(&expected_shutdown_state));
}
