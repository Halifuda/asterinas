// SPDX-License-Identifier: MPL-2.0

//! Implements the exFAT filesystem owner, mount admission, allocation, and VFS registration.

use alloc::{collections::BTreeMap, string::String, vec::Vec};
use core::sync::atomic::{AtomicU8, Ordering};

use aster_block::{BlockDevice, bio::BioStatus};
use ostd::{
    mm::VmIo,
    sync::{RwMutex, RwMutexReadGuard, RwMutexWriteGuard},
};

use super::{
    bitmap::{AllocationBitmap, BitmapOp, ClusterRange},
    boot::BootRegion,
    device_io,
    fat::FatReader,
    inconsistent_bitmap_accounting,
    inode::{ClusterMap, ExfatInode},
    invalid_mount_input, invalid_operation_input, not_mounted, read_only_conflict,
    unsupported_remount_delta,
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

pub(super) struct ExfatFs {
    allocator: RwMutex<Option<AllocationBitmap>>,
    block_device: Arc<dyn BlockDevice>,
    boot_region: BootRegion,
    fs_event_subscriber_stats: FsEventSubscriberStats,
    inode_cache: RwMutex<BTreeMap<u64, Weak<ExfatInode>>>,
    mount_runtime_projection: Arc<MountRuntimeProjection>,
    pub(in crate::fs::fs_impls::exfat_refactor) mount_runtime: RwMutex<MountRuntimeState>,
    pub(super) root_inode: RwMutex<Option<Arc<ExfatInode>>>,
    source: Option<String>,
    pub(super) mount_state: RwMutex<Option<MountedVolumeState>>,
    upcase_table: RwMutex<Option<Arc<UpcaseTable>>>,
}

impl FileSystem for ExfatFs {
    fn name(&self) -> &'static str {
        "exfat_refactor"
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
                        mount_state.volume_flags.volume_dirty = true;
                        mount_state.dirty_bracket_opened_by_mount = false;
                    }
                    return Err(error);
                }
            }

            let mut mount_state = self.mount_state.write();
            {
                let mount_state = mount_state.as_mut().ok_or_else(not_mounted)?;
                if mount_state.forced_shutdown {
                    return_errno!(Errno::EIO);
                }
            }

            let has_pending_regular_file_sync = self
                .live_cached_inodes()
                .into_iter()
                .any(|inode| inode.has_pending_regular_file_sync());
            if has_pending_regular_file_sync {
                drop(mount_state);
                continue;
            }

            if let Err(error) = self.release_lazy_reclaimed_clusters() {
                mount_state
                    .as_mut()
                    .ok_or_else(not_mounted)?
                    .volume_flags
                    .volume_dirty = true;
                mount_state
                    .as_mut()
                    .ok_or_else(not_mounted)?
                    .dirty_bracket_opened_by_mount = false;
                return Err(error);
            }

            if let Err(error) = self.flush_dirty_allocation_bitmap() {
                mount_state
                    .as_mut()
                    .ok_or_else(not_mounted)?
                    .volume_flags
                    .volume_dirty = true;
                mount_state
                    .as_mut()
                    .ok_or_else(not_mounted)?
                    .dirty_bracket_opened_by_mount = false;
                return Err(error);
            }

            let flush_status = match self.block_device.sync() {
                Ok(status) => status,
                Err(_) => {
                    mount_state
                        .as_mut()
                        .ok_or_else(not_mounted)?
                        .volume_flags
                        .volume_dirty = true;
                    mount_state
                        .as_mut()
                        .ok_or_else(not_mounted)?
                        .dirty_bracket_opened_by_mount = false;
                    return_errno!(Errno::EIO);
                }
            };
            if flush_status != BioStatus::Complete {
                mount_state
                    .as_mut()
                    .ok_or_else(not_mounted)?
                    .volume_flags
                    .volume_dirty = true;
                mount_state
                    .as_mut()
                    .ok_or_else(not_mounted)?
                    .dirty_bracket_opened_by_mount = false;
                return_errno!(Errno::EIO);
            }
            if let Err(error) = self.commit_published_allocation_bitmap() {
                return Err(error);
            }

            let clean_flags = {
                let mount_state = mount_state.as_mut().ok_or_else(not_mounted)?;
                if mount_state.flags.contains(FsFlags::RDONLY)
                    || !mount_state.volume_flags.volume_dirty
                    || !mount_state.dirty_bracket_opened_by_mount
                {
                    return Ok(());
                }
                VolumeFlags {
                    volume_dirty: false,
                    ..mount_state.volume_flags
                }
            };
            if let Err(error) = self
                .boot_region
                .write_volume_flags(self.block_device.as_ref(), clean_flags)
            {
                mount_state
                    .as_mut()
                    .ok_or_else(not_mounted)?
                    .volume_flags
                    .volume_dirty = true;
                mount_state
                    .as_mut()
                    .ok_or_else(not_mounted)?
                    .dirty_bracket_opened_by_mount = false;
                return Err(error);
            }
            let flush_status = match self.block_device.sync() {
                Ok(status) => status,
                Err(_) => {
                    mount_state
                        .as_mut()
                        .ok_or_else(not_mounted)?
                        .volume_flags
                        .volume_dirty = true;
                    mount_state
                        .as_mut()
                        .ok_or_else(not_mounted)?
                        .dirty_bracket_opened_by_mount = false;
                    return Err(Error::new(Errno::EIO));
                }
            };
            if flush_status != BioStatus::Complete {
                mount_state
                    .as_mut()
                    .ok_or_else(not_mounted)?
                    .volume_flags
                    .volume_dirty = true;
                mount_state
                    .as_mut()
                    .ok_or_else(not_mounted)?
                    .dirty_bracket_opened_by_mount = false;
                return_errno!(Errno::EIO);
            }

            let mount_state = mount_state.as_mut().ok_or_else(not_mounted)?;
            mount_state.volume_flags = clean_flags;
            mount_state.dirty_bracket_opened_by_mount = false;
            return Ok(());
        }
    }

    fn root_inode(&self) -> Arc<dyn Inode> {
        let root_inode = self.root_inode.read();
        let root_inode = root_inode
            .as_ref()
            .unwrap_or_else(|| unreachable!("mounted exFAT instances must keep a root inode"));
        let root_inode: Arc<dyn Inode> = root_inode.clone();
        root_inode
    }

    fn sb(&self) -> SuperBlock {
        match self.super_block_snapshot() {
            Ok(super_block) => super_block,
            Err(_) => unreachable!("mounted exFAT instances must keep superblock state"),
        }
    }

    fn flags(&self) -> FsFlags {
        let mount_state = self.mount_state.read();
        let mount_state = mount_state
            .as_ref()
            .unwrap_or_else(|| unreachable!("mounted exFAT instances must keep filesystem flags"));
        mount_state.flags
    }

    fn set_fs_flags(&self, flags: FsFlags, data: Option<CString>, _ctx: &Context) -> Result<()> {
        let current_options = {
            let mount_state = self.mount_state.read();
            mount_state
                .as_ref()
                .ok_or_else(not_mounted)?
                .options
                .clone()
        };
        let next_options = match data.as_deref() {
            Some(args) => MountOptions::parse(flags, Some(args))?,
            None => current_options.with_flags(flags),
        };
        self.remount_active(flags, &next_options)?;
        Ok(())
    }

    fn fs_event_subscriber_stats(&self) -> &FsEventSubscriberStats {
        &self.fs_event_subscriber_stats
    }
}

