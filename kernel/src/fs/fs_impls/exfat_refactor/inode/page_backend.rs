// SPDX-License-Identifier: MPL-2.0

//! Bridges the exFAT inode owner to the generic page-cache backend.
//!
//! Method groups: weak-inode backend ownership, owner-local callback snapshots, page read/write
//! BIO callbacks, page count, and inode page-cache accessors.

use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use aster_block::{BlockDevice, bio::BioStatus};
use io_util::batch::IoBatch;
use ostd::mm::io::util::HasVmReaderWriter;

use super::{
    super::{
        boot::BootRegion,
        fs::{MountRuntimeProjection, MountRuntimeState},
    },
    ClusterMap, ExfatInode,
    state::InodeStateWriteGuard,
};
use crate::{
    fs::{file::InodeType, vfs::inode::Metadata},
    prelude::*,
    vm::page_cache::{CachePageExt, LockedCachePage, PageCache, PageCacheBackend},
};

#[derive(Clone)]
pub(super) struct PageCacheContext {
    pub(super) cluster_map: Arc<ClusterMap>,
    pub(super) data_length: usize,
    pub(super) mount_runtime: Arc<MountRuntimeProjection>,
    pub(super) valid_data_length: usize,
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

    fn page_cache_context(
        &self,
    ) -> Result<(
        Arc<ExfatInode>,
        PageCacheContext,
        Arc<dyn BlockDevice>,
        BootRegion,
        MountRuntimeState,
    )> {
        let inode = self.upgrade_inode()?;
        let fs = inode
            .fs
            .upgrade()
            .ok_or_else(|| Error::with_message(Errno::EIO, "exFAT filesystem is not mounted"))?;
        let block_device = fs.immutable_block_device();
        let boot_region = fs.immutable_boot_region();
        let page_cache_context = inode.active_page_cache_context().ok_or_else(|| {
            Error::with_message(
                Errno::EIO,
                "regular exFAT file page-cache context is not published",
            )
        })?;
        let mount_runtime = page_cache_context.mount_runtime.snapshot();
        Ok((
            inode,
            page_cache_context,
            block_device,
            boot_region,
            mount_runtime,
        ))
    }
}

pub(super) struct FragmentedPageIo {
    page: Option<LockedCachePage>,
    pending: AtomicUsize,
    failed: AtomicBool,
    is_read: bool,
}

impl FragmentedPageIo {
    pub(super) fn new(page: LockedCachePage, pending: usize, is_read: bool) -> Arc<Self> {
        Arc::new(Self {
            page: Some(page),
            pending: AtomicUsize::new(pending),
            failed: AtomicBool::new(false),
            is_read,
        })
    }

    pub(super) fn complete(self: Arc<Self>, status: BioStatus) {
        if status != BioStatus::Complete && status != BioStatus::Zeros {
            self.failed.store(true, Ordering::Release);
        }

        if self.pending.fetch_sub(1, Ordering::AcqRel) != 1 {
            return;
        }

        let page = self
            .page
            .as_ref()
            .expect("fragmented page completion must still own the locked page");
        if self.is_read {
            if !self.failed.load(Ordering::Acquire) {
                page.set_up_to_date();
            }
            return;
        }

        page.clear_writing_back();
        if self.failed.load(Ordering::Acquire) {
            ostd::error!("exFAT writeback failed for a fragmented cached page; data may be lost");
        }
    }
}

