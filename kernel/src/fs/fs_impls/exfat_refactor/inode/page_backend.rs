// SPDX-License-Identifier: MPL-2.0

use aster_block::{
    BlockDevice,
    bio::{BioType, BioWaiter},
};
use ostd::mm::io::util::HasVmReaderWriter;

use super::{super::boot::BootRegion, ExfatInode, ExfatInodeStream};
use crate::{
    fs::{
        file::InodeType,
        vfs::{
            file_system::FsFlags,
            inode::Inode,
            page_cache::{CachePage, PageCache, PageCacheBackend},
        },
    },
    prelude::*,
    vm::vmo::Vmo,
};

pub(super) struct ExfatFilePageBackend {
    inode: Arc<ExfatInode>,
}

impl ExfatFilePageBackend {
    pub(super) fn new(inode: Arc<ExfatInode>) -> Self {
        Self { inode }
    }
}

impl PageCacheBackend for ExfatFilePageBackend {
    fn read_page_async(&self, idx: usize, frame: &CachePage) -> Result<BioWaiter> {
        let fs =
            self.inode.fs.upgrade().ok_or_else(|| {
                Error::with_message(Errno::EIO, "exFAT filesystem is not mounted")
            })?;
        let admission = fs.published_lookup_state().map_err(Error::from)?;
        if admission.anomaly.clear_to_zero || admission.anomaly.media_failure {
            return_errno!(Errno::EIO);
        }

        let (_owner_guard, stream, data_length, valid_data_length) =
            self.inode.admitted_regular_file_stream_snapshot()?;
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
            &stream,
            data_length,
            file_offset,
            initialized_sector_len,
            BioType::Read,
        )
    }

    fn write_page_async(&self, idx: usize, frame: &CachePage) -> Result<BioWaiter> {
        let fs =
            self.inode.fs.upgrade().ok_or_else(|| {
                Error::with_message(Errno::EIO, "exFAT filesystem is not mounted")
            })?;
        let admission = fs.published_lookup_state().map_err(Error::from)?;
        if admission.anomaly.clear_to_zero || admission.anomaly.media_failure {
            return_errno!(Errno::EIO);
        }
        if admission.options.fs_flags.contains(FsFlags::RDONLY) {
            return_errno!(Errno::EROFS);
        }

        let (_owner_guard, stream, data_length, valid_data_length) =
            self.inode.admitted_regular_file_stream_snapshot()?;
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
            &stream,
            data_length,
            file_offset,
            initialized_sector_len,
            BioType::Write,
        )
    }

    fn npages(&self) -> usize {
        self.inode.metadata.read().size.div_ceil(PAGE_SIZE)
    }
}

impl ExfatInode {
    pub(super) fn page_cache_vmo(&self) -> Option<Arc<Vmo>> {
        if self.type_() != InodeType::File {
            return None;
        }

        self.page_cache
            .call_once(|| {
                let this = self.this.upgrade()?;
                let backend: Arc<dyn PageCacheBackend> = Arc::new(ExfatFilePageBackend::new(this));
                let capacity = self.metadata.read().size;
                PageCache::with_capacity(capacity, Arc::downgrade(&backend)).ok()
            })
            .as_ref()
            .map(|page_cache| page_cache.pages().clone())
    }
}