impl ExfatFs {
    // ---- Mount lifecycle ----

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
            inode_cache: RwMutex::new(BTreeMap::new()),
            mount_runtime_projection: Arc::new(MountRuntimeProjection::new(
                MountRuntimeState::default(),
            )),
            mount_runtime: RwMutex::new(MountRuntimeState::default()),
            root_inode: RwMutex::new(None),
            source,
            mount_state: RwMutex::new(None),
            upcase_table: RwMutex::new(None),
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
        let root_inode =
            ExfatInode::new_root(&fs, boot_region.root_dir_cluster, boot_region.cluster_size);
        let mount_state = MountedVolumeState {
            volume_flags: flags,
            dirty_bracket_opened_by_mount: false,
            flags: options.fs_flags,
            options: options.clone(),
            forced_shutdown: false,
        };
        fs.activate_mount_state(bitmap, root_inode.clone(), upcase_table, mount_state);
        let super_block = fs.super_block_snapshot()?;
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
        *self.allocator.write() = Some(bitmap);
        *self.root_inode.write() = Some(root_inode);
        *self.upcase_table.write() = Some(upcase_table);
        *self.mount_state.write() = Some(mount_state);
        self.publish_mount_runtime(mount_runtime);
    }

    fn remount_active(&self, next_flags: FsFlags, next_options: &MountOptions) -> Result<FsFlags> {
        let mut mount_state = self.mount_state.write();
        let mount_state = mount_state.as_mut().ok_or_else(not_mounted)?;
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
        if !mount_state.flags.contains(FsFlags::RDONLY) && next_flags.contains(FsFlags::RDONLY) {
            mount_state.dirty_bracket_opened_by_mount = false;
        }
        mount_state.flags = next_flags;
        mount_state.options = next_options.with_flags(next_flags);
        self.publish_mount_runtime(MountRuntimeState {
            forced_shutdown: mount_state.forced_shutdown,
            clear_to_zero: mount_state.volume_flags.clear_to_zero,
            media_failure: mount_state.volume_flags.media_failure,
            read_only: mount_state.options.fs_flags.contains(FsFlags::RDONLY),
        });
        Ok(next_flags)
    }

    pub(in crate::fs::fs_impls::exfat_refactor) fn mount_runtime_projection(
        &self,
    ) -> Arc<MountRuntimeProjection> {
        self.mount_runtime_projection.clone()
    }

    pub(in crate::fs::fs_impls::exfat_refactor) fn publish_mount_runtime(
        &self,
        mount_runtime: MountRuntimeState,
    ) {
        *self.mount_runtime.write() = mount_runtime;
        self.mount_runtime_projection.publish(mount_runtime);
    }

    pub(in crate::fs::fs_impls::exfat_refactor) fn publish_forced_shutdown_runtime(&self) {
        let mut mount_runtime = self.mount_runtime.write();
        mount_runtime.forced_shutdown = true;
        self.mount_runtime_projection.publish(*mount_runtime);
    }
}

