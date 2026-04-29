// SPDX-License-Identifier: MPL-2.0

//! Implements the exFAT filesystem owner, mount admission, allocation, and VFS registration.

use alloc::{collections::BTreeMap, string::String, vec::Vec};

use aster_block::{BlockDevice, bio::BioStatus};
use ostd::{
    mm::VmIo,
    sync::{RwMutex, RwMutexReadGuard, RwMutexWriteGuard},
};

use super::{
    bitmap::{AllocationBitmap, AllocationBitmapUpdate, ClusterRange},
    boot::BootRegion,
    fat::FatReader,
    device_io, inconsistent_bitmap_accounting, invalid_mount_input, invalid_operation_input,
    read_only_conflict, unpublished_state, unsupported_remount_delta,
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

pub(super) struct ExfatFs {
    allocator: RwLock<Option<AllocationBitmap>>,
    block_device: Arc<dyn BlockDevice>,
    boot_region: BootRegion,
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
        loop {
            let live_inodes = self.live_cached_inodes();
            for inode in &live_inodes {
                if let Err(error) = inode.sync_all() {
                    if let Some(publication) = self.state.write().as_mut() {
                        publication.anomaly.volume_dirty = true;
                    }
                    return Err(error);
                }
            }

            let mut state = self.state.write();
            {
                let publication = state.as_mut().ok_or_else(unpublished_state)?;
                if publication.forced_shutdown {
                    return_errno!(Errno::EIO);
                }
                if publication.flags.contains(FsFlags::RDONLY) {
                    return Err(read_only_conflict());
                }
            }

            if self
                .live_cached_inodes()
                .into_iter()
                .any(|inode| inode.has_pending_regular_file_sync())
            {
                drop(state);
                continue;
            }

            let flush_status = match self.block_device.sync() {
                Ok(status) => status,
                Err(_) => {
                    state
                        .as_mut()
                        .ok_or_else(unpublished_state)?
                        .anomaly
                        .volume_dirty = true;
                    return_errno!(Errno::EIO);
                }
            };
            if flush_status != BioStatus::Complete {
                state
                    .as_mut()
                    .ok_or_else(unpublished_state)?
                    .anomaly
                    .volume_dirty = true;
                return_errno!(Errno::EIO);
            }

            let clean_anomaly = {
                let publication = state.as_mut().ok_or_else(unpublished_state)?;
                if !publication.anomaly.volume_dirty {
                    return Ok(());
                }
                VolumeAnomalyState {
                    volume_dirty: false,
                    ..publication.anomaly
                }
            };
            if let Err(error) = self
                .boot_region
                .write_volume_anomaly_state(self.block_device.as_ref(), clean_anomaly)
            {
                state
                    .as_mut()
                    .ok_or_else(unpublished_state)?
                    .anomaly
                    .volume_dirty = true;
                return Err(error);
            }

            let flush_status = match self.block_device.sync() {
                Ok(status) => status,
                Err(_) => {
                    state
                        .as_mut()
                        .ok_or_else(unpublished_state)?
                        .anomaly
                        .volume_dirty = true;
                    return Err(Error::new(Errno::EIO));
                }
            };
            if flush_status != BioStatus::Complete {
                state
                    .as_mut()
                    .ok_or_else(unpublished_state)?
                    .anomaly
                    .volume_dirty = true;
                return_errno!(Errno::EIO);
            }

            state
                .as_mut()
                .ok_or_else(unpublished_state)?
                .anomaly = clean_anomaly;
            return Ok(());
        }
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
                .ok_or_else(unpublished_state)?
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

    fn new(
        block_device: Arc<dyn BlockDevice>,
        boot_region: BootRegion,
        source: Option<String>,
    ) -> Arc<Self> {
        Arc::new(Self {
            allocator: RwLock::new(None),
            block_device,
            boot_region,
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
    ) -> Result<(Arc<ExfatFs>, Arc<dyn Inode>, SuperBlock, FsFlags)>
    {
        let (boot_region, anomaly, bitmap, upcase_table) =
            BootRegion::load_mount_state(block_device.as_ref())?;
        let fs = Self::new(
            block_device.clone(),
            boot_region,
            source.map(ToString::to_string),
        );
        let root_inode =
            ExfatInode::new_root(&fs, boot_region.root_dir_cluster, boot_region.cluster_size);
        let publication = MountedVolumeState {
            anomaly,
            flags: options.fs_flags,
            options: options.clone(),
            root_inode: root_inode.clone(),
            upcase_table,
            forced_shutdown: false,
        };
        fs.publish_mount_state(bitmap, publication);
        let super_block = fs.super_block_snapshot()?;
        let root_inode: Arc<dyn Inode> = root_inode;
        Ok((fs, root_inode, super_block, options.fs_flags))
    }

    fn publish_mount_state(&self, bitmap: AllocationBitmap, publication: MountedVolumeState) {
        *self.allocator.write() = Some(bitmap);
        *self.state.write() = Some(publication);
    }

    fn remount_published(
        &self,
        next_flags: FsFlags,
        next_options: &ExfatMountOptions,
    ) -> Result<FsFlags> {
        let mut state = self.state.write();
        let publication = state.as_mut().ok_or_else(unpublished_state)?;
        let changed_flags = publication.flags ^ next_flags;
        if changed_flags.intersects(
            FsFlags::SYNCHRONOUS
                | FsFlags::MANDLOCK
                | FsFlags::DIRSYNC
                | FsFlags::SILENT
                | FsFlags::LAZYTIME,
        ) {
            return Err(unsupported_remount_delta());
        }
        if publication.flags.contains(FsFlags::RDONLY) && !next_flags.contains(FsFlags::RDONLY) {
            return Err(read_only_conflict());
        }
        if publication.options.iocharset != next_options.iocharset
            || publication.options.keep_last_dots != next_options.keep_last_dots
            || publication.options.zero_size_dir != next_options.zero_size_dir
        {
            return Err(unsupported_remount_delta());
        }
        publication.flags = next_flags;
        publication.options = next_options.with_flags(next_flags);
        Ok(next_flags)
    }

    // Admission

    pub(super) fn published_lookup_state(&self) -> Result<PublishedLookupState> {
        let state = self.state.read();
        let publication = state.as_ref().ok_or_else(unpublished_state)?;
        Ok(PublishedLookupState {
            block_device: self.block_device.clone(),
            boot_region: self.boot_region,
            anomaly: publication.anomaly,
            upcase_table: publication.upcase_table.clone(),
            options: publication.options.clone(),
            forced_shutdown: publication.forced_shutdown,
        })
    }

    pub(super) fn admitted_lookup_state(&self) -> Result<AdmittedLookupState<'_>> {
        let state = self.state.read();
        let (anomaly, upcase_table, options, forced_shutdown) = {
            let publication = state.as_ref().ok_or_else(unpublished_state)?;
            (
                publication.anomaly,
                publication.upcase_table.clone(),
                publication.options.clone(),
                publication.forced_shutdown,
            )
        };
        Ok(AdmittedLookupState {
            state_guard: state,
            block_device: self.block_device.clone(),
            boot_region: self.boot_region,
            anomaly,
            upcase_table,
            options,
            forced_shutdown,
        })
    }

    pub(super) fn admitted_mutation_state(&self) -> Result<AdmittedMutationState<'_>> {
        let mut state = self.state.write();
        let (anomaly, upcase_table, options, forced_shutdown) = {
            let publication = state.as_mut().ok_or_else(unpublished_state)?;
            (
                publication.anomaly,
                publication.upcase_table.clone(),
                publication.options.clone(),
                publication.forced_shutdown,
            )
        };
        Ok(AdmittedMutationState {
            state_guard: state,
            block_device: self.block_device.clone(),
            boot_region: self.boot_region,
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
        bitmap: &AllocationBitmap,
    ) -> Result<SuperBlock> {
        let total_clusters = self.boot_region.cluster_count_usize()?;
        let free_clusters = total_clusters
            .checked_sub(bitmap.used_clusters())
            .ok_or_else(inconsistent_bitmap_accounting)?;
        Ok(SuperBlock {
            magic: EXFAT_SUPER_MAGIC,
            bsize: self.boot_region.cluster_size,
            blocks: total_clusters,
            bfree: free_clusters,
            bavail: free_clusters,
            files: 0,
            ffree: 0,
            fsid: u64::from(self.boot_region.volume_serial_number),
            namelen: UpcaseTable::NAME_MAX,
            frsize: self.boot_region.cluster_size,
            flags: u64::from(publication.flags.bits()),
            container_dev_id: self.block_device.id(),
        })
    }

    fn super_block_snapshot(&self) -> Result<SuperBlock> {
        let state = self.state.read();
        let publication = state.as_ref().ok_or_else(unpublished_state)?;
        let allocator = self.allocator.read();
        let bitmap = allocator.as_ref().ok_or_else(unpublished_state)?;
        self.build_super_block(publication, bitmap)
    }

    pub(super) fn allocate_free_space_with_publication(
        &self,
        publication: &MountedVolumeState,
        requested_clusters: usize,
    ) -> Result<Vec<ClusterRange>> {
        if publication.flags.contains(FsFlags::RDONLY) {
            return Err(read_only_conflict());
        }

        let mut allocator = self.allocator.write();
        let bitmap = allocator.as_mut().ok_or_else(unpublished_state)?;
        if requested_clusters == 0 {
            return Err(invalid_operation_input());
        }

        let mut fat_reader = FatReader::new(self.block_device.as_ref(), &self.boot_region);
        let allocated_ranges =
            bitmap.find_free_ranges(&self.boot_region, &mut fat_reader, requested_clusters)?;
        let normalized_ranges =
            bitmap.validate_and_normalize_ranges(&self.boot_region, &allocated_ranges)?;
        let allocated_clusters = bitmap.apply_normalized_ranges(
            self.block_device.as_ref(),
            &self.boot_region,
            &normalized_ranges,
            AllocationBitmapUpdate::Allocate,
        )?;
        if allocated_clusters != requested_clusters {
            return Err(inconsistent_bitmap_accounting());
        }
        bitmap.record_allocated_clusters(allocated_clusters)?;
        Ok(allocated_ranges)
    }

    pub(super) fn free_allocated_space_with_publication(
        &self,
        publication: &mut MountedVolumeState,
        ranges: &[ClusterRange],
    ) -> Result<()> {
        if publication.flags.contains(FsFlags::RDONLY) {
            return Err(read_only_conflict());
        }

        let mut allocator = self.allocator.write();
        let bitmap = allocator.as_mut().ok_or_else(unpublished_state)?;
        let normalized_ranges = bitmap.validate_and_normalize_ranges(&self.boot_region, ranges)?;
        let released_clusters = bitmap.apply_normalized_ranges(
            self.block_device.as_ref(),
            &self.boot_region,
            &normalized_ranges,
            AllocationBitmapUpdate::Free,
        )?;
        bitmap.record_released_clusters(released_clusters)?;
        if publication.options.discard {
            // Current Asterinas block devices expose no trim BIO, so downgrade
            // only the advisory policy.
            publication.options.discard = false;
        }
        Ok(())
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
        if let Some(cached_inode) = self.inode_cache.read().get(&ino).and_then(Weak::upgrade) {
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

    fn live_cached_inodes(&self) -> Vec<Arc<ExfatInode>> {
        let mut inode_cache = self.inode_cache.write();
        let mut live_inodes = Vec::with_capacity(inode_cache.len());
        inode_cache.retain(|_, inode| match inode.upgrade() {
            Some(inode) => {
                live_inodes.push(inode);
                true
            }
            None => false,
        });
        live_inodes
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
    pub(super) fn read(block_device: &dyn BlockDevice, boot_region: &BootRegion) -> Result<Self> {
        let mut boot_sector = vec![0; boot_region.sector_size];
        block_device
            .read_bytes(0, &mut boot_sector)
            .map_err(|_| device_io())?;
        let volume_flags = u16::from_le_bytes([boot_sector[106], boot_sector[107]]);
        Ok(Self {
            clear_to_zero: volume_flags & 0x0008 != 0,
            media_failure: volume_flags & 0x0004 != 0,
            volume_dirty: volume_flags & 0x0002 != 0,
        })
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
    fn parse(fs_flags: FsFlags, args: Option<&CStr>) -> Result<Self> {
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
                        .ok_or_else(invalid_mount_input)?;
                    if !iocharset.eq_ignore_ascii_case("utf8") {
                        return Err(invalid_mount_input());
                    }
                    options.iocharset = "utf8".to_string();
                }
                _ => return Err(invalid_mount_input()),
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
