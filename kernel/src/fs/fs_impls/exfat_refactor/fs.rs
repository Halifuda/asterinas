// SPDX-License-Identifier: MPL-2.0

use aster_block::{
    bio::BioStatus,
    BlockDevice,
};

use super::{
    bitmap::AllocationBitmapRecord,
    boot::{self, BootRegion, VolumeAnomalyState},
    inode::ExfatInode,
    upcase::UpcaseTable,
};
use crate::{
    fs::{
        vfs::{
            file_system::{FileSystem, FsEventSubscriberStats, FsFlags, SuperBlock},
            inode::Inode,
            registry::{FsProperties, FsType},
        },
    },
    prelude::*,
};

const EXFAT_SUPER_MAGIC: u64 = 0x2011_BAB0;

#[derive(Clone)]
struct PublishedMountState {
    pub(super) anomaly: VolumeAnomalyState,
    pub(super) boot_region: BootRegion,
    flags: FsFlags,
    options: ExfatMountOptions,
    root_inode: Arc<ExfatInode>,
    pub(super) upcase_table: Arc<UpcaseTable>,
}

struct AllocatorState {
    pub(super) bitmap: AllocationBitmapRecord,
    used_clusters: usize,
    pub(super) used_clusters_from_recount: bool,
}

pub(crate) struct ExfatFs {
    allocator: RwLock<Option<AllocatorState>>,
    block_device: Arc<dyn BlockDevice>,
    fs_event_subscriber_stats: FsEventSubscriberStats,
    source: Option<String>,
    state: RwLock<Option<PublishedMountState>>,
}

impl ExfatFs {
    fn new(block_device: Arc<dyn BlockDevice>, source: Option<String>) -> Arc<Self> {
        Arc::new(Self {
            allocator: RwLock::new(None),
            block_device,
            fs_event_subscriber_stats: FsEventSubscriberStats::new(),
            source,
            state: RwLock::new(None),
        })
    }

    pub(super) fn container_device_id(&self) -> device_id::DeviceId {
        self.block_device.id()
    }

    fn build_super_block(
        &self,
        publication: &PublishedMountState,
        allocator: &AllocatorState,
    ) -> core::result::Result<SuperBlock, MountVolumeStateError> {
        let total_clusters = publication.boot_region.cluster_count_usize()?;
        let free_clusters = total_clusters
            .checked_sub(allocator.used_clusters)
            .ok_or(MountVolumeStateError::InconsistentAccounting)?;
        Ok(SuperBlock {
            magic: EXFAT_SUPER_MAGIC,
            bsize: publication.boot_region.cluster_size,
            blocks: total_clusters,
            bfree: free_clusters,
            bavail: free_clusters,
            files: 0,
            ffree: 0,
            fsid: u64::from(publication.boot_region.volume_serial_number),
            namelen: UpcaseTable::NAME_MAX,
            frsize: publication.boot_region.cluster_size,
            flags: u64::from(publication.flags.bits()),
            container_dev_id: self.block_device.id(),
        })
    }

    fn current_options(&self) -> core::result::Result<ExfatMountOptions, MountVolumeStateError> {
        let state = self.state.read();
        let publication = state
            .as_ref()
            .ok_or(MountVolumeStateError::UnpublishedState)?;
        Ok(publication.options)
    }

    fn publish_mount_state(
        &self,
        allocator_state: AllocatorState,
        publication: PublishedMountState,
    ) {
        *self.allocator.write() = Some(allocator_state);
        *self.state.write() = Some(publication);
    }

    fn published_flags(&self) -> core::result::Result<FsFlags, MountVolumeStateError> {
        let state = self.state.read();
        let publication = state
            .as_ref()
            .ok_or(MountVolumeStateError::UnpublishedState)?;
        Ok(publication.flags)
    }

    fn published_root_inode(&self) -> core::result::Result<Arc<dyn Inode>, MountVolumeStateError> {
        let state = self.state.read();
        let publication = state
            .as_ref()
            .ok_or(MountVolumeStateError::UnpublishedState)?;
        let root_inode: Arc<dyn Inode> = publication.root_inode.clone();
        Ok(root_inode)
    }

    fn super_block_snapshot(&self) -> core::result::Result<SuperBlock, MountVolumeStateError> {
        let state = self.state.read();
        let publication = state
            .as_ref()
            .ok_or(MountVolumeStateError::UnpublishedState)?;
        let allocator = self.allocator.read();
        let allocator_state = allocator
            .as_ref()
            .ok_or(MountVolumeStateError::UnpublishedState)?;
        self.build_super_block(publication, allocator_state)
    }
}

