// SPDX-License-Identifier: MPL-2.0

//! Owns parent-directory entry-set validation, rewrite preparation, and persistence.
//!
//! Method groups: validated entry-set lookup, rewrite preparation, and persistence.

use ostd::mm::VmIo;

use super::{
    super::{
        boot::BootRegion, dir_entry_format as direntry, fs::FsState, invalid_on_disk_layout,
        invalid_operation_input,
    },
    ClusterMap, ExfatInode,
    state::InodeStateWriteGuard,
};
use crate::{
    fs::{file::InodeType, vfs::inode::Metadata},
    prelude::*,
};

pub(super) struct PreparedEntrySetWrite {
    slot_range: direntry::DirEntrySlotRange,
    entry_set_bytes: Vec<u8>,
    old_entry_set_bytes: Vec<u8>,
    page_dirty_states: Vec<(usize, bool)>,
}

impl ExfatInode {
    pub(super) fn read_validated_entry_set(
        &self,
        self_inode_state_guard: &InodeStateWriteGuard<'_>,
        parent_inode_state_guard: &InodeStateWriteGuard<'_>,
        parent_cluster_map_generation: &ClusterMap,
        boot_region: &BootRegion,
    ) -> Result<(direntry::DirEntrySlotRange, Vec<u8>)> {
        let parent_cluster_map = parent_cluster_map_generation.stream_extension();
        if parent_inode_state_guard.dir_entry_stream() != parent_cluster_map {
            return Err(invalid_on_disk_layout());
        }
        let parent_inode = self_inode_state_guard.parent().ok_or_else(|| {
            Error::with_message(Errno::EIO, "ordinary exFAT inode parent is not mounted")
        })?;
        if !parent_inode_state_guard.guards_inode(parent_inode.as_ref()) {
            return Err(Error::new(Errno::EINVAL));
        }
        let logical_end = match parent_cluster_map.data_length {
            Some(data_length) => data_length,
            None => parent_cluster_map_generation.allocated_byte_length(boot_region)?,
        };
        let directory_bytes = parent_inode.read_directory_snapshot_from_page_cache(
            parent_inode_state_guard.metadata(),
            Arc::new(parent_cluster_map_generation.clone()),
            logical_end,
        )?;
        let fallback_entry_index = usize::try_from(self_inode_state_guard.metadata().ino as u32)
            .map_err(|_| Error::new(Errno::EIO))?;

        if let Some(hinted_slot_range) = self.entry_set_location_hint()? {
            let hinted_ino =
                self.entry_location_ino(parent_cluster_map, hinted_slot_range.first_entry_index())?;
            if hinted_ino != self_inode_state_guard.metadata().ino {
                self.clear_entry_set_location_hint();
            } else {
                match self.try_read_validated_entry_set_at(
                    self_inode_state_guard,
                    boot_region,
                    &directory_bytes,
                    hinted_slot_range,
                ) {
                    Ok(Some((validated_slot_range, entry_set_bytes))) => {
                        self.store_entry_set_location_hint(validated_slot_range)?;
                        return Ok((validated_slot_range, entry_set_bytes));
                    }
                    Ok(None) => {
                        self.clear_entry_set_location_hint();
                    }
                    Err(error) if error.error() == Errno::EUCLEAN => {
                        self.clear_entry_set_location_hint();
                    }
                    Err(error) => return Err(error),
                }
            }
        }

        let primary_slot_range = direntry::DirEntrySlotRange::new(fallback_entry_index, 1)?;
        let primary_entry_bytes = directory_bytes
            .get(direntry::slot_range_bytes(primary_slot_range)?)
            .ok_or_else(invalid_on_disk_layout)?
            .to_vec();
        let fallback_slot_range =
            direntry::file_primary_entry_slot_range(fallback_entry_index, &primary_entry_bytes)?;
        let (validated_slot_range, entry_set_bytes) = self
            .try_read_validated_entry_set_at(
                self_inode_state_guard,
                boot_region,
                &directory_bytes,
                fallback_slot_range,
            )?
            .ok_or_else(invalid_on_disk_layout)?;
        self.store_entry_set_location_hint(validated_slot_range)?;
        Ok((validated_slot_range, entry_set_bytes))
    }

