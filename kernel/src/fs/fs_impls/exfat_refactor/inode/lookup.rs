// SPDX-License-Identifier: MPL-2.0

use alloc::string::String;

use aster_block::BlockDevice;

use super::{
    super::{
        boot::BootRegion,
        direntry::{
            self, DIRECTORY_ENTRY_SIZE, DirectoryEntryAnomalyKind, DirectoryEntrySlotRange,
            FileEntrySetView, ScannedDirectoryEntry,
        },
        fs::ExfatFsError,
    },
    ExfatFs, ExfatInode, UpcaseTable,
};
use crate::{
    fs::{
        file::{InodeType, chmod},
        utils::DirentVisitor,
        vfs::inode::Inode,
    },
    prelude::*,
};

impl ExfatInode {
    pub(super) fn lookup_child_by_name(
        &self,
        fs: &Arc<ExfatFs>,
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        upcase_table: &UpcaseTable,
        lookup_name: &[u16],
        lookup_name_hash: u16,
    ) -> core::result::Result<Option<Arc<dyn Inode>>, ExfatFsError> {
        let (_owner_guard, stream) = self.admitted_directory_snapshot()?;
        let directory_bytes =
            Self::read_directory_bytes_for_stream(block_device, boot_region, stream)?;
        let Some(entry_view) = Self::locate_named_child_view(
            &directory_bytes,
            stream.data_length.is_none(),
            upcase_table,
            lookup_name,
            lookup_name_hash,
        )?
        else {
            return Ok(None);
        };
        let slot_range = entry_view.slot_range();
        let (inode_type, first_cluster, data_length, no_fat_chain) =
            entry_view.child_metadata(boot_region)?;
        let ino = (u64::from(stream.first_cluster) << 32)
            | u64::from(
                u32::try_from(slot_range.first_entry_index())
                    .map_err(|_| ExfatFsError::InvalidOnDiskLayout)?,
            );
        let entry_set = directory_bytes
            .get(direntry::slot_range_bytes(slot_range)?)
            .ok_or(ExfatFsError::InvalidOnDiskLayout)?;
        let stream_entry = entry_set
            .get(DIRECTORY_ENTRY_SIZE..DIRECTORY_ENTRY_SIZE * 2)
            .ok_or(ExfatFsError::InvalidOnDiskLayout)?;
        let valid_data_length = usize::try_from(u64::from_le_bytes([
            stream_entry[8],
            stream_entry[9],
            stream_entry[10],
            stream_entry[11],
            stream_entry[12],
            stream_entry[13],
            stream_entry[14],
            stream_entry[15],
        ]))
        .map_err(|_| ExfatFsError::InvalidOnDiskLayout)?;
        if valid_data_length > data_length {
            return Err(ExfatFsError::InvalidOnDiskLayout);
        }

        let child_inode = Self::new_child(
            fs,
            self.this.clone(),
            ino,
            inode_type,
            boot_region.cluster_size,
            data_length,
            first_cluster,
            data_length,
            valid_data_length,
            no_fat_chain,
        );
        if inode_type == InodeType::File {
            let last_accessed_timestamp = entry_set
                .get(
                    direntry::LAST_ACCESSED_TIMESTAMP_OFFSET
                        ..direntry::LAST_ACCESSED_TIMESTAMP_OFFSET + 4,
                )
                .ok_or(ExfatFsError::InvalidOnDiskLayout)?
                .try_into()
                .map_err(|_| ExfatFsError::InvalidOnDiskLayout)?;
            let last_accessed_utc_offset = *entry_set
                .get(direntry::LAST_ACCESSED_UTC_OFFSET_OFFSET)
                .ok_or(ExfatFsError::InvalidOnDiskLayout)?;
            let last_access_at = Self::decoded_exfat_timestamp(
                last_accessed_timestamp,
                None,
                last_accessed_utc_offset,
            )?;
            let last_modified_timestamp = entry_set
                .get(
                    direntry::LAST_MODIFIED_TIMESTAMP_OFFSET
                        ..direntry::LAST_MODIFIED_TIMESTAMP_OFFSET + 4,
                )
                .ok_or(ExfatFsError::InvalidOnDiskLayout)?
                .try_into()
                .map_err(|_| ExfatFsError::InvalidOnDiskLayout)?;
            let last_modified_ten_ms_increment = *entry_set
                .get(direntry::LAST_MODIFIED_10MS_INCREMENT_OFFSET)
                .ok_or(ExfatFsError::InvalidOnDiskLayout)?;
            let last_modified_utc_offset = *entry_set
                .get(direntry::LAST_MODIFIED_UTC_OFFSET_OFFSET)
                .ok_or(ExfatFsError::InvalidOnDiskLayout)?;
            let last_modify_at = Self::decoded_exfat_timestamp(
                last_modified_timestamp,
                Some(last_modified_ten_ms_increment),
                last_modified_utc_offset,
            )?;
            let allocated_sectors = Self::regular_file_allocated_sectors(boot_region, data_length)?;
            let mut metadata = child_inode.metadata.write();
            if entry_view.file_attributes() & direntry::FILE_ATTRIBUTE_READ_ONLY != 0 {
                metadata.mode = chmod!(metadata.mode, a-w);
            }
            metadata.last_access_at = last_access_at;
            metadata.last_meta_change_at = last_modify_at;
            metadata.last_modify_at = last_modify_at;
            metadata.nr_sectors_allocated = allocated_sectors;
            metadata.size = data_length;
        }
        let child_inode: Arc<dyn Inode> = child_inode;
        Ok(Some(child_inode))
    }

