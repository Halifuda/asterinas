// SPDX-License-Identifier: MPL-2.0

//! Implements the exFAT filesystem owner, mount admission, allocation, and VFS registration.

use alloc::{collections::BTreeMap, string::String, vec::Vec};
use core::sync::atomic::{AtomicU8, Ordering};

use aster_block::{BlockDevice, bio::BioStatus};
use ostd::{
    mm::VmIo,
    sync::{RwMutexReadGuard, RwMutexWriteGuard},
};

use super::{
    bitmap::{AllocationBitmap, BitmapOp, ClusterRange},
    boot::BootRegion,
    device_io,
    inconsistent_bitmap_accounting,
    inode::{ClusterMap, ExfatInode},
    invalid_operation_input, not_mounted,
    upcase::UpcaseTable,
};
use crate::{
    fs::vfs::{
        file_system::{FileSystem, FsEventSubscriberStats, FsFlags, SuperBlock},
        inode::Inode,
        registry::{FsCreationCtx, FsProperties, FsType},
    },
    prelude::*,
};

const EXFAT_SUPER_MAGIC: u64 = 0x2011_BAB0;

fn invalid_mount_input() -> Error {
    Error::new(Errno::EINVAL)
}

fn unsupported_remount_delta() -> Error {
    Error::with_message(Errno::EINVAL, "unsupported exFAT remount delta")
}

pub(super) struct ExfatFs {
    allocation_state: RwMutex<Option<AllocationBitmap>>,
    block_device: Arc<dyn BlockDevice>,
    boot_region: BootRegion,
    pub(super) fs_state: RwMutex<FsState>,
    fs_event_subscriber_stats: FsEventSubscriberStats,
    mount_runtime_projection: Arc<MountRuntimeProjection>,
    source: Option<String>,
}

#[derive(Default)]
pub(super) struct FsState {
    pub(super) inode_cache: BTreeMap<u64, Weak<ExfatInode>>,
    pub(super) mount_runtime: MountRuntimeState,
    pub(super) root_inode: Option<Arc<ExfatInode>>,
    pub(super) mount_state: Option<MountedVolumeState>,
    pub(super) upcase_table: Option<Arc<UpcaseTable>>,
}