    fn try_read_validated_entry_set_at(
        &self,
        self_inode_state_guard: &InodeStateWriteGuard<'_>,
        boot_region: &BootRegion,
        directory_bytes: &[u8],
        slot_range: direntry::DirEntrySlotRange,
    ) -> Result<Option<(direntry::DirEntrySlotRange, Vec<u8>)>> {
        let current_cluster_map = self_inode_state_guard.dir_entry_stream();
        let expected_inode_type = self_inode_state_guard.metadata().type_;
        let allow_stale_regular_file_cluster_map = expected_inode_type == InodeType::File
            && self_inode_state_guard
                .dirty_state()
                .has_deferred_regular_file_publish();
        let entry_set_bytes = directory_bytes
            .get(direntry::slot_range_bytes(slot_range)?)
            .ok_or_else(invalid_on_disk_layout)?
            .to_vec();
        let zero_based_slot_range = direntry::DirEntrySlotRange::new(0, slot_range.entry_count())?;
        let entry_view = match direntry::scan_dir_entry(false, &entry_set_bytes, 0) {
            Ok(direntry::ScannedDirEntry::File(entry_view))
                if entry_view.slot_range() == zero_based_slot_range =>
            {
                entry_view
            }
            Ok(_) => return Ok(None),
            Err(error) if error.error() == Errno::EUCLEAN => return Err(error),
            Err(error) => return Err(error),
        };
        let (inode_type, _first_cluster, _data_length, _no_fat_chain) =
            entry_view.child_metadata(boot_region)?;
        match expected_inode_type {
            InodeType::Dir => {
                if inode_type != InodeType::Dir || !entry_view.is_directory() {
                    return Ok(None);
                }
            }
            InodeType::File => {
                if inode_type != InodeType::File || entry_view.is_directory() {
                    return Ok(None);
                }
            }
            _ => {
                return Err(invalid_on_disk_layout());
            }
        }
        let validated_cluster_map = entry_view.cluster_map()?;
        if !allow_stale_regular_file_cluster_map && validated_cluster_map != current_cluster_map {
            return Ok(None);
        }
        let validated_slot_range = direntry::DirEntrySlotRange::new(
            slot_range.first_entry_index(),
            entry_view.slot_range().entry_count(),
        )?;
        if validated_slot_range != slot_range {
            return Ok(None);
        }
        Ok(Some((validated_slot_range, entry_set_bytes)))
    }

