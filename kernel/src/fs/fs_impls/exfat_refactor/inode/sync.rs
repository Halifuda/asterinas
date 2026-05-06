// SPDX-License-Identifier: MPL-2.0

//! Publishes regular-file dirty state through page-cache and block-device synchronization.
//!
//! Method groups: sync-scope classification, pending-sync detection, and VFS sync dispatch.

use aster_block::bio::BioStatus;

use super::{ExfatInode, state::ExfatInodeDirtyState};
use crate::prelude::*;

#[derive(Clone, Copy)]
pub(super) enum InodeSyncScope {
    Data,
    All,
}

impl InodeSyncScope {
    fn needs_device_sync(self, dirty_state: ExfatInodeDirtyState) -> bool {
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

        let Some(data_length) = self.cluster_map.read().data_length else {
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
        let lookup_mount_snapshot = fs.lookup_mount_snapshot()?;
        if lookup_mount_snapshot.anomaly.clear_to_zero
            || lookup_mount_snapshot.anomaly.media_failure
        {
            return_errno!(Errno::EIO);
        }

        let (inode_state_guard, _cluster_map, data_length, _valid_data_length) =
            self.admitted_regular_file_cluster_map_snapshot()?;
        let admitted_dirty_state = *self.dirty_state.read();
        let needs_device_sync = scope.needs_device_sync(admitted_dirty_state);
        let page_cache = self
            .page_cache
            .get()
            .and_then(|maybe_page_cache| maybe_page_cache.as_ref());
        let block_device = lookup_mount_snapshot.block_device.clone();
        drop(inode_state_guard);
        drop(lookup_mount_snapshot);

        let needs_page_writeback = page_cache.is_some_and(|page_cache| {
            data_length != 0 && page_cache.has_dirty_pages(0..data_length)
        });

        if needs_page_writeback {
            if let Some(page_cache) = page_cache {
                page_cache.evict_range(0..data_length)?;
                let lookup_mount_snapshot = fs.lookup_mount_snapshot()?;
                if lookup_mount_snapshot.anomaly.clear_to_zero
                    || lookup_mount_snapshot.anomaly.media_failure
                {
                    return_errno!(Errno::EIO);
                }

                let (_inode_state_guard, _cluster_map, _data_length, _valid_data_length) =
                    self.admitted_regular_file_cluster_map_snapshot()?;
            }
        }

        if needs_page_writeback || needs_device_sync {
            match block_device.sync()? {
                BioStatus::Complete => {
                    let lookup_mount_snapshot = fs.lookup_mount_snapshot()?;
                    if lookup_mount_snapshot.anomaly.clear_to_zero
                        || lookup_mount_snapshot.anomaly.media_failure
                    {
                        return_errno!(Errno::EIO);
                    }

                    let (_inode_state_guard, _cluster_map, _data_length, _valid_data_length) =
                        self.admitted_regular_file_cluster_map_snapshot()?;
                    let mut dirty_state = self.dirty_state.write();
                    match scope {
                        InodeSyncScope::Data => dirty_state.publish_data(admitted_dirty_state),
                        InodeSyncScope::All => dirty_state.publish_all(admitted_dirty_state),
                    }
                    Ok(())
                }
                _ => return_errno!(Errno::EIO),
            }
        } else {
            Ok(())
        }
    }
}
