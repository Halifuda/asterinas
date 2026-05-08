// SPDX-License-Identifier: MPL-2.0

//! Commits regular-file dirty state through page-cache and block-device synchronization.
//!
//! Method groups: sync-scope classification, pending-sync detection, and VFS sync dispatch.

use aster_block::bio::BioStatus;

use super::{ExfatInode, page_backend::PageCacheContext, state::InodeDirtyState};
use crate::prelude::*;

#[derive(Clone, Copy)]
pub(super) enum InodeSyncScope {
    Data,
    All,
}

impl InodeSyncScope {
    fn needs_device_sync(self, dirty_state: InodeDirtyState) -> bool {
        match self {
            Self::Data => dirty_state.needs_sync_data(),
            Self::All => dirty_state.needs_sync_all(),
        }
    }
}

impl ExfatInode {
    pub(in crate::fs::fs_impls::exfat_refactor) fn has_pending_regular_file_sync(&self) -> bool {
        if self.metadata.read().type_ != crate::fs::file::InodeType::File {
            return false;
        }

        let dirty_state = *self.dirty_state.read();
        if dirty_state.needs_sync_all() {
            return true;
        }

        let Some(data_length) = self.dir_entry_stream.read().data_length else {
            return false;
        };
        self.page_cache
            .get()
            .and_then(|maybe_page_cache| maybe_page_cache.as_ref())
            .is_some_and(|page_cache| {
                data_length != 0 && page_cache.has_dirty_pages(0..data_length)
            })
    }

    pub(super) fn sync_regular_file(&self, scope: InodeSyncScope) -> Result<()> {
        let fs = self
            .fs
            .upgrade()
            .ok_or_else(|| Error::with_message(Errno::EIO, "exFAT filesystem is not mounted"))?;
        let mount_state = fs.mount_state_read_guard()?;
        let block_device = fs.immutable_block_device();
        let boot_region = fs.immutable_boot_region();
        if mount_state.forced_shutdown
            || mount_state.flags.clear_to_zero
            || mount_state.flags.media_failure
        {
            return_errno!(Errno::EIO);
        }

        let page_cache = self
            .page_cache
            .get()
            .and_then(|maybe_page_cache| maybe_page_cache.as_ref());
        let inode_state = self.inode_state.write();
        let cluster_map = self.current_cluster_map(&inode_state)?;
        let (data_length, valid_data_length) = cluster_map.validated_lengths()?;

        let dirty_state_snapshot = *self.dirty_state.read();
        let needs_device_sync = scope.needs_device_sync(dirty_state_snapshot);
        let needs_page_writeback = page_cache.is_some_and(|page_cache| {
            data_length != 0 && page_cache.has_dirty_pages(0..data_length)
        });
        if !needs_page_writeback && !needs_device_sync {
            return Ok(());
        }

        if needs_page_writeback {
            if let Some(page_cache) = page_cache {
                let _page_cache_context = self.install_page_cache_context(
                    &inode_state,
                    PageCacheContext {
                        flags: mount_state.flags,
                        block_device: block_device.clone(),
                        boot_region,
                        cluster_map: cluster_map.clone(),
                        data_length,
                        read_only: mount_state
                            .options
                            .fs_flags
                            .contains(crate::fs::vfs::file_system::FsFlags::RDONLY),
                        valid_data_length,
                    },
                );
                page_cache.evict_range(0..data_length)?;
            }
        }

        if block_device.sync()? != BioStatus::Complete {
            return_errno!(Errno::EIO);
        }

        let mut dirty_state = self.dirty_state.write();
        match scope {
            InodeSyncScope::Data => dirty_state.commit_data(dirty_state_snapshot),
            InodeSyncScope::All => dirty_state.commit_all(dirty_state_snapshot),
        }
        Ok(())
    }
}