impl FileSystem for ExfatFs {
    fn name(&self) -> &'static str {
        "exfat_refactor"
    }

    fn source(&self) -> Option<&str> {
        self.source.as_deref()
    }

    fn sync(&self) -> Result<()> {
        let mut fs_state = self.fs_state.write();
        if fs_state
            .mount_state
            .as_ref()
            .ok_or_else(not_mounted)?
            .forced_shutdown
        {
            return_errno!(Errno::EIO);
        }
        let live_inodes = Self::live_cached_inodes(&mut fs_state);
        for inode in &live_inodes {
            if let Err(error) = inode.sync_regular_file_with_fs_guard(
                self,
                &mut fs_state,
                super::inode::sync::InodeSyncScope::All,
            ) {
                Self::mark_mount_dirty_after_failure(&mut fs_state);
                return Err(error);
            }
        }

        let is_read_only = fs_state
            .mount_state
            .as_ref()
            .ok_or_else(not_mounted)?
            .options
            .fs_flags
            .contains(FsFlags::RDONLY);
        let mut allocation_guard = self.allocation_guard()?;
        match allocation_guard.release_lazy_reclaimed_clusters() {
            Ok(true) => Self::disable_unsupported_discard_after_release(&mut fs_state),
            Ok(false) => {}
            Err(error) => {
                Self::mark_mount_dirty_after_failure(&mut fs_state);
                return Err(error);
            }
        }
        if let Err(error) = allocation_guard.publish_dirty_ranges() {
            Self::mark_mount_dirty_after_failure(&mut fs_state);
            return Err(error);
        }
        if self.block_device.sync().map_err(|_| Error::new(Errno::EIO))? != BioStatus::Complete {
            Self::mark_mount_dirty_after_failure(&mut fs_state);
            return_errno!(Errno::EIO);
        }
        if let Err(error) = allocation_guard.commit_published_ranges() {
            Self::mark_mount_dirty_after_failure(&mut fs_state);
            return Err(error);
        }
        drop(allocation_guard);

        let mount_state = fs_state.mount_state.as_mut().ok_or_else(not_mounted)?;
        if is_read_only
            || !mount_state.volume_flags.volume_dirty
            || !mount_state.dirty_bracket_opened_by_mount
        {
            return Ok(());
        }
        let clean_flags = VolumeFlags {
            volume_dirty: false,
            ..mount_state.volume_flags
        };
        if let Err(error) = self
            .boot_region
            .write_volume_flags(self.block_device.as_ref(), clean_flags)
        {
            Self::mark_mount_dirty_after_failure(&mut fs_state);
            return Err(error);
        }
        if self.block_device.sync().map_err(|_| Error::new(Errno::EIO))? != BioStatus::Complete {
            Self::mark_mount_dirty_after_failure(&mut fs_state);
            return_errno!(Errno::EIO);
        }
        let mount_state = fs_state.mount_state.as_mut().ok_or_else(not_mounted)?;
        mount_state.volume_flags = clean_flags;
        mount_state.dirty_bracket_opened_by_mount = false;
        Ok(())
    }

    fn root_inode(&self) -> Arc<dyn Inode> {
        let fs_state = self.fs_state.read();
        let root_inode = fs_state
            .root_inode
            .as_ref()
            .unwrap_or_else(|| unreachable!("mounted exFAT instances must keep a root inode"));
        let root_inode: Arc<dyn Inode> = root_inode.clone();
        root_inode
    }

    fn sb(&self) -> SuperBlock {
        let fs_state = self.fs_state.read();
        let mount_state = fs_state
            .mount_state
            .as_ref()
            .unwrap_or_else(|| unreachable!("mounted exFAT instances must keep mount state"));
        let allocation_state = self.allocation_state.read();
        let bitmap = allocation_state
            .as_ref()
            .unwrap_or_else(|| unreachable!("mounted exFAT instances must keep allocator state"));
        match self.build_super_block(mount_state, bitmap) {
            Ok(super_block) => super_block,
            Err(_) => unreachable!("mounted exFAT instances must keep superblock state"),
        }
    }

    fn flags(&self) -> FsFlags {
        let fs_state = self.fs_state.read();
        let mount_state = fs_state
            .mount_state
            .as_ref()
            .unwrap_or_else(|| unreachable!("mounted exFAT instances must keep filesystem flags"));
        mount_state.flags
    }

    fn set_fs_flags(&self, flags: FsFlags, data: Option<CString>, _ctx: &Context) -> Result<()> {
        let mut fs_state = self.fs_state.write();
        let current_options = fs_state
            .mount_state
            .as_ref()
            .ok_or_else(not_mounted)?
            .options
            .clone();
        let next_options = match data.as_deref() {
            Some(args) => MountOptions::parse(flags, Some(args))?,
            None => current_options.with_flags(flags),
        };
        self.remount_active(&mut fs_state, flags, &next_options)?;
        Ok(())
    }

    fn fs_event_subscriber_stats(&self) -> &FsEventSubscriberStats {
        &self.fs_event_subscriber_stats
    }
}

impl ExfatFs {
    pub(super) fn disable_unsupported_discard_after_release(fs_state: &mut FsState) {
        if let Some(mount_state) = fs_state.mount_state.as_mut()
            && mount_state.options.discard
        {
            mount_state.options.discard = false;
        }
    }

    pub(super) fn mark_mount_dirty_after_failure(fs_state: &mut FsState) {
        if let Some(mount_state) = fs_state.mount_state.as_mut() {
            mount_state.volume_flags.volume_dirty = true;
            mount_state.dirty_bracket_opened_by_mount = false;
        }
    }

    // ---- Mount lifecycle ----

    fn new(
        block_device: Arc<dyn BlockDevice>,
        boot_region: BootRegion,
        source: Option<String>,
    ) -> Arc<Self> {
        Arc::new(Self {
            allocation_state: RwMutex::new(None),
            block_device,
            boot_region,
            fs_state: RwMutex::new(FsState::default()),
            fs_event_subscriber_stats: FsEventSubscriberStats::new(),
            mount_runtime_projection: Arc::new(MountRuntimeProjection::new(
                MountRuntimeState::default(),
            )),
            source,
        })
    }