impl FileSystem for ExfatFs {
    fn name(&self) -> &'static str {
        "exfat"
    }

    fn source(&self) -> Option<&str> {
        self.source.as_deref()
    }

    fn sync(&self) -> Result<()> {
        // TODO: Replace this mount-only flush seam once `meso_08` owns dirty-state
        // persistence and can clear or rewrite volume flags during steady-state sync.
        match self.block_device.sync()? {
            BioStatus::Complete => Ok(()),
            _ => return_errno!(Errno::EIO),
        }
    }

    fn root_inode(&self) -> Arc<dyn Inode> {
        match self.published_root_inode() {
            Ok(root_inode) => root_inode,
            Err(_) => unreachable!("mounted exFAT instances must publish a root inode"),
        }
    }

    fn sb(&self) -> SuperBlock {
        match self.super_block_snapshot() {
            Ok(super_block) => super_block,
            Err(_) => unreachable!("mounted exFAT instances must publish superblock state"),
        }
    }

    fn flags(&self) -> FsFlags {
        match self.published_flags() {
            Ok(flags) => flags,
            Err(_) => unreachable!("mounted exFAT instances must publish filesystem flags"),
        }
    }

    fn set_fs_flags(&self, flags: FsFlags, data: Option<CString>, _ctx: &Context) -> Result<()> {
        let current_options = self.current_options()?;
        let next_options = match data.as_deref() {
            Some(args) => ExfatMountOptions::parse(flags, Some(args))?,
            None => current_options.with_flags(flags),
        };
        mount_volume_state(
            MountVolumeStateTarget::Published { fs: self },
            MountVolumeStateOperation::Remount {
                next_flags: flags,
                next_options: &next_options,
            },
        )?;
        Ok(())
    }

    fn fs_event_subscriber_stats(&self) -> &FsEventSubscriberStats {
        &self.fs_event_subscriber_stats
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ExfatMountOptions {
    discard: bool,
    fs_flags: FsFlags,
}

impl ExfatMountOptions {
    fn parse(
        fs_flags: FsFlags,
        args: Option<&CStr>,
    ) -> core::result::Result<Self, MountVolumeStateError> {
        let mut options = Self {
            discard: false,
            fs_flags,
        };
        let Some(args) = args else {
            return Ok(options);
        };
        for entry in args.to_string_lossy().split(',') {
            if entry.is_empty() {
                continue;
            }
            match entry {
                "discard" => options.discard = true,
                "nodiscard" => options.discard = false,
                _ => return Err(MountVolumeStateError::InvalidMountInput),
            }
        }
        Ok(options)
    }

    fn with_flags(self, fs_flags: FsFlags) -> Self {
        Self { fs_flags, ..self }
    }
}

pub(crate) enum MountVolumeStateTarget<'a> {
    Candidate {
        block_device: &'a Arc<dyn BlockDevice>,
        source: Option<&'a str>,
        options: &'a ExfatMountOptions,
    },
    Published {
        fs: &'a ExfatFs,
    },
}

pub(crate) enum MountVolumeStateOperation<'a> {
    Mount,
    Remount {
        next_flags: FsFlags,
        next_options: &'a ExfatMountOptions,
    },
}

