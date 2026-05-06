// SPDX-License-Identifier: MPL-2.0

//! Bridges the exFAT inode owner to the generic page-cache backend.
//!
//! Method groups: weak-inode backend ownership, owner-local callback snapshots, page read/write
//! BIO callbacks, page count, and inode page-cache accessors.

use aster_block::{
    BlockDevice,
    bio::{BioType, BioWaiter},
};
use ostd::mm::io::util::HasVmReaderWriter;

use super::{
    super::{boot::BootRegion, fs::VolumeAnomalyState},
    ExfatInode, RegularFileClusterMapGeneration,
    state::InodeStateWriteGuard,
};
use crate::{
    fs::{
        file::InodeType,
        vfs::{
            file_system::FsFlags,
            page_cache::{CachePage, PageCache, PageCacheBackend},
        },
    },
    prelude::*,
    vm::vmo::Vmo,
};

#[derive(Clone)]
pub(super) struct RegularFilePageCacheState {
    pub(super) anomaly: VolumeAnomalyState,
    pub(super) block_device: Arc<dyn BlockDevice>,
    pub(super) boot_region: BootRegion,
    pub(super) cluster_map: Arc<RegularFileClusterMapGeneration>,
    pub(super) data_length: usize,
    pub(super) read_only: bool,
    pub(super) valid_data_length: usize,
}

pub(super) struct RegularFilePageCacheGuard<'a> {
    inode: &'a ExfatInode,
}

impl Drop for RegularFilePageCacheGuard<'_> {
    fn drop(&mut self) {
        *self.inode.regular_file_page_cache_state.write() = None;
    }
}

pub(super) struct ExfatFilePageBackend {
    inode: Weak<ExfatInode>,
}

impl ExfatFilePageBackend {
    pub(super) fn new(inode: Weak<ExfatInode>) -> Self {
        Self { inode }
    }

    fn upgrade_inode(&self) -> Result<Arc<ExfatInode>> {
        self.inode
            .upgrade()
            .ok_or_else(|| Error::with_message(Errno::EIO, "exFAT inode is not published"))
    }

    pub(super) fn inode_weak(&self) -> Weak<ExfatInode> {
        self.inode.clone()
    }

    fn regular_file_page_cache_state(
        &self,
    ) -> Result<(Arc<ExfatInode>, RegularFilePageCacheState)> {
        let inode = self.upgrade_inode()?;
        if let Some(page_cache_state) = inode.active_regular_file_page_cache_state() {
            return Ok((inode, page_cache_state));
        }

        let fs = inode
            .fs
            .upgrade()
            .ok_or_else(|| Error::with_message(Errno::EIO, "exFAT filesystem is not mounted"))?;
        let lookup_mount_snapshot = fs.lookup_mount_snapshot()?;
        let (cluster_map, data_length, valid_data_length) =
            inode.regular_file_cluster_map_snapshot()?;
        Ok((
            inode,
            RegularFilePageCacheState {
                anomaly: lookup_mount_snapshot.anomaly,
                block_device: lookup_mount_snapshot.block_device,
                boot_region: lookup_mount_snapshot.boot_region,
                cluster_map,
                data_length,
                read_only: lookup_mount_snapshot
                    .options
                    .fs_flags
                    .contains(FsFlags::RDONLY),
                valid_data_length,
            },
        ))
    }
}

impl PageCacheBackend for ExfatFilePageBackend {
    fn read_page_async(&self, idx: usize, frame: &CachePage) -> Result<BioWaiter> {
        let (_inode, page_cache_state) = self.regular_file_page_cache_state()?;
        if page_cache_state.anomaly.clear_to_zero || page_cache_state.anomaly.media_failure {
            return_errno!(Errno::EIO);
        }
        let (file_offset, initialized_len) = ExfatInode::regular_file_page_range(
            idx,
            page_cache_state.data_length,
            page_cache_state.valid_data_length,
        )?;
        let initialized_sector_len =
            initialized_len - (initialized_len % page_cache_state.boot_region.sector_size);
        if initialized_sector_len < PAGE_SIZE {
            frame
                .writer()
                .skip(initialized_sector_len)
                .fill_zeros(PAGE_SIZE - initialized_sector_len);
        }
        if initialized_sector_len == 0 {
            return Ok(BioWaiter::new());
        }

        ExfatInode::regular_file_page_waiter(
            &page_cache_state.block_device,
            &page_cache_state.boot_region,
            frame,
            page_cache_state.cluster_map.as_ref(),
            page_cache_state.data_length,
            file_offset,
            initialized_sector_len,
            BioType::Read,
        )
    }

    fn write_page_async(&self, idx: usize, frame: &CachePage) -> Result<BioWaiter> {
        let (_inode, page_cache_state) = self.regular_file_page_cache_state()?;
        if page_cache_state.anomaly.clear_to_zero || page_cache_state.anomaly.media_failure {
            return_errno!(Errno::EIO);
        }
        if page_cache_state.read_only {
            return_errno!(Errno::EROFS);
        }
        let (file_offset, initialized_len) = ExfatInode::regular_file_page_range(
            idx,
            page_cache_state.data_length,
            page_cache_state.valid_data_length,
        )?;
        let initialized_sector_len = initialized_len
            .div_ceil(page_cache_state.boot_region.sector_size)
            .checked_mul(page_cache_state.boot_region.sector_size)
            .ok_or_else(|| Error::new(Errno::EINVAL))?;
        if initialized_sector_len == 0 {
            return Ok(BioWaiter::new());
        }

        ExfatInode::regular_file_page_waiter(
            &page_cache_state.block_device,
            &page_cache_state.boot_region,
            frame,
            page_cache_state.cluster_map.as_ref(),
            page_cache_state.data_length,
            file_offset,
            initialized_sector_len,
            BioType::Write,
        )
    }

    fn npages(&self) -> usize {
        self.inode.upgrade().map_or(0, |inode| {
            inode.active_regular_file_page_cache_state().map_or_else(
                || inode.metadata_projection().size.div_ceil(PAGE_SIZE),
                |page_cache_state| page_cache_state.data_length.div_ceil(PAGE_SIZE),
            )
        })
    }
}

impl ExfatInode {
    pub(super) fn install_regular_file_page_cache_state(
        &self,
        _inode_state_guard: &InodeStateWriteGuard<'_>,
        page_cache_state: RegularFilePageCacheState,
    ) -> RegularFilePageCacheGuard<'_> {
        let mut active_page_cache_state = self.regular_file_page_cache_state.write();
        debug_assert!(active_page_cache_state.is_none());
        *active_page_cache_state = Some(page_cache_state);
        RegularFilePageCacheGuard { inode: self }
    }

    fn active_regular_file_page_cache_state(&self) -> Option<RegularFilePageCacheState> {
        self.regular_file_page_cache_state.read().clone()
    }

    pub(super) fn weak_self(&self) -> Weak<Self> {
        self.page_backend.inode_weak()
    }

    pub(super) fn page_cache_handle(&self) -> Option<&PageCache> {
        if self.metadata.read().type_ != InodeType::File {
            return None;
        }

        self.page_cache
            .call_once(|| {
                let backend: Arc<dyn PageCacheBackend> = self.page_backend.clone();
                let capacity = self.metadata.read().size;
                PageCache::with_capacity(capacity, Arc::downgrade(&backend)).ok()
            })
            .as_ref()
    }

    pub(super) fn page_cache_vmo(&self) -> Option<Arc<Vmo>> {
        self.page_cache_handle()
            .map(|page_cache| page_cache.pages().clone())
    }
}
