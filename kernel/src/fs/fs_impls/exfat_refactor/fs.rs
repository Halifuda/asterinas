// SPDX-License-Identifier: MPL-2.0

use alloc::vec::Vec;

use aster_block::{BlockDevice, bio::BioStatus};

use super::{
    bitmap::{AllocationBitmap, AllocationBitmapUpdate, ClusterRange},
    boot::{BootRegion, VolumeAnomalyState},
    fat::FatReader,
    inode::ExfatInode,
    upcase::UpcaseTable,
};
use crate::{
    fs::vfs::{
        file_system::{FileSystem, FsEventSubscriberStats, FsFlags, SuperBlock},
        inode::Inode,
        registry::{FsProperties, FsType},
    },
    prelude::*,
};

const EXFAT_SUPER_MAGIC: u64 = 0x2011_BAB0;

#[derive(Clone)]
struct MountedVolumeState {
    anomaly: VolumeAnomalyState,
    boot_region: BootRegion,
    flags: FsFlags,
    options: ExfatMountOptions,
    root_inode: Arc<ExfatInode>,
    upcase_table: Arc<UpcaseTable>,
}

struct FreeSpaceAllocatorState {
    pub(super) bitmap: AllocationBitmap,
    used_clusters: usize,
    pub(super) used_clusters_from_recount: bool,
}

impl FreeSpaceAllocatorState {
    fn allocate_clusters(
        &mut self,
        block_device: &dyn BlockDevice,
        boot_region: &BootRegion,
        requested_clusters: usize,
    ) -> core::result::Result<Vec<ClusterRange>, FreeSpaceAccountingError> {
        if requested_clusters == 0 {
            return Err(FreeSpaceAccountingError::InvalidOperationInput);
        }

        let mut fat_reader = FatReader::new(block_device, boot_region);
        let allocated_ranges =
            self.bitmap
                .find_free_ranges(boot_region, &mut fat_reader, requested_clusters)?;
        let allocated_clusters = self.bitmap.apply_update(
            block_device,
            boot_region,
            &allocated_ranges,
            AllocationBitmapUpdate::Allocate,
        )?;
        if allocated_clusters != requested_clusters {
            return Err(FreeSpaceAccountingError::InconsistentAccounting);
        }
        self.used_clusters = self
            .used_clusters
            .checked_add(allocated_clusters)
            .ok_or(FreeSpaceAccountingError::InconsistentAccounting)?;
        Ok(allocated_ranges)
    }

    fn free_clusters(
        &mut self,
        block_device: &dyn BlockDevice,
        boot_region: &BootRegion,
        ranges: &[ClusterRange],
    ) -> core::result::Result<(), FreeSpaceAccountingError> {
        let released_clusters = self.bitmap.apply_update(
            block_device,
            boot_region,
            ranges,
            AllocationBitmapUpdate::Free,
        )?;
        self.used_clusters = self
            .used_clusters
            .checked_sub(released_clusters)
            .ok_or(FreeSpaceAccountingError::InconsistentAccounting)?;
        Ok(())
    }

    fn recount(
        &mut self,
        block_device: &dyn BlockDevice,
        boot_region: &BootRegion,
    ) -> core::result::Result<(), FreeSpaceAccountingError> {
        let mut fat_reader = FatReader::new(block_device, boot_region);
        let used_clusters = self
            .bitmap
            .recount_used_clusters(boot_region, &mut fat_reader)?;
        self.used_clusters = used_clusters;
        self.used_clusters_from_recount = true;
        Ok(())
    }

