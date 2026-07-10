// SPDX-License-Identifier: MPL-2.0

//! Stores inode cluster-map, dirty-state, inode-state admission, timestamp, and guard-order helpers.
//!
//! Method groups: dirty-state transitions, directory access, regular-file snapshots,
//! directory byte I/O, timestamp conversion, child construction, and ordered write guards.

use alloc::{collections::BTreeSet, vec, vec::Vec};
use core::{cell::RefCell, ops::Range, time::Duration};

use aster_block::BlockDevice;
use ostd::{
    mm::VmIo,
    sync::{RwMutexReadGuard, RwMutexWriteGuard},
};
use time::{Date, Month, OffsetDateTime, PrimitiveDateTime, Time, UtcOffset};

use super::{
    super::{
        bitmap::ClusterRange,
        boot::BootRegion,
        device_io,
        direntry::{DIRECTORY_ENTRY_SIZE, DirEntrySlotRange},
        fat::{ChainVisitControl, FatChainStep, FatReader},
        fs::{ExfatFs, MountOptions, MountStateReadGuard, MountStateWriteGuard},
        invalid_on_disk_layout, invalid_operation_input,
        upcase::UpcaseTable,
    },
    ExfatInode,
};
use crate::{
    fs::{file::InodeType, vfs::inode::Metadata},
    prelude::*,
};

pub(super) struct InodeState {
    pub(super) dirty_state: InodeDirtyState,
    pub(super) dirty_file_retention: Option<Arc<ExfatInode>>,
    pub(super) metadata: Metadata,
    pub(super) parent: Weak<ExfatInode>,
    pub(super) cluster_map: Option<Arc<ClusterMap>>,
    pub(super) dir_entry_stream: StreamExtensionDirEntry,
}

pub(super) struct InodeStateReadGuard<'a> {
    inode: &'a ExfatInode,
    guard: RwMutexReadGuard<'a, InodeState>,
}

impl<'a> InodeStateReadGuard<'a> {
    fn new(inode: &'a ExfatInode, guard: RwMutexReadGuard<'a, InodeState>) -> Self {
        Self { inode, guard }
    }

    pub(super) fn metadata(&self) -> Metadata {
        self.guard.metadata
    }

    pub(super) fn parent(&self) -> Option<Arc<ExfatInode>> {
        self.guard.parent.upgrade()
    }

    pub(super) fn dir_entry_stream(&self) -> StreamExtensionDirEntry {
        self.guard.dir_entry_stream
    }

    pub(super) fn cached_cluster_map(&self) -> Option<Arc<ClusterMap>> {
        self.guard.cluster_map.clone()
    }

    pub(super) fn dirty_state(&self) -> InodeDirtyState {
        self.guard.dirty_state
    }

    pub(super) fn page_cache_context(&self) -> Option<super::page_backend::PageCacheContext> {
        self.inode.page_cache_context.read().clone()
    }
}

pub(super) struct InodeStateWriteGuard<'a> {
    inode: &'a ExfatInode,
    guard: RefCell<RwMutexWriteGuard<'a, InodeState>>,
}

impl<'a> InodeStateWriteGuard<'a> {
    fn new(inode: &'a ExfatInode, guard: RwMutexWriteGuard<'a, InodeState>) -> Self {
        Self {
            inode,
            guard: RefCell::new(guard),
        }
    }

    pub(super) fn metadata(&self) -> Metadata {
        self.guard.borrow().metadata
    }

    pub(super) fn guards_inode(&self, inode: &ExfatInode) -> bool {
        core::ptr::eq(self.inode, inode)
    }

    pub(super) fn with_metadata_mut<R>(
        &self,
        update_metadata_fn: impl FnOnce(&mut Metadata) -> R,
    ) -> R {
        let mut inode_state = self.guard.borrow_mut();
        update_metadata_fn(&mut inode_state.metadata)
    }

    pub(super) fn parent(&self) -> Option<Arc<ExfatInode>> {
        self.guard.borrow().parent.upgrade()
    }

    pub(super) fn set_parent(&self, parent: Weak<ExfatInode>) {
        self.guard.borrow_mut().parent = parent;
    }

    pub(super) fn dir_entry_stream(&self) -> StreamExtensionDirEntry {
        self.guard.borrow().dir_entry_stream
    }

    pub(super) fn replace_dir_entry_stream(
        &self,
        dir_entry_stream: StreamExtensionDirEntry,
    ) -> StreamExtensionDirEntry {
        let mut inode_state = self.guard.borrow_mut();
        core::mem::replace(&mut inode_state.dir_entry_stream, dir_entry_stream)
    }

    pub(super) fn set_cached_cluster_map(&self, cluster_map: Arc<ClusterMap>) {
        self.guard.borrow_mut().cluster_map = Some(cluster_map);
    }

    pub(super) fn cached_cluster_map(&self) -> Option<Arc<ClusterMap>> {
        self.guard.borrow().cluster_map.clone()
    }

    pub(super) fn page_cache_context(&self) -> Option<super::page_backend::PageCacheContext> {
        self.inode.page_cache_context.read().clone()
    }

    pub(super) fn replace_page_cache_context(
        &self,
        page_cache_context: super::page_backend::PageCacheContext,
    ) -> Option<super::page_backend::PageCacheContext> {
        self.inode
            .page_cache_context
            .write()
            .replace(page_cache_context)
    }