    fn mount_candidate(
        block_device: &Arc<dyn BlockDevice>,
        source: Option<&str>,
        options: &MountOptions,
    ) -> Result<(Arc<ExfatFs>, Arc<dyn Inode>, SuperBlock, FsFlags)> {
        let (boot_region, flags, bitmap, upcase_table) =
            BootRegion::load_mount_state(block_device.as_ref())?;
        let mut bitmap = bitmap;
        bitmap.load_resident_bitmap(block_device.as_ref(), &boot_region)?;
        let fs = Self::new(
            block_device.clone(),
            boot_region,
            source.map(ToString::to_string),
        );
        let root_stream = super::inode::StreamExtensionDirEntry {
            data_length: None,
            first_cluster: boot_region.root_dir_cluster,
            valid_data_length: None,
            no_fat_chain: false,
        };
        let root_cluster_map = Arc::new(ExfatInode::resolve_cluster_map(
            block_device,
            &boot_region,
            root_stream,
        )?);
        let root_inode = ExfatInode::new_root(&fs, root_cluster_map, boot_region.cluster_size)?;
        let mount_state = MountedVolumeState {
            volume_flags: flags,
            dirty_bracket_opened_by_mount: false,
            flags: options.fs_flags,
            options: options.clone(),
            forced_shutdown: false,
        };
        fs.activate_mount_state(bitmap, root_inode.clone(), upcase_table, mount_state);
        let super_block = {
            let fs_state = fs.fs_state.read();
            let mount_state = fs_state.mount_state.as_ref().ok_or_else(not_mounted)?;
            let allocation_state = fs.allocation_state.read();
            let bitmap = allocation_state.as_ref().ok_or_else(not_mounted)?;
            fs.build_super_block(mount_state, bitmap)?
        };
        let root_inode: Arc<dyn Inode> = root_inode;
        Ok((fs, root_inode, super_block, options.fs_flags))
    }

    fn activate_mount_state(
        &self,
        bitmap: AllocationBitmap,
        root_inode: Arc<ExfatInode>,
        upcase_table: Arc<UpcaseTable>,
        mount_state: MountedVolumeState,
    ) {
        let mount_runtime = MountRuntimeState {
            forced_shutdown: mount_state.forced_shutdown,
            clear_to_zero: mount_state.volume_flags.clear_to_zero,
            media_failure: mount_state.volume_flags.media_failure,
            read_only: mount_state.options.fs_flags.contains(FsFlags::RDONLY),
        };
        let mut fs_state = self.fs_state.write();
        *self.allocation_state.write() = Some(bitmap);
        fs_state.root_inode = Some(root_inode);
        fs_state.upcase_table = Some(upcase_table);
        fs_state.mount_state = Some(mount_state);
        fs_state.mount_runtime = mount_runtime;
        self.mount_runtime_projection.publish(mount_runtime);
    }

    fn remount_active(
        &self,
        fs_state: &mut FsState,
        next_flags: FsFlags,
        next_options: &MountOptions,
    ) -> Result<FsFlags> {
        let mount_state = fs_state.mount_state.as_mut().ok_or_else(not_mounted)?;
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
            return Err(Error::new(Errno::EROFS));
        }
        if mount_state.options.iocharset != next_options.iocharset
            || mount_state.options.keep_last_dots != next_options.keep_last_dots
            || mount_state.options.zero_size_dir != next_options.zero_size_dir
        {
            return Err(unsupported_remount_delta());
        }
        if !mount_state.flags.contains(FsFlags::RDONLY) && next_flags.contains(FsFlags::RDONLY) {
            mount_state.dirty_bracket_opened_by_mount = false;
        }
        mount_state.flags = next_flags;
        mount_state.options = next_options.with_flags(next_flags);
        let mount_runtime = MountRuntimeState {
            forced_shutdown: mount_state.forced_shutdown,
            clear_to_zero: mount_state.volume_flags.clear_to_zero,
            media_failure: mount_state.volume_flags.media_failure,
            read_only: mount_state.options.fs_flags.contains(FsFlags::RDONLY),
        };
        fs_state.mount_runtime = mount_runtime;
        self.mount_runtime_projection.publish(mount_runtime);
        Ok(next_flags)
    }

    pub(in crate::fs::fs_impls::exfat_refactor) fn mount_runtime_projection(
        &self,
    ) -> Arc<MountRuntimeProjection> {
        self.mount_runtime_projection.clone()
    }

}

