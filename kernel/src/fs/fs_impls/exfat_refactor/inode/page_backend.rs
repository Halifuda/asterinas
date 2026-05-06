// SPDX-License-Identifier: MPL-2.0

//! Bridges the exFAT inode owner to the generic page-cache backend.
//!
//! Method groups: weak-inode backend ownership, page read/write BIO callbacks, page count,
//! and inode page-cache accessors.

use aster_block::bio::{BioType, BioWaiter};
use ostd::mm::io::util::HasVmReaderWriter;

use super::ExfatInode;
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
}

impl PageCacheBackend for ExfatFilePageBackend {
    fn read_page_async(&self, idx: usize, frame: &CachePage) -> Result<BioWaiter> {
        let inode = self.upgrade_inode()?;
        let fs = inode
            .fs
            .upgrade()
            .ok_or_else(|| Error::with_message(Errno::EIO, "exFAT filesystem is not mounted"))?;
        let admission = fs.published_lookup_state().map_err(Error::from)?;
        if admission.anomaly.clear_to_zero || admission.anomaly.media_failure {
            return_errno!(Errno::EIO);
        }

        let (_inode_state_guard, cluster_map, data_length, valid_data_length) =
            inode.admitted_regular_file_cluster_map_snapshot()?;
        let (file_offset, initialized_len) =
            ExfatInode::regular_file_page_range(idx, data_length, valid_data_length)?;
        let initialized_sector_len =
            initialized_len - (initialized_len % admission.boot_region.sector_size);
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
            &admission.block_device,
            &admission.boot_region,
            frame,
            &cluster_map,
            data_length,
            file_offset,
            initialized_sector_len,
            BioType::Read,
        )
    }

    fn write_page_async(&self, idx: usize, frame: &CachePage) -> Result<BioWaiter> {
        let inode = self.upgrade_inode()?;
        let fs = inode
            .fs
            .upgrade()
            .ok_or_else(|| Error::with_message(Errno::EIO, "exFAT filesystem is not mounted"))?;
        let admission = fs.published_lookup_state().map_err(Error::from)?;
        if admission.anomaly.clear_to_zero || admission.anomaly.media_failure {
            return_errno!(Errno::EIO);
        }
        if admission.options.fs_flags.contains(FsFlags::RDONLY) {
            return_errno!(Errno::EROFS);
        }

        let (_inode_state_guard, cluster_map, data_length, valid_data_length) =
            inode.admitted_regular_file_cluster_map_snapshot()?;
        let (file_offset, initialized_len) =
            ExfatInode::regular_file_page_range(idx, data_length, valid_data_length)?;
        let initialized_sector_len = initialized_len
            .div_ceil(admission.boot_region.sector_size)
            .checked_mul(admission.boot_region.sector_size)
            .ok_or_else(|| Error::new(Errno::EINVAL))?;
        if initialized_sector_len == 0 {
            return Ok(BioWaiter::new());
        }

        ExfatInode::regular_file_page_waiter(
            &admission.block_device,
            &admission.boot_region,
            frame,
            &cluster_map,
            data_length,
            file_offset,
            initialized_sector_len,
            BioType::Write,
        )
    }

    fn npages(&self) -> usize {
        self.inode
            .upgrade()
            .map(|inode| inode.metadata.read().size.div_ceil(PAGE_SIZE))
            .unwrap_or(0)
    }
}

impl ExfatInode {
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
