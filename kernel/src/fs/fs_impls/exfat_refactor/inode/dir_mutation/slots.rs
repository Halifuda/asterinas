// SPDX-License-Identifier: MPL-2.0

//! Owns directory vacant-slot discovery and slot reservation helpers.
//!
//! Method groups: vacant-slot scan and slot reservation.

use core::ops::Range;

use aster_block::BlockDevice;

use super::super::{ExfatInode, StreamExtensionDirEntry, state::InodeStateWriteGuard};
use crate::{
    fs::fs_impls::exfat_refactor::{
        boot::BootRegion,
        dir_entry_format::{
            self as direntry, DIRECTORY_ENTRY_SIZE, DirEntrySlotRange, MutableDirEntrySlotSpan,
            ScannedDirEntry,
        },
        fs::{AllocGuard, FsState},
        invalid_on_disk_layout, invalid_operation_input,
    },
    prelude::*,
};

impl ExfatInode {
    fn find_vacant_entry_slots(
        is_root_directory: bool,
        directory_bytes: &[u8],
        required_entry_count: usize,
    ) -> Result<Option<DirEntrySlotRange>> {
        if required_entry_count == 0 {
            return Err(invalid_operation_input());
        }
        if !directory_bytes.len().is_multiple_of(DIRECTORY_ENTRY_SIZE) {
            return Err(invalid_on_disk_layout());
        }

        let total_entries = directory_bytes.len() / DIRECTORY_ENTRY_SIZE;
        let mut run_length = 0usize;
        let mut run_start_index = 0usize;
        let mut entry_index = 0usize;
        loop {
            let scan_start_index = entry_index;
            match direntry::scan_dir_entry(is_root_directory, directory_bytes, entry_index)? {
                ScannedDirEntry::EndOfDirectory { entry_index } => {
                    if entry_index != scan_start_index {
                        run_length = 0;
                        run_start_index = entry_index;
                    }
                    let available_entries = total_entries
                        .checked_sub(entry_index)
                        .ok_or(invalid_on_disk_layout())?;
                    if run_length == 0 {
                        run_start_index = entry_index;
                    }
                    run_length = run_length
                        .checked_add(available_entries)
                        .ok_or(invalid_on_disk_layout())?;
                    if run_length >= required_entry_count {
                        return Ok(Some(DirEntrySlotRange::new(
                            run_start_index,
                            required_entry_count,
                        )?));
                    }
                    return Ok(None);
                }
                ScannedDirEntry::Vacant(slot_range) => {
                    if run_length == 0 || slot_range.first_entry_index() != scan_start_index {
                        run_start_index = slot_range.first_entry_index();
                        run_length = 0;
                    }
                    run_length = run_length
                        .checked_add(slot_range.entry_count())
                        .ok_or(invalid_on_disk_layout())?;
                    if run_length >= required_entry_count {
                        return Ok(Some(DirEntrySlotRange::new(
                            run_start_index,
                            required_entry_count,
                        )?));
                    }
                    entry_index = slot_range.next_entry_index()?;
                }
                ScannedDirEntry::File(entry_view) => {
                    run_length = 0;
                    entry_index = entry_view.slot_range().next_entry_index()?;
                }
                ScannedDirEntry::Issue { .. } => {
                    return Err(invalid_on_disk_layout());
                }
            }
        }
    }

    pub(super) fn reserve_directory_entry_slots(
        &self,
        mut cluster_map: StreamExtensionDirEntry,
        allocation_guard: &mut AllocGuard<'_>,
        fs_state: &mut FsState,
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        parent_inode_state_guard: Option<&InodeStateWriteGuard<'_>>,
        self_inode_state_guard: &InodeStateWriteGuard<'_>,
        required_entry_count: usize,
    ) -> Result<(StreamExtensionDirEntry, Vec<u8>, DirEntrySlotRange)> {
        loop {
            let cluster_map_generation = self.cluster_map_for_write_guard(
                self_inode_state_guard,
                allocation_guard,
                cluster_map,
            )?;
            let logical_end = match cluster_map.data_length {
                Some(data_length) => data_length,
                None => cluster_map_generation.allocated_byte_length(boot_region)?,
            };
            let directory_bytes = self.read_directory_snapshot_from_page_cache(
                self_inode_state_guard.metadata(),
                cluster_map_generation,
                logical_end,
            )?;
            if let Some(slot_range) = Self::find_vacant_entry_slots(
                cluster_map.data_length.is_none(),
                &directory_bytes,
                required_entry_count,
            )? {
                return Ok((cluster_map, directory_bytes, slot_range));
            }
            cluster_map = self.grow_directory_cluster_map(
                cluster_map,
                allocation_guard,
                fs_state,
                block_device,
                boot_region,
                parent_inode_state_guard,
                self_inode_state_guard,
            )?;
        }
    }

    pub(super) fn reserve_rename_destination_slot(
        &self,
        cluster_map: StreamExtensionDirEntry,
        current_directory_bytes: Vec<u8>,
        reusable_slot_range: Option<DirEntrySlotRange>,
        fs_state: &mut FsState,
        allocation_guard: &mut AllocGuard<'_>,
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        parent_inode_state_guard: Option<&InodeStateWriteGuard<'_>>,
        self_inode_state_guard: &InodeStateWriteGuard<'_>,
        required_entry_count: usize,
    ) -> Result<(StreamExtensionDirEntry, Vec<u8>, DirEntrySlotRange, bool)> {
        if let Some(slot_range) = reusable_slot_range
            .filter(|slot_range| slot_range.entry_count() >= required_entry_count)
        {
            return Ok((cluster_map, current_directory_bytes, slot_range, false));
        }
        let (updated_cluster_map, directory_bytes, slot_range) = self
            .reserve_directory_entry_slots(
                cluster_map,
                allocation_guard,
                fs_state,
                block_device,
                boot_region,
                parent_inode_state_guard,
                self_inode_state_guard,
                required_entry_count,
            )?;
        Ok((updated_cluster_map, directory_bytes, slot_range, true))
    }

    pub(super) fn prepare_invalidated_slot_mutation(
        directory_bytes: &[u8],
        slot_range: DirEntrySlotRange,
    ) -> Result<(Range<usize>, Vec<u8>, Vec<u8>)> {
        let byte_range = direntry::slot_range_bytes(slot_range)?;
        let old_bytes = directory_bytes
            .get(byte_range.clone())
            .ok_or_else(invalid_on_disk_layout)?
            .to_vec();
        let mut new_bytes = old_bytes.clone();
        let mut invalidated_entry_set =
            MutableDirEntrySlotSpan::new(slot_range, new_bytes.as_mut_slice())?;
        direntry::invalidate_entry_set(&mut invalidated_entry_set)?;
        Ok((byte_range, old_bytes, new_bytes))
    }

    pub(super) fn prepare_renamed_slot_mutation(
        directory_bytes: &[u8],
        destination_slot_range: DirEntrySlotRange,
        renamed_entry_set: &[u8],
    ) -> Result<(Range<usize>, Vec<u8>, Vec<u8>)> {
        let (byte_range, old_bytes, mut new_bytes) =
            Self::prepare_invalidated_slot_mutation(directory_bytes, destination_slot_range)?;
        new_bytes
            .get_mut(..renamed_entry_set.len())
            .ok_or_else(invalid_on_disk_layout)?
            .copy_from_slice(renamed_entry_set);
        Ok((byte_range, old_bytes, new_bytes))
    }
}
