// SPDX-License-Identifier: MPL-2.0

use alloc::{collections::BTreeMap, string::String, vec::Vec};

use aster_block::{BlockDevice, bio::BioStatus};
use ostd::{
    mm::VmIo,
    sync::{RwMutex, RwMutexReadGuard, RwMutexWriteGuard},
};

use super::{
    bitmap::{AllocationBitmap, AllocationBitmapUpdate, ClusterRange},
    boot::BootRegion,
    direntry,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ExfatFsError {
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

impl From<ExfatFsError> for Error {
    fn from(error: ExfatFsError) -> Self {
        match error {
            ExfatFsError::InvalidOperationInput => {
                Error::with_message(Errno::EINVAL, "invalid exFAT operation input")
            }
            ExfatFsError::InvalidMountInput => Error::new(Errno::EINVAL),
            ExfatFsError::InvalidOnDiskLayout => {
                Error::with_message(Errno::EUCLEAN, "invalid exFAT on-disk layout")
            }
            ExfatFsError::DeviceIo => Error::new(Errno::EIO),
            ExfatFsError::NoSpace => Error::new(Errno::ENOSPC),
            ExfatFsError::UnsupportedRemountDelta => {
                Error::with_message(Errno::EINVAL, "unsupported exFAT remount delta")
            }
            ExfatFsError::ReadOnlyConflict => Error::new(Errno::EROFS),
            ExfatFsError::UnpublishedState => {
                Error::with_message(Errno::EINVAL, "filesystem state is not published")
            }
            ExfatFsError::InconsistentAccounting => {
                Error::with_message(Errno::EUCLEAN, "exFAT allocator accounting is inconsistent")
            }
        }
    }
}

pub(super) struct ExfatFs {
    allocator: RwLock<Option<FreeSpaceAllocatorState>>,
    block_device: Arc<dyn BlockDevice>,
    fs_event_subscriber_stats: FsEventSubscriberStats,
    inode_cache: RwLock<BTreeMap<u64, Weak<ExfatInode>>>,
    source: Option<String>,
    pub(super) state: RwMutex<Option<MountedVolumeState>>,
}

impl FileSystem for ExfatFs {
    fn name(&self) -> &'static str {
        "exfat"
    }

    fn source(&self) -> Option<&str> {
        self.source.as_deref()
    }

    fn sync(&self) -> Result<()> {
        let mut state = self.state.write();
        {
            let publication = state.as_mut().ok_or(ExfatFsError::UnpublishedState)?;
            if publication.forced_shutdown {
                return_errno!(Errno::EIO);
            }
            if publication.flags.contains(FsFlags::RDONLY) {
                return Err(ExfatFsError::ReadOnlyConflict.into());
            }
        }

        let flush_status = match self.block_device.sync() {
            Ok(status) => status,
            Err(_) => {
                state
                    .as_mut()
                    .ok_or(ExfatFsError::UnpublishedState)?
                    .anomaly
                    .volume_dirty = true;
                return_errno!(Errno::EIO);
            }
        };
        if flush_status != BioStatus::Complete {
            state
                .as_mut()
                .ok_or(ExfatFsError::UnpublishedState)?
                .anomaly
                .volume_dirty = true;
            return_errno!(Errno::EIO);
        }

        let clean_anomaly = {
            let publication = state.as_mut().ok_or(ExfatFsError::UnpublishedState)?;
            if publication.forced_shutdown {
                return_errno!(Errno::EIO);
            }
            if publication.flags.contains(FsFlags::RDONLY) {
                return Err(ExfatFsError::ReadOnlyConflict.into());
            }
            if !publication.anomaly.volume_dirty {
                return Ok(());
            }
            VolumeAnomalyState {
                volume_dirty: false,
                ..publication.anomaly
            }
        };
        let boot_region = state
            .as_ref()
            .ok_or(ExfatFsError::UnpublishedState)?
            .boot_region;
        if let Err(error) =
            boot_region.write_volume_anomaly_state(self.block_device.as_ref(), clean_anomaly)
        {
            state
                .as_mut()
                .ok_or(ExfatFsError::UnpublishedState)?
                .anomaly
                .volume_dirty = true;
            return Err(error.into());
        }

        {
            let publication = state.as_mut().ok_or(ExfatFsError::UnpublishedState)?;
            if publication.flags.contains(FsFlags::RDONLY) {
                publication.anomaly.volume_dirty = true;
                return Err(ExfatFsError::ReadOnlyConflict.into());
            }
        }

        let flush_status = match self.block_device.sync() {
            Ok(status) => status,
            Err(_) => {
                state
                    .as_mut()
                    .ok_or(ExfatFsError::UnpublishedState)?
                    .anomaly
                    .volume_dirty = true;
                return Err(Error::new(Errno::EIO));
            }
        };
        if flush_status != BioStatus::Complete {
            state
                .as_mut()
                .ok_or(ExfatFsError::UnpublishedState)?
                .anomaly
                .volume_dirty = true;
            return_errno!(Errno::EIO);
        }
        state
            .as_mut()
            .ok_or(ExfatFsError::UnpublishedState)?
            .anomaly = clean_anomaly;
        Ok(())
    }

    fn root_inode(&self) -> Arc<dyn Inode> {
        let state = self.state.read();
        let publication = state
            .as_ref()
            .unwrap_or_else(|| unreachable!("mounted exFAT instances must publish a root inode"));
        let root_inode: Arc<dyn Inode> = publication.root_inode.clone();
        root_inode
    }

    fn sb(&self) -> SuperBlock {
        match self.super_block_snapshot() {
            Ok(super_block) => super_block,
            Err(_) => unreachable!("mounted exFAT instances must publish superblock state"),
        }
    }

    fn flags(&self) -> FsFlags {
        let state = self.state.read();
        let publication = state.as_ref().unwrap_or_else(|| {
            unreachable!("mounted exFAT instances must publish filesystem flags")
        });
        publication.flags
    }

    fn set_fs_flags(&self, flags: FsFlags, data: Option<CString>, _ctx: &Context) -> Result<()> {
        let current_options = {
            let state = self.state.read();
            state
                .as_ref()
                .ok_or(ExfatFsError::UnpublishedState)?
                .options
                .clone()
        };
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

impl ExfatFs {
    // Construction and mount publication

    fn new(block_device: Arc<dyn BlockDevice>, source: Option<String>) -> Arc<Self> {
        Arc::new(Self {
            allocator: RwLock::new(None),
            block_device,
            fs_event_subscriber_stats: FsEventSubscriberStats::new(),
            inode_cache: RwLock::new(BTreeMap::new()),
            source,
            state: RwMutex::new(None),
        })
    }

    fn mount_candidate(
        block_device: &Arc<dyn BlockDevice>,
        source: Option<&str>,
        options: &ExfatMountOptions,
    ) -> core::result::Result<(Arc<ExfatFs>, Arc<dyn Inode>, SuperBlock, FsFlags), ExfatFsError>
    {
        let (boot_region, anomaly, bitmap, upcase_table, used_clusters, used_clusters_from_recount) =
            BootRegion::load_mount_state(block_device.as_ref())?;
        let fs = Self::new(block_device.clone(), source.map(ToString::to_string));
        let root_inode =
            ExfatInode::new_root(&fs, boot_region.root_dir_cluster, boot_region.cluster_size);
        let publication = MountedVolumeState {
            anomaly,
            boot_region,
            flags: options.fs_flags,
            options: options.clone(),
            root_inode: root_inode.clone(),
            upcase_table,
            forced_shutdown: false,
        };
        let allocator_state = FreeSpaceAllocatorState {
            bitmap,
            used_clusters,
            used_clusters_from_recount,
        };
        fs.publish_mount_state(allocator_state, publication);
        let super_block = fs.super_block_snapshot()?;
        let root_inode: Arc<dyn Inode> = root_inode;
        Ok((fs, root_inode, super_block, options.fs_flags))
    }

    fn publish_mount_state(
        &self,
        allocator_state: FreeSpaceAllocatorState,
        publication: MountedVolumeState,
    ) {
        *self.allocator.write() = Some(allocator_state);
        *self.state.write() = Some(publication);
    }

    fn remount_published(
        &self,
        next_flags: FsFlags,
        next_options: &ExfatMountOptions,
    ) -> core::result::Result<FsFlags, ExfatFsError> {
        let mut state = self.state.write();
        let publication = state.as_mut().ok_or(ExfatFsError::UnpublishedState)?;
        let changed_flags = publication.flags ^ next_flags;
        if changed_flags.intersects(
            FsFlags::SYNCHRONOUS
                | FsFlags::MANDLOCK
                | FsFlags::DIRSYNC
                | FsFlags::SILENT
                | FsFlags::LAZYTIME,
        ) {
            return Err(ExfatFsError::UnsupportedRemountDelta);
        }
        if publication.flags.contains(FsFlags::RDONLY) && !next_flags.contains(FsFlags::RDONLY) {
            return Err(ExfatFsError::ReadOnlyConflict);
        }
        if publication.options.iocharset != next_options.iocharset
            || publication.options.keep_last_dots != next_options.keep_last_dots
            || publication.options.zero_size_dir != next_options.zero_size_dir
        {
            return Err(ExfatFsError::UnsupportedRemountDelta);
        }
        publication.flags = next_flags;
        publication.options = next_options.with_flags(next_flags);
        Ok(next_flags)
    }

    // Admission

    pub(super) fn published_lookup_state(
        &self,
    ) -> core::result::Result<PublishedLookupState, ExfatFsError> {
        let state = self.state.read();
        let publication = state.as_ref().ok_or(ExfatFsError::UnpublishedState)?;
        Ok(PublishedLookupState {
            block_device: self.block_device.clone(),
            boot_region: publication.boot_region,
            anomaly: publication.anomaly,
            upcase_table: publication.upcase_table.clone(),
            options: publication.options.clone(),
            forced_shutdown: publication.forced_shutdown,
        })
    }

    pub(super) fn admitted_lookup_state(
        &self,
    ) -> core::result::Result<AdmittedLookupState<'_>, ExfatFsError> {
        let state = self.state.read();
        let (boot_region, anomaly, upcase_table, options, forced_shutdown) = {
            let publication = state.as_ref().ok_or(ExfatFsError::UnpublishedState)?;
            (
                publication.boot_region,
                publication.anomaly,
                publication.upcase_table.clone(),
                publication.options.clone(),
                publication.forced_shutdown,
            )
        };
        Ok(AdmittedLookupState {
            state_guard: state,
            block_device: self.block_device.clone(),
            boot_region,
            anomaly,
            upcase_table,
            options,
            forced_shutdown,
        })
    }

    pub(super) fn admitted_mutation_state(
        &self,
    ) -> core::result::Result<AdmittedMutationState<'_>, ExfatFsError> {
        let mut state = self.state.write();
        let (boot_region, anomaly, upcase_table, options, forced_shutdown) = {
            let publication = state.as_mut().ok_or(ExfatFsError::UnpublishedState)?;
            (
                publication.boot_region,
                publication.anomaly,
                publication.upcase_table.clone(),
                publication.options.clone(),
                publication.forced_shutdown,
            )
        };
        Ok(AdmittedMutationState {
            state_guard: state,
            block_device: self.block_device.clone(),
            boot_region,
            anomaly,
            upcase_table,
            options,
            forced_shutdown,
        })
    }

    // Superblock

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

    fn super_block_snapshot(&self) -> core::result::Result<SuperBlock, ExfatFsError> {
        let state = self.state.read();
        let publication = state.as_ref().ok_or(ExfatFsError::UnpublishedState)?;
        let allocator = self.allocator.read();
        let allocator_state = allocator.as_ref().ok_or(ExfatFsError::UnpublishedState)?;
        let snapshot = allocator_state.snapshot(&publication.boot_region)?;
        Ok(self.build_super_block(publication, &snapshot))
    }

    pub(super) fn free_allocated_space(
        &self,
        ranges: &[ClusterRange],
    ) -> core::result::Result<FreeSpaceSnapshot, ExfatFsError> {
        let mut state = self.state.write();
        let publication = state.as_mut().ok_or(ExfatFsError::UnpublishedState)?;
        self.free_allocated_space_with_publication(publication, ranges)
    }

    pub(super) fn allocate_free_space_with_publication(
        &self,
        publication: &MountedVolumeState,
        requested_clusters: usize,
    ) -> core::result::Result<(Vec<ClusterRange>, FreeSpaceSnapshot), ExfatFsError> {
        if publication.flags.contains(FsFlags::RDONLY) {
            return Err(ExfatFsError::ReadOnlyConflict);
        }

        let mut allocator = self.allocator.write();
        let allocator_state = allocator.as_mut().ok_or(ExfatFsError::UnpublishedState)?;
        let allocated_ranges = allocator_state.allocate_clusters(
            self.block_device.as_ref(),
            &publication.boot_region,
            requested_clusters,
        )?;
        let snapshot = allocator_state.snapshot(&publication.boot_region)?;
        Ok((allocated_ranges, snapshot))
    }

    pub(super) fn free_allocated_space_with_publication(
        &self,
        publication: &mut MountedVolumeState,
        ranges: &[ClusterRange],
    ) -> core::result::Result<FreeSpaceSnapshot, ExfatFsError> {
        if publication.flags.contains(FsFlags::RDONLY) {
            return Err(ExfatFsError::ReadOnlyConflict);
        }

        let mut allocator = self.allocator.write();
        let allocator_state = allocator.as_mut().ok_or(ExfatFsError::UnpublishedState)?;
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

    fn cached_free_space_snapshot(&self) -> core::result::Result<FreeSpaceSnapshot, ExfatFsError> {
        let state = self.state.read();
        let publication = state.as_ref().ok_or(ExfatFsError::UnpublishedState)?;
        let allocator = self.allocator.read();
        let allocator_state = allocator.as_ref().ok_or(ExfatFsError::UnpublishedState)?;
        allocator_state.snapshot(&publication.boot_region)
    }

    fn recount_free_space(&self) -> core::result::Result<FreeSpaceSnapshot, ExfatFsError> {
        let state = self.state.write();
        let publication = state.as_ref().ok_or(ExfatFsError::UnpublishedState)?;
        let mut allocator = self.allocator.write();
        let allocator_state = allocator.as_mut().ok_or(ExfatFsError::UnpublishedState)?;
        allocator_state.recount(self.block_device.as_ref(), &publication.boot_region)?;
        allocator_state.snapshot(&publication.boot_region)
    }

    // Helpers

    pub(super) fn container_device_id(&self) -> device_id::DeviceId {
        self.block_device.id()
    }

    pub(super) fn get_or_create_cached_inode(
        &self,
        ino: u64,
        create_inode_fn: impl FnOnce() -> Arc<ExfatInode>,
    ) -> Arc<ExfatInode> {
        if let Some(cached_inode) = self
            .inode_cache
            .read()
            .get(&ino)
            .and_then(Weak::upgrade)
        {
            return cached_inode;
        }

        let mut inode_cache = self.inode_cache.write();
        if let Some(cached_inode) = inode_cache.get(&ino).and_then(Weak::upgrade) {
            return cached_inode;
        }

        let inode = create_inode_fn();
        inode_cache.insert(ino, Arc::downgrade(&inode));
        inode
    }
}

pub(super) struct PublishedLookupState {
    pub(super) block_device: Arc<dyn BlockDevice>,
    pub(super) boot_region: BootRegion,
    pub(super) anomaly: VolumeAnomalyState,
    pub(super) upcase_table: Arc<UpcaseTable>,
    pub(super) options: ExfatMountOptions,
    pub(super) forced_shutdown: bool,
}

pub(super) struct AdmittedLookupState<'a> {
    pub(super) state_guard: RwMutexReadGuard<'a, Option<MountedVolumeState>>,
    pub(super) block_device: Arc<dyn BlockDevice>,
    pub(super) boot_region: BootRegion,
    pub(super) anomaly: VolumeAnomalyState,
    pub(super) upcase_table: Arc<UpcaseTable>,
    pub(super) options: ExfatMountOptions,
    pub(super) forced_shutdown: bool,
}

pub(super) struct AdmittedMutationState<'a> {
    pub(super) state_guard: RwMutexWriteGuard<'a, Option<MountedVolumeState>>,
    pub(super) block_device: Arc<dyn BlockDevice>,
    pub(super) boot_region: BootRegion,
    pub(super) anomaly: VolumeAnomalyState,
    pub(super) upcase_table: Arc<UpcaseTable>,
    pub(super) options: ExfatMountOptions,
    pub(super) forced_shutdown: bool,
}

