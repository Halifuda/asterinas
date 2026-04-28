// SPDX-License-Identifier: MPL-2.0

use alloc::{collections::BTreeSet, string::String, sync::Arc, vec, vec::Vec};
use core::{
    fmt,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
};

use aster_block::{
    BlockDevice, BlockDeviceMeta, SECTOR_SIZE,
    bio::{BioEnqueueError, BioStatus, BioType, SubmittedBio},
};
use device_id::DeviceId;
use ostd::{
    mm::{FrameAllocOptions, HasSize, PAGE_SIZE, Segment, VmIo, io::util::HasVmReaderWriter},
    prelude::ktest,
};

use super::*;
use crate::{
    fs::{
        file::InodeType,
        vfs::{file_system::FsFlags, inode::Inode},
    },
    thread::{Thread, kernel_thread::ThreadOptions},
};

const ALLOCATION_BITMAP_ENTRY_TYPE: u8 = 0x81;
const CLEAN_TEST_VOLUME_FLAGS: u16 = TEST_VOLUME_FLAGS & !0x0002;
const END_OF_DIRECTORY_ENTRY_TYPE: u8 = 0x00;
const TEST_VOLUME_FLAGS: u16 = 0x000E;
const UPCASE_TABLE_ENTRY_TYPE: u8 = 0x82;
static EXFAT_IMAGE: &[u8] = include_bytes!("../../../../../../test/initramfs/build/exfat.img");

#[derive(Clone)]
struct DirectoryEntryLocation {
    boot_region: super::super::boot::BootRegion,
    offset: usize,
}

struct ExfatRefactorMemoryDisk {
    blocks: Segment<()>,
}

impl ExfatRefactorMemoryDisk {
    fn new() -> Arc<Self> {
        let blocks = FrameAllocOptions::new()
            .zeroed(false)
            .alloc_segment(EXFAT_IMAGE.len().div_ceil(PAGE_SIZE))
            .unwrap();
        blocks.write_bytes(0, EXFAT_IMAGE).unwrap();
        Arc::new(Self { blocks })
    }

    fn read_bytes(&self, offset: usize, len: usize) -> Vec<u8> {
        let mut bytes = vec![0; len];
        self.blocks.read_bytes(offset, &mut bytes).unwrap();
        bytes
    }

    fn sectors_count(&self) -> usize {
        self.blocks.size() / SECTOR_SIZE
    }

    fn write_bytes(&self, offset: usize, bytes: &[u8]) {
        self.blocks.write_bytes(offset, bytes).unwrap();
    }
}

impl fmt::Debug for ExfatRefactorMemoryDisk {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ExfatRefactorMemoryDisk")
            .field("sectors_count", &self.sectors_count())
            .finish()
    }
}

struct ExfatRefactorFailingReadDisk {
    fail_range: core::ops::Range<usize>,
    inner: Arc<ExfatRefactorMemoryDisk>,
}

struct ExfatRefactorToggleFailingReadDisk {
    fail_range: core::ops::Range<usize>,
    fail_reads: AtomicBool,
    inner: Arc<ExfatRefactorMemoryDisk>,
}

struct ExfatRefactorToggleFailingWriteDisk {
    fail_range: core::ops::Range<usize>,
    fail_writes: AtomicBool,
    inner: Arc<ExfatRefactorMemoryDisk>,
}

struct ExfatRefactorCountedFailingFlushDisk {
    fail_flush_number: usize,
    flush_count: AtomicUsize,
    inner: Arc<ExfatRefactorMemoryDisk>,
}

impl ExfatRefactorFailingReadDisk {
    fn new(inner: Arc<ExfatRefactorMemoryDisk>, fail_offset: usize, fail_len: usize) -> Arc<Self> {
        Arc::new(Self {
            fail_range: fail_offset..fail_offset.checked_add(fail_len).unwrap(),
            inner,
        })
    }

    fn overlaps_failure_range(&self, start: usize, end: usize) -> bool {
        start < self.fail_range.end && self.fail_range.start < end
    }
}

impl ExfatRefactorToggleFailingReadDisk {
    fn new(inner: Arc<ExfatRefactorMemoryDisk>, fail_offset: usize, fail_len: usize) -> Arc<Self> {
        Arc::new(Self {
            fail_range: fail_offset..fail_offset.checked_add(fail_len).unwrap(),
            fail_reads: AtomicBool::new(false),
            inner,
        })
    }

    fn enable_failures(&self) {
        self.fail_reads.store(true, Ordering::Relaxed);
    }

    fn overlaps_failure_range(&self, start: usize, end: usize) -> bool {
        start < self.fail_range.end && self.fail_range.start < end
    }
}

impl ExfatRefactorToggleFailingWriteDisk {
    fn new(inner: Arc<ExfatRefactorMemoryDisk>, fail_offset: usize, fail_len: usize) -> Arc<Self> {
        Arc::new(Self {
            fail_range: fail_offset..fail_offset.checked_add(fail_len).unwrap(),
            fail_writes: AtomicBool::new(false),
            inner,
        })
    }

    fn enable_failures(&self) {
        self.fail_writes.store(true, Ordering::Relaxed);
    }

    fn overlaps_failure_range(&self, start: usize, end: usize) -> bool {
        start < self.fail_range.end && self.fail_range.start < end
    }
}

impl ExfatRefactorCountedFailingFlushDisk {
    fn new(inner: Arc<ExfatRefactorMemoryDisk>, fail_flush_number: usize) -> Arc<Self> {
        Arc::new(Self {
            fail_flush_number,
            flush_count: AtomicUsize::new(0),
            inner,
        })
    }

    fn should_fail_flush(&self) -> bool {
        self.flush_count.fetch_add(1, Ordering::Relaxed) + 1 == self.fail_flush_number
    }
}

impl fmt::Debug for ExfatRefactorFailingReadDisk {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ExfatRefactorFailingReadDisk")
            .field("fail_range", &self.fail_range)
            .field("sectors_count", &self.inner.sectors_count())
            .finish()
    }
}

impl fmt::Debug for ExfatRefactorToggleFailingReadDisk {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ExfatRefactorToggleFailingReadDisk")
            .field("fail_range", &self.fail_range)
            .field("sectors_count", &self.inner.sectors_count())
            .finish()
    }
}

