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
    device_io,
    fat::FatReader,
    inconsistent_bitmap_accounting,
    inode::ExfatInode,
    invalid_mount_input, invalid_operation_input, read_only_conflict, unpublished_state,
    unsupported_remount_delta,
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
    allocator: RwMutex<Option<AllocationBitmap>>,
    block_device: Arc<dyn BlockDevice>,
    boot_region: BootRegion,
    fs_event_subscriber_stats: FsEventSubscriberStats,
    inode_cache: RwLock<BTreeMap<u64, Weak<ExfatInode>>>,
    pub(super) root_inode: RwMutex<Option<Arc<ExfatInode>>>,
    source: Option<String>,
    pub(super) mount_state: RwMutex<Option<MountedVolumeState>>,
    upcase_table: RwMutex<Option<Arc<UpcaseTable>>>,
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
                    if let Some(mount_state) = self.mount_state.write().as_mut() {
                        mount_state.anomaly.volume_dirty = true;
                    }
                    return Err(error);
                }
            }

            let mut mount_state = self.mount_state.write();
            {
                let mount_state = mount_state.as_mut().ok_or_else(unpublished_state)?;
                if mount_state.forced_shutdown {
                    return_errno!(Errno::EIO);
                }
                if mount_state.flags.contains(FsFlags::RDONLY) {
                    return Err(read_only_conflict());
                }
            }

            if self
                .live_cached_inodes()
                .into_iter()
                .any(|inode| inode.has_pending_regular_file_sync())
            {
                drop(mount_state);
                continue;
            }

            let flush_status = match self.block_device.sync() {
                Ok(status) => status,
                Err(_) => {
                    mount_state
                        .as_mut()
                        .ok_or_else(unpublished_state)?
                        .anomaly
                        .volume_dirty = true;
                    return_errno!(Errno::EIO);
                }
            };
            if flush_status != BioStatus::Complete {
                mount_state
                    .as_mut()
                    .ok_or_else(unpublished_state)?
                    .anomaly
                    .volume_dirty = true;
                return_errno!(Errno::EIO);
            }

            let clean_anomaly = {
                let mount_state = mount_state.as_mut().ok_or_else(unpublished_state)?;
                if !mount_state.anomaly.volume_dirty {
                    return Ok(());
                }
                VolumeAnomalyState {
                    volume_dirty: false,
                    ..mount_state.anomaly
                }
            };
            if let Err(error) = self
                .boot_region
                .write_volume_anomaly_state(self.block_device.as_ref(), clean_anomaly)
            {
                mount_state
                    .as_mut()
                    .ok_or_else(unpublished_state)?
                    .anomaly
                    .volume_dirty = true;
                return Err(error);
            }

            let flush_status = match self.block_device.sync() {
                Ok(status) => status,
                Err(_) => {
                    mount_state
                        .as_mut()
                        .ok_or_else(unpublished_state)?
                        .anomaly
                        .volume_dirty = true;
                    return Err(Error::new(Errno::EIO));
                }
            };
            if flush_status != BioStatus::Complete {
                mount_state
                    .as_mut()
                    .ok_or_else(unpublished_state)?
                    .anomaly
                    .volume_dirty = true;
                return_errno!(Errno::EIO);
            }

            mount_state.as_mut().ok_or_else(unpublished_state)?.anomaly = clean_anomaly;
            return Ok(());
        }
    }

    fn root_inode(&self) -> Arc<dyn Inode> {
        let root_inode = self.root_inode.read();
        let root_inode = root_inode
            .as_ref()
            .unwrap_or_else(|| unreachable!("mounted exFAT instances must publish a root inode"));
        let root_inode: Arc<dyn Inode> = root_inode.clone();
        root_inode
    }

    fn sb(&self) -> SuperBlock {
        match self.super_block_snapshot() {
            Ok(super_block) => super_block,
            Err(_) => unreachable!("mounted exFAT instances must publish superblock state"),
        }
    }

    fn flags(&self) -> FsFlags {
        let mount_state = self.mount_state.read();
        let mount_state = mount_state.as_ref().unwrap_or_else(|| {
            unreachable!("mounted exFAT instances must publish filesystem flags")
        });
        mount_state.flags
    }

    fn set_fs_flags(&self, flags: FsFlags, data: Option<CString>, _ctx: &Context) -> Result<()> {
        let current_options = {
            let mount_state = self.mount_state.read();
            mount_state
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
            allocator: RwMutex::new(None),
            block_device,
            boot_region,
            fs_event_subscriber_stats: FsEventSubscriberStats::new(),
            inode_cache: RwLock::new(BTreeMap::new()),
            root_inode: RwMutex::new(None),
            source,
            mount_state: RwMutex::new(None),
            upcase_table: RwMutex::new(None),
        })
    }

    fn mount_candidate(
        block_device: &Arc<dyn BlockDevice>,
        source: Option<&str>,
        options: &ExfatMountOptions,
    ) -> Result<(Arc<ExfatFs>, Arc<dyn Inode>, SuperBlock, FsFlags)> {
        let (boot_region, anomaly, bitmap, upcase_table) =
            BootRegion::load_mount_state(block_device.as_ref())?;
        let fs = Self::new(
            block_device.clone(),
            boot_region,
            source.map(ToString::to_string),
        );
        let root_inode =
            ExfatInode::new_root(&fs, boot_region.root_dir_cluster, boot_region.cluster_size);
        let mount_state = MountedVolumeState {
            anomaly,
            flags: options.fs_flags,
            options: options.clone(),
            forced_shutdown: false,
        };
        fs.publish_mount_state(bitmap, root_inode.clone(), upcase_table, mount_state);
        let super_block = fs.super_block_snapshot()?;
        let root_inode: Arc<dyn Inode> = root_inode;
        Ok((fs, root_inode, super_block, options.fs_flags))
    }

    fn publish_mount_state(
        &self,
        bitmap: AllocationBitmap,
        root_inode: Arc<ExfatInode>,
        upcase_table: Arc<UpcaseTable>,
        mount_state: MountedVolumeState,
    ) {
        *self.allocator.write() = Some(bitmap);
        *self.root_inode.write() = Some(root_inode);
        *self.upcase_table.write() = Some(upcase_table);
        *self.mount_state.write() = Some(mount_state);
    }

    fn remount_published(
        &self,
        next_flags: FsFlags,
        next_options: &ExfatMountOptions,
    ) -> Result<FsFlags> {
        let mut mount_state = self.mount_state.write();
        let mount_state = mount_state.as_mut().ok_or_else(unpublished_state)?;
        let changed_flags = mount_state.flags ^ next_flags;
        if changed_flags.intersects(
            FsFlags::SYNCHRONOUS
                | FsFlags::MANDLOCK
                | FsFlags::DIRSYNC
                | FsFlags::SILENT
                | FsFlags::LAZYTIME,
        ) {
            return Err(unsupported_remount_delta());
        }
        if mount_state.flags.contains(FsFlags::RDONLY) && !next_flags.contains(FsFlags::RDONLY) {
            return Err(read_only_conflict());
        }
        if mount_state.options.iocharset != next_options.iocharset
            || mount_state.options.keep_last_dots != next_options.keep_last_dots
            || mount_state.options.zero_size_dir != next_options.zero_size_dir
        {
            return Err(unsupported_remount_delta());
        }
        mount_state.flags = next_flags;
        mount_state.options = next_options.with_flags(next_flags);
        Ok(next_flags)
    }

    // Admission

    pub(super) fn lookup_mount_snapshot(&self) -> Result<LookupMountSnapshot> {
        let mount_state = self.mount_state.read();
        let mount_state = mount_state.as_ref().ok_or_else(unpublished_state)?;
        let upcase_table = self.upcase_table.read();
        let upcase_table = upcase_table.as_ref().ok_or_else(unpublished_state)?;
        Ok(LookupMountSnapshot {
            block_device: self.block_device.clone(),
            boot_region: self.boot_region,
            anomaly: mount_state.anomaly,
            upcase_table: upcase_table.clone(),
            options: mount_state.options.clone(),
            forced_shutdown: mount_state.forced_shutdown,
        })
    }

    pub(super) fn lookup_mount_state(&self) -> Result<LookupMountState<'_>> {
        let mount_state = self.mount_state.read();
        let upcase_table = {
            let upcase_table = self.upcase_table.read();
            upcase_table.as_ref().ok_or_else(unpublished_state)?.clone()
        };
        let (anomaly, options, forced_shutdown) = {
            let mount_state = mount_state.as_ref().ok_or_else(unpublished_state)?;
            (
                mount_state.anomaly,
                mount_state.options.clone(),
                mount_state.forced_shutdown,
            )
        };
        Ok(LookupMountState {
            state_guard: mount_state,
            block_device: self.block_device.clone(),
            boot_region: self.boot_region,
            anomaly,
            upcase_table,
            options,
            forced_shutdown,
        })
    }

    pub(super) fn mutation_mount_state(&self) -> Result<MutationMountState<'_>> {
        let mut mount_state = self.mount_state.write();
        let upcase_table = {
            let upcase_table = self.upcase_table.read();
            upcase_table.as_ref().ok_or_else(unpublished_state)?.clone()
        };
        let (anomaly, options, forced_shutdown) = {
            let mount_state = mount_state.as_mut().ok_or_else(unpublished_state)?;
            (
                mount_state.anomaly,
                mount_state.options.clone(),
                mount_state.forced_shutdown,
            )
        };
        Ok(MutationMountState {
            state_guard: mount_state,
            block_device: self.block_device.clone(),
            boot_region: self.boot_region,
            anomaly,
            upcase_table,
            options,
            forced_shutdown,
        })
    }

    // A3 renames the primary mutation carrier to `mutation_mount_state()`, but
    // `inode/file_mutation.rs` still depends on this wrapper outside the packet
    // write-set. Remove it once that file moves to `mutation_mount_state()`.
    pub(super) fn admitted_mutation_state(&self) -> Result<MutationMountState<'_>> {
        self.mutation_mount_state()
    }

    // Superblock

    fn build_super_block(
        &self,
        mount_state: &MountedVolumeState,
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
            flags: u64::from(mount_state.flags.bits()),
            container_dev_id: self.block_device.id(),
        })
    }

    fn super_block_snapshot(&self) -> Result<SuperBlock> {
        let mount_state = self.mount_state.read();
        let mount_state = mount_state.as_ref().ok_or_else(unpublished_state)?;
        let allocator = self.allocator.read();
        let bitmap = allocator.as_ref().ok_or_else(unpublished_state)?;
        self.build_super_block(mount_state, bitmap)
    }

    pub(super) fn allocate_free_space_with_publication(
        &self,
        mount_state: &MountedVolumeState,
        requested_clusters: usize,
    ) -> Result<Vec<ClusterRange>> {
        if mount_state.flags.contains(FsFlags::RDONLY) {
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
        mount_state: &mut MountedVolumeState,
        ranges: &[ClusterRange],
    ) -> Result<()> {
        if mount_state.flags.contains(FsFlags::RDONLY) {
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
        if mount_state.options.discard {
            // Current Asterinas block devices expose no trim BIO, so downgrade
            // only the advisory policy.
            mount_state.options.discard = false;
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

pub(super) struct LookupMountSnapshot {
    pub(super) block_device: Arc<dyn BlockDevice>,
    pub(super) boot_region: BootRegion,
    pub(super) anomaly: VolumeAnomalyState,
    pub(super) upcase_table: Arc<UpcaseTable>,
    pub(super) options: ExfatMountOptions,
    pub(super) forced_shutdown: bool,
}

pub(super) struct LookupMountState<'a> {
    pub(super) state_guard: RwMutexReadGuard<'a, Option<MountedVolumeState>>,
    pub(super) block_device: Arc<dyn BlockDevice>,
    pub(super) boot_region: BootRegion,
    pub(super) anomaly: VolumeAnomalyState,
    pub(super) upcase_table: Arc<UpcaseTable>,
    pub(super) options: ExfatMountOptions,
    pub(super) forced_shutdown: bool,
}

pub(super) struct MutationMountState<'a> {
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