impl PageCacheBackend for ExfatFilePageBackend {
    fn read_page_async(
        &self,
        idx: usize,
        locked_page: LockedCachePage,
        io_batch: &mut IoBatch,
    ) -> Result<()> {
        let (_inode, page_cache_context, block_device, boot_region, mount_runtime) =
            self.page_cache_context()?;
        if mount_runtime.forced_shutdown
            || mount_runtime.clear_to_zero
            || mount_runtime.media_failure
        {
            return_errno!(Errno::EIO);
        }
        let (file_offset, initialized_len) = ExfatInode::regular_file_page_range(
            idx,
            page_cache_context.data_length,
            page_cache_context.valid_data_length,
        )?;
        let initialized_sector_len = initialized_len
            .div_ceil(boot_region.sector_size)
            .checked_mul(boot_region.sector_size)
            .ok_or_else(|| Error::new(Errno::EINVAL))?;
        if initialized_sector_len < PAGE_SIZE {
            locked_page
                .writer()
                .skip(initialized_sector_len)
                .fill_zeros(PAGE_SIZE - initialized_sector_len);
        }
        if initialized_sector_len == 0 {
            locked_page.set_up_to_date();
            return Ok(());
        }

        ExfatInode::submit_regular_file_page_io(
            &block_device,
            &boot_region,
            locked_page,
            page_cache_context.cluster_map.as_ref(),
            page_cache_context.data_length,
            file_offset,
            initialized_sector_len,
            io_batch,
            true,
        )
    }

    fn write_page_async(
        &self,
        idx: usize,
        locked_page: LockedCachePage,
        io_batch: &mut IoBatch,
    ) -> Result<()> {
        let (_inode, page_cache_context, block_device, boot_region, mount_runtime) =
            self.page_cache_context()?;
        if mount_runtime.forced_shutdown
            || mount_runtime.clear_to_zero
            || mount_runtime.media_failure
        {
            return_errno!(Errno::EIO);
        }
        if mount_runtime.read_only {
            return_errno!(Errno::EROFS);
        }
        let (file_offset, initialized_len) = ExfatInode::regular_file_page_range(
            idx,
            page_cache_context.data_length,
            page_cache_context.valid_data_length,
        )?;
        let initialized_sector_len = initialized_len
            .div_ceil(boot_region.sector_size)
            .checked_mul(boot_region.sector_size)
            .ok_or_else(|| Error::new(Errno::EINVAL))?;
        if initialized_sector_len == 0 {
            locked_page.set_up_to_date();
            return Ok(());
        }

        locked_page.wait_until_finish_writing_back();
        locked_page.set_writing_back();
        locked_page.set_up_to_date();

        ExfatInode::submit_regular_file_page_io(
            &block_device,
            &boot_region,
            locked_page,
            page_cache_context.cluster_map.as_ref(),
            page_cache_context.data_length,
            file_offset,
            initialized_sector_len,
            io_batch,
            false,
        )
    }
}

impl ExfatInode {
    pub(super) fn page_cache_context_for_mapping(
        &self,
        cluster_map: Arc<ClusterMap>,
        data_length: usize,
        valid_data_length: usize,
    ) -> Result<PageCacheContext> {
        let fs = self
            .fs
            .upgrade()
            .ok_or_else(|| Error::with_message(Errno::EIO, "exFAT filesystem is not mounted"))?;
        Ok(PageCacheContext {
            cluster_map,
            data_length,
            mount_runtime: fs.mount_runtime_projection(),
            valid_data_length,
        })
    }

    pub(super) fn install_page_cache_context(
        &self,
        inode_state_guard: &InodeStateWriteGuard<'_>,
        page_cache_context: PageCacheContext,
    ) -> Option<PageCacheContext> {
        inode_state_guard.replace_page_cache_context(page_cache_context)
    }

    pub(super) fn active_page_cache_context(&self) -> Option<PageCacheContext> {
        self.page_cache_context.read().clone()
    }

    pub(super) fn weak_self(&self) -> Weak<Self> {
        self.page_backend.inode_weak()
    }

    pub(super) fn page_cache_handle(&self) -> Option<&PageCache> {
        let metadata = self.inode_state_read_guard().metadata();
        self.page_cache_handle_for_metadata(metadata)
    }

    pub(super) fn page_cache_handle_for_metadata(&self, metadata: Metadata) -> Option<&PageCache> {
        if metadata.type_ != InodeType::File {
            return None;
        }

        self.page_cache
            .call_once(|| {
                let backend: Arc<dyn PageCacheBackend> = self.page_backend.clone();
                let capacity = metadata.size;
                PageCache::new_with_backend(capacity, Arc::downgrade(&backend)).ok()
            })
            .as_ref()
    }
}