    pub(super) fn restore_page_cache_context(
        &self,
        page_cache_context: Option<super::page_backend::PageCacheContext>,
    ) {
        *self.inode.page_cache_context.write() = page_cache_context;
    }

    pub(super) fn with_dirty_state_mut<R>(
        &self,
        update_dirty_state_fn: impl FnOnce(&mut InodeDirtyState) -> R,
    ) -> R {
        let mut inode_state = self.guard.borrow_mut();
        update_dirty_state_fn(&mut inode_state.dirty_state)
    }

    pub(super) fn dirty_state(&self) -> InodeDirtyState {
        self.guard.borrow().dirty_state
    }

    pub(super) fn has_dirty_file_retention(&self) -> bool {
        self.guard.borrow().dirty_file_retention.is_some()
    }

    pub(super) fn set_dirty_file_retention(&self, retained_inode: Option<Arc<ExfatInode>>) {
        self.guard.borrow_mut().dirty_file_retention = retained_inode;
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(in crate::fs::fs_impls::exfat_refactor) struct StreamExtensionDirEntry {
    // `None` is reserved for the unbounded root directory; ordinary files and
    // directories always keep `Some(data_length)`.
    pub(in crate::fs::fs_impls::exfat_refactor) data_length: Option<usize>,
    pub(in crate::fs::fs_impls::exfat_refactor) first_cluster: u32,
    // `None` is reserved for the unbounded root directory.
    pub(in crate::fs::fs_impls::exfat_refactor) valid_data_length: Option<usize>,
    pub(in crate::fs::fs_impls::exfat_refactor) no_fat_chain: bool,
}

#[derive(Clone)]
pub(in crate::fs::fs_impls::exfat_refactor) struct ClusterMap {
    stream_extension: StreamExtensionDirEntry,
    cluster_ranges: Vec<ClusterRange>,
}

impl ClusterMap {
    pub(super) fn stream_extension(&self) -> StreamExtensionDirEntry {
        self.stream_extension
    }

    pub(super) fn cluster_ranges(&self) -> &[ClusterRange] {
        &self.cluster_ranges
    }

    pub(super) fn validated_lengths(&self) -> Result<(usize, usize)> {
        let Some(data_length) = self.stream_extension.data_length else {
            return_errno!(Errno::EINVAL);
        };
        let Some(valid_data_length) = self.stream_extension.valid_data_length else {
            return_errno!(Errno::EINVAL);
        };
        if valid_data_length > data_length {
            return_errno!(Errno::EINVAL);
        }
        if data_length == 0 {
            if self.stream_extension.first_cluster != 0 || valid_data_length != 0 {
                return_errno!(Errno::EINVAL);
            }
            if !self.cluster_ranges.is_empty() {
                return_errno!(Errno::EINVAL);
            }
            return Ok((0, 0));
        }
        Ok((data_length, valid_data_length))
    }

    pub(super) fn mapped_cluster(
        &self,
        boot_region: &BootRegion,
        cluster_index: usize,
    ) -> Result<u32> {
        let (data_length, _) = self.validated_lengths()?;
        let allocated_clusters = data_length.div_ceil(boot_region.cluster_size);
        let materialized_clusters =
            self.cluster_ranges
                .iter()
                .try_fold(0usize, |total_clusters, range| {
                    total_clusters
                        .checked_add(range.cluster_count)
                        .ok_or_else(|| Error::new(Errno::EINVAL))
                })?;
        if materialized_clusters != allocated_clusters {
            return_errno!(Errno::EINVAL);
        }
        if cluster_index >= allocated_clusters {
            return_errno!(Errno::EINVAL);
        }

        let mut remaining_clusters = cluster_index;
        for range in &self.cluster_ranges {
            if remaining_clusters < range.cluster_count {
                return range
                    .start_cluster
                    .checked_add(
                        u32::try_from(remaining_clusters).map_err(|_| Error::new(Errno::EINVAL))?,
                    )
                    .ok_or_else(|| Error::new(Errno::EINVAL));
            }
            remaining_clusters -= range.cluster_count;
        }
        return_errno!(Errno::EINVAL);
    }
}

/// Classifies which dirty portions of an inode still need persistence.
///
/// `Metadata` means only inode entry-set metadata remains dirty. `Data` means
/// file-content dirty state remains outstanding and therefore also implies the
/// later entry-set write-back that made that content visible. `DataAndMetadata` keeps
/// both generations outstanding until `sync_all()` clears the captured window.
#[derive(Clone, Copy, Eq, PartialEq)]
pub(super) enum DirtyLevel {
    Clean,
    Metadata,
    Data,
    DataAndMetadata,
}

/// Tracks the dirty generations that Level-2 inode state has marked.
///
/// `content_generation` advances when mutation marks new data visibility or
/// stream-shape state dirty. `metadata_generation` advances when metadata-only
/// mutation marks dirty state. `next_generation` stays monotonic so sync can
/// clear only the captured window that was proven durable after wakeup.
#[derive(Clone, Copy, Default)]
pub(super) struct InodeDirtyState {
    next_generation: u64,
    content_generation: Option<u64>,
    metadata_generation: Option<u64>,
}

impl InodeDirtyState {
    fn next_dirty_generation(&mut self) -> u64 {
        self.next_generation = self.next_generation.saturating_add(1);
        self.next_generation
    }

    pub(super) fn dirty_level(self) -> DirtyLevel {
        match (self.content_generation, self.metadata_generation) {
            (None, None) => DirtyLevel::Clean,
            (None, Some(_)) => DirtyLevel::Metadata,
            (Some(_), None) => DirtyLevel::Data,
            (Some(_), Some(_)) => DirtyLevel::DataAndMetadata,
        }
    }

    pub(super) fn mark_content_dirty(&mut self) {
        let generation = self.next_dirty_generation();
        self.content_generation = Some(generation);
        self.metadata_generation = None;
    }

    pub(super) fn mark_metadata_dirty(&mut self) {
        self.metadata_generation = Some(self.next_dirty_generation());
    }

    pub(super) fn needs_sync_data(self) -> bool {
        matches!(
            self.dirty_level(),
            DirtyLevel::Data | DirtyLevel::DataAndMetadata
        )
    }

    pub(super) fn needs_sync_all(self) -> bool {
        self.dirty_level() != DirtyLevel::Clean
    }

    pub(super) fn has_deferred_regular_file_publish(self) -> bool {
        self.content_generation.is_some()
    }

    pub(super) fn clear_detached_regular_file_publish_debt(&mut self) {
        self.content_generation = None;
        self.metadata_generation = None;
    }

    fn clear_committed_content(&mut self, synced_state: Self) {
        if synced_state
            .content_generation
            .zip(self.content_generation)
            .is_some_and(|(synced_generation, current_generation)| {
                current_generation <= synced_generation
            })
        {
            self.content_generation = None;
        }
    }

    fn clear_committed_metadata(&mut self, synced_state: Self) {
        if synced_state
            .metadata_generation
            .zip(self.metadata_generation)
            .is_some_and(|(synced_generation, current_generation)| {
                current_generation <= synced_generation
            })
        {
            self.metadata_generation = None;
        }
    }

    pub(super) fn commit_data(&mut self, synced_state: Self) {
        self.clear_committed_content(synced_state);
    }

    pub(super) fn commit_all(&mut self, synced_state: Self) {
        self.clear_committed_content(synced_state);
        self.clear_committed_metadata(synced_state);
    }
}

#[derive(Clone, Copy)]
pub(super) enum InodeTimestampField {
    Accessed,
    Modified,
}

pub(super) struct MountAccessGuard<'a> {
    fs: &'a ExfatFs,
    mount_state: MountStateAccessGuard<'a>,
}

enum MountStateAccessGuard<'a> {
    Read(MountStateReadGuard<'a>),
    Write(MountStateWriteGuard<'a>),
}

impl<'a> MountAccessGuard<'a> {
    pub(super) fn block_device(&self) -> Arc<dyn BlockDevice> {
        self.fs.immutable_block_device()
    }

    pub(super) fn boot_region(&self) -> BootRegion {
        self.fs.immutable_boot_region()
    }

    pub(super) fn forced_shutdown(&self) -> bool {
        match &self.mount_state {
            MountStateAccessGuard::Read(mount_state) => mount_state.forced_shutdown,
            MountStateAccessGuard::Write(mount_state) => mount_state.forced_shutdown,
        }
    }

    pub(super) fn options(&self) -> MountOptions {
        match &self.mount_state {
            MountStateAccessGuard::Read(mount_state) => mount_state.options.clone(),
            MountStateAccessGuard::Write(mount_state) => mount_state.options.clone(),
        }
    }

    pub(super) fn write_guard_mut(&mut self) -> Result<&mut MountStateWriteGuard<'a>> {
        let MountStateAccessGuard::Write(mount_state) = &mut self.mount_state else {
            return_errno_with_message!(
                Errno::EINVAL,
                "lookup mount access has no mutable mount state"
            );
        };
        Ok(mount_state)
    }

    pub(super) fn upcase_table(&self) -> Arc<UpcaseTable> {
        match &self.mount_state {
            MountStateAccessGuard::Read(mount_state) => mount_state.upcase_table.clone(),
            MountStateAccessGuard::Write(mount_state) => mount_state.upcase_table.clone(),
        }
    }
}

impl ExfatInode {
    pub(super) fn inode_state_read_guard(&self) -> InodeStateReadGuard<'_> {
        InodeStateReadGuard::new(self, self.inode_state.read())
    }