// ---- Mount guards ----
impl ExfatFs {
    pub(super) fn mount_state_read_guard(&self) -> Result<MountStateReadGuard<'_>> {
        let mount_state = self.mount_state.read();
        let upcase_table = {
            let upcase_table = self.upcase_table.read();
            upcase_table.as_ref().ok_or_else(not_mounted)?.clone()
        };
        let (flags, options, forced_shutdown) = {
            let mount_state = mount_state.as_ref().ok_or_else(not_mounted)?;
            (
                mount_state.volume_flags,
                mount_state.options.clone(),
                mount_state.forced_shutdown,
            )
        };
        Ok(MountStateReadGuard {
            state_guard: mount_state,
            flags,
            upcase_table,
            options,
            forced_shutdown,
        })
    }

    pub(super) fn mount_state_write_guard(&self) -> Result<MountStateWriteGuard<'_>> {
        let mut mount_state = self.mount_state.write();
        let upcase_table = {
            let upcase_table = self.upcase_table.read();
            upcase_table.as_ref().ok_or_else(not_mounted)?.clone()
        };
        let (flags, options, forced_shutdown) = {
            let mount_state = mount_state.as_mut().ok_or_else(not_mounted)?;
            (
                mount_state.volume_flags,
                mount_state.options.clone(),
                mount_state.forced_shutdown,
            )
        };
        Ok(MountStateWriteGuard {
            state_guard: mount_state,
            flags,
            upcase_table,
            options,
            forced_shutdown,
        })
    }
}

