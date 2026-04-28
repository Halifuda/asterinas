// SPDX-License-Identifier: MPL-2.0

use alloc::{string::String, vec::Vec};

use aster_block::{BlockDevice, bio::BioStatus};
use ostd::sync::{RwMutex, RwMutexReadGuard, RwMutexWriteGuard};

use super::{
    bitmap::{AllocationBitmap, AllocationBitmapUpdate, ClusterRange},
    boot::{BootRegion, VolumeAnomalyState},
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
    process::credentials::capabilities::CapSet,
};

const EXFAT_SUPER_MAGIC: u64 = 0x2011_BAB0;

#[derive(Clone)]
pub(super) struct MountedVolumeState {
    anomaly: VolumeAnomalyState,
    boot_region: BootRegion,
    flags: FsFlags,
    options: ExfatMountOptions,
    root_inode: Arc<ExfatInode>,
    upcase_table: Arc<UpcaseTable>,
    forced_shutdown: bool,
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
    ) -> core::result::Result<Vec<ClusterRange>, MountVolumeStateError> {
        if requested_clusters == 0 {
            return Err(MountVolumeStateError::InvalidOperationInput);
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
            return Err(MountVolumeStateError::InconsistentAccounting);
        }
        self.used_clusters = self
            .used_clusters
            .checked_add(allocated_clusters)
            .ok_or(MountVolumeStateError::InconsistentAccounting)?;
        Ok(allocated_ranges)
    }

    fn free_clusters(
        &mut self,
        block_device: &dyn BlockDevice,
        boot_region: &BootRegion,
        ranges: &[ClusterRange],
    ) -> core::result::Result<(), MountVolumeStateError> {
        let released_clusters = self.bitmap.apply_update(
            block_device,
            boot_region,
            ranges,
            AllocationBitmapUpdate::Free,
        )?;
        self.used_clusters = self
            .used_clusters
            .checked_sub(released_clusters)
            .ok_or(MountVolumeStateError::InconsistentAccounting)?;
        Ok(())
    }

    fn recount(
        &mut self,
        block_device: &dyn BlockDevice,
        boot_region: &BootRegion,
    ) -> core::result::Result<(), MountVolumeStateError> {
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
    ) -> core::result::Result<FreeSpaceSnapshot, MountVolumeStateError> {
        let total_clusters = boot_region.cluster_count_usize()?;
        let free_clusters = total_clusters
            .checked_sub(self.used_clusters)
            .ok_or(MountVolumeStateError::InconsistentAccounting)?;
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

pub(super) struct ExfatFs {
    allocator: RwLock<Option<FreeSpaceAllocatorState>>,
    block_device: Arc<dyn BlockDevice>,
    fs_event_subscriber_stats: FsEventSubscriberStats,
    source: Option<String>,
    state: RwMutex<Option<MountedVolumeState>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct VolumeIdentityEntries {
    pub(super) guid: Option<[u8; 16]>,
    pub(super) label: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum VolumeIdentityQuery {
    Guid,
    Label,
    LabelAndGuid,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum VolumeIdentityUpdate {
    Guid(Option<[u8; 16]>),
    Label(Option<String>),
    LabelAndGuid {
        guid: Option<[u8; 16]>,
        label: Option<String>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum VolumeAdminRequest {
    ForceShutdown,
    TrimFreeSpace,
    UpdateIdentity(VolumeIdentityUpdate),
}

impl ExfatFs {
    fn new(block_device: Arc<dyn BlockDevice>, source: Option<String>) -> Arc<Self> {
        Arc::new(Self {
            allocator: RwLock::new(None),
            block_device,
            fs_event_subscriber_stats: FsEventSubscriberStats::new(),
            source,
            state: RwMutex::new(None),
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
        if publication.options.iocharset != next_options.iocharset
            || publication.options.keep_last_dots != next_options.keep_last_dots
            || publication.options.zero_size_dir != next_options.zero_size_dir
        {
            return Err(MountVolumeStateError::UnsupportedRemountDelta);
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

    #[cfg(ktest)]
    fn current_options(&self) -> core::result::Result<ExfatMountOptions, MountVolumeStateError> {
        let state = self.state.read();
        let publication = state
            .as_ref()
            .ok_or(MountVolumeStateError::UnpublishedState)?;
        Ok(publication.options.clone())
    }

    fn publish_mount_state(
        &self,
        allocator_state: FreeSpaceAllocatorState,
        publication: MountedVolumeState,
    ) {
        *self.allocator.write() = Some(allocator_state);
        *self.state.write() = Some(publication);
    }

    #[cfg(ktest)]
    fn published_flags(&self) -> core::result::Result<FsFlags, MountVolumeStateError> {
        let state = self.state.read();
        let publication = state
            .as_ref()
            .ok_or(MountVolumeStateError::UnpublishedState)?;
        Ok(publication.flags)
    }

    pub(super) fn published_lookup_state(
        &self,
    ) -> core::result::Result<
        (
            Arc<dyn BlockDevice>,
            BootRegion,
            VolumeAnomalyState,
            Arc<UpcaseTable>,
            ExfatMountOptions,
        ),
        MountVolumeStateError,
    > {
        let state = self.state.read();
        let publication = state
            .as_ref()
            .ok_or(MountVolumeStateError::UnpublishedState)?;
        Ok((
            self.block_device.clone(),
            publication.boot_region,
            publication.anomaly,
            publication.upcase_table.clone(),
            publication.options.clone(),
        ))
    }

    pub(super) fn admitted_lookup_state(
        &self,
    ) -> core::result::Result<
        (
            RwMutexReadGuard<'_, Option<MountedVolumeState>>,
            Arc<dyn BlockDevice>,
            BootRegion,
            VolumeAnomalyState,
            Arc<UpcaseTable>,
            ExfatMountOptions,
        ),
        MountVolumeStateError,
    > {
        let state = self.state.read();
        let (boot_region, anomaly, upcase_table, options) = {
            let publication = state
                .as_ref()
                .ok_or(MountVolumeStateError::UnpublishedState)?;
            (
                publication.boot_region,
                publication.anomaly,
                publication.upcase_table.clone(),
                publication.options.clone(),
            )
        };
        Ok((
            state,
            self.block_device.clone(),
            boot_region,
            anomaly,
            upcase_table,
            options,
        ))
    }

    pub(super) fn admitted_mutation_state(
        &self,
    ) -> core::result::Result<
        (
            RwMutexWriteGuard<'_, Option<MountedVolumeState>>,
            Arc<dyn BlockDevice>,
            BootRegion,
            VolumeAnomalyState,
            Arc<UpcaseTable>,
            ExfatMountOptions,
        ),
        MountVolumeStateError,
    > {
        let mut state = self.state.write();
        let dirty_anomaly = {
            let publication = state
                .as_mut()
                .ok_or(MountVolumeStateError::UnpublishedState)?;
            if publication.forced_shutdown {
                return Err(MountVolumeStateError::DeviceIo);
            }
            if publication.flags.contains(FsFlags::RDONLY) {
                return Err(MountVolumeStateError::ReadOnlyConflict);
            }
            (!publication.anomaly.volume_dirty).then_some(VolumeAnomalyState {
                volume_dirty: true,
                ..publication.anomaly
            })
        };
        if let Some(dirty_anomaly) = dirty_anomaly {
            let boot_region = state
                .as_ref()
                .ok_or(MountVolumeStateError::UnpublishedState)?
                .boot_region;
            if let Err(error) =
                boot_region.write_volume_anomaly_state(self.block_device.as_ref(), dirty_anomaly)
            {
                state
                    .as_mut()
                    .ok_or(MountVolumeStateError::UnpublishedState)?
                    .anomaly
                    .volume_dirty = true;
                return Err(error);
            }
            let flush_status = match self.block_device.sync() {
                Ok(status) => status,
                Err(_) => {
                    state
                        .as_mut()
                        .ok_or(MountVolumeStateError::UnpublishedState)?
                        .anomaly
                        .volume_dirty = true;
                    return Err(MountVolumeStateError::DeviceIo);
                }
            };
            if flush_status != BioStatus::Complete {
                state
                    .as_mut()
                    .ok_or(MountVolumeStateError::UnpublishedState)?
                    .anomaly
                    .volume_dirty = true;
                return Err(MountVolumeStateError::DeviceIo);
            }
            state
                .as_mut()
                .ok_or(MountVolumeStateError::UnpublishedState)?
                .anomaly = dirty_anomaly;
        }
        let (boot_region, anomaly, upcase_table, options) = {
            let publication = state
                .as_mut()
                .ok_or(MountVolumeStateError::UnpublishedState)?;
            (
                publication.boot_region,
                publication.anomaly,
                publication.upcase_table.clone(),
                publication.options.clone(),
            )
        };
        Ok((
            state,
            self.block_device.clone(),
            boot_region,
            anomaly,
            upcase_table,
            options,
        ))
    }

    pub(crate) fn handle_volume_admin_request(
        &self,
        request: VolumeAdminRequest,
        ctx: &Context,
    ) -> Result<()> {
        let is_privileged = ctx
            .posix_thread
            .credentials()
            .effective_capset()
            .contains(CapSet::SYS_ADMIN);
        let ensure_privileged_fn = || {
            if is_privileged {
                return Ok(());
            }
            return_errno_with_message!(
                Errno::EPERM,
                "exFAT volume administration requires SYS_ADMIN"
            )
        };
        match request {
            VolumeAdminRequest::ForceShutdown => {
                ensure_privileged_fn()?;
                self.admit_forced_shutdown().map_err(Error::from)
            }
            VolumeAdminRequest::TrimFreeSpace => {
                ensure_privileged_fn()?;
                self.administrative_trim_free_space()
            }
            VolumeAdminRequest::UpdateIdentity(update) => {
                ensure_privileged_fn()?;
                self.update_volume_identity(update)
            }
        }
    }

    pub(super) fn query_volume_identity(
        &self,
        query: VolumeIdentityQuery,
    ) -> Result<VolumeIdentityEntries> {
        match query {
            VolumeIdentityQuery::Label => {
                let (state, block_device, boot_region, _anomaly, _upcase_table, _options) =
                    self.admitted_lookup_state()?;
                let root_inode = state
                    .as_ref()
                    .ok_or(MountVolumeStateError::UnpublishedState)?
                    .root_inode
                    .clone();
                let label = root_inode.read_root_directory(
                    &block_device,
                    &boot_region,
                    direntry::read_volume_label,
                )?;
                let label = match label {
                    Some(label) => String::from_utf16(&label)
                        .map(Some)
                        .map_err(|_| MountVolumeStateError::InvalidOnDiskLayout.into())?,
                    None => None,
                };
                Ok(VolumeIdentityEntries { guid: None, label })
            }
            VolumeIdentityQuery::Guid | VolumeIdentityQuery::LabelAndGuid => {
                return_errno_with_message!(
                    Errno::EOPNOTSUPP,
                    "exFAT volume GUID administration is not supported"
                );
            }
        }
    }

    pub(super) fn update_volume_identity(&self, update: VolumeIdentityUpdate) -> Result<()> {
        match update {
            VolumeIdentityUpdate::Label(label) => {
                let admitted_label = match label {
                    None => None,
                    Some(label) if label.is_empty() => None,
                    Some(label) => {
                        let admitted_label: Vec<u16> = label.encode_utf16().collect();
                        if admitted_label.len() > 11 {
                            return_errno_with_message!(Errno::EINVAL, "invalid exFAT volume label");
                        }
                        Some(admitted_label)
                    }
                };
                let (state, block_device, boot_region, _anomaly, _upcase_table, _options) =
                    self.admitted_mutation_state()?;
                let root_inode = state
                    .as_ref()
                    .ok_or(MountVolumeStateError::UnpublishedState)?
                    .root_inode
                    .clone();

                root_inode
                    .rewrite_root_directory(&block_device, &boot_region, |directory_bytes| {
                        direntry::write_volume_label(directory_bytes, admitted_label.as_deref())
                    })
                    .map_err(Error::from)
            }
            VolumeIdentityUpdate::Guid(_) | VolumeIdentityUpdate::LabelAndGuid { .. } => {
                return_errno_with_message!(
                    Errno::EOPNOTSUPP,
                    "exFAT volume GUID administration is not supported"
                );
            }
        }
    }

    pub(super) fn admit_forced_shutdown(&self) -> core::result::Result<(), MountVolumeStateError> {
        let mut state = self.state.write();
        let publication = state
            .as_mut()
            .ok_or(MountVolumeStateError::UnpublishedState)?;
        publication.forced_shutdown = true;
        Ok(())
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
        let snapshot = allocator_state.snapshot(&publication.boot_region)?;
        Ok(self.build_super_block(publication, &snapshot))
    }

    #[cfg(ktest)]
    fn cached_free_space_snapshot(
        &self,
    ) -> core::result::Result<FreeSpaceSnapshot, MountVolumeStateError> {
        let state = self.state.read();
        let publication = state
            .as_ref()
            .ok_or(MountVolumeStateError::UnpublishedState)?;
        let allocator = self.allocator.read();
        let allocator_state = allocator
            .as_ref()
            .ok_or(MountVolumeStateError::UnpublishedState)?;
        allocator_state.snapshot(&publication.boot_region)
    }

    pub(super) fn allocate_free_space(
        &self,
        requested_clusters: usize,
    ) -> core::result::Result<(Vec<ClusterRange>, FreeSpaceSnapshot), MountVolumeStateError>
    {
        let state = self.state.write();
        let publication = state
            .as_ref()
            .ok_or(MountVolumeStateError::UnpublishedState)?;
        self.allocate_free_space_with_publication(publication, requested_clusters)
    }

    pub(super) fn free_allocated_space(
        &self,
        ranges: &[ClusterRange],
    ) -> core::result::Result<FreeSpaceSnapshot, MountVolumeStateError> {
        let mut state = self.state.write();
        let publication = state
            .as_mut()
            .ok_or(MountVolumeStateError::UnpublishedState)?;
        self.free_allocated_space_with_publication(publication, ranges)
    }

    pub(super) fn allocate_free_space_with_publication(
        &self,
        publication: &MountedVolumeState,
        requested_clusters: usize,
    ) -> core::result::Result<(Vec<ClusterRange>, FreeSpaceSnapshot), MountVolumeStateError>
    {
        if publication.flags.contains(FsFlags::RDONLY) {
            return Err(MountVolumeStateError::ReadOnlyConflict);
        }

        let mut allocator = self.allocator.write();
        let allocator_state = allocator
            .as_mut()
            .ok_or(MountVolumeStateError::UnpublishedState)?;
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
    ) -> core::result::Result<FreeSpaceSnapshot, MountVolumeStateError> {
        if publication.flags.contains(FsFlags::RDONLY) {
            return Err(MountVolumeStateError::ReadOnlyConflict);
        }

        let mut allocator = self.allocator.write();
        let allocator_state = allocator
            .as_mut()
            .ok_or(MountVolumeStateError::UnpublishedState)?;
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

    #[cfg(ktest)]
    fn recount_free_space(
        &self,
    ) -> core::result::Result<FreeSpaceSnapshot, MountVolumeStateError> {
        let state = self.state.write();
        let publication = state
            .as_ref()
            .ok_or(MountVolumeStateError::UnpublishedState)?;
        let mut allocator = self.allocator.write();
        let allocator_state = allocator
            .as_mut()
            .ok_or(MountVolumeStateError::UnpublishedState)?;
        allocator_state.recount(self.block_device.as_ref(), &publication.boot_region)?;
        allocator_state.snapshot(&publication.boot_region)
    }

    pub(super) fn administrative_trim_free_space(&self) -> Result<()> {
        let state = self.state.write();
        let publication = state
            .as_ref()
            .ok_or(MountVolumeStateError::UnpublishedState)?;
        if publication.forced_shutdown {
            return Err(Error::new(Errno::EIO));
        }
        if publication.flags.contains(FsFlags::RDONLY) {
            return Err(MountVolumeStateError::ReadOnlyConflict.into());
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
        let mut state = self.state.write();
        {
            let publication = state
                .as_mut()
                .ok_or(MountVolumeStateError::UnpublishedState)?;
            if publication.forced_shutdown {
                return Err(Error::new(Errno::EIO));
            }
            if publication.flags.contains(FsFlags::RDONLY) {
                return Err(MountVolumeStateError::ReadOnlyConflict.into());
            }
        }

        let flush_status = match self.block_device.sync() {
            Ok(status) => status,
            Err(_) => {
                state
                    .as_mut()
                    .ok_or(MountVolumeStateError::UnpublishedState)?
                    .anomaly
                    .volume_dirty = true;
                return Err(Error::new(Errno::EIO));
            }
        };
        if flush_status != BioStatus::Complete {
            state
                .as_mut()
                .ok_or(MountVolumeStateError::UnpublishedState)?
                .anomaly
                .volume_dirty = true;
            return_errno!(Errno::EIO);
        }

        let clean_anomaly = {
            let publication = state
                .as_mut()
                .ok_or(MountVolumeStateError::UnpublishedState)?;
            if publication.forced_shutdown {
                return Err(Error::new(Errno::EIO));
            }
            if publication.flags.contains(FsFlags::RDONLY) {
                return Err(MountVolumeStateError::ReadOnlyConflict.into());
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
            .ok_or(MountVolumeStateError::UnpublishedState)?
            .boot_region;
        if let Err(error) =
            boot_region.write_volume_anomaly_state(self.block_device.as_ref(), clean_anomaly)
        {
            state
                .as_mut()
                .ok_or(MountVolumeStateError::UnpublishedState)?
                .anomaly
                .volume_dirty = true;
            return Err(error.into());
        }

        {
            let publication = state
                .as_mut()
                .ok_or(MountVolumeStateError::UnpublishedState)?;
            if publication.flags.contains(FsFlags::RDONLY) {
                publication.anomaly.volume_dirty = true;
                return Err(MountVolumeStateError::ReadOnlyConflict.into());
            }
        }

        let flush_status = match self.block_device.sync() {
            Ok(status) => status,
            Err(_) => {
                state
                    .as_mut()
                    .ok_or(MountVolumeStateError::UnpublishedState)?
                    .anomaly
                    .volume_dirty = true;
                return Err(Error::new(Errno::EIO));
            }
        };
        if flush_status != BioStatus::Complete {
            state
                .as_mut()
                .ok_or(MountVolumeStateError::UnpublishedState)?
                .anomaly
                .volume_dirty = true;
            return_errno!(Errno::EIO);
        }
        state
            .as_mut()
            .ok_or(MountVolumeStateError::UnpublishedState)?
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
                .ok_or(MountVolumeStateError::UnpublishedState)?
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct ExfatMountOptions {
    discard: bool,
    pub(super) fs_flags: FsFlags,
    pub(super) iocharset: String,
    pub(super) keep_last_dots: bool,
    pub(super) zero_size_dir: bool,
}

impl ExfatMountOptions {
    fn parse(
        fs_flags: FsFlags,
        args: Option<&CStr>,
    ) -> core::result::Result<Self, MountVolumeStateError> {
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
                        .ok_or(MountVolumeStateError::InvalidMountInput)?;
                    if !iocharset.eq_ignore_ascii_case("utf8") {
                        return Err(MountVolumeStateError::InvalidMountInput);
                    }
                    options.iocharset = "utf8".to_string();
                }
                _ => return Err(MountVolumeStateError::InvalidMountInput),
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum MountVolumeStateError {
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