    pub(super) fn inode_state_write_guard(&self) -> InodeStateWriteGuard<'_> {
        InodeStateWriteGuard::new(self, self.inode_state.write())
    }

    // Directory access

    pub(super) fn mount_access_read_guard<'a>(
        &self,
        fs: &'a Arc<ExfatFs>,
    ) -> Result<MountAccessGuard<'a>> {
        if self.inode_state_read_guard().metadata().type_ != InodeType::Dir {
            return_errno!(Errno::ENOTDIR);
        }
        Ok(MountAccessGuard {
            fs: fs.as_ref(),
            mount_state: MountStateAccessGuard::Read(fs.mount_state_read_guard()?),
        })
    }

    pub(super) fn mount_access_write_guard<'a>(
        &self,
        fs: &'a Arc<ExfatFs>,
    ) -> Result<MountAccessGuard<'a>> {
        if self.inode_state_read_guard().metadata().type_ != InodeType::Dir {
            return_errno!(Errno::ENOTDIR);
        }
        Ok(MountAccessGuard {
            fs: fs.as_ref(),
            mount_state: MountStateAccessGuard::Write(fs.mount_state_write_guard()?),
        })
    }

    pub(super) fn directory_snapshot(
        &self,
    ) -> Result<(InodeStateReadGuard<'_>, StreamExtensionDirEntry)> {
        let inode_state_guard = self.inode_state_read_guard();
        let cluster_map = inode_state_guard.dir_entry_stream();
        Ok((inode_state_guard, cluster_map))
    }
}