impl fmt::Debug for ExfatRefactorToggleFailingWriteDisk {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ExfatRefactorToggleFailingWriteDisk")
            .field("fail_range", &self.fail_range)
            .field("sectors_count", &self.inner.sectors_count())
            .finish()
    }
}

impl fmt::Debug for ExfatRefactorCountedFailingFlushDisk {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ExfatRefactorCountedFailingFlushDisk")
            .field("fail_flush_number", &self.fail_flush_number)
            .field("sectors_count", &self.inner.sectors_count())
            .finish()
    }
}

impl BlockDevice for ExfatRefactorMemoryDisk {
    fn enqueue(&self, bio: SubmittedBio) -> core::result::Result<(), BioEnqueueError> {
        let bio_type = bio.type_();
        if bio_type == BioType::Flush {
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
                    .write(self.blocks.reader().skip(current_offset)),
                BioType::Write => self
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
        BlockDeviceMeta {
            max_nr_segments_per_bio: usize::MAX,
            nr_sectors: self.sectors_count(),
        }
    }

    fn name(&self) -> &str {
        "exfat-refactor-test"
    }

    fn id(&self) -> DeviceId {
        DeviceId::null()
    }
}

impl BlockDevice for ExfatRefactorFailingReadDisk {
    fn enqueue(&self, bio: SubmittedBio) -> core::result::Result<(), BioEnqueueError> {
        let bio_type = bio.type_();
        if bio_type == BioType::Flush {
            bio.complete(BioStatus::Complete);
            return Ok(());
        }

        let mut current_offset = bio.sid_range().start.to_offset();
        for segment in bio.segments() {
            let segment_end = current_offset.checked_add(segment.nbytes()).unwrap();
            if bio_type == BioType::Read && self.overlaps_failure_range(current_offset, segment_end)
            {
                bio.complete(BioStatus::IoError);
                return Ok(());
            }

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
        "exfat-refactor-failing-read-test"
    }

    fn id(&self) -> DeviceId {
        DeviceId::null()
    }
}

impl BlockDevice for ExfatRefactorToggleFailingReadDisk {
    fn enqueue(&self, bio: SubmittedBio) -> core::result::Result<(), BioEnqueueError> {
        let bio_type = bio.type_();
        if bio_type == BioType::Flush {
            bio.complete(BioStatus::Complete);
            return Ok(());
        }

        let mut current_offset = bio.sid_range().start.to_offset();
        for segment in bio.segments() {
            let segment_end = current_offset.checked_add(segment.nbytes()).unwrap();
            if bio_type == BioType::Read
                && self.fail_reads.load(Ordering::Relaxed)
                && self.overlaps_failure_range(current_offset, segment_end)
            {
                bio.complete(BioStatus::IoError);
                return Ok(());
            }

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
        "exfat-refactor-toggle-failing-read-test"
    }

    fn id(&self) -> DeviceId {
        DeviceId::null()
    }
}

impl BlockDevice for ExfatRefactorToggleFailingWriteDisk {
    fn enqueue(&self, bio: SubmittedBio) -> core::result::Result<(), BioEnqueueError> {
        let bio_type = bio.type_();
        if bio_type == BioType::Flush {
            bio.complete(BioStatus::Complete);
            return Ok(());
        }

        let mut current_offset = bio.sid_range().start.to_offset();
        for segment in bio.segments() {
            let segment_end = current_offset.checked_add(segment.nbytes()).unwrap();
            if bio_type == BioType::Write
                && self.fail_writes.load(Ordering::Relaxed)
                && self.overlaps_failure_range(current_offset, segment_end)
            {
                bio.complete(BioStatus::IoError);
                return Ok(());
            }

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
        "exfat-refactor-toggle-failing-write-test"
    }

    fn id(&self) -> DeviceId {
        DeviceId::null()
    }
}

impl BlockDevice for ExfatRefactorCountedFailingFlushDisk {
    fn enqueue(&self, bio: SubmittedBio) -> core::result::Result<(), BioEnqueueError> {
        let bio_type = bio.type_();
        if bio_type == BioType::Flush {
            bio.complete(if self.should_fail_flush() {
                BioStatus::IoError
            } else {
                BioStatus::Complete
            });
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
        "exfat-refactor-counted-failing-flush-test"
    }

    fn id(&self) -> DeviceId {
        DeviceId::null()
    }
}

fn assert_same_super_block(left: &SuperBlock, right: &SuperBlock) {
    assert_eq!(left.magic, right.magic);
    assert_eq!(left.bsize, right.bsize);
    assert_eq!(left.blocks, right.blocks);
    assert_eq!(left.bfree, right.bfree);
    assert_eq!(left.bavail, right.bavail);
    assert_eq!(left.files, right.files);
    assert_eq!(left.ffree, right.ffree);
    assert_eq!(left.fsid, right.fsid);
    assert_eq!(left.namelen, right.namelen);
    assert_eq!(left.frsize, right.frsize);
    assert_eq!(left.flags, right.flags);
    assert_eq!(left.container_dev_id, right.container_dev_id);
}

fn assert_snapshot_matches_super_block(snapshot: &FreeSpaceSnapshot, super_block: &SuperBlock) {
    assert_eq!(super_block.blocks, snapshot.total_clusters);
    assert_eq!(super_block.bfree, snapshot.free_clusters);
    assert_eq!(super_block.bavail, snapshot.free_clusters);
    assert_eq!(
        snapshot.total_clusters,
        snapshot
            .free_clusters
            .checked_add(snapshot.used_clusters)
            .unwrap()
    );
}

fn cluster_offset(boot_region: &super::super::boot::BootRegion, cluster: u32) -> usize {
    let cluster_index = u64::from(cluster - 2);
    let sectors_per_cluster = u64::try_from(boot_region.sectors_per_cluster).unwrap();
    let sector_index = cluster_index
        .checked_mul(sectors_per_cluster)
        .and_then(|offset| offset.checked_add(u64::from(boot_region.cluster_heap_offset_sectors)))
        .unwrap();
    let sector_size = u64::try_from(boot_region.sector_size).unwrap();
    usize::try_from(sector_index.checked_mul(sector_size).unwrap()).unwrap()
}

fn default_mount_options() -> ExfatMountOptions {
    ExfatMountOptions {
        discard: false,
        fs_flags: FsFlags::empty(),
        iocharset: String::from("utf8"),
        keep_last_dots: false,
        zero_size_dir: false,
    }
}

fn find_directory_entry(
    disk: &Arc<ExfatRefactorMemoryDisk>,
    entry_type: u8,
) -> DirectoryEntryLocation {
    let validated_mount = super::super::test_support::load_validated_mount(disk.as_ref()).unwrap();
    let boot_region = validated_mount.boot_region;
    let mut current_cluster = boot_region.root_dir_cluster;
    let mut visited_clusters = BTreeSet::new();

    loop {
        assert!(visited_clusters.insert(current_cluster));
        let cluster_offset = cluster_offset(&boot_region, current_cluster);
        let cluster_bytes = disk.read_bytes(cluster_offset, boot_region.cluster_size);
        for (index, entry) in cluster_bytes.chunks_exact(32).enumerate() {
            if entry[0] == entry_type {
                return DirectoryEntryLocation {
                    boot_region,
                    offset: cluster_offset + index * 32,
                };
            }
            if entry[0] == END_OF_DIRECTORY_ENTRY_TYPE {
                break;
            }
        }

        current_cluster =
            next_cluster(disk, &boot_region, current_cluster).expect("expected root entry");
    }
}

fn init_mount_volume_state_test_runtime() {
    crate::time::clocks::init_for_ktest();
}

fn mount_disk(
    disk: &Arc<ExfatRefactorMemoryDisk>,
    options: ExfatMountOptions,
) -> core::result::Result<(Arc<ExfatFs>, Arc<dyn Inode>, SuperBlock, FsFlags), MountVolumeStateError>
{
    let block_device: Arc<dyn BlockDevice> = disk.clone();
    mount_block_device(&block_device, options)
}

fn mount_block_device(
    block_device: &Arc<dyn BlockDevice>,
    options: ExfatMountOptions,
) -> core::result::Result<(Arc<ExfatFs>, Arc<dyn Inode>, SuperBlock, FsFlags), MountVolumeStateError>
{
    ExfatFs::mount_candidate(block_device, Some("exfat-refactor-test"), &options)
}

fn mounted_fs(
    disk: &Arc<ExfatRefactorMemoryDisk>,
    options: ExfatMountOptions,
) -> (Arc<ExfatFs>, Arc<dyn Inode>, SuperBlock, FsFlags) {
    mount_disk(disk, options).unwrap()
}

fn assert_administrative_trim_rejection_preserves_state(fs: &Arc<ExfatFs>, expected_errno: Errno) {
    let before_snapshot = fs.cached_free_space_snapshot().unwrap();
    let before_super_block = fs.sb();
    let before_options = fs.current_options().unwrap();
    let before_flags = fs.published_flags().unwrap();

    let error = fs.administrative_trim_free_space().unwrap_err();
    assert_eq!(error.error(), expected_errno);

    let after_snapshot = fs.cached_free_space_snapshot().unwrap();
    let after_super_block = fs.sb();
    let after_options = fs.current_options().unwrap();
    let after_flags = fs.published_flags().unwrap();

    assert_eq!(after_snapshot, before_snapshot);
    assert_same_super_block(&before_super_block, &after_super_block);
    assert_eq!(after_options, before_options);
    assert_eq!(after_flags, before_flags);
}

fn assert_cached_reporting_matches_snapshot(fs: &Arc<ExfatFs>, expected: &FreeSpaceSnapshot) {
    let cached_snapshot = fs.cached_free_space_snapshot().unwrap();
    let super_block = fs.sb();

    assert_eq!(cached_snapshot, *expected);
    assert_snapshot_matches_super_block(&cached_snapshot, &super_block);
}

fn assert_observed_volume_posture(
    fs: &Arc<ExfatFs>,
    expected_flags: FsFlags,
    expected_volume_flags: u16,
) {
    let state = fs.state.read();
    let publication = state.as_ref().unwrap();

    assert_eq!(publication.flags, expected_flags);
    assert_eq!(
        publication.anomaly.volume_dirty,
        expected_volume_flags & 0x0002 != 0
    );
    assert_eq!(
        publication.anomaly.media_failure,
        expected_volume_flags & 0x0004 != 0
    );
    assert_eq!(
        publication.anomaly.clear_to_zero,
        expected_volume_flags & 0x0008 != 0
    );
}

fn boot_volume_flags(disk: &Arc<ExfatRefactorMemoryDisk>) -> u16 {
    let volume_flags = disk.read_bytes(106, 2);
    u16::from_le_bytes([volume_flags[0], volume_flags[1]])
}

fn next_cluster(
    disk: &Arc<ExfatRefactorMemoryDisk>,
    boot_region: &super::super::boot::BootRegion,
    current_cluster: u32,
) -> Option<u32> {
    let fat_offset = u64::from(boot_region.fat_offset_sectors)
        .checked_mul(u64::try_from(boot_region.sector_size).unwrap())
        .unwrap();
    let entry_offset = fat_offset
        .checked_add(u64::from(current_cluster) * 4)
        .unwrap();
    let entry_bytes = disk.read_bytes(usize::try_from(entry_offset).unwrap(), 4);
    let next_cluster = u32::from_le_bytes([
        entry_bytes[0],
        entry_bytes[1],
        entry_bytes[2],
        entry_bytes[3],
    ]);
    if next_cluster >= 0xFFFF_FFF8 {
        None
    } else {
        Some(next_cluster)
    }
}

fn upcase_data_offset(disk: &Arc<ExfatRefactorMemoryDisk>) -> usize {
    let directory_entry = find_directory_entry(disk, UPCASE_TABLE_ENTRY_TYPE);
    let directory_bytes = disk.read_bytes(directory_entry.offset, 32);
    let first_cluster = u32::from_le_bytes([
        directory_bytes[20],
        directory_bytes[21],
        directory_bytes[22],
        directory_bytes[23],
    ]);
    cluster_offset(&directory_entry.boot_region, first_cluster)
}

fn allocation_bitmap_data_offset(disk: &Arc<ExfatRefactorMemoryDisk>) -> usize {
    let directory_entry = find_directory_entry(disk, ALLOCATION_BITMAP_ENTRY_TYPE);
    let directory_bytes = disk.read_bytes(directory_entry.offset, 32);
    let first_cluster = u32::from_le_bytes([
        directory_bytes[20],
        directory_bytes[21],
        directory_bytes[22],
        directory_bytes[23],
    ]);
    cluster_offset(&directory_entry.boot_region, first_cluster)
}

mod volume_admin_identity;
mod volume_admin_identity_integration;
mod volume_sync_integration;

#[ktest]
fn mount_volume_state_mount_publishes_root_inode_superblock_and_defaults() {
    init_mount_volume_state_test_runtime();

    let disk = ExfatRefactorMemoryDisk::new();
    let validated_mount = super::super::test_support::load_validated_mount(disk.as_ref())
        .unwrap_or_else(|error| {
            let gate =
                super::super::test_support::diagnose_invalid_on_disk_layout_gate(disk.as_ref());
            panic!(
                "baseline fixture load_validated_mount failed with {:?}; diagnostic gate: {}",
                error, gate
            );
        });
    let options = default_mount_options();
    let (fs, root_inode, super_block, flags) = mounted_fs(&disk, options);

    let total_clusters = validated_mount.boot_region.cluster_count_usize().unwrap();
    let free_clusters = total_clusters
        .checked_sub(validated_mount.used_clusters)
        .unwrap();

    assert_eq!(flags, FsFlags::empty());
    assert_eq!(
        root_inode.ino(),
        u64::from(validated_mount.boot_region.root_dir_cluster)
    );
    assert_eq!(root_inode.type_(), InodeType::Dir);
    assert!(Arc::ptr_eq(&root_inode, &fs.root_inode()));
    assert_eq!(super_block.magic, EXFAT_SUPER_MAGIC);
    assert_eq!(super_block.bsize, validated_mount.boot_region.cluster_size);
    assert_eq!(super_block.blocks, total_clusters);
    assert_eq!(super_block.bfree, free_clusters);
    assert_eq!(super_block.bavail, free_clusters);
    assert_eq!(
        super_block.fsid,
        u64::from(validated_mount.boot_region.volume_serial_number)
    );
    assert_eq!(
        super_block.namelen,
        super::super::upcase::UpcaseTable::NAME_MAX
    );
    assert_eq!(super_block.flags, 0);
    assert_eq!(fs.current_options().unwrap(), default_mount_options());
    let state = fs.state.read();
    let publication = state.as_ref().unwrap();
    assert!(
        publication
            .upcase_table
            .names_equal(&[u16::from(b'a')], &[u16::from(b'A')])
    );
    assert!(
        !publication
            .upcase_table
            .names_equal(&[u16::from(b'a')], &[u16::from(b'B')])
    );
}

#[ktest]
fn mount_volume_state_root_and_superblock_reads_are_stable() {
    init_mount_volume_state_test_runtime();

    let disk = ExfatRefactorMemoryDisk::new();
    let (fs, root_inode, super_block, _) = mounted_fs(&disk, default_mount_options());

    for _ in 0..3 {
        let reread_root = fs.root_inode();
        assert!(Arc::ptr_eq(&root_inode, &reread_root));

        let reread_super_block = fs.sb();
        assert_same_super_block(&super_block, &reread_super_block);
    }
}

#[ktest]
fn mount_volume_state_recount_fallback_marks_cached_accounting() {
    init_mount_volume_state_test_runtime();

    let disk = ExfatRefactorMemoryDisk::new();
    disk.write_bytes(112, &[0xFF]);
    let (fs, _, super_block, _) = mounted_fs(&disk, default_mount_options());

    let allocator = fs.allocator.read();
    let allocator_state = allocator.as_ref().unwrap();

    assert!(allocator_state.used_clusters_from_recount);
    assert_eq!(
        super_block.blocks,
        super_block
            .bfree
            .checked_add(allocator_state.used_clusters)
            .unwrap()
    );
}

#[ktest]
fn mount_volume_state_preserves_volume_anomaly_flags() {
    init_mount_volume_state_test_runtime();

    let disk = ExfatRefactorMemoryDisk::new();
    disk.write_bytes(106, &TEST_VOLUME_FLAGS.to_le_bytes());
    let (fs, _, _, _) = mounted_fs(&disk, default_mount_options());

    let state = fs.state.read();
    let publication = state.as_ref().unwrap();

    assert!(publication.anomaly.volume_dirty);
    assert!(publication.anomaly.media_failure);
    assert!(publication.anomaly.clear_to_zero);
}

#[ktest]
fn filesystem_sync_and_volume_state_posture_and_admission_boundary_observation_and_sync_preserve_anomaly_overlays_root_and_superblock_visibility()
 {
    init_mount_volume_state_test_runtime();

    let disk = ExfatRefactorMemoryDisk::new();
    disk.write_bytes(106, &TEST_VOLUME_FLAGS.to_le_bytes());
    let (fs, root_inode, super_block, flags) = mounted_fs(&disk, default_mount_options());

    assert_eq!(flags, FsFlags::empty());
    assert_observed_volume_posture(&fs, FsFlags::empty(), TEST_VOLUME_FLAGS);

    fs.sync().unwrap();

    assert_observed_volume_posture(&fs, FsFlags::empty(), CLEAN_TEST_VOLUME_FLAGS);
    assert!(Arc::ptr_eq(&root_inode, &fs.root_inode()));
    assert_same_super_block(&super_block, &fs.sb());
}

#[ktest]
fn filesystem_sync_and_volume_state_posture_and_admission_boundary_read_only_admission_and_sync_reject_without_state_drift()
 {
    init_mount_volume_state_test_runtime();

    let disk = ExfatRefactorMemoryDisk::new();
    disk.write_bytes(106, &TEST_VOLUME_FLAGS.to_le_bytes());
    let options = ExfatMountOptions {
        fs_flags: FsFlags::RDONLY,
        ..default_mount_options()
    };
    let (fs, root_inode, super_block, flags) = mounted_fs(&disk, options);

    assert_eq!(flags, FsFlags::RDONLY);
    assert_eq!(super_block.flags, u64::from(FsFlags::RDONLY.bits()));
    assert_observed_volume_posture(&fs, FsFlags::RDONLY, TEST_VOLUME_FLAGS);
    assert_eq!(
        fs.admitted_mutation_state().err(),
        Some(MountVolumeStateError::ReadOnlyConflict)
    );

    let error = fs.sync().unwrap_err();
    assert_eq!(error.error(), Errno::EROFS);

    assert_observed_volume_posture(&fs, FsFlags::RDONLY, TEST_VOLUME_FLAGS);
    assert!(Arc::ptr_eq(&root_inode, &fs.root_inode()));
    assert_same_super_block(&super_block, &fs.sb());
}

#[ktest]
fn filesystem_sync_and_volume_state_dirty_admission_persists_dirty_boot_flag_and_preserves_existing_anomaly_overlays()
 {
    init_mount_volume_state_test_runtime();

    let disk = ExfatRefactorMemoryDisk::new();
    disk.write_bytes(106, &CLEAN_TEST_VOLUME_FLAGS.to_le_bytes());
    let (fs, root_inode, super_block, flags) = mounted_fs(&disk, default_mount_options());

    assert_eq!(flags, FsFlags::empty());
    assert_observed_volume_posture(&fs, FsFlags::empty(), CLEAN_TEST_VOLUME_FLAGS);
    assert_eq!(boot_volume_flags(&disk), CLEAN_TEST_VOLUME_FLAGS);

    let (state_guard, _, _, anomaly, _, _) = fs.admitted_mutation_state().unwrap();

    assert!(anomaly.volume_dirty);
    assert!(anomaly.media_failure);
    assert!(anomaly.clear_to_zero);
    assert_eq!(boot_volume_flags(&disk), TEST_VOLUME_FLAGS);
    drop(state_guard);

    assert_observed_volume_posture(&fs, FsFlags::empty(), TEST_VOLUME_FLAGS);
    assert!(Arc::ptr_eq(&root_inode, &fs.root_inode()));
    assert_same_super_block(&super_block, &fs.sb());
}

#[ktest]
fn filesystem_sync_and_volume_state_sync_clears_only_volume_dirty_and_repeated_clean_syncs_stay_stable()
 {
    init_mount_volume_state_test_runtime();

    let disk = ExfatRefactorMemoryDisk::new();
    disk.write_bytes(106, &TEST_VOLUME_FLAGS.to_le_bytes());
    let (fs, root_inode, super_block, flags) = mounted_fs(&disk, default_mount_options());

    assert_eq!(flags, FsFlags::empty());
    assert_observed_volume_posture(&fs, FsFlags::empty(), TEST_VOLUME_FLAGS);
    assert_eq!(boot_volume_flags(&disk), TEST_VOLUME_FLAGS);

    fs.sync().unwrap();

    assert_observed_volume_posture(&fs, FsFlags::empty(), CLEAN_TEST_VOLUME_FLAGS);
    assert_eq!(boot_volume_flags(&disk), CLEAN_TEST_VOLUME_FLAGS);
    assert!(Arc::ptr_eq(&root_inode, &fs.root_inode()));
    assert_same_super_block(&super_block, &fs.sb());

    for _ in 0..2 {
        fs.sync().unwrap();
        assert_observed_volume_posture(&fs, FsFlags::empty(), CLEAN_TEST_VOLUME_FLAGS);
        assert_eq!(boot_volume_flags(&disk), CLEAN_TEST_VOLUME_FLAGS);
        assert!(Arc::ptr_eq(&root_inode, &fs.root_inode()));
        assert_same_super_block(&super_block, &fs.sb());
    }
}

#[ktest]
fn filesystem_sync_and_volume_state_sync_boot_flag_write_failure_keeps_dirty_posture_and_anomaly_overlays()
 {
    init_mount_volume_state_test_runtime();

    let disk = ExfatRefactorMemoryDisk::new();
    disk.write_bytes(106, &TEST_VOLUME_FLAGS.to_le_bytes());
    let failing_disk = ExfatRefactorToggleFailingWriteDisk::new(disk.clone(), 0, SECTOR_SIZE);
    let block_device: Arc<dyn BlockDevice> = failing_disk.clone();
    let (fs, root_inode, super_block, flags) =
        mount_block_device(&block_device, default_mount_options()).unwrap();

    assert_eq!(flags, FsFlags::empty());
    assert_observed_volume_posture(&fs, FsFlags::empty(), TEST_VOLUME_FLAGS);
    assert_eq!(boot_volume_flags(&disk), TEST_VOLUME_FLAGS);

    failing_disk.enable_failures();

    let error = fs.sync().unwrap_err();
    assert_eq!(error.error(), Errno::EIO);

    assert_observed_volume_posture(&fs, FsFlags::empty(), TEST_VOLUME_FLAGS);
    assert_eq!(boot_volume_flags(&disk), TEST_VOLUME_FLAGS);
    assert!(Arc::ptr_eq(&root_inode, &fs.root_inode()));
    assert_same_super_block(&super_block, &fs.sb());
}

#[ktest]
fn filesystem_sync_and_volume_state_sync_final_flush_failure_withholds_clean_publication_until_barrier_completion()
 {
    init_mount_volume_state_test_runtime();

    let disk = ExfatRefactorMemoryDisk::new();
    disk.write_bytes(106, &TEST_VOLUME_FLAGS.to_le_bytes());
    let failing_disk = ExfatRefactorCountedFailingFlushDisk::new(disk.clone(), 2);
    let block_device: Arc<dyn BlockDevice> = failing_disk;
    let (fs, root_inode, super_block, flags) =
        mount_block_device(&block_device, default_mount_options()).unwrap();

    assert_eq!(flags, FsFlags::empty());
    assert_observed_volume_posture(&fs, FsFlags::empty(), TEST_VOLUME_FLAGS);
    assert_eq!(boot_volume_flags(&disk), TEST_VOLUME_FLAGS);

    let error = fs.sync().unwrap_err();
    assert_eq!(error.error(), Errno::EIO);

    assert_observed_volume_posture(&fs, FsFlags::empty(), TEST_VOLUME_FLAGS);
    assert_eq!(boot_volume_flags(&disk), CLEAN_TEST_VOLUME_FLAGS);
    assert!(Arc::ptr_eq(&root_inode, &fs.root_inode()));
    assert_same_super_block(&super_block, &fs.sb());
}

#[ktest]
fn filesystem_sync_and_volume_state_forced_shutdown_admission_is_monotonic_and_suppresses_mutation_and_trim()
 {
    init_mount_volume_state_test_runtime();

    let disk = ExfatRefactorMemoryDisk::new();
    let options = ExfatMountOptions {
        discard: true,
        ..default_mount_options()
    };
    let (fs, root_inode, super_block, flags) = mounted_fs(&disk, options.clone());

    assert_eq!(flags, FsFlags::empty());
    assert_eq!(fs.current_options().unwrap(), options);
    assert!(!fs.state.read().as_ref().unwrap().forced_shutdown);

    fs.admit_forced_shutdown().unwrap();
    fs.admit_forced_shutdown().unwrap();

    {
        let state = fs.state.read();
        let publication = state.as_ref().unwrap();
        assert!(publication.forced_shutdown);
        assert_eq!(publication.flags, FsFlags::empty());
        assert_eq!(publication.options, options);
    }

    assert_eq!(
        fs.admitted_mutation_state().err(),
        Some(MountVolumeStateError::DeviceIo)
    );

    let trim_error = fs.administrative_trim_free_space().unwrap_err();
    assert_eq!(trim_error.error(), Errno::EIO);

    let sync_error = fs.sync().unwrap_err();
    assert_eq!(sync_error.error(), Errno::EIO);

    assert!(fs.state.read().as_ref().unwrap().forced_shutdown);
    assert_eq!(fs.current_options().unwrap(), options);
    assert_eq!(fs.published_flags().unwrap(), FsFlags::empty());
    assert!(Arc::ptr_eq(&root_inode, &fs.root_inode()));
    assert_same_super_block(&super_block, &fs.sb());
}

#[ktest]
fn filesystem_sync_and_volume_state_forced_shutdown_sync_and_observation_preserve_existing_anomaly_posture()
 {
    init_mount_volume_state_test_runtime();

    let disk = ExfatRefactorMemoryDisk::new();
    disk.write_bytes(106, &TEST_VOLUME_FLAGS.to_le_bytes());
    let (fs, root_inode, super_block, flags) = mounted_fs(&disk, default_mount_options());

    assert_eq!(flags, FsFlags::empty());
    assert_observed_volume_posture(&fs, FsFlags::empty(), TEST_VOLUME_FLAGS);
    assert_eq!(boot_volume_flags(&disk), TEST_VOLUME_FLAGS);

    fs.admit_forced_shutdown().unwrap();

    let sync_error = fs.sync().unwrap_err();
    assert_eq!(sync_error.error(), Errno::EIO);
    assert_eq!(
        fs.admitted_mutation_state().err(),
        Some(MountVolumeStateError::DeviceIo)
    );

    assert!(fs.state.read().as_ref().unwrap().forced_shutdown);
    assert_observed_volume_posture(&fs, FsFlags::empty(), TEST_VOLUME_FLAGS);
    assert_eq!(boot_volume_flags(&disk), TEST_VOLUME_FLAGS);
    assert!(Arc::ptr_eq(&root_inode, &fs.root_inode()));
    assert_same_super_block(&super_block, &fs.sb());
}

#[ktest]
fn mount_volume_state_rejects_invalid_boot_region() {
    init_mount_volume_state_test_runtime();

    let disk = ExfatRefactorMemoryDisk::new();
    disk.write_bytes(3, b"XFAT    ");

    assert_eq!(
        mount_disk(&disk, default_mount_options()).err(),
        Some(MountVolumeStateError::InvalidOnDiskLayout)
    );
}

#[ktest]
fn mount_volume_state_rejects_boot_region_device_io() {
    init_mount_volume_state_test_runtime();

    let disk = ExfatRefactorMemoryDisk::new();
    let failing_disk = ExfatRefactorFailingReadDisk::new(disk, 0, SECTOR_SIZE);
    let block_device: Arc<dyn BlockDevice> = failing_disk;

    assert_eq!(
        mount_block_device(&block_device, default_mount_options()).err(),
        Some(MountVolumeStateError::DeviceIo)
    );
}

#[ktest]
fn mount_volume_state_rejects_inconsistent_allocation_bitmap() {
    init_mount_volume_state_test_runtime();

    let disk = ExfatRefactorMemoryDisk::new();
    let directory_entry = find_directory_entry(&disk, ALLOCATION_BITMAP_ENTRY_TYPE);
    disk.write_bytes(directory_entry.offset + 24, &1u64.to_le_bytes());

    assert_eq!(
        mount_disk(&disk, default_mount_options()).err(),
        Some(MountVolumeStateError::InconsistentAccounting)
    );
}

#[ktest]
fn mount_volume_state_rejects_allocation_bitmap_device_io() {
    init_mount_volume_state_test_runtime();

    let disk = ExfatRefactorMemoryDisk::new();
    let bitmap_offset = allocation_bitmap_data_offset(&disk);
    let failing_disk = ExfatRefactorFailingReadDisk::new(disk, bitmap_offset, SECTOR_SIZE);
    let block_device: Arc<dyn BlockDevice> = failing_disk;

    assert_eq!(
        mount_block_device(&block_device, default_mount_options()).err(),
        Some(MountVolumeStateError::DeviceIo)
    );
}

#[ktest]
fn mount_volume_state_rejects_invalid_upcase_table() {
    init_mount_volume_state_test_runtime();

    let disk = ExfatRefactorMemoryDisk::new();
    let data_offset = upcase_data_offset(&disk);
    let original_byte = disk.read_bytes(data_offset, 1);
    disk.write_bytes(data_offset, &[original_byte[0] ^ 0xFF]);

    assert_eq!(
        mount_disk(&disk, default_mount_options()).err(),
        Some(MountVolumeStateError::InvalidOnDiskLayout)
    );
}

#[ktest]
fn mount_volume_state_remount_allows_discard_and_rejects_immutable_delta() {
    init_mount_volume_state_test_runtime();

    let disk = ExfatRefactorMemoryDisk::new();
    let (fs, _, _, _) = mounted_fs(&disk, default_mount_options());
    let discard_options = ExfatMountOptions {
        discard: true,
        ..default_mount_options()
    };

    let flags = fs
        .remount_published(FsFlags::empty(), &discard_options)
        .unwrap();
    assert_eq!(flags, FsFlags::empty());

    {
        let state = fs.state.read();
        let publication = state.as_ref().unwrap();
        assert!(publication.options.discard);
        assert_eq!(publication.flags, FsFlags::empty());
    }

    let unsupported_flags = FsFlags::SYNCHRONOUS;
    let unsupported_options = ExfatMountOptions {
        fs_flags: unsupported_flags,
        ..default_mount_options()
    };
    assert_eq!(
        fs.remount_published(unsupported_flags, &unsupported_options)
            .err(),
        Some(MountVolumeStateError::UnsupportedRemountDelta)
    );

    let state = fs.state.read();
    let publication = state.as_ref().unwrap();
    assert!(publication.options.discard);
    assert_eq!(publication.options.fs_flags, FsFlags::empty());
    assert_eq!(publication.flags, FsFlags::empty());
}

#[ktest]
fn administrative_trim_free_space_fast_fails_with_eopnotsupp_without_mutating_state() {
    init_mount_volume_state_test_runtime();

    let disk = ExfatRefactorMemoryDisk::new();
    let options = ExfatMountOptions {
        discard: true,
        ..default_mount_options()
    };
    let (fs, _, _, _) = mounted_fs(&disk, options);

    assert!(fs.current_options().unwrap().discard);

    assert_administrative_trim_rejection_preserves_state(&fs, Errno::EOPNOTSUPP);

    assert!(fs.current_options().unwrap().discard);
}

#[ktest]
fn administrative_trim_free_space_rejects_read_only_before_unsupported_trim() {
    init_mount_volume_state_test_runtime();

    let disk = ExfatRefactorMemoryDisk::new();
    let options = ExfatMountOptions {
        discard: true,
        fs_flags: FsFlags::RDONLY,
        ..default_mount_options()
    };
    let (fs, _, super_block, flags) = mounted_fs(&disk, options);

    assert_eq!(flags, FsFlags::RDONLY);
    assert_eq!(super_block.flags, u64::from(FsFlags::RDONLY.bits()));
    assert!(fs.current_options().unwrap().discard);

    assert_administrative_trim_rejection_preserves_state(&fs, Errno::EROFS);

    assert!(fs.current_options().unwrap().discard);
    assert_eq!(fs.published_flags().unwrap(), FsFlags::RDONLY);
}

#[ktest]
fn free_space_accounting_and_discard_integration_allocate_free_updates_superblock_and_discard_policy()
 {
    init_mount_volume_state_test_runtime();

    let disk = ExfatRefactorMemoryDisk::new();
    let options = ExfatMountOptions {
        discard: true,
        ..default_mount_options()
    };
    let (fs, _, _, _) = mounted_fs(&disk, options);

    let initial_snapshot = fs.cached_free_space_snapshot().unwrap();
    assert_snapshot_matches_super_block(&initial_snapshot, &fs.sb());
    assert!(fs.current_options().unwrap().discard);

    let (allocated_ranges, allocated_snapshot) = fs.allocate_free_space(1).unwrap();
    assert_eq!(
        allocated_snapshot.used_clusters,
        initial_snapshot.used_clusters.checked_add(1).unwrap()
    );
    assert_eq!(
        allocated_snapshot.free_clusters,
        initial_snapshot.free_clusters.checked_sub(1).unwrap()
    );
    assert_eq!(
        allocated_snapshot.used_clusters_from_recount,
        initial_snapshot.used_clusters_from_recount
    );
    assert_snapshot_matches_super_block(&allocated_snapshot, &fs.sb());

    let freed_snapshot = fs.free_allocated_space(&allocated_ranges).unwrap();
    assert_eq!(freed_snapshot, initial_snapshot);
    assert_cached_reporting_matches_snapshot(&fs, &freed_snapshot);
    assert!(!fs.current_options().unwrap().discard);
    assert_eq!(fs.published_flags().unwrap(), FsFlags::empty());
}

#[ktest]
fn free_space_accounting_and_discard_integration_failures_preserve_snapshot_and_trim_boundary() {
    init_mount_volume_state_test_runtime();

    let disk = ExfatRefactorMemoryDisk::new();
    let bitmap_offset = allocation_bitmap_data_offset(&disk);
    let failing_disk = ExfatRefactorToggleFailingReadDisk::new(disk, bitmap_offset, SECTOR_SIZE);
    let block_device: Arc<dyn BlockDevice> = failing_disk.clone();
    let options = ExfatMountOptions {
        discard: true,
        ..default_mount_options()
    };
    let (fs, _, _, _) = mount_block_device(&block_device, options).unwrap();

    let (allocated_ranges, _) = fs.allocate_free_space(1).unwrap();
    let preserved_snapshot = fs.free_allocated_space(&allocated_ranges).unwrap();
    let preserved_super_block = fs.sb();

    assert_snapshot_matches_super_block(&preserved_snapshot, &preserved_super_block);
    assert!(!fs.current_options().unwrap().discard);

    failing_disk.enable_failures();

    assert_eq!(
        fs.recount_free_space().err(),
        Some(MountVolumeStateError::DeviceIo)
    );
    assert_cached_reporting_matches_snapshot(&fs, &preserved_snapshot);
    assert_same_super_block(&preserved_super_block, &fs.sb());

    let trim_error = fs.administrative_trim_free_space().unwrap_err();
    assert_eq!(trim_error.error(), Errno::EOPNOTSUPP);
    assert_cached_reporting_matches_snapshot(&fs, &preserved_snapshot);
    assert_same_super_block(&preserved_super_block, &fs.sb());
    assert!(!fs.current_options().unwrap().discard);
    assert_eq!(fs.published_flags().unwrap(), FsFlags::empty());
}

#[ktest]
fn free_space_accounting_and_discard_integration_repeated_snapshots_and_trim_rejections_stay_stable()
 {
    init_mount_volume_state_test_runtime();

    let disk = ExfatRefactorMemoryDisk::new();
    let options = ExfatMountOptions {
        discard: true,
        ..default_mount_options()
    };
    let (fs, _, _, _) = mounted_fs(&disk, options);

    let (allocated_ranges, _) = fs.allocate_free_space(1).unwrap();
    let stable_snapshot = fs.free_allocated_space(&allocated_ranges).unwrap();
    let stable_super_block = fs.sb();

    for _ in 0..3 {
        assert_cached_reporting_matches_snapshot(&fs, &stable_snapshot);
        assert_same_super_block(&stable_super_block, &fs.sb());
        assert!(!fs.current_options().unwrap().discard);
    }

    for _ in 0..2 {
        let trim_error = fs.administrative_trim_free_space().unwrap_err();
        assert_eq!(trim_error.error(), Errno::EOPNOTSUPP);
        assert_cached_reporting_matches_snapshot(&fs, &stable_snapshot);
        assert_same_super_block(&stable_super_block, &fs.sb());
        assert!(!fs.current_options().unwrap().discard);
    }
}

#[ktest]
fn free_space_accounting_and_discard_integration_snapshot_linearizes_under_allocator_contention() {
    init_mount_volume_state_test_runtime();

    let disk = ExfatRefactorMemoryDisk::new();
    let options = ExfatMountOptions {
        discard: true,
        ..default_mount_options()
    };
    let (fs, _, _, _) = mounted_fs(&disk, options);
    let initial_snapshot = fs.cached_free_space_snapshot().unwrap();
    let allocated_used_clusters = initial_snapshot.used_clusters.checked_add(1).unwrap();
    let saw_allocated_snapshot = Arc::new(AtomicBool::new(false));
    let saw_initial_snapshot = Arc::new(AtomicBool::new(false));
    let saw_allocated_recount = Arc::new(AtomicBool::new(false));
    let allow_free = Arc::new(AtomicBool::new(false));
    let mutator_done = Arc::new(AtomicBool::new(false));
    let observed_used_clusters = Arc::new(Mutex::new(Vec::new()));

    let snapshot_thread = {
        let fs = fs.clone();
        let saw_allocated_snapshot = saw_allocated_snapshot.clone();
        let saw_initial_snapshot = saw_initial_snapshot.clone();
        let allow_free = allow_free.clone();
        let mutator_done = mutator_done.clone();
        let observed_used_clusters = observed_used_clusters.clone();

        ThreadOptions::new(move || {
            for _ in 0..512 {
                let snapshot = fs.cached_free_space_snapshot().unwrap();
                let super_block = fs.sb();

                assert_snapshot_matches_super_block(&snapshot, &super_block);
                assert!(
                    snapshot.used_clusters == initial_snapshot.used_clusters
                        || snapshot.used_clusters == allocated_used_clusters
                );

                if snapshot.used_clusters == initial_snapshot.used_clusters {
                    saw_initial_snapshot.store(true, Ordering::Relaxed);
                }
                if snapshot.used_clusters == allocated_used_clusters {
                    saw_allocated_snapshot.store(true, Ordering::Relaxed);
                    allow_free.store(true, Ordering::Relaxed);
                }

                observed_used_clusters.lock().push(snapshot.used_clusters);
                if mutator_done.load(Ordering::Relaxed)
                    && saw_initial_snapshot.load(Ordering::Relaxed)
                    && saw_allocated_snapshot.load(Ordering::Relaxed)
                {
                    break;
                }
                Thread::yield_now();
            }
        })
        .spawn()
    };

    let recount_thread = {
        let fs = fs.clone();
        let mutator_done = mutator_done.clone();
        let saw_allocated_recount = saw_allocated_recount.clone();

        ThreadOptions::new(move || {
            for _ in 0..512 {
                let snapshot = fs.recount_free_space().unwrap();
                let super_block = fs.sb();

                assert_snapshot_matches_super_block(&snapshot, &super_block);
                assert!(
                    snapshot.used_clusters == initial_snapshot.used_clusters
                        || snapshot.used_clusters == allocated_used_clusters
                );

                if snapshot.used_clusters == allocated_used_clusters {
                    saw_allocated_recount.store(true, Ordering::Relaxed);
                    break;
                }
                if mutator_done.load(Ordering::Relaxed) {
                    break;
                }
                Thread::yield_now();
            }
        })
        .spawn()
    };

    let mutator_thread = {
        let fs = fs.clone();
        let allow_free = allow_free.clone();
        let saw_allocated_recount = saw_allocated_recount.clone();
        let mutator_done = mutator_done.clone();

        ThreadOptions::new(move || {
            let (allocated_ranges, allocated_snapshot) = fs.allocate_free_space(1).unwrap();
            assert_eq!(allocated_snapshot.used_clusters, allocated_used_clusters);
            assert_snapshot_matches_super_block(&allocated_snapshot, &fs.sb());

            for _ in 0..512 {
                if allow_free.load(Ordering::Relaxed)
                    && saw_allocated_recount.load(Ordering::Relaxed)
                {
                    break;
                }
                Thread::yield_now();
            }

            let freed_snapshot = fs.free_allocated_space(&allocated_ranges).unwrap();
            assert_eq!(
                freed_snapshot.total_clusters,
                initial_snapshot.total_clusters
            );
            assert_eq!(freed_snapshot.free_clusters, initial_snapshot.free_clusters);
            assert_eq!(freed_snapshot.used_clusters, initial_snapshot.used_clusters);
            assert_snapshot_matches_super_block(&freed_snapshot, &fs.sb());
            mutator_done.store(true, Ordering::Relaxed);
        })
        .spawn()
    };

    snapshot_thread.join();
    recount_thread.join();
    mutator_thread.join();

    let observed_used_clusters = observed_used_clusters.lock();
    assert!(!observed_used_clusters.is_empty());
    assert!(saw_initial_snapshot.load(Ordering::Relaxed));
    assert!(saw_allocated_snapshot.load(Ordering::Relaxed));
    assert!(saw_allocated_recount.load(Ordering::Relaxed));
    assert!(observed_used_clusters.iter().all(|used_clusters| {
        *used_clusters == initial_snapshot.used_clusters
            || *used_clusters == allocated_used_clusters
    }));

    let final_snapshot = fs.cached_free_space_snapshot().unwrap();
    assert_eq!(
        final_snapshot.total_clusters,
        initial_snapshot.total_clusters
    );
    assert_eq!(final_snapshot.free_clusters, initial_snapshot.free_clusters);
    assert_eq!(final_snapshot.used_clusters, initial_snapshot.used_clusters);
    assert_snapshot_matches_super_block(&final_snapshot, &fs.sb());
    assert!(!fs.current_options().unwrap().discard);
    assert_eq!(fs.published_flags().unwrap(), FsFlags::empty());
}