    pub(super) fn rewrite_validated_entry_set_with_guard_classified(
        &self,
        fs_state: &mut FsState,
        self_inode_state_guard: &InodeStateWriteGuard<'_>,
        parent_inode_state_guard: &InodeStateWriteGuard<'_>,
        boot_region: &BootRegion,
        rewrite_entry_set_fn: impl FnOnce(direntry::FileEntrySetView<'_>) -> Result<Option<Vec<u8>>>,
        allow_not_exposed_rollback: bool,
    ) -> Result<Result<bool>> {
        let Some(prepared_entry_set_write) = self.prepare_rewritten_entry_set_write_with_guard(
            self_inode_state_guard,
            parent_inode_state_guard,
            boot_region,
            rewrite_entry_set_fn,
        )?
        else {
            return Ok(Ok(false));
        };
        let parent_inode = self_inode_state_guard.parent().ok_or_else(|| {
            Error::with_message(Errno::EIO, "ordinary exFAT inode parent is not mounted")
        })?;
        if !parent_inode_state_guard.guards_inode(parent_inode.as_ref()) {
            return Err(Error::new(Errno::EINVAL));
        }
        self.persist_prepared_entry_set_write_classified(
            fs_state,
            prepared_entry_set_write,
            parent_inode.as_ref(),
            parent_inode_state_guard.metadata(),
            allow_not_exposed_rollback,
        )
    }

    pub(super) fn prepare_rewritten_entry_set_write_with_guard(
        &self,
        self_inode_state_guard: &InodeStateWriteGuard<'_>,
        parent_inode_state_guard: &InodeStateWriteGuard<'_>,
        boot_region: &BootRegion,
        rewrite_entry_set_fn: impl FnOnce(direntry::FileEntrySetView<'_>) -> Result<Option<Vec<u8>>>,
    ) -> Result<Option<PreparedEntrySetWrite>> {
        let parent_cluster_map = parent_inode_state_guard.dir_entry_stream();
        let parent_cluster_map_generation = parent_inode_state_guard
            .cached_cluster_map()
            .filter(|generation| generation.stream_extension() == parent_cluster_map)
            .ok_or_else(invalid_on_disk_layout)?;
        let (slot_range, mut entry_set_bytes) = self.read_validated_entry_set(
            self_inode_state_guard,
            parent_inode_state_guard,
            &parent_cluster_map_generation,
            boot_region,
        )?;
        let entry_view = match direntry::scan_dir_entry(false, &entry_set_bytes, 0)? {
            direntry::ScannedDirEntry::File(entry_view) => entry_view,
            _ => return Err(invalid_on_disk_layout()),
        };
        if entry_view.slot_range().entry_count() != slot_range.entry_count() {
            return Err(invalid_on_disk_layout());
        }

        let Some(updated_entry_set_bytes) = rewrite_entry_set_fn(entry_view)? else {
            return Ok(None);
        };
        if updated_entry_set_bytes.len() != entry_set_bytes.len() {
            return Err(invalid_on_disk_layout());
        }
        let old_entry_set_bytes = entry_set_bytes.clone();
        entry_set_bytes.copy_from_slice(&updated_entry_set_bytes);

        let slot_byte_range = direntry::slot_range_bytes(slot_range)?;
        let parent_inode = self_inode_state_guard.parent().ok_or_else(|| {
            Error::with_message(Errno::EIO, "ordinary exFAT inode parent is not mounted")
        })?;
        let parent_metadata = parent_inode_state_guard.metadata();
        let page_cache = parent_inode
            .page_cache_handle(parent_metadata)
            .cloned()
            .ok_or_else(|| {
                Error::with_message(Errno::EIO, "directory exFAT inode has no page cache")
            })?;
        let mut prefaulted_old_bytes = vec![0; slot_byte_range.len()];
        let mut writer = VmWriter::from(prefaulted_old_bytes.as_mut_slice()).to_fallible();
        page_cache
            .read(slot_byte_range.start, &mut writer)
            .map_err(Error::from)?;
        if prefaulted_old_bytes != old_entry_set_bytes {
            return Err(invalid_operation_input());
        }
        let start_page = slot_byte_range.start / PAGE_SIZE;
        let end_page = (slot_byte_range.end - 1) / PAGE_SIZE;
        let page_dirty_states = (start_page..=end_page)
            .map(|page_idx| {
                let page_start = page_idx
                    .checked_mul(PAGE_SIZE)
                    .ok_or_else(invalid_operation_input)?;
                let page_end = page_start.saturating_add(PAGE_SIZE).min(page_cache.size());
                Ok((page_idx, page_cache.has_dirty_pages(page_start..page_end)))
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Some(PreparedEntrySetWrite {
            slot_range,
            entry_set_bytes,
            old_entry_set_bytes,
            page_dirty_states,
        }))
    }

    pub(super) fn persist_prepared_entry_set_write_classified(
        &self,
        fs_state: &mut FsState,
        prepared_entry_set_write: PreparedEntrySetWrite,
        parent_inode: &ExfatInode,
        parent_metadata: Metadata,
        allow_not_exposed_rollback: bool,
    ) -> Result<Result<bool>> {
        let PreparedEntrySetWrite {
            slot_range,
            entry_set_bytes,
            old_entry_set_bytes,
            page_dirty_states,
        } = prepared_entry_set_write;
        let slot_byte_range = direntry::slot_range_bytes(slot_range)?;
        let page_cache = parent_inode
            .page_cache_handle(parent_metadata)
            .cloned()
            .ok_or_else(|| {
                Error::with_message(Errno::EIO, "directory exFAT inode has no page cache")
            })?;
        let apply_result = {
            let mut reader = VmReader::from(entry_set_bytes.as_slice()).to_fallible();
            page_cache
                .write(slot_byte_range.start, &mut reader)
                .map_err(Error::from)
        };
        let mut persist_error = None;
        if let Err(error) = apply_result {
            if allow_not_exposed_rollback {
                let start_page = slot_byte_range.start / PAGE_SIZE;
                let end_page = (slot_byte_range.end - 1) / PAGE_SIZE;
                let mut old_byte_offset = 0usize;
                let mut page_restores = Vec::new();
                for page_idx in start_page..=end_page {
                    let page_start = page_idx
                        .checked_mul(PAGE_SIZE)
                        .ok_or_else(invalid_operation_input)?;
                    let page_end = page_start.saturating_add(PAGE_SIZE);
                    let segment_start = slot_byte_range.start.max(page_start);
                    let segment_end = slot_byte_range.end.min(page_end);
                    let segment_len = segment_end
                        .checked_sub(segment_start)
                        .ok_or_else(invalid_operation_input)?;
                    let was_dirty = page_dirty_states
                        .iter()
                        .find_map(|(captured_page_idx, was_dirty)| {
                            (*captured_page_idx == page_idx).then_some(*was_dirty)
                        })
                        .ok_or_else(invalid_operation_input)?;
                    let old_byte_end = old_byte_offset
                        .checked_add(segment_len)
                        .ok_or_else(invalid_operation_input)?;
                    page_restores.push((
                        page_idx,
                        (segment_start - page_start)..(segment_end - page_start),
                        &old_entry_set_bytes[old_byte_offset..old_byte_end],
                        was_dirty,
                    ));
                    old_byte_offset = old_byte_end;
                }
                match page_cache.restore_prefaulted_pages(page_restores) {
                    Ok(()) => return Err(error),
                    Err(_restore_error) => {
                        let rewrite_result = {
                            let mut reader =
                                VmReader::from(entry_set_bytes.as_slice()).to_fallible();
                            page_cache
                                .write(slot_byte_range.start, &mut reader)
                                .map_err(Error::from)
                        };
                        match rewrite_result {
                            Ok(()) => persist_error = Some(error),
                            Err(_) => {
                                if let Some(fs) = self.fs.upgrade() {
                                    fs.latch_forced_shutdown(fs_state);
                                }
                                return Ok(Err(error));
                            }
                        }
                    }
                }
            } else {
                let rewrite_result = {
                    let mut reader = VmReader::from(entry_set_bytes.as_slice()).to_fallible();
                    page_cache
                        .write(slot_byte_range.start, &mut reader)
                        .map_err(Error::from)
                };
                match rewrite_result {
                    Ok(()) => persist_error = Some(error),
                    Err(_) => {
                        if let Some(fs) = self.fs.upgrade() {
                            fs.latch_forced_shutdown(fs_state);
                        }
                        return Ok(Err(error));
                    }
                }
            }
        }
        let flush_start = (slot_byte_range.start / PAGE_SIZE)
            .checked_mul(PAGE_SIZE)
            .ok_or_else(invalid_operation_input)?;
        let flush_end = ((slot_byte_range.end - 1) / PAGE_SIZE)
            .checked_add(1)
            .and_then(|page_idx| page_idx.checked_mul(PAGE_SIZE))
            .ok_or_else(invalid_operation_input)?
            .min(page_cache.size());
        if let Err(error) = page_cache.flush_range(flush_start..flush_end) {
            persist_error = Some(persist_error.unwrap_or(error));
        }
        if let Some(error) = persist_error {
            let _ = self.store_entry_set_location_hint(slot_range);
            Ok(Err(error))
        } else {
            match self.store_entry_set_location_hint(slot_range) {
                Ok(()) => Ok(Ok(true)),
                Err(error) => Ok(Err(error)),
            }
        }
    }
}