// ---- Superblock ----
impl ExfatFs {
    pub(super) fn publish_dirty_admission(
        &self,
        mount_state: &mut MountedVolumeState,
    ) -> Result<()> {
        if mount_state.volume_flags.volume_dirty {
            return Ok(());
        }

        let dirty_flags = VolumeFlags {
            volume_dirty: true,
            ..mount_state.volume_flags
        };
        if let Err(error) = self
            .boot_region
            .write_volume_flags(self.block_device.as_ref(), dirty_flags)
        {
            mount_state.volume_flags.volume_dirty = true;
            mount_state.dirty_bracket_opened_by_mount = false;
            return Err(error);
        }

        let flush_status = match self.block_device.sync() {
            Ok(status) => status,
            Err(_) => {
                mount_state.volume_flags.volume_dirty = true;
                mount_state.dirty_bracket_opened_by_mount = false;
                return_errno!(Errno::EIO);
            }
        };
        if flush_status != BioStatus::Complete {
            mount_state.volume_flags.volume_dirty = true;
            mount_state.dirty_bracket_opened_by_mount = false;
            return_errno!(Errno::EIO);
        }

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

    fn super_block_snapshot(&self) -> Result<SuperBlock> {
        let mount_state = self.mount_state.read();
        let mount_state = mount_state.as_ref().ok_or_else(not_mounted)?;
        let allocator = self.allocator.read();
        let bitmap = allocator.as_ref().ok_or_else(not_mounted)?;
        self.build_super_block(mount_state, bitmap)
    }
}

// ---- Allocation ----

pub(super) struct ClusterAllocGuard<'a> {
    fs: &'a Arc<ExfatFs>,
    mount_state: &'a mut MountedVolumeState,
    allocated_ranges: Option<Vec<ClusterRange>>,
}

impl<'a> ClusterAllocGuard<'a> {
    pub(super) fn allocate(
        fs: &'a Arc<ExfatFs>,
        mount_state: &'a mut MountedVolumeState,
        requested_clusters: usize,
        preferred_start_cluster: Option<u32>,
    ) -> Result<Self> {
        let allocated_ranges =
            fs.allocate_clusters(mount_state, requested_clusters, preferred_start_cluster)?;
        Ok(Self {
            fs,
            mount_state,
            allocated_ranges: Some(allocated_ranges),
        })
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

    pub(super) fn commit(mut self) {
        self.allocated_ranges = None;
    }
}

impl Drop for ClusterAllocGuard<'_> {
    fn drop(&mut self) {
        if let Some(allocated_ranges) = self.allocated_ranges.take() {
            let _ = self.fs.free_clusters(self.mount_state, &allocated_ranges);
        }
    }
}

impl ExfatFs {
    pub(super) fn flush_dirty_allocation_bitmap(&self) -> Result<()> {
        let mut allocator = self.allocator.write();
        let allocation_bitmap = allocator.as_mut().ok_or_else(not_mounted)?;
        allocation_bitmap.publish_dirty_ranges(self.block_device.as_ref(), &self.boot_region)
    }

    pub(super) fn commit_published_allocation_bitmap(&self) -> Result<()> {
        let mut allocator = self.allocator.write();
        let allocation_bitmap = allocator.as_mut().ok_or_else(not_mounted)?;
        allocation_bitmap.commit_published_ranges()
    }

    pub(super) fn allocate_clusters(
        &self,
        mount_state: &MountedVolumeState,
        requested_clusters: usize,
        preferred_start_cluster: Option<u32>,
    ) -> Result<Vec<ClusterRange>> {
        if mount_state.flags.contains(FsFlags::RDONLY) {
            return Err(read_only_conflict());
        }

        if requested_clusters == 0 {
            return Err(invalid_operation_input());
        }
        self.release_lazy_reclaimed_clusters()?;

        let mut allocator = self.allocator.write();
        let allocation_bitmap = allocator.as_mut().ok_or_else(not_mounted)?;

        let mut fat_reader = FatReader::new(self.block_device.as_ref(), &self.boot_region);
        let allocated_ranges = allocation_bitmap.find_free_ranges(
            self.block_device.as_ref(),
            &self.boot_region,
            &mut fat_reader,
            requested_clusters,
            preferred_start_cluster,
        )?;
        let allocated_clusters = allocation_bitmap.apply_cluster_ranges(
            self.block_device.as_ref(),
            &self.boot_region,
            &allocated_ranges,
            BitmapOp::Allocate,
        )?;
        if allocated_clusters != requested_clusters {
            return Err(inconsistent_bitmap_accounting());
        }
        allocation_bitmap.record_allocated_clusters(allocated_clusters)?;
        Ok(allocated_ranges)
    }

