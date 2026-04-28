// SPDX-License-Identifier: MPL-2.0

use super::*;

impl ExfatInode {
    pub(super) fn sync_regular_file(&self, scope: FileSyncScope) -> Result<()> {
        let fs = self
            .fs
            .upgrade()
            .ok_or_else(|| Error::with_message(Errno::EIO, "exFAT filesystem is not mounted"))?;
        let (state_guard, block_device, _boot_region, anomaly, _upcase_table, _options) =
            fs.admitted_lookup_state().map_err(Error::from)?;
        if anomaly.clear_to_zero || anomaly.media_failure {
            return_errno!(Errno::EIO);
        }

        let (owner_guard, _stream, data_length, _valid_data_length) =
            self.admitted_regular_file_stream_snapshot()?;
        let admitted_dirty_state = *self.dirty_state.read();
        let needs_device_sync = scope.needs_device_sync(admitted_dirty_state);
        let page_cache = self
            .page_cache
            .get()
            .and_then(|maybe_page_cache| maybe_page_cache.as_ref());
        drop(owner_guard);
        drop(state_guard);

        let needs_page_writeback = page_cache.is_some_and(|page_cache| {
            data_length != 0 && page_cache.has_dirty_pages(0..data_length)
        });

        if needs_page_writeback {
            if let Some(page_cache) = page_cache {
                page_cache.evict_range(0..data_length)?;
                let (_state_guard, _block_device, _boot_region, anomaly, _upcase_table, _options) =
                    fs.admitted_lookup_state().map_err(Error::from)?;
                if anomaly.clear_to_zero || anomaly.media_failure {
                    return_errno!(Errno::EIO);
                }

                let (_owner_guard, _stream, _data_length, _valid_data_length) =
                    self.admitted_regular_file_stream_snapshot()?;
            }
        }

        if needs_page_writeback || needs_device_sync {
            match block_device.sync()? {
                BioStatus::Complete => {
                    let (_state_guard, _block_device, _boot_region, anomaly, _upcase_table, _options) =
                        fs.admitted_lookup_state().map_err(Error::from)?;
                    if anomaly.clear_to_zero || anomaly.media_failure {
                        return_errno!(Errno::EIO);
                    }

                    let (_owner_guard, _stream, _data_length, _valid_data_length) =
                        self.admitted_regular_file_stream_snapshot()?;
                    let mut dirty_state = self.dirty_state.write();
                    match scope {
                        FileSyncScope::Data => dirty_state.publish_data(admitted_dirty_state),
                        FileSyncScope::All => dirty_state.publish_all(admitted_dirty_state),
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