// ---- Cluster map resolution ----
impl ExfatInode {
    fn resolve_cluster_map(
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        cluster_map: StreamExtensionDirEntry,
    ) -> Result<ClusterMap> {
        let Some(data_length) = cluster_map.data_length else {
            return_errno!(Errno::EINVAL);
        };
        let Some(valid_data_length) = cluster_map.valid_data_length else {
            return_errno!(Errno::EINVAL);
        };
        if valid_data_length > data_length {
            return_errno!(Errno::EINVAL);
        }
        if data_length == 0 {
            if cluster_map.first_cluster != 0 || valid_data_length != 0 {
                return_errno!(Errno::EINVAL);
            }
            return Ok(ClusterMap {
                stream_extension: cluster_map,
                cluster_ranges: Vec::new(),
            });
        }

        boot_region.validate_stream_data(
            cluster_map.first_cluster,
            u64::try_from(data_length).map_err(|_| Error::new(Errno::EINVAL))?,
        )?;
        let allocated_clusters = data_length.div_ceil(boot_region.cluster_size);
        let cluster_ranges = if cluster_map.no_fat_chain {
            let last_cluster = cluster_map
                .first_cluster
                .checked_add(
                    u32::try_from(allocated_clusters.saturating_sub(1))
                        .map_err(|_| invalid_on_disk_layout())?,
                )
                .ok_or_else(invalid_on_disk_layout)?;
            if !boot_region.is_valid_cluster(last_cluster) {
                return Err(invalid_on_disk_layout());
            }
            vec![ClusterRange {
                start_cluster: cluster_map.first_cluster,
                cluster_count: allocated_clusters,
            }]
        } else {
            let mut cluster_ranges: Vec<ClusterRange> = Vec::new();
            let mut current_cluster = cluster_map.first_cluster;
            let mut visited_clusters = BTreeSet::new();
            let mut fat_reader = FatReader::new(block_device.as_ref(), boot_region);
            for cluster_index in 0..allocated_clusters {
                if !visited_clusters.insert(current_cluster) {
                    return Err(invalid_on_disk_layout());
                }
                match cluster_ranges.last_mut() {
                    Some(range)
                        if range.start_cluster.checked_add(
                            u32::try_from(range.cluster_count)
                                .map_err(|_| invalid_on_disk_layout())?,
                        ) == Some(current_cluster) =>
                    {
                        range.cluster_count += 1;
                    }
                    _ => cluster_ranges.push(ClusterRange {
                        start_cluster: current_cluster,
                        cluster_count: 1,
                    }),
                }
                if cluster_index + 1 == allocated_clusters {
                    break;
                }
                current_cluster = match fat_reader.next_cluster(current_cluster)? {
                    FatChainStep::Continue(next_cluster) => next_cluster,
                    FatChainStep::End => return Err(invalid_on_disk_layout()),
                };
            }
            cluster_ranges
        };
        Ok(ClusterMap {
            stream_extension: cluster_map,
            cluster_ranges,
        })
    }

    pub(super) fn cluster_map_for_read_guard(
        &self,
        inode_state_guard: &InodeStateReadGuard<'_>,
        cluster_map: StreamExtensionDirEntry,
    ) -> Result<Arc<ClusterMap>> {
        if let Some(generation) = inode_state_guard
            .cached_cluster_map()
            .filter(|generation| generation.stream_extension() == cluster_map)
        {
            return Ok(generation);
        }
        let fs = self
            .fs
            .upgrade()
            .ok_or_else(|| Error::with_message(Errno::EIO, "exFAT filesystem is not mounted"))?;
        Ok(Arc::new(Self::resolve_cluster_map(
            &fs.immutable_block_device(),
            &fs.immutable_boot_region(),
            cluster_map,
        )?))
    }

    pub(super) fn cluster_map_for_write_guard(
        &self,
        inode_state_guard: &InodeStateWriteGuard<'_>,
        cluster_map: StreamExtensionDirEntry,
    ) -> Result<Arc<ClusterMap>> {
        if let Some(generation) = inode_state_guard
            .cached_cluster_map()
            .filter(|generation| generation.stream_extension() == cluster_map)
        {
            return Ok(generation);
        }
        let fs = self
            .fs
            .upgrade()
            .ok_or_else(|| Error::with_message(Errno::EIO, "exFAT filesystem is not mounted"))?;
        let generation = Arc::new(Self::resolve_cluster_map(
            &fs.immutable_block_device(),
            &fs.immutable_boot_region(),
            cluster_map,
        )?);
        inode_state_guard.set_cached_cluster_map(generation.clone());
        Ok(generation)
    }

    pub(super) fn current_cluster_map(
        &self,
        inode_state_guard: &InodeStateWriteGuard<'_>,
    ) -> Result<Arc<ClusterMap>> {
        if inode_state_guard.metadata().type_ != InodeType::File {
            return_errno!(Errno::EOPNOTSUPP);
        }
        if let Some(page_cache_context) = inode_state_guard.page_cache_context() {
            return Ok(page_cache_context.cluster_map);
        }
        let cluster_map = inode_state_guard.dir_entry_stream();
        let generation = self.cluster_map_for_write_guard(inode_state_guard, cluster_map)?;
        let (data_length, valid_data_length) = generation.validated_lengths()?;
        let page_cache_context = self.page_cache_context_for_mapping(
            generation.clone(),
            data_length,
            valid_data_length,
        )?;
        let _ = inode_state_guard.replace_page_cache_context(page_cache_context);
        Ok(generation)
    }

