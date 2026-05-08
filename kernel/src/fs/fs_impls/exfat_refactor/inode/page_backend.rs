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
    super::{boot::BootRegion, fs::VolumeFlags},
    ClusterMap, ExfatInode,
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
pub(super) struct PageCacheContext {
    pub(super) flags: VolumeFlags,
    pub(super) block_device: Arc<dyn BlockDevice>,
    pub(super) boot_region: BootRegion,
    pub(super) cluster_map: Arc<ClusterMap>,
    pub(super) data_length: usize,
    pub(super) read_only: bool,
    pub(super) valid_data_length: usize,
}

pub(super) struct PageCacheContextGuard<'a> {
    inode: &'a ExfatInode,
}

impl Drop for PageCacheContextGuard<'_> {
    fn drop(&mut self) {
        *self.inode.page_cache_context.write() = None;
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
            .ok_or_else(|| Error::with_message(Errno::EIO, "exFAT inode is not mounted"))
    }

    pub(super) fn inode_weak(&self) -> Weak<ExfatInode> {
        self.inode.clone()
    }

    fn page_cache_context(&self) -> Result<(Arc<ExfatInode>, PageCacheContext)> {
        let inode = self.upgrade_inode()?;
        if let Some(page_cache_context) = inode.active_page_cache_context() {
            return Ok((inode, page_cache_context));
        }

        let fs = inode
            .fs
            .upgrade()
            .ok_or_else(|| Error::with_message(Errno::EIO, "exFAT filesystem is not mounted"))?;
        let mount_state = fs.mount_state_read_guard()?;
        let block_device = fs.immutable_block_device();
        let boot_region = fs.immutable_boot_region();
        let (cluster_map, data_length, valid_data_length) = inode.cluster_map_snapshot()?;
        Ok((
            inode,
            PageCacheContext {
                flags: mount_state.flags,
                block_device,
                boot_region,
                cluster_map,
                data_length,
                read_only: mount_state.options.fs_flags.contains(FsFlags::RDONLY),
                valid_data_length,
            },
        ))
    }
}

impl PageCacheBackend for ExfatFilePageBackend {
    fn read_page_async(&self, idx: usize, frame: &CachePage) -> Result<BioWaiter> {
        let (_inode, page_cache_context) = self.page_cache_context()?;
        if page_cache_context.flags.clear_to_zero || page_cache_context.flags.media_failure {
            return_errno!(Errno::EIO);
        }
        let (file_offset, initialized_len) = ExfatInode::regular_file_page_range(
            idx,
            page_cache_context.data_length,
            page_cache_context.valid_data_length,
        )?;
        let initialized_sector_len = initialized_len
            .div_ceil(page_cache_context.boot_region.sector_size)
            .checked_mul(page_cache_context.boot_region.sector_size)
            .ok_or_else(|| Error::new(Errno::EINVAL))?;
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
            &page_cache_context.block_device,
            &page_cache_context.boot_region,
            frame,
            page_cache_context.cluster_map.as_ref(),
            page_cache_context.data_length,
            file_offset,
            initialized_sector_len,
            BioType::Read,
        )
    }

    fn write_page_async(&self, idx: usize, frame: &CachePage) -> Result<BioWaiter> {
        let (_inode, page_cache_context) = self.page_cache_context()?;
        if page_cache_context.flags.clear_to_zero || page_cache_context.flags.media_failure {
            return_errno!(Errno::EIO);
        }
        if page_cache_context.read_only {
            return_errno!(Errno::EROFS);
        }
        let (file_offset, initialized_len) = ExfatInode::regular_file_page_range(
            idx,
            page_cache_context.data_length,
            page_cache_context.valid_data_length,
        )?;
        let initialized_sector_len = initialized_len
            .div_ceil(page_cache_context.boot_region.sector_size)
            .checked_mul(page_cache_context.boot_region.sector_size)
            .ok_or_else(|| Error::new(Errno::EINVAL))?;
        if initialized_sector_len == 0 {
            return Ok(BioWaiter::new());
        }

        ExfatInode::regular_file_page_waiter(
            &page_cache_context.block_device,
            &page_cache_context.boot_region,
            frame,
            page_cache_context.cluster_map.as_ref(),
            page_cache_context.data_length,
            file_offset,
            initialized_sector_len,
            BioType::Write,
        )
    }

    fn npages(&self) -> usize {
        self.inode.upgrade().map_or(0, |inode| {
            inode.active_page_cache_context().map_or_else(
                || inode.metadata_projection().size.div_ceil(PAGE_SIZE),
                |page_cache_context| page_cache_context.data_length.div_ceil(PAGE_SIZE),
            )
        })
    }
}

impl ExfatInode {
    pub(super) fn install_page_cache_context(
        &self,
        _inode_state_guard: &InodeStateWriteGuard<'_>,
        page_cache_context: PageCacheContext,
    ) -> PageCacheContextGuard<'_> {
        let mut active_page_cache_context = self.page_cache_context.write();
        debug_assert!(active_page_cache_context.is_none());
        *active_page_cache_context = Some(page_cache_context);
        PageCacheContextGuard { inode: self }
    }

    fn active_page_cache_context(&self) -> Option<PageCacheContext> {
        self.page_cache_context.read().clone()
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