// ---- Superblock ----
impl ExfatFs {
    pub(super) fn publish_dirty_admission(
        &self,
        fs_state: &mut FsState,
    ) -> Result<()> {
        let current_flags = fs_state
            .mount_state
            .as_ref()
            .ok_or_else(not_mounted)?
            .volume_flags;
        if current_flags.volume_dirty {
            return Ok(());
        }

        let dirty_flags = VolumeFlags {
            volume_dirty: true,
            ..current_flags
        };
        if let Err(error) = self
            .boot_region
            .write_volume_flags(self.block_device.as_ref(), dirty_flags)
        {
            Self::mark_mount_dirty_after_failure(fs_state);
            return Err(error);
        }

        let flush_status = match self.block_device.sync() {
            Ok(status) => status,
            Err(_) => {
                Self::mark_mount_dirty_after_failure(fs_state);
                return_errno!(Errno::EIO);
            }
        };
        if flush_status != BioStatus::Complete {
            Self::mark_mount_dirty_after_failure(fs_state);
            return_errno!(Errno::EIO);
        }

        let mount_state = fs_state.mount_state.as_mut().ok_or_else(not_mounted)?;
        mount_state.volume_flags = dirty_flags;
        mount_state.dirty_bracket_opened_by_mount = true;
        Ok(())
    }

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

}

// ---- Allocation ----

pub(super) struct AllocGuard<'a> {
    allocation_state: RwMutexWriteGuard<'a, Option<AllocationBitmap>>,
    allocated_ranges: Option<Vec<ClusterRange>>,
    block_device: &'a dyn BlockDevice,
    boot_region: &'a BootRegion,
}

pub(super) type AllocReadGuard<'a> = RwMutexReadGuard<'a, Option<AllocationBitmap>>;

impl AllocGuard<'_> {
    pub(super) fn allocate(
        &mut self,
        requested_clusters: usize,
        preferred_start_cluster: Option<u32>,
    ) -> Result<()> {
        if requested_clusters == 0 || self.allocated_ranges.is_some() {
            return Err(invalid_operation_input());
        }
        let allocation_bitmap = self
            .allocation_state
            .as_mut()
            .ok_or_else(not_mounted)?;
        allocation_bitmap.release_lazy_reclaimed_clusters(self.boot_region)?;
        let allocated_ranges = allocation_bitmap.find_free_ranges(
            self.boot_region,
            requested_clusters,
            preferred_start_cluster,
        )?;
        let allocated_cluster_count = allocated_ranges.iter().try_fold(
            0usize,
            |total_clusters, range| {
                total_clusters
                    .checked_add(range.cluster_count)
                    .ok_or_else(inconsistent_bitmap_accounting)
            },
        )?;
        if allocated_cluster_count != requested_clusters {
            return Err(inconsistent_bitmap_accounting());
        }
        allocation_bitmap.apply_cluster_ranges(
            self.boot_region,
            &allocated_ranges,
            BitmapOp::Allocate,
        )?;
        self.allocated_ranges = Some(allocated_ranges);
        Ok(())
    }

    pub(super) fn ranges(&self) -> &[ClusterRange] {
        self.allocated_ranges
            .as_deref()
            .unwrap_or_else(|| unreachable!("committed allocation guards hold no ranges"))
    }

    pub(super) fn single_cluster(&self) -> Result<u32> {
        match self.ranges() {
            [allocated_range] if allocated_range.cluster_count == 1 => {
                Ok(allocated_range.start_cluster)
            }
            _ => Err(inconsistent_bitmap_accounting()),
        }
    }

    pub(super) fn commit_allocation(&mut self) {
        self.allocated_ranges = None;
    }

    pub(super) fn rollback_allocation(&mut self) -> Result<bool> {
        let Some(allocated_ranges) = self.allocated_ranges.as_ref().cloned() else {
            return Ok(false);
        };
        self.free_clusters(&allocated_ranges)?;
        self.allocated_ranges = None;
        Ok(true)
    }

    pub(super) fn free_clusters(&mut self, ranges: &[ClusterRange]) -> Result<()> {
        let allocation_bitmap = self
            .allocation_state
            .as_mut()
            .ok_or_else(not_mounted)?;
        allocation_bitmap.apply_cluster_ranges(
            self.boot_region,
            ranges,
            BitmapOp::Free,
        )?;
        Ok(())
    }

    pub(super) fn lazy_reclaim_clusters(
        &mut self,
        cluster_map: Arc<ClusterMap>,
        ranges: Vec<ClusterRange>,
    ) -> Result<()> {
        let allocation_bitmap = self
            .allocation_state
            .as_mut()
            .ok_or_else(not_mounted)?;
        allocation_bitmap.lazy_reclaim_clusters(cluster_map, ranges);
        Ok(())
    }

    pub(super) fn release_lazy_reclaimed_clusters(&mut self) -> Result<bool> {
        self.allocation_state
            .as_mut()
            .ok_or_else(not_mounted)?
            .release_lazy_reclaimed_clusters(self.boot_region)
    }

    pub(super) fn publish_dirty_ranges(&mut self) -> Result<()> {
        self.allocation_state
            .as_mut()
            .ok_or_else(not_mounted)?
            .publish_dirty_ranges(self.block_device, self.boot_region)
    }

    pub(super) fn commit_published_ranges(&mut self) -> Result<()> {
        self.allocation_state
            .as_mut()
            .ok_or_else(not_mounted)?
            .commit_published_ranges()
    }
}