    pub(super) fn cluster_map_snapshot(&self) -> Result<(Arc<ClusterMap>, usize, usize)> {
        let inode_state_guard = self.inode_state_read_guard();
        match inode_state_guard.metadata().type_ {
            InodeType::Dir => return_errno!(Errno::EISDIR),
            InodeType::File => {}
            _ => return_errno!(Errno::EOPNOTSUPP),
        }

        if let Some(page_cache_context) = inode_state_guard.page_cache_context() {
            if page_cache_context.valid_data_length > page_cache_context.data_length {
                return_errno!(Errno::EINVAL);
            }
            return Ok((
                page_cache_context.cluster_map,
                page_cache_context.data_length,
                page_cache_context.valid_data_length,
            ));
        }

        let cluster_map = inode_state_guard.dir_entry_stream();
        let generation = self.cluster_map_for_read_guard(&inode_state_guard, cluster_map)?;
        let (data_length, valid_data_length) = generation.validated_lengths()?;
        *self.page_cache_context.write() = Some(self.page_cache_context_for_mapping(
            generation.clone(),
            data_length,
            valid_data_length,
        )?);
        Ok((generation, data_length, valid_data_length))
    }

    pub(super) fn replace_cluster_map(
        &self,
        inode_state_guard: &InodeStateWriteGuard<'_>,
        cluster_map: StreamExtensionDirEntry,
    ) -> Result<Arc<ClusterMap>> {
        let previous_generation = self
            .cluster_map_for_write_guard(inode_state_guard, inode_state_guard.dir_entry_stream())?;
        let next_generation = self.cluster_map_for_write_guard(inode_state_guard, cluster_map)?;
        let (data_length, valid_data_length) = next_generation.validated_lengths()?;
        let _ = inode_state_guard.replace_dir_entry_stream(cluster_map);
        inode_state_guard.set_cached_cluster_map(next_generation.clone());
        let page_cache_context =
            self.page_cache_context_for_mapping(next_generation, data_length, valid_data_length)?;
        let _ = inode_state_guard.replace_page_cache_context(page_cache_context);
        Ok(previous_generation)
    }

    // Directory I/O
}

// ---- Directory byte I/O ----
impl ExfatInode {
    pub(super) fn visit_directory_byte_range_for_cluster_map(
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        cluster_map: StreamExtensionDirEntry,
        byte_range: Range<usize>,
        mut visit_chunk_fn: impl FnMut(usize, Range<usize>) -> Result<()>,
    ) -> Result<()> {
        let range_length = byte_range
            .end
            .checked_sub(byte_range.start)
            .ok_or_else(invalid_operation_input)?;
        if range_length == 0 {
            return Ok(());
        }

        match cluster_map.data_length {
            Some(data_length) => {
                if data_length == 0 {
                    if cluster_map.first_cluster != 0 {
                        return Err(invalid_on_disk_layout());
                    }
                    return Err(invalid_on_disk_layout());
                }
                if data_length % DIRECTORY_ENTRY_SIZE != 0 || byte_range.end > data_length {
                    return Err(invalid_on_disk_layout());
                }

                let data_length_u64 =
                    u64::try_from(data_length).map_err(|_| invalid_on_disk_layout())?;
                boot_region.validate_stream_data(cluster_map.first_cluster, data_length_u64)?;
            }
            None if cluster_map.first_cluster == 0 => return Err(invalid_on_disk_layout()),
            None => {}
        }

        let clusters_to_skip = byte_range.start / boot_region.cluster_size;
        let mut current_cluster = cluster_map.first_cluster;
        let mut fat_reader = (cluster_map.data_length.is_none() || !cluster_map.no_fat_chain)
            .then(|| FatReader::new(block_device.as_ref(), boot_region));
        for _ in 0..clusters_to_skip {
            current_cluster = Self::advance_cluster(current_cluster, fat_reader.as_mut())?
                .ok_or_else(invalid_on_disk_layout)?;
        }

        let mut remaining = range_length;
        let mut request_offset = 0usize;
        let mut intra_cluster_offset = byte_range.start % boot_region.cluster_size;
        while remaining != 0 {
            let bytes_in_cluster = boot_region
                .cluster_size
                .checked_sub(intra_cluster_offset)
                .ok_or_else(invalid_on_disk_layout)?;
            let bytes_to_visit = remaining.min(bytes_in_cluster);
            let chunk_end = request_offset
                .checked_add(bytes_to_visit)
                .ok_or_else(invalid_on_disk_layout)?;
            let cluster_offset = boot_region.cluster_offset(current_cluster)?;
            let byte_offset = cluster_offset
                .checked_add(intra_cluster_offset)
                .ok_or_else(invalid_on_disk_layout)?;
            visit_chunk_fn(byte_offset, request_offset..chunk_end)?;
            remaining -= bytes_to_visit;
            request_offset = chunk_end;
            if remaining == 0 {
                return Ok(());
            }

            current_cluster = Self::advance_cluster(current_cluster, fat_reader.as_mut())?
                .ok_or_else(invalid_on_disk_layout)?;
            intra_cluster_offset = 0;
        }
        Ok(())
    }