    pub(super) fn locate_named_child(
        directory_bytes: &[u8],
        is_root_directory: bool,
        boot_region: &BootRegion,
        upcase_table: &UpcaseTable,
        lookup_name: &[u16],
        lookup_name_hash: u16,
    ) -> core::result::Result<
        Option<(DirectoryEntrySlotRange, InodeType, u32, usize, usize, bool)>,
        ExfatFsError,
    > {
        let Some(entry_view) = Self::locate_named_child_view(
            directory_bytes,
            is_root_directory,
            upcase_table,
            lookup_name,
            lookup_name_hash,
        )?
        else {
            return Ok(None);
        };
        let slot_range = entry_view.slot_range();
        let (inode_type, first_cluster, data_length, no_fat_chain) =
            entry_view.child_metadata(boot_region)?;
        let entry_set = directory_bytes
            .get(direntry::slot_range_bytes(slot_range)?)
            .ok_or(ExfatFsError::InvalidOnDiskLayout)?;
        let stream_entry = entry_set
            .get(DIRECTORY_ENTRY_SIZE..DIRECTORY_ENTRY_SIZE * 2)
            .ok_or(ExfatFsError::InvalidOnDiskLayout)?;
        let valid_data_length = usize::try_from(u64::from_le_bytes([
            stream_entry[8],
            stream_entry[9],
            stream_entry[10],
            stream_entry[11],
            stream_entry[12],
            stream_entry[13],
            stream_entry[14],
            stream_entry[15],
        ]))
        .map_err(|_| ExfatFsError::InvalidOnDiskLayout)?;
        if valid_data_length > data_length {
            return Err(ExfatFsError::InvalidOnDiskLayout);
        }
        Ok(Some((
            slot_range,
            inode_type,
            first_cluster,
            data_length,
            valid_data_length,
            no_fat_chain,
        )))
    }