pub(crate) enum MountVolumeStateOutcome {
    Mounted {
        fs: Arc<ExfatFs>,
        root_inode: Arc<dyn Inode>,
        super_block: SuperBlock,
        flags: FsFlags,
    },
    Remounted {
        flags: FsFlags,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum MountVolumeStateError {
    InvalidMountInput,
    InvalidOnDiskLayout,
    DeviceIo,
    UnsupportedRemountDelta,
    ReadOnlyConflict,
    UnpublishedState,
    InconsistentAccounting,
}

impl From<MountVolumeStateError> for Error {
    fn from(error: MountVolumeStateError) -> Self {
        match error {
            MountVolumeStateError::InvalidMountInput => Error::new(Errno::EINVAL),
            MountVolumeStateError::InvalidOnDiskLayout => {
                Error::with_message(Errno::EUCLEAN, "invalid exFAT on-disk layout")
            }
            MountVolumeStateError::DeviceIo => Error::new(Errno::EIO),
            MountVolumeStateError::UnsupportedRemountDelta => {
                Error::with_message(Errno::EINVAL, "unsupported exFAT remount delta")
            }
            MountVolumeStateError::ReadOnlyConflict => Error::new(Errno::EROFS),
            MountVolumeStateError::UnpublishedState => {
                Error::with_message(Errno::EINVAL, "filesystem state is not published")
            }
            MountVolumeStateError::InconsistentAccounting => {
                Error::with_message(Errno::EUCLEAN, "exFAT allocator accounting is inconsistent")
            }
        }
    }
}

pub(crate) fn mount_volume_state(
    target: MountVolumeStateTarget<'_>,
    operation: MountVolumeStateOperation<'_>,
) -> core::result::Result<MountVolumeStateOutcome, MountVolumeStateError> {
    match (target, operation) {
        (
            MountVolumeStateTarget::Candidate {
                block_device,
                source,
                options,
            },
            MountVolumeStateOperation::Mount,
        ) => mount_candidate(block_device, source, options),
        (
            MountVolumeStateTarget::Published { fs },
            MountVolumeStateOperation::Remount {
                next_flags,
                next_options,
            },
        ) => remount_published(fs, next_flags, next_options),
        _ => Err(MountVolumeStateError::InvalidMountInput),
    }
}

fn mount_candidate(
    block_device: &Arc<dyn BlockDevice>,
    source: Option<&str>,
    options: &ExfatMountOptions,
) -> core::result::Result<MountVolumeStateOutcome, MountVolumeStateError> {
    let validated_mount = boot::ValidatedMount::load(block_device.as_ref())?;
    let fs = ExfatFs::new(block_device.clone(), source.map(ToString::to_string));
    let root_inode = ExfatInode::new_root(
        &fs,
        validated_mount.boot_region.root_dir_cluster,
        validated_mount.boot_region.cluster_size,
    );
    let publication = PublishedMountState {
        anomaly: validated_mount.anomaly,
        boot_region: validated_mount.boot_region,
        flags: options.fs_flags,
        options: *options,
        root_inode: root_inode.clone(),
        upcase_table: validated_mount.upcase_table,
    };
    let allocator_state = AllocatorState {
        bitmap: validated_mount.bitmap,
        used_clusters: validated_mount.used_clusters,
        used_clusters_from_recount: validated_mount.used_clusters_from_recount,
    };
    fs.publish_mount_state(allocator_state, publication);
    let super_block = fs.super_block_snapshot()?;
    let root_inode: Arc<dyn Inode> = root_inode;
    Ok(MountVolumeStateOutcome::Mounted {
        fs,
        root_inode,
        super_block,
        flags: options.fs_flags,
    })
}

fn remount_published(
    fs: &ExfatFs,
    next_flags: FsFlags,
    next_options: &ExfatMountOptions,
) -> core::result::Result<MountVolumeStateOutcome, MountVolumeStateError> {
    let mut state = fs.state.write();
    let publication = state
        .as_mut()
        .ok_or(MountVolumeStateError::UnpublishedState)?;
    let changed_flags = publication.flags ^ next_flags;
    if changed_flags.intersects(
        FsFlags::SYNCHRONOUS
            | FsFlags::MANDLOCK
            | FsFlags::DIRSYNC
            | FsFlags::SILENT
            | FsFlags::LAZYTIME,
    ) {
        return Err(MountVolumeStateError::UnsupportedRemountDelta);
    }
    if publication.flags.contains(FsFlags::RDONLY) && !next_flags.contains(FsFlags::RDONLY) {
        return Err(MountVolumeStateError::ReadOnlyConflict);
    }
    publication.flags = next_flags;
    publication.options = next_options.with_flags(next_flags);
    Ok(MountVolumeStateOutcome::Remounted { flags: next_flags })
}

pub(crate) fn init() {
    crate::fs::vfs::registry::register(&ExfatFsType).unwrap();
}

pub(super) struct ExfatFsType;

impl FsType for ExfatFsType {
    fn name(&self) -> &'static str {
        "exfat"
    }

    fn properties(&self) -> FsProperties {
        FsProperties::NEED_DISK
    }

    fn create(
        &self,
        flags: FsFlags,
        args: Option<CString>,
        disk: Option<Arc<dyn BlockDevice>>,
    ) -> Result<Arc<dyn FileSystem>> {
        let block_device = disk.ok_or(Error::new(Errno::EINVAL))?;
        let options = ExfatMountOptions::parse(flags, args.as_deref())?;
        let outcome = mount_volume_state(
            MountVolumeStateTarget::Candidate {
                block_device: &block_device,
                source: Some(block_device.name()),
                options: &options,
            },
            MountVolumeStateOperation::Mount,
        )?;
        match outcome {
            MountVolumeStateOutcome::Mounted { fs, .. } => Ok(fs as Arc<dyn FileSystem>),
            _ => Err(MountVolumeStateError::InvalidMountInput.into()),
        }
    }

    fn sysnode(&self) -> Option<Arc<dyn aster_systree::SysNode>> {
        None
    }
}

#[cfg(ktest)]
mod tests {
    use alloc::{
        collections::BTreeSet,
        sync::Arc,
        vec,
        vec::Vec,
    };
    use core::fmt;