    pub(super) fn read_directory_bytes_for_cluster_map(
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        cluster_map: StreamExtensionDirEntry,
    ) -> Result<Vec<u8>> {
        let Some(data_length) = cluster_map.data_length else {
            let mut directory_bytes = Vec::new();
            let mut fat_reader = FatReader::new(block_device.as_ref(), boot_region);
            fat_reader.walk_cluster_chain(cluster_map.first_cluster, |_, cluster_bytes| {
                directory_bytes.extend_from_slice(cluster_bytes);
                Ok(ChainVisitControl::Continue)
            })?;
            return Ok(directory_bytes);
        };

        if data_length == 0 {
            if cluster_map.first_cluster != 0 {
                return Err(invalid_on_disk_layout());
            }
            return Ok(Vec::new());
        }
        if data_length % DIRECTORY_ENTRY_SIZE != 0 {
            return Err(invalid_on_disk_layout());
        }

        let data_length_u64 = u64::try_from(data_length).map_err(|_| invalid_on_disk_layout())?;
        boot_region.validate_stream_data(cluster_map.first_cluster, data_length_u64)?;
        if cluster_map.no_fat_chain {
            let mut remaining = data_length;
            let mut write_offset = 0usize;
            let mut directory_bytes = vec![0; data_length];
            let mut current_cluster = cluster_map.first_cluster;
            while remaining != 0 {
                let cluster_start = boot_region.cluster_offset(current_cluster)?;
                let bytes_to_read = remaining.min(boot_region.cluster_size);
                let write_end = write_offset
                    .checked_add(bytes_to_read)
                    .ok_or_else(invalid_on_disk_layout)?;
                block_device
                    .read_bytes(cluster_start, &mut directory_bytes[write_offset..write_end])
                    .map_err(|_| device_io())?;
                remaining -= bytes_to_read;
                write_offset = write_end;
                if remaining == 0 {
                    return Ok(directory_bytes);
                }
                current_cluster = Self::advance_cluster(current_cluster, None)?
                    .ok_or_else(invalid_on_disk_layout)?;
            }
            return Err(invalid_on_disk_layout());
        }

        let mut remaining = data_length;
        let mut directory_bytes = Vec::with_capacity(data_length);
        let mut fat_reader = FatReader::new(block_device.as_ref(), boot_region);
        fat_reader.walk_cluster_chain(cluster_map.first_cluster, |_, cluster_bytes| {
            let bytes_to_copy = remaining.min(cluster_bytes.len());
            directory_bytes.extend_from_slice(&cluster_bytes[..bytes_to_copy]);
            remaining -= bytes_to_copy;
            Ok(if remaining == 0 {
                ChainVisitControl::Stop
            } else {
                ChainVisitControl::Continue
            })
        })?;
        if remaining != 0 {
            return Err(invalid_on_disk_layout());
        }
        Ok(directory_bytes)
    }

    pub(super) fn write_directory_bytes_for_cluster_map(
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        directory_bytes: &[u8],
        cluster_map: StreamExtensionDirEntry,
    ) -> Result<()> {
        let expected_length = match cluster_map.data_length {
            Some(data_length) => data_length,
            None => directory_bytes.len(),
        };
        if directory_bytes.len() != expected_length {
            return Err(invalid_operation_input());
        }
        if directory_bytes.is_empty() {
            return Ok(());
        }
        if cluster_map.data_length.is_some() && cluster_map.no_fat_chain {
            let mut remaining = directory_bytes;
            let mut current_cluster = cluster_map.first_cluster;
            while !remaining.is_empty() {
                let bytes_to_write = remaining.len().min(boot_region.cluster_size);
                block_device
                    .write_bytes(
                        boot_region.cluster_offset(current_cluster)?,
                        &remaining[..bytes_to_write],
                    )
                    .map_err(|_| device_io())?;
                remaining = &remaining[bytes_to_write..];
                if remaining.is_empty() {
                    return Ok(());
                }
                current_cluster = Self::advance_cluster(current_cluster, None)?
                    .ok_or_else(invalid_on_disk_layout)?;
            }
            return Err(invalid_on_disk_layout());
        }

        let mut remaining = directory_bytes;
        let mut current_cluster = cluster_map.first_cluster;
        let mut fat_reader =
            (!cluster_map.no_fat_chain).then(|| FatReader::new(block_device.as_ref(), boot_region));
        while !remaining.is_empty() {
            let bytes_to_write = remaining.len().min(boot_region.cluster_size);
            block_device
                .write_bytes(
                    boot_region.cluster_offset(current_cluster)?,
                    &remaining[..bytes_to_write],
                )
                .map_err(|_| device_io())?;
            remaining = &remaining[bytes_to_write..];
            if remaining.is_empty() {
                break;
            }
            current_cluster = match Self::advance_cluster(current_cluster, fat_reader.as_mut())? {
                Some(next_cluster) => next_cluster,
                None => return Err(invalid_on_disk_layout()),
            };
        }
        Ok(())
    }

    pub(super) fn initialize_directory_cluster(
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        first_cluster: u32,
    ) -> Result<()> {
        let cluster_offset = boot_region.cluster_offset(first_cluster)?;
        let cluster_bytes = vec![0; boot_region.cluster_size];
        block_device
            .write_bytes(cluster_offset, &cluster_bytes)
            .map_err(|_| device_io())
    }

    pub(super) fn advance_cluster(
        current_cluster: u32,
        fat_reader: Option<&mut FatReader<'_>>,
    ) -> Result<Option<u32>> {
        match fat_reader {
            Some(fat_reader) => match fat_reader.next_cluster(current_cluster) {
                Ok(FatChainStep::Continue(next_cluster)) => Ok(Some(next_cluster)),
                Ok(FatChainStep::End) => Ok(None),
                Err(error) => Err(error),
            },
            None => current_cluster
                .checked_add(1)
                .map(Some)
                .ok_or(invalid_on_disk_layout()),
        }
    }