impl ExfatFs {
    pub(super) fn allocation_read_guard(&self) -> Result<AllocReadGuard<'_>> {
        let allocation_state = self.allocation_state.read();
        if allocation_state.is_none() {
            return Err(not_mounted());
        }
        Ok(allocation_state)
    }

    pub(super) fn allocation_guard(&self) -> Result<AllocGuard<'_>> {
        Ok(AllocGuard {
            allocation_state: self.allocation_state.write(),
            allocated_ranges: None,
            block_device: self.block_device.as_ref(),
            boot_region: &self.boot_region,
        })
    }

    pub(super) fn immutable_block_device(&self) -> Arc<dyn BlockDevice> {
        self.block_device.clone()
    }

    pub(super) fn immutable_boot_region(&self) -> BootRegion {
        self.boot_region
    }

    pub(super) fn container_device_id(&self) -> device_id::DeviceId {
        self.block_device.id()
    }
}

impl Drop for ExfatFs {
    fn drop(&mut self) {
        let mut fs_state = self.fs_state.write();
        for inode in Self::live_cached_inodes(&mut fs_state) {
            inode
                .inode_state_write_guard()
                .set_dirty_file_retention(None);
        }
    }
}

// ---- Inode cache ----
impl ExfatFs {
    pub(super) fn peek_cached_inode(fs_state: &FsState, ino: u64) -> Option<Arc<ExfatInode>> {
        fs_state
            .inode_cache
            .get(&ino)
            .and_then(Weak::upgrade)
    }

    pub(super) fn remove_cached_inode(fs_state: &mut FsState, ino: u64) {
        fs_state.inode_cache.remove(&ino);
    }

    pub(super) fn rebind_rename_inode_cache(
        fs_state: &mut FsState,
        old_source_ino: u64,
        new_source_ino: u64,
        source_inode: &Arc<ExfatInode>,
        replaced_target_ino: Option<u64>,
    ) {
        fs_state.inode_cache.remove(&old_source_ino);
        if let Some(replaced_target_ino) = replaced_target_ino {
            fs_state.inode_cache.remove(&replaced_target_ino);
        }
        fs_state
            .inode_cache
            .insert(new_source_ino, Arc::downgrade(source_inode));
    }

    fn live_cached_inodes(fs_state: &mut FsState) -> Vec<Arc<ExfatInode>> {
        let mut live_inodes = Vec::with_capacity(fs_state.inode_cache.len());
        fs_state.inode_cache.retain(|_, inode| match inode.upgrade() {
            Some(inode) => {
                live_inodes.push(inode);
                true
            }
            None => false,
        });
        live_inodes
    }
}