    use aster_block::{
        bio::{BioEnqueueError, BioStatus, BioType, SubmittedBio},
        BlockDevice, BlockDeviceMeta, SECTOR_SIZE,
    };
    use device_id::DeviceId;
    use ostd::{
        mm::{
            io::util::HasVmReaderWriter, FrameAllocOptions, HasSize, PAGE_SIZE, Segment, VmIo,
        },
        prelude::ktest,
    };

    use super::*;
    use crate::fs::{
        file::InodeType,
        vfs::{file_system::FsFlags, inode::Inode},
    };

    const ALLOCATION_BITMAP_ENTRY_TYPE: u8 = 0x81;
    const END_OF_DIRECTORY_ENTRY_TYPE: u8 = 0x00;
    const TEST_VOLUME_FLAGS: u16 = 0x000E;
    const UPCASE_TABLE_ENTRY_TYPE: u8 = 0x82;
    static EXFAT_IMAGE: &[u8] = include_bytes!("../../../../../test/initramfs/build/exfat.img");

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

    impl fmt::Debug for ExfatRefactorFailingReadDisk {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter
                .debug_struct("ExfatRefactorFailingReadDisk")
                .field("fail_range", &self.fail_range)
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
                if bio_type == BioType::Read
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
            "exfat-refactor-failing-read-test"
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
    ) -> core::result::Result<MountVolumeStateOutcome, MountVolumeStateError> {
        let block_device: Arc<dyn BlockDevice> = disk.clone();
        mount_block_device(&block_device, options)
    }

    fn mount_block_device(
        block_device: &Arc<dyn BlockDevice>,
        options: ExfatMountOptions,
    ) -> core::result::Result<MountVolumeStateOutcome, MountVolumeStateError> {
        mount_volume_state(
            MountVolumeStateTarget::Candidate {
                block_device,
                source: Some("exfat-refactor-test"),
                options: &options,
            },
            MountVolumeStateOperation::Mount,
        )
    }

    fn mounted_fs(
        disk: &Arc<ExfatRefactorMemoryDisk>,
        options: ExfatMountOptions,
    ) -> (Arc<ExfatFs>, Arc<dyn Inode>, SuperBlock, FsFlags) {
        match mount_disk(disk, options).unwrap() {
            MountVolumeStateOutcome::Mounted {
                fs,
                root_inode,
                super_block,
                flags,
            } => (fs, root_inode, super_block, flags),
            _ => panic!("mount returned a non-mount outcome"),
        }
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
        let next_cluster =
            u32::from_le_bytes([entry_bytes[0], entry_bytes[1], entry_bytes[2], entry_bytes[3]]);
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
        assert_eq!(root_inode.ino(), u64::from(validated_mount.boot_region.root_dir_cluster));
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
        assert_eq!(super_block.namelen, super::super::upcase::UpcaseTable::NAME_MAX);
        assert_eq!(super_block.flags, 0);
        assert_eq!(fs.current_options().unwrap(), default_mount_options());
        assert!(fs
            .state
            .read()
            .as_ref()
            .unwrap()
            .upcase_table
            .data
            .len()
            > 0);
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
        let failing_disk =
            ExfatRefactorFailingReadDisk::new(disk, bitmap_offset, SECTOR_SIZE);
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
            fs_flags: FsFlags::empty(),
        };

        match mount_volume_state(
            MountVolumeStateTarget::Published { fs: &fs },
            MountVolumeStateOperation::Remount {
                next_flags: FsFlags::empty(),
                next_options: &discard_options,
            },
        )
        .unwrap()
        {
            MountVolumeStateOutcome::Remounted { flags } => assert_eq!(flags, FsFlags::empty()),
            _ => panic!("remount returned a non-remount outcome"),
        }

        {
            let state = fs.state.read();
            let publication = state.as_ref().unwrap();
            assert!(publication.options.discard);
            assert_eq!(publication.flags, FsFlags::empty());
        }

        let unsupported_flags = FsFlags::SYNCHRONOUS;
        let unsupported_options = ExfatMountOptions {
            discard: false,
            fs_flags: unsupported_flags,
        };
        assert_eq!(
            mount_volume_state(
                MountVolumeStateTarget::Published { fs: &fs },
                MountVolumeStateOperation::Remount {
                    next_flags: unsupported_flags,
                    next_options: &unsupported_options,
                },
            )
            .err(),
            Some(MountVolumeStateError::UnsupportedRemountDelta)
        );

        let state = fs.state.read();
        let publication = state.as_ref().unwrap();
        assert!(publication.options.discard);
        assert_eq!(publication.options.fs_flags, FsFlags::empty());
        assert_eq!(publication.flags, FsFlags::empty());
    }
}