    // Dirty tracking

    // The caller must already hold Level-2 inode-state authority, so these
    // helpers never reacquire `inode_state` internally.
}

// ---- Dirty-state transitions ----
impl ExfatInode {
    pub(super) fn mark_content_dirty(&self, inode_state_guard: &InodeStateWriteGuard<'_>) {
        inode_state_guard.with_dirty_state_mut(InodeDirtyState::mark_content_dirty);
        if inode_state_guard.metadata().type_ != InodeType::File {
            return;
        }
        if !inode_state_guard.has_dirty_file_retention() {
            inode_state_guard.set_dirty_file_retention(self.weak_self().upgrade());
        }
    }

    pub(super) fn mark_metadata_dirty(&self, inode_state_guard: &InodeStateWriteGuard<'_>) {
        if inode_state_guard.metadata().type_ == InodeType::Dir {
            return;
        }
        inode_state_guard.with_dirty_state_mut(InodeDirtyState::mark_metadata_dirty);
    }

    pub(in crate::fs::fs_impls::exfat_refactor) fn clear_dirty_file_retention(&self) {
        self.inode_state_write_guard()
            .set_dirty_file_retention(None);
    }

    pub(super) fn clear_detached_regular_file_publish_debt_with_guard(
        &self,
        inode_state_guard: &InodeStateWriteGuard<'_>,
    ) {
        inode_state_guard
            .with_dirty_state_mut(InodeDirtyState::clear_detached_regular_file_publish_debt);
        inode_state_guard.set_dirty_file_retention(None);
    }

    pub(super) fn clear_dirty_file_retention_if_not_needed_with_guard(
        &self,
        inode_state_guard: &InodeStateWriteGuard<'_>,
        dirty_state: InodeDirtyState,
    ) {
        if dirty_state.has_deferred_regular_file_publish() {
            return;
        }
        inode_state_guard.set_dirty_file_retention(None);
    }

    // Identity

    pub(super) fn entry_location_ino(
        &self,
        cluster_map: StreamExtensionDirEntry,
        entry_index: usize,
    ) -> Result<u64> {
        Ok((u64::from(cluster_map.first_cluster) << 32)
            | u64::from(u32::try_from(entry_index).map_err(|_| invalid_on_disk_layout())?))
    }
}

// ---- Timestamps, name validation, write guards ----
impl ExfatInode {
    pub(super) fn child_inode_from_directory_entry(
        parent: &Self,
        fs: &Arc<ExfatFs>,
        boot_region: &BootRegion,
        parent_first_cluster: u32,
        slot_range: DirEntrySlotRange,
        inode_type: InodeType,
        first_cluster: u32,
        data_length: usize,
        valid_data_length: usize,
        no_fat_chain: bool,
    ) -> Result<Arc<Self>> {
        let child_ino = (u64::from(parent_first_cluster) << 32)
            | u64::from(
                u32::try_from(slot_range.first_entry_index())
                    .map_err(|_| invalid_on_disk_layout())?,
            );
        let child_inode = Self::new_child(
            fs,
            parent.weak_self(),
            child_ino,
            inode_type,
            boot_region.cluster_size,
            data_length,
            first_cluster,
            data_length,
            valid_data_length,
            no_fat_chain,
        );
        if inode_type == InodeType::File {
            child_inode.store_entry_set_location_hint(slot_range)?;
        }
        Ok(child_inode)
    }

    // Input validation

    pub(super) fn validate_name(
        name: &str,
        options: &MountOptions,
    ) -> core::result::Result<Vec<u16>, Error> {
        let normalized_name = if options.keep_last_dots {
            name
        } else {
            name.trim_end_matches('.')
        };
        if normalized_name.is_empty() || normalized_name == "." || normalized_name == ".." {
            return_errno_with_message!(Errno::EINVAL, "invalid exFAT name");
        }

        let mut name = Vec::new();
        for character in normalized_name.chars() {
            if character <= '\u{001F}'
                || matches!(
                    character,
                    '"' | '*' | '/' | ':' | '<' | '>' | '?' | '\\' | '|'
                )
            {
                return_errno_with_message!(Errno::EINVAL, "invalid exFAT name");
            }
            let mut encoded = [0u16; 2];
            name.extend(character.encode_utf16(&mut encoded).iter().copied());
        }
        if name.len() > UpcaseTable::NAME_MAX {
            return_errno!(Errno::ENAMETOOLONG);
        }
        Ok(name)
    }

    // Timestamp codec