#[derive(Clone)]
pub(super) struct MountedVolumeState {
    pub(super) volume_flags: VolumeFlags,
    pub(super) dirty_bracket_opened_by_mount: bool,
    pub(super) flags: FsFlags,
    pub(super) options: MountOptions,
    pub(super) forced_shutdown: bool,
}

#[derive(Clone, Copy, Default)]
pub(in crate::fs::fs_impls::exfat_refactor) struct MountRuntimeState {
    pub(in crate::fs::fs_impls::exfat_refactor) forced_shutdown: bool,
    pub(in crate::fs::fs_impls::exfat_refactor) clear_to_zero: bool,
    pub(in crate::fs::fs_impls::exfat_refactor) media_failure: bool,
    pub(in crate::fs::fs_impls::exfat_refactor) read_only: bool,
}

pub(in crate::fs::fs_impls::exfat_refactor) struct MountRuntimeProjection {
    bits: AtomicU8,
}

impl MountRuntimeProjection {
    const CLEAR_TO_ZERO: u8 = 1 << 1;
    const FORCED_SHUTDOWN: u8 = 1 << 0;
    const MEDIA_FAILURE: u8 = 1 << 2;
    const READ_ONLY: u8 = 1 << 3;

    fn new(mount_runtime: MountRuntimeState) -> Self {
        Self {
            bits: AtomicU8::new(Self::encode(mount_runtime)),
        }
    }

    fn publish(&self, mount_runtime: MountRuntimeState) {
        self.bits
            .store(Self::encode(mount_runtime), Ordering::Release);
    }

    pub(in crate::fs::fs_impls::exfat_refactor) fn snapshot(&self) -> MountRuntimeState {
        Self::decode(self.bits.load(Ordering::Acquire))
    }

    fn encode(mount_runtime: MountRuntimeState) -> u8 {
        let mut bits = 0;
        if mount_runtime.forced_shutdown {
            bits |= Self::FORCED_SHUTDOWN;
        }
        if mount_runtime.clear_to_zero {
            bits |= Self::CLEAR_TO_ZERO;
        }
        if mount_runtime.media_failure {
            bits |= Self::MEDIA_FAILURE;
        }
        if mount_runtime.read_only {
            bits |= Self::READ_ONLY;
        }
        bits
    }

    fn decode(bits: u8) -> MountRuntimeState {
        MountRuntimeState {
            forced_shutdown: bits & Self::FORCED_SHUTDOWN != 0,
            clear_to_zero: bits & Self::CLEAR_TO_ZERO != 0,
            media_failure: bits & Self::MEDIA_FAILURE != 0,
            read_only: bits & Self::READ_ONLY != 0,
        }
    }
}

#[derive(Clone, Copy)]
pub(super) struct VolumeFlags {
    pub(super) clear_to_zero: bool,
    pub(super) media_failure: bool,
    pub(super) volume_dirty: bool,
}

impl VolumeFlags {
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
pub(super) struct MountOptions {
    discard: bool,
    pub(super) fs_flags: FsFlags,
    pub(super) iocharset: String,
    pub(super) keep_last_dots: bool,
    pub(super) zero_size_dir: bool,
}

impl MountOptions {
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

struct ExfatFsType;

impl FsType for ExfatFsType {
    fn name(&self) -> &'static str {
        "exfat_refactor"
    }

    fn properties(&self) -> FsProperties {
        FsProperties::NEED_DISK
    }

    fn create(&self, fs_creation_ctx: &FsCreationCtx) -> Result<Arc<dyn FileSystem>> {
        let block_device = fs_creation_ctx.resolve_block_device()?;
        let options = MountOptions::parse(fs_creation_ctx.flags(), fs_creation_ctx.args())?;
        let (fs, ..) = ExfatFs::mount_candidate(&block_device, fs_creation_ctx.source(), &options)?;
        Ok(fs as Arc<dyn FileSystem>)
    }

    fn sysnode(&self) -> Option<Arc<dyn aster_systree::SysNode>> {
        None
    }
}
