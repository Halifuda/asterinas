// SPDX-License-Identifier: MPL-2.0

//! Commits regular-file dirty state through page-cache and block-device synchronization.
//!
//! Method groups: sync-scope classification, pending-sync detection, and VFS sync dispatch.

use aster_block::bio::BioStatus;

use super::{ExfatInode, state::InodeDirtyState};
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

        let data_length = if let Some(page_cache_context) = self.active_page_cache_context() {
            page_cache_context.data_length
        } else {
            let Some(data_length) = self.dir_entry_stream.read().data_length else {
                return false;
            };
            data_length
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
        let _ = self.current_cluster_map(&inode_state)?;
        let data_length = self
            .active_page_cache_context()
            .map(|page_cache_context| page_cache_context.data_length)
            .ok_or_else(|| Error::new(Errno::EINVAL))?;

        let dirty_state_snapshot = *self.dirty_state.read();
        let needs_device_sync = scope.needs_device_sync(dirty_state_snapshot);
        let needs_regular_file_publish = dirty_state_snapshot.has_deferred_regular_file_publish();
        let needs_page_writeback = page_cache.is_some_and(|page_cache| {
            data_length != 0 && page_cache.has_dirty_pages(0..data_length)
        });
        if !needs_page_writeback && !needs_device_sync {
            return Ok(());
        }

        if needs_page_writeback {
            if let Some(page_cache) = page_cache {
                page_cache.flush_range(0..data_length)?;
            }
        }

        if needs_regular_file_publish {
            fs.flush_dirty_allocation_bitmap()?;
            let parent = self.parent.read().upgrade().ok_or_else(|| {
                Error::with_message(Errno::EIO, "ordinary exFAT directory parent is not mounted")
            })?;
            let parent_inode_state_guard = parent.inode_state.write();
            self.publish_live_regular_file_entry_set(
                &parent_inode_state_guard,
                &block_device,
                &fs.immutable_boot_region(),
            )?;
        }

        if block_device.sync()? != BioStatus::Complete {
            return_errno!(Errno::EIO);
        }
        fs.commit_published_allocation_bitmap()?;

        let mut dirty_state = self.dirty_state.write();
        match scope {
            InodeSyncScope::Data => dirty_state.commit_data(dirty_state_snapshot),
            InodeSyncScope::All => dirty_state.commit_all(dirty_state_snapshot),
        }
        self.clear_dirty_file_retention_if_not_needed(*dirty_state);
        Ok(())
    }
}