    pub(super) fn locate_named_child_view<'a>(
        directory_bytes: &'a [u8],
        is_root_directory: bool,
        upcase_table: &UpcaseTable,
        lookup_name: &[u16],
        lookup_name_hash: u16,
    ) -> core::result::Result<Option<FileEntrySetView<'a>>, ExfatFsError> {
        let mut entry_index = 0usize;
        loop {
            match direntry::scan_directory_entry(is_root_directory, directory_bytes, entry_index)? {
                ScannedDirectoryEntry::EndOfDirectory { .. } => return Ok(None),
                ScannedDirectoryEntry::Vacant(slot_range) => {
                    entry_index = slot_range.next_entry_index()?;
                }
                ScannedDirectoryEntry::File(entry_view) => {
                    let candidate_name = entry_view.name()?;
                    if entry_view.stored_name_hash() == lookup_name_hash
                        && upcase_table.names_equal(lookup_name, &candidate_name)
                    {
                        return Ok(Some(entry_view));
                    }
                    entry_index = entry_view.slot_range().next_entry_index()?;
                }
                ScannedDirectoryEntry::Anomaly { kind, slot_range } => {
                    if kind == DirectoryEntryAnomalyKind::BenignUnrecognizedEntrySet {
                        entry_index = slot_range.next_entry_index()?;
                        continue;
                    }
                    return Err(ExfatFsError::InvalidOnDiskLayout);
                }
            }
        }
    }

    pub(super) fn readdir_at_impl(
        &self,
        offset: usize,
        visitor: &mut dyn DirentVisitor,
    ) -> Result<usize> {
        if self.type_() != InodeType::Dir {
            return_errno!(Errno::ENOTDIR);
        }

        let fs = self
            .fs
            .upgrade()
            .ok_or_else(|| Error::with_message(Errno::EIO, "exFAT filesystem is not mounted"))?;
        let admission = fs.admitted_lookup_state().map_err(Error::from)?;
        let (_owner_guard, stream) = self.admitted_directory_snapshot().map_err(Error::from)?;
        let directory_bytes = Self::read_directory_bytes_for_stream(
            &admission.block_device,
            &admission.boot_region,
            stream,
        )
        .map_err(Error::from)?;

        let mut next_offset = offset;
        if next_offset == 0 {
            visitor.visit(".", self.ino(), self.type_(), next_offset)?;
            next_offset += 1;
        }
        if next_offset == 1 {
            visitor.visit("..", self.ino(), self.type_(), next_offset)?;
            next_offset += 1;
        }

        let mut visible_offset = 2usize;
        let mut entry_index = 0usize;
        loop {
            match direntry::scan_directory_entry(
                stream.data_length.is_none(),
                &directory_bytes,
                entry_index,
            )? {
                ScannedDirectoryEntry::EndOfDirectory { .. } => break,
                ScannedDirectoryEntry::Vacant(slot_range) => {
                    entry_index = slot_range.next_entry_index()?;
                }
                ScannedDirectoryEntry::File(entry_view) => {
                    let candidate_name = entry_view.name()?;
                    let (inode_type, _, _, _) =
                        entry_view.child_metadata(&admission.boot_region)?;

                    if visible_offset >= offset {
                        let entry_name = String::from_utf16(&candidate_name)
                            .map_err(|_| ExfatFsError::InvalidOnDiskLayout)?;
                        let entry_ino = (u64::from(stream.first_cluster) << 32)
                            | u64::from(
                                u32::try_from(entry_view.slot_range().first_entry_index())
                                    .map_err(|_| ExfatFsError::InvalidOnDiskLayout)?,
                            );
                        visitor.visit(&entry_name, entry_ino, inode_type, visible_offset)?;
                        next_offset = visible_offset
                            .checked_add(1)
                            .ok_or(ExfatFsError::InvalidOnDiskLayout)?;
                    }
                    visible_offset = visible_offset
                        .checked_add(1)
                        .ok_or(ExfatFsError::InvalidOnDiskLayout)?;
                    entry_index = entry_view.slot_range().next_entry_index()?;
                }
                ScannedDirectoryEntry::Anomaly { kind, slot_range } => {
                    if kind == DirectoryEntryAnomalyKind::BenignUnrecognizedEntrySet {
                        entry_index = slot_range.next_entry_index()?;
                        continue;
                    }
                    return Err(ExfatFsError::InvalidOnDiskLayout.into());
                }
            }
        }
        Ok(next_offset.saturating_sub(offset))
    }

    pub(super) fn lookup_impl(&self, name: &str) -> Result<Arc<dyn Inode>> {
        if self.type_() != InodeType::Dir {
            return_errno!(Errno::ENOTDIR);
        }
        if name == "." || name == ".." {
            let inode: Arc<dyn Inode> = self
                .this
                .upgrade()
                .ok_or_else(|| Error::with_message(Errno::EIO, "exFAT inode is not published"))?;
            return Ok(inode);
        }

        let fs = self
            .fs
            .upgrade()
            .ok_or_else(|| Error::with_message(Errno::EIO, "exFAT filesystem is not mounted"))?;
        let admission = fs.admitted_lookup_state().map_err(Error::from)?;

        let lookup_name = Self::admitted_name(name, &admission.options)?;
        let lookup_name_hash = admission.upcase_table.name_hash(&lookup_name);
        let child_inode = self
            .lookup_child_by_name(
                &fs,
                &admission.block_device,
                &admission.boot_region,
                &admission.upcase_table,
                &lookup_name,
                lookup_name_hash,
            )
            .map_err(Error::from)?;
        if let Some(child_inode) = child_inode {
            return Ok(child_inode);
        }

        return_errno!(Errno::ENOENT);
    }
}