#[derive(Clone)]
pub(super) struct MountedVolumeState {
    pub(super) anomaly: VolumeAnomalyState,
    pub(super) boot_region: BootRegion,
    pub(super) flags: FsFlags,
    pub(super) options: ExfatMountOptions,
    pub(super) root_inode: Arc<ExfatInode>,
    pub(super) upcase_table: Arc<UpcaseTable>,
    pub(super) forced_shutdown: bool,
}

#[derive(Clone, Copy)]
pub(super) struct VolumeAnomalyState {
    pub(super) clear_to_zero: bool,
    pub(super) media_failure: bool,
    pub(super) volume_dirty: bool,
}

impl VolumeAnomalyState {
    pub(super) fn read(
        block_device: &dyn BlockDevice,
        boot_region: &BootRegion,
    ) -> core::result::Result<Self, ExfatFsError> {
        let mut boot_sector = vec![0; boot_region.sector_size];
        block_device
            .read_bytes(0, &mut boot_sector)
            .map_err(|_| ExfatFsError::DeviceIo)?;
        let volume_flags = u16::from_le_bytes([boot_sector[106], boot_sector[107]]);
        Ok(Self {
            clear_to_zero: volume_flags & 0x0008 != 0,
            media_failure: volume_flags & 0x0004 != 0,
            volume_dirty: volume_flags & 0x0002 != 0,
        })
    }
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
    ) -> core::result::Result<Vec<ClusterRange>, ExfatFsError> {
        if requested_clusters == 0 {
            return Err(ExfatFsError::InvalidOperationInput);
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
            return Err(ExfatFsError::InconsistentAccounting);
        }
        self.used_clusters = self
            .used_clusters
            .checked_add(allocated_clusters)
            .ok_or(ExfatFsError::InconsistentAccounting)?;
        Ok(allocated_ranges)
    }

    fn free_clusters(
        &mut self,
        block_device: &dyn BlockDevice,
        boot_region: &BootRegion,
        ranges: &[ClusterRange],
    ) -> core::result::Result<(), ExfatFsError> {
        let released_clusters = self.bitmap.apply_update(
            block_device,
            boot_region,
            ranges,
            AllocationBitmapUpdate::Free,
        )?;
        self.used_clusters = self
            .used_clusters
            .checked_sub(released_clusters)
            .ok_or(ExfatFsError::InconsistentAccounting)?;
        Ok(())
    }

    fn recount(
        &mut self,
        block_device: &dyn BlockDevice,
        boot_region: &BootRegion,
    ) -> core::result::Result<(), ExfatFsError> {
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
    ) -> core::result::Result<FreeSpaceSnapshot, ExfatFsError> {
        let total_clusters = boot_region.cluster_count_usize()?;
        let free_clusters = total_clusters
            .checked_sub(self.used_clusters)
            .ok_or(ExfatFsError::InconsistentAccounting)?;
        Ok(FreeSpaceSnapshot {
            total_clusters,
            free_clusters,
            used_clusters: self.used_clusters,
            used_clusters_from_recount: self.used_clusters_from_recount,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct FreeSpaceSnapshot {
    total_clusters: usize,
    free_clusters: usize,
    used_clusters: usize,
    used_clusters_from_recount: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct ExfatMountOptions {
    discard: bool,
    pub(super) fs_flags: FsFlags,
    pub(super) iocharset: String,
    pub(super) keep_last_dots: bool,
    pub(super) zero_size_dir: bool,
}

impl ExfatMountOptions {
    fn parse(fs_flags: FsFlags, args: Option<&CStr>) -> core::result::Result<Self, ExfatFsError> {
        let mut options = Self {
            discard: false,
            fs_flags,
            iocharset: "utf8".to_string(),
            keep_last_dots: false,
            zero_size_dir: false,
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
                "keep_last_dots" => options.keep_last_dots = true,
                "nokeep_last_dots" => options.keep_last_dots = false,
                "zero_size_dir" => options.zero_size_dir = true,
                "nozero_size_dir" => options.zero_size_dir = false,
                _ if entry.starts_with("iocharset=") => {
                    let iocharset = entry
                        .split_once('=')
                        .map(|(_, value)| value)
                        .ok_or(ExfatFsError::InvalidMountInput)?;
                    if !iocharset.eq_ignore_ascii_case("utf8") {
                        return Err(ExfatFsError::InvalidMountInput);
                    }
                    options.iocharset = "utf8".to_string();
                }
                _ => return Err(ExfatFsError::InvalidMountInput),
            }
        }
        Ok(options)
    }

    fn with_flags(&self, fs_flags: FsFlags) -> Self {
        Self {
            fs_flags,
            ..self.clone()
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
