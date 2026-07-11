// SPDX-License-Identifier: MPL-2.0

//! Bridges the exFAT inode owner to the generic page-cache backend.
//!
//! Method groups: callback context publication, page read/write BIO callbacks, page count, and
//! inode page-cache accessors.

use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use aster_block::{
    BlockDevice,
    bio::{BioStatus, BioType},
};
use io_util::batch::IoBatch;
use ostd::{mm::io::util::HasVmReaderWriter, sync::RwMutex};

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
    block_device: Arc<dyn BlockDevice>,
    boot_region: BootRegion,
    pub(super) page_cache_context: RwMutex<Option<PageCacheContext>>,
}

impl ExfatFilePageBackend {
    pub(super) fn new(block_device: Arc<dyn BlockDevice>, boot_region: BootRegion) -> Self {
        Self {
            block_device,
            boot_region,
            page_cache_context: RwMutex::new(None),
        }
    }

    fn active_page_cache_context(&self) -> Result<(PageCacheContext, MountRuntimeState)> {
        let page_cache_context = self
            .page_cache_context
            .read()
            .clone()
            .ok_or_else(|| {
                Error::with_message(
                    Errno::EIO,
                    "regular exFAT file page-cache context is not published",
                )
            })?;
        let mount_runtime = page_cache_context.mount_runtime.snapshot();
        Ok((page_cache_context, mount_runtime))
    }
}

pub(super) struct FragmentedPageIo {
    page: LockedCachePage,
    pending: AtomicUsize,
    failed: AtomicBool,
    bio_type: BioType,
}

impl FragmentedPageIo {
    pub(super) fn new(page: LockedCachePage, pending: usize, bio_type: BioType) -> Arc<Self> {
        Arc::new(Self {
            page,
            pending: AtomicUsize::new(pending),
            failed: AtomicBool::new(false),
            bio_type,
        })
    }

    pub(super) fn complete(self: Arc<Self>, status: BioStatus) {
        self.complete_pending(
            1,
            status != BioStatus::Complete && status != BioStatus::Zeros,
        );
    }

    pub(super) fn fail_unsubmitted(&self, unsubmitted_bios: usize) {
        self.complete_pending(unsubmitted_bios, true);
    }

    fn complete_pending(&self, completed_bios: usize, has_failed: bool) {
        if has_failed {
            self.failed.store(true, Ordering::Release);
        }

        if self.pending.fetch_sub(completed_bios, Ordering::AcqRel) != completed_bios {
            return;
        }

        if matches!(self.bio_type, BioType::Read) {
            if !self.failed.load(Ordering::Acquire) {
                self.page.set_up_to_date();
            }
            return;
        }

        self.page.clear_writing_back();
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
        let (page_cache_context, mount_runtime) = self.active_page_cache_context()?;
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
            .div_ceil(self.boot_region.sector_size)
            .checked_mul(self.boot_region.sector_size)
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
            &self.block_device,
            &self.boot_region,
            locked_page,
            page_cache_context.cluster_map.as_ref(),
            page_cache_context.data_length,
            file_offset,
            initialized_sector_len,
            io_batch,
            BioType::Read,
        )
    }

    fn write_page_async(
        &self,
        idx: usize,
        locked_page: LockedCachePage,
        io_batch: &mut IoBatch,
    ) -> Result<()> {
        let (page_cache_context, mount_runtime) = self.active_page_cache_context()?;
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
            .div_ceil(self.boot_region.sector_size)
            .checked_mul(self.boot_region.sector_size)
            .ok_or_else(|| Error::new(Errno::EINVAL))?;
        if initialized_sector_len == 0 {
            locked_page.set_up_to_date();
            return Ok(());
        }

        locked_page.wait_until_finish_writing_back();
        locked_page.set_writing_back();
        locked_page.set_up_to_date();

        ExfatInode::submit_regular_file_page_io(
            &self.block_device,
            &self.boot_region,
            locked_page,
            page_cache_context.cluster_map.as_ref(),
            page_cache_context.data_length,
            file_offset,
            initialized_sector_len,
            io_batch,
            BioType::Write,
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
        self.page_backend.page_cache_context.read().clone()
    }

    pub(super) fn weak_self(&self) -> Weak<Self> {
        self.weak_self.clone()
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