    fn snapshot(
        &self,
        boot_region: &BootRegion,
    ) -> core::result::Result<FreeSpaceSnapshot, FreeSpaceAccountingError> {
        let total_clusters = boot_region.cluster_count_usize()?;
        let free_clusters = total_clusters
            .checked_sub(self.used_clusters)
            .ok_or(FreeSpaceAccountingError::InconsistentAccounting)?;
        Ok(FreeSpaceSnapshot {
            total_clusters,
            free_clusters,
            used_clusters: self.used_clusters,
            used_clusters_from_recount: self.used_clusters_from_recount,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct FreeSpaceSnapshot {
    total_clusters: usize,
    free_clusters: usize,
    used_clusters: usize,
    used_clusters_from_recount: bool,
}

pub(crate) struct ExfatFs {
    allocator: RwLock<Option<FreeSpaceAllocatorState>>,
    block_device: Arc<dyn BlockDevice>,
    fs_event_subscriber_stats: FsEventSubscriberStats,
    source: Option<String>,
    state: RwLock<Option<MountedVolumeState>>,
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

    fn mount_candidate(
        block_device: &Arc<dyn BlockDevice>,
        source: Option<&str>,
        options: &ExfatMountOptions,
    ) -> core::result::Result<
        (Arc<ExfatFs>, Arc<dyn Inode>, SuperBlock, FsFlags),
        MountVolumeStateError,
    > {
        let (
            boot_region,
            anomaly,
            bitmap,
            upcase_table,
            used_clusters,
            used_clusters_from_recount,
        ) = BootRegion::load_mount_state(block_device.as_ref())?;
        let fs = Self::new(block_device.clone(), source.map(ToString::to_string));
        let root_inode =
            ExfatInode::new_root(&fs, boot_region.root_dir_cluster, boot_region.cluster_size);
        let publication = MountedVolumeState {
            anomaly,
            boot_region,
            flags: options.fs_flags,
            options: *options,
            root_inode: root_inode.clone(),
            upcase_table,
        };
        let allocator_state = FreeSpaceAllocatorState {
            bitmap,
            used_clusters,
            used_clusters_from_recount,
        };
        fs.publish_mount_state(allocator_state, publication);
        let super_block = fs
            .super_block_snapshot()
            .map_err(MountVolumeStateError::from)?;
        let root_inode: Arc<dyn Inode> = root_inode;
        Ok((fs, root_inode, super_block, options.fs_flags))
    }

    fn remount_published(
        &self,
        next_flags: FsFlags,
        next_options: &ExfatMountOptions,
    ) -> core::result::Result<FsFlags, MountVolumeStateError> {
        let mut state = self.state.write();
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
        Ok(next_flags)
    }

    fn build_super_block(
        &self,
        publication: &MountedVolumeState,
        snapshot: &FreeSpaceSnapshot,
    ) -> SuperBlock {
        SuperBlock {
            magic: EXFAT_SUPER_MAGIC,
            bsize: publication.boot_region.cluster_size,
            blocks: snapshot.total_clusters,
            bfree: snapshot.free_clusters,
            bavail: snapshot.free_clusters,
            files: 0,
            ffree: 0,
            fsid: u64::from(publication.boot_region.volume_serial_number),
            namelen: UpcaseTable::NAME_MAX,
            frsize: publication.boot_region.cluster_size,
            flags: u64::from(publication.flags.bits()),
            container_dev_id: self.block_device.id(),
        }
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
        allocator_state: FreeSpaceAllocatorState,
        publication: MountedVolumeState,
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

    fn super_block_snapshot(&self) -> core::result::Result<SuperBlock, FreeSpaceAccountingError> {
        let snapshot = self.cached_free_space_snapshot()?;
        let state = self.state.read();
        let publication = state
            .as_ref()
            .ok_or(FreeSpaceAccountingError::UnpublishedState)?;
        Ok(self.build_super_block(publication, &snapshot))
    }

    fn cached_free_space_snapshot(
        &self,
    ) -> core::result::Result<FreeSpaceSnapshot, FreeSpaceAccountingError> {
        let state = self.state.read();
        let publication = state
            .as_ref()
            .ok_or(FreeSpaceAccountingError::UnpublishedState)?;
        let allocator = self.allocator.read();
        let allocator_state = allocator
            .as_ref()
            .ok_or(FreeSpaceAccountingError::UnpublishedState)?;
        allocator_state.snapshot(&publication.boot_region)
    }

    fn allocate_free_space(
        &self,
        requested_clusters: usize,
    ) -> core::result::Result<
        (Vec<ClusterRange>, FreeSpaceSnapshot),
        FreeSpaceAccountingError,
    > {
        let state = self.state.write();
        let publication = state
            .as_ref()
            .ok_or(FreeSpaceAccountingError::UnpublishedState)?;
        if publication.flags.contains(FsFlags::RDONLY) {
            return Err(FreeSpaceAccountingError::ReadOnlyConflict);
        }

        let mut allocator = self.allocator.write();
        let allocator_state = allocator
            .as_mut()
            .ok_or(FreeSpaceAccountingError::UnpublishedState)?;
        let allocated_ranges = allocator_state.allocate_clusters(
            self.block_device.as_ref(),
            &publication.boot_region,
            requested_clusters,
        )?;
        let snapshot = allocator_state.snapshot(&publication.boot_region)?;
        Ok((allocated_ranges, snapshot))
    }

    fn free_allocated_space(
        &self,
        ranges: &[ClusterRange],
    ) -> core::result::Result<FreeSpaceSnapshot, FreeSpaceAccountingError> {
        let mut state = self.state.write();
        let publication = state
            .as_mut()
            .ok_or(FreeSpaceAccountingError::UnpublishedState)?;
        if publication.flags.contains(FsFlags::RDONLY) {
            return Err(FreeSpaceAccountingError::ReadOnlyConflict);
        }

        let mut allocator = self.allocator.write();
        let allocator_state = allocator
            .as_mut()
            .ok_or(FreeSpaceAccountingError::UnpublishedState)?;
        allocator_state.free_clusters(
            self.block_device.as_ref(),
            &publication.boot_region,
            ranges,
        )?;
        if publication.options.discard {
            // Current Asterinas block devices expose no trim BIO, so downgrade
            // only the advisory policy.
            publication.options.discard = false;
        }
        allocator_state.snapshot(&publication.boot_region)
    }

    fn recount_free_space(
        &self,
    ) -> core::result::Result<FreeSpaceSnapshot, FreeSpaceAccountingError> {
        let state = self.state.write();
        let publication = state
            .as_ref()
            .ok_or(FreeSpaceAccountingError::UnpublishedState)?;
        let mut allocator = self.allocator.write();
        let allocator_state = allocator
            .as_mut()
            .ok_or(FreeSpaceAccountingError::UnpublishedState)?;
        allocator_state.recount(self.block_device.as_ref(), &publication.boot_region)?;
        allocator_state.snapshot(&publication.boot_region)
    }

    fn administrative_trim_free_space(&self) -> Result<()> {
        let state = self.state.write();
        let publication = state
            .as_ref()
            .ok_or(FreeSpaceAccountingError::UnpublishedState)?;
        if publication.flags.contains(FsFlags::RDONLY) {
            return Err(FreeSpaceAccountingError::ReadOnlyConflict.into());
        }

        Err(Error::new(Errno::EOPNOTSUPP))
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
        self.remount_published(flags, &next_options)?;
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

// TODO: `MountVolumeStateError` remains the mount-owned error seam while
// `test_support/` diagnostics still classify mount bootstrap failures here and
// `meso_02` free-space accounting still converts through it. Once `meso_02`
// accepts its final owner-local error boundary and test-only diagnostics
// localize their failure strings, narrow this enum back to mount bootstrap and
// remount failures.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum MountVolumeStateError {
    InvalidOperationInput,
    InvalidMountInput,
    InvalidOnDiskLayout,
    DeviceIo,
    NoSpace,
    UnsupportedRemountDelta,
    ReadOnlyConflict,
    UnpublishedState,
    InconsistentAccounting,
}

impl From<MountVolumeStateError> for Error {
    fn from(error: MountVolumeStateError) -> Self {
        match error {
            MountVolumeStateError::InvalidOperationInput => {
                Error::with_message(Errno::EINVAL, "invalid exFAT operation input")
            }
            MountVolumeStateError::InvalidMountInput => Error::new(Errno::EINVAL),
            MountVolumeStateError::InvalidOnDiskLayout => {
                Error::with_message(Errno::EUCLEAN, "invalid exFAT on-disk layout")
            }
            MountVolumeStateError::DeviceIo => Error::new(Errno::EIO),
            MountVolumeStateError::NoSpace => Error::new(Errno::ENOSPC),
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

// TODO: This bridge remains temporary while `mount_candidate()` still converts
// cached free-space snapshot failures through `MountVolumeStateError`.
// Administrative trim keeps its explicit `EOPNOTSUPP` on the owner-local
// `ExfatFs::administrative_trim_free_space()` boundary until mount bootstrap no
// longer depends on this conversion; then collapse on the final free-space
// error owner and remove this bridge.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum FreeSpaceAccountingError {
    InvalidOperationInput,
    InvalidOnDiskLayout,
    DeviceIo,
    NoSpace,
    ReadOnlyConflict,
    UnpublishedState,
    InconsistentAccounting,
}

impl From<MountVolumeStateError> for FreeSpaceAccountingError {
    fn from(error: MountVolumeStateError) -> Self {
        match error {
            MountVolumeStateError::InvalidOperationInput
            | MountVolumeStateError::InvalidMountInput
            | MountVolumeStateError::UnsupportedRemountDelta => {
                FreeSpaceAccountingError::InvalidOperationInput
            }
            MountVolumeStateError::InvalidOnDiskLayout => {
                FreeSpaceAccountingError::InvalidOnDiskLayout
            }
            MountVolumeStateError::DeviceIo => FreeSpaceAccountingError::DeviceIo,
            MountVolumeStateError::NoSpace => FreeSpaceAccountingError::NoSpace,
            MountVolumeStateError::ReadOnlyConflict => FreeSpaceAccountingError::ReadOnlyConflict,
            MountVolumeStateError::UnpublishedState => FreeSpaceAccountingError::UnpublishedState,
            MountVolumeStateError::InconsistentAccounting => {
                FreeSpaceAccountingError::InconsistentAccounting
            }
        }
    }
}

impl From<FreeSpaceAccountingError> for MountVolumeStateError {
    fn from(error: FreeSpaceAccountingError) -> Self {
        match error {
            FreeSpaceAccountingError::InvalidOperationInput => {
                MountVolumeStateError::InvalidOperationInput
            }
            FreeSpaceAccountingError::InvalidOnDiskLayout => {
                MountVolumeStateError::InvalidOnDiskLayout
            }
            FreeSpaceAccountingError::DeviceIo => MountVolumeStateError::DeviceIo,
            FreeSpaceAccountingError::NoSpace => MountVolumeStateError::NoSpace,
            FreeSpaceAccountingError::ReadOnlyConflict => MountVolumeStateError::ReadOnlyConflict,
            FreeSpaceAccountingError::UnpublishedState => MountVolumeStateError::UnpublishedState,
            FreeSpaceAccountingError::InconsistentAccounting => {
                MountVolumeStateError::InconsistentAccounting
            }
        }
    }
}

impl From<FreeSpaceAccountingError> for Error {
    fn from(error: FreeSpaceAccountingError) -> Self {
        match error {
            FreeSpaceAccountingError::InvalidOperationInput => {
                Error::with_message(Errno::EINVAL, "invalid exFAT operation input")
            }
            FreeSpaceAccountingError::InvalidOnDiskLayout => {
                Error::with_message(Errno::EUCLEAN, "invalid exFAT on-disk layout")
            }
            FreeSpaceAccountingError::DeviceIo => Error::new(Errno::EIO),
            FreeSpaceAccountingError::NoSpace => Error::new(Errno::ENOSPC),
            FreeSpaceAccountingError::ReadOnlyConflict => Error::new(Errno::EROFS),
            FreeSpaceAccountingError::UnpublishedState => {
                Error::with_message(Errno::EINVAL, "filesystem state is not published")
            }
            FreeSpaceAccountingError::InconsistentAccounting => {
                Error::with_message(Errno::EUCLEAN, "exFAT allocator accounting is inconsistent")
            }
        }
    }
}

pub(crate) fn init() {
    if let Err(error) = crate::fs::vfs::registry::register(&ExfatFsType) {
        warn!("failed to register exFAT refactor filesystem: {:?}", error);
    }
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
        let (fs, ..) =
            ExfatFs::mount_candidate(&block_device, Some(block_device.name()), &options)?;
        Ok(fs as Arc<dyn FileSystem>)
    }

    fn sysnode(&self) -> Option<Arc<dyn aster_systree::SysNode>> {
        None
    }
}

#[cfg(ktest)]
#[path = "test_support/fs.rs"]
mod tests;