    pub(super) fn free_clusters(
        &self,
        mount_state: &mut MountedVolumeState,
        ranges: &[ClusterRange],
    ) -> Result<()> {
        if mount_state.flags.contains(FsFlags::RDONLY) {
            return Err(read_only_conflict());
        }

        let mut allocator = self.allocator.write();
        let allocation_bitmap = allocator.as_mut().ok_or_else(not_mounted)?;
        let released_clusters = allocation_bitmap.apply_cluster_ranges(
            self.block_device.as_ref(),
            &self.boot_region,
            ranges,
            BitmapOp::Free,
        )?;
        allocation_bitmap.record_released_clusters(released_clusters)?;
        if mount_state.options.discard {
            // Current Asterinas block devices expose no trim BIO, so downgrade
            // only the advisory policy.
            mount_state.options.discard = false;
        }
        Ok(())
    }

    pub(super) fn lazy_reclaim_clusters(
        &self,
        cluster_map: Arc<ClusterMap>,
        ranges: Vec<ClusterRange>,
    ) -> Result<()> {
        let mut allocator = self.allocator.write();
        let allocation_bitmap = allocator.as_mut().ok_or_else(not_mounted)?;
        allocation_bitmap.lazy_reclaim_clusters(cluster_map, ranges);
        Ok(())
    }

    pub(super) fn release_lazy_reclaimed_clusters(&self) -> Result<()> {
        let mut allocator = self.allocator.write();
        let allocation_bitmap = allocator.as_mut().ok_or_else(not_mounted)?;
        allocation_bitmap
            .release_lazy_reclaimed_clusters(self.block_device.as_ref(), &self.boot_region)
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
        for inode in self.live_cached_inodes() {
            inode.clear_dirty_file_retention();
        }
    }
}

// ---- Inode cache ----
impl ExfatFs {
    pub(super) fn peek_cached_inode(&self, ino: u64) -> Option<Arc<ExfatInode>> {
        self.inode_cache.read().get(&ino).and_then(Weak::upgrade)
    }

    pub(super) fn get_or_create_cached_inode(
        &self,
        ino: u64,
        create_inode_fn: impl FnOnce() -> Arc<ExfatInode>,
    ) -> Arc<ExfatInode> {
        if let Some(cached_inode) = self.inode_cache.read().get(&ino).and_then(Weak::upgrade) {
            if !cached_inode.is_detached_regular_file() {
                return cached_inode;
            }
        }

        let mut inode_cache = self.inode_cache.write();
        if let Some(cached_inode) = inode_cache.get(&ino).and_then(Weak::upgrade) {
            if !cached_inode.is_detached_regular_file() {
                return cached_inode;
            }
            inode_cache.remove(&ino);
        }

        let inode = create_inode_fn();
        inode_cache.insert(ino, Arc::downgrade(&inode));
        inode
    }

    pub(super) fn remove_cached_inode(&self, ino: u64) {
        self.inode_cache.write().remove(&ino);
    }

    pub(super) fn rebind_rename_inode_cache(
        &self,
        old_source_ino: u64,
        new_source_ino: u64,
        source_inode: &Arc<ExfatInode>,
        replaced_target_ino: Option<u64>,
    ) {
        let mut inode_cache = self.inode_cache.write();
        inode_cache.remove(&old_source_ino);
        if let Some(replaced_target_ino) = replaced_target_ino {
            inode_cache.remove(&replaced_target_ino);
        }
        inode_cache.insert(new_source_ino, Arc::downgrade(source_inode));
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

pub(super) struct MountStateReadGuard<'a> {
    pub(super) state_guard: RwMutexReadGuard<'a, Option<MountedVolumeState>>,
    pub(super) flags: VolumeFlags,
    pub(super) upcase_table: Arc<UpcaseTable>,
    pub(super) options: MountOptions,
    pub(super) forced_shutdown: bool,
}

pub(super) struct MountStateWriteGuard<'a> {
    pub(super) state_guard: RwMutexWriteGuard<'a, Option<MountedVolumeState>>,
    pub(super) flags: VolumeFlags,
    pub(super) upcase_table: Arc<UpcaseTable>,
    pub(super) options: MountOptions,
    pub(super) forced_shutdown: bool,
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

    pub(in crate::fs::fs_impls::exfat_refactor) fn publish(
        &self,
        mount_runtime: MountRuntimeState,
    ) {
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

pub(super) struct ExfatFsType;

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