    pub(super) fn decoded_exfat_timestamp(
        timestamp_bytes: [u8; 4],
        ten_ms_increment: Option<u8>,
        utc_offset_byte: u8,
    ) -> Result<Duration> {
        if timestamp_bytes == [0; 4] && ten_ms_increment.unwrap_or(0) == 0 {
            return Ok(Duration::ZERO);
        }

        let encoded_date = u16::from_le_bytes([timestamp_bytes[2], timestamp_bytes[3]]);
        let encoded_year = 1980i32 + i32::from(encoded_date >> 9);
        let encoded_month =
            u8::try_from((encoded_date >> 5) & 0x0f).map_err(|_| invalid_on_disk_layout())?;
        let encoded_day =
            u8::try_from(encoded_date & 0x1f).map_err(|_| invalid_on_disk_layout())?;
        let month = Month::try_from(encoded_month).map_err(|_| invalid_on_disk_layout())?;
        let date = Date::from_calendar_date(encoded_year, month, encoded_day)
            .map_err(|_| invalid_on_disk_layout())?;
        let encoded_time = u16::from_le_bytes([timestamp_bytes[0], timestamp_bytes[1]]);
        let hour =
            u8::try_from((encoded_time >> 11) & 0x1f).map_err(|_| invalid_on_disk_layout())?;
        let minute =
            u8::try_from((encoded_time >> 5) & 0x3f).map_err(|_| invalid_on_disk_layout())?;
        let mut seconds = u8::try_from(encoded_time & 0x1f)
            .map_err(|_| invalid_on_disk_layout())?
            .checked_mul(2)
            .ok_or_else(invalid_on_disk_layout)?;
        let mut milliseconds = 0u16;

        if let Some(ten_ms_increment) = ten_ms_increment {
            if ten_ms_increment >= 200 {
                return Err(invalid_on_disk_layout());
            }

            seconds = seconds
                .checked_add(ten_ms_increment / 100)
                .ok_or(invalid_on_disk_layout())?;
            milliseconds = u16::from(ten_ms_increment % 100) * 10;
        }

        let time = Time::from_hms_milli(hour, minute, seconds, milliseconds)
            .map_err(|_| invalid_on_disk_layout())?;

        let utc_offset = Self::exfat_utc_offset(utc_offset_byte)?;
        let date_time = PrimitiveDateTime::new(date, time).assume_offset(utc_offset);
        let unix_timestamp_nanos = u64::try_from(date_time.unix_timestamp_nanos())
            .map_err(|_| invalid_on_disk_layout())?;
        Ok(Duration::from_nanos(unix_timestamp_nanos))
    }

    pub(super) fn exfat_utc_offset(utc_offset_byte: u8) -> Result<UtcOffset> {
        if utc_offset_byte & 0x80 == 0 {
            return Ok(UtcOffset::UTC);
        }

        let quarter_hours = (((utc_offset_byte & 0x7f) as i8) << 1) >> 1;
        UtcOffset::from_whole_seconds(i32::from(quarter_hours) * 15 * 60)
            .map_err(|_| invalid_on_disk_layout())
    }

    pub(super) fn encoded_exfat_timestamp_fields(
        timestamp: Duration,
        utc_offset_byte: u8,
    ) -> Result<([u8; 4], u8, u8)> {
        let unix_nanos =
            i128::try_from(timestamp.as_nanos()).map_err(|_| Error::new(Errno::EINVAL))?;
        let utc_offset = Self::exfat_utc_offset(utc_offset_byte)?;
        let date_time = OffsetDateTime::from_unix_timestamp_nanos(unix_nanos)
            .map_err(|_| Error::new(Errno::EINVAL))?
            .to_offset(utc_offset);
        let encoded_utc_offset = if utc_offset_byte & 0x80 == 0 {
            0
        } else {
            utc_offset_byte
        };
        let (
            encoded_year,
            encoded_month,
            encoded_day,
            encoded_hour,
            encoded_minute,
            encoded_second,
            encoded_millisecond,
        ) = match date_time.year() {
            ..1980 => (1980, 1u8, 1u8, 0u8, 0u8, 0u8, 0u16),
            2108.. => (2107, 12u8, 31u8, 23u8, 59u8, 59u8, 990u16),
            year => (
                year,
                date_time.month() as u8,
                date_time.day(),
                date_time.hour(),
                date_time.minute(),
                date_time.second(),
                date_time.millisecond(),
            ),
        };
        let date = ((u16::try_from(encoded_year - 1980).map_err(|_| Error::new(Errno::EINVAL))?)
            << 9)
            | (u16::from(encoded_month) << 5)
            | u16::from(encoded_day);
        let time = (u16::from(encoded_hour) << 11)
            | (u16::from(encoded_minute) << 5)
            | u16::from(encoded_second / 2);
        let date_bytes = date.to_le_bytes();
        let time_bytes = time.to_le_bytes();
        let hundredths_increment = u16::from(encoded_second % 2) * 100 + (encoded_millisecond / 10);
        Ok((
            [time_bytes[0], time_bytes[1], date_bytes[0], date_bytes[1]],
            u8::try_from(hundredths_increment).map_err(|_| Error::new(Errno::EINVAL))?,
            encoded_utc_offset,
        ))
    }

    // Misc computation

    pub(super) fn regular_file_allocated_sectors(
        boot_region: &BootRegion,
        data_length: usize,
    ) -> Result<usize> {
        let allocated_clusters = if data_length == 0 {
            0
        } else {
            data_length.div_ceil(boot_region.cluster_size)
        };
        allocated_clusters
            .checked_mul(boot_region.sectors_per_cluster)
            .ok_or(invalid_on_disk_layout())
    }

    // Other helpers

    pub(super) fn directory_write_guards_by_ino<'a>(
        mut directories: Vec<&'a ExfatInode>,
    ) -> Vec<InodeStateWriteGuard<'a>> {
        directories.sort_by_key(|directory| directory.inode_state_read_guard().metadata().ino);
        directories.dedup_by_key(|directory| directory.inode_state_read_guard().metadata().ino);
        directories
            .into_iter()
            .map(ExfatInode::inode_state_write_guard)
            .collect()
    }
}
