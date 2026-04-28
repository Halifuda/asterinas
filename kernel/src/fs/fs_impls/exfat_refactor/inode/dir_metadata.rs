// SPDX-License-Identifier: MPL-2.0

use core::time::Duration;

use aster_block::BlockDevice;

use super::{
    super::{
        boot::BootRegion,
        direntry::{self, FileEntrySetFieldUpdates, FileEntrySetView, ScannedDirectoryEntry},
        fs::ExfatFsError,
    },
    ExfatFs, ExfatInode, InodeRewriteTarget,
};
use crate::{
    fs::{
        file::{InodeType, chmod, mkmod},
        vfs::inode::Metadata,
    },
    prelude::*,
    time::clocks::RealTimeCoarseClock,
};

impl ExfatInode {
    // Read projection

    pub(super) fn directory_metadata_projection(&self) -> Result<Metadata> {
        let metadata = *self.metadata.read();
        if metadata.type_ != InodeType::Dir {
            return Ok(metadata);
        }

        let fs = self
            .fs
            .upgrade()
            .ok_or_else(|| Error::with_message(Errno::EIO, "exFAT filesystem is not mounted"))?;
        let (_state_guard, block_device, boot_region, anomaly, _upcase_table, _options) =
            fs.admitted_lookup_state().map_err(Error::from)?;
        if anomaly.clear_to_zero || anomaly.media_failure {
            return_errno!(Errno::EIO);
        }

        let Some(parent) = self.parent.upgrade() else {
            if self.stream.read().data_length.is_none() {
                return Ok(metadata);
            }
            return Err(Error::with_message(
                Errno::EIO,
                "ordinary exFAT directory parent is not published",
            ));
        };
        let _parent_guard = parent.admission.read();
        let parent_stream = *parent.stream.read();
        let mut metadata = *self.metadata.read();
        let directory_bytes =
            Self::read_directory_bytes_for_stream(&block_device, &boot_region, parent_stream)
                .map_err(Error::from)?;
        let entry_index =
            usize::try_from(metadata.ino as u32).map_err(|_| Error::new(Errno::EIO))?;
        let entry_view = match direntry::scan_directory_entry(
            parent_stream.data_length.is_none(),
            &directory_bytes,
            entry_index,
        )
        .map_err(Error::from)?
        {
            ScannedDirectoryEntry::File(entry_view) => entry_view,
            _ => return Err(Error::from(ExfatFsError::InvalidOnDiskLayout)),
        };
        if entry_view.file_attributes() & direntry::FILE_ATTRIBUTE_DIRECTORY == 0 {
            return Err(Error::from(ExfatFsError::InvalidOnDiskLayout));
        }

        let (inode_type, _first_cluster, data_length, _no_fat_chain) = entry_view
            .child_metadata(&boot_region)
            .map_err(Error::from)?;
        if inode_type != InodeType::Dir {
            return Err(Error::from(ExfatFsError::InvalidOnDiskLayout));
        }

        let entry_set = directory_bytes
            .get(direntry::slot_range_bytes(entry_view.slot_range()).map_err(Error::from)?)
            .ok_or(ExfatFsError::InvalidOnDiskLayout)
            .map_err(Error::from)?;
        let create_timestamp = entry_set
            .get(direntry::CREATE_TIMESTAMP_OFFSET..direntry::CREATE_TIMESTAMP_OFFSET + 4)
            .ok_or(ExfatFsError::InvalidOnDiskLayout)
            .map_err(Error::from)?
            .try_into()
            .map_err(|_| Error::from(ExfatFsError::InvalidOnDiskLayout))?;
        let create_ten_ms_increment = *entry_set
            .get(direntry::CREATE_10MS_INCREMENT_OFFSET)
            .ok_or(ExfatFsError::InvalidOnDiskLayout)
            .map_err(Error::from)?;
        let create_utc_offset = *entry_set
            .get(direntry::CREATE_UTC_OFFSET_OFFSET)
            .ok_or(ExfatFsError::InvalidOnDiskLayout)
            .map_err(Error::from)?;
        let _create_at = Self::decoded_exfat_timestamp(
            create_timestamp,
            Some(create_ten_ms_increment),
            create_utc_offset,
        )
        .map_err(Error::from)?;
        let last_accessed_timestamp = entry_set
            .get(
                direntry::LAST_ACCESSED_TIMESTAMP_OFFSET
                    ..direntry::LAST_ACCESSED_TIMESTAMP_OFFSET + 4,
            )
            .ok_or(ExfatFsError::InvalidOnDiskLayout)
            .map_err(Error::from)?
            .try_into()
            .map_err(|_| Error::from(ExfatFsError::InvalidOnDiskLayout))?;
        let last_accessed_utc_offset = *entry_set
            .get(direntry::LAST_ACCESSED_UTC_OFFSET_OFFSET)
            .ok_or(ExfatFsError::InvalidOnDiskLayout)
            .map_err(Error::from)?;
        let last_access_at =
            Self::decoded_exfat_timestamp(last_accessed_timestamp, None, last_accessed_utc_offset)
                .map_err(Error::from)?;
        let last_modified_timestamp = entry_set
            .get(
                direntry::LAST_MODIFIED_TIMESTAMP_OFFSET
                    ..direntry::LAST_MODIFIED_TIMESTAMP_OFFSET + 4,
            )
            .ok_or(ExfatFsError::InvalidOnDiskLayout)
            .map_err(Error::from)?
            .try_into()
            .map_err(|_| Error::from(ExfatFsError::InvalidOnDiskLayout))?;
        let last_modified_ten_ms_increment = *entry_set
            .get(direntry::LAST_MODIFIED_10MS_INCREMENT_OFFSET)
            .ok_or(ExfatFsError::InvalidOnDiskLayout)
            .map_err(Error::from)?;
        let last_modified_utc_offset = *entry_set
            .get(direntry::LAST_MODIFIED_UTC_OFFSET_OFFSET)
            .ok_or(ExfatFsError::InvalidOnDiskLayout)
            .map_err(Error::from)?;
        let last_modify_at = Self::decoded_exfat_timestamp(
            last_modified_timestamp,
            Some(last_modified_ten_ms_increment),
            last_modified_utc_offset,
        )
        .map_err(Error::from)?;
        let writable_bits = metadata.mode & mkmod!(a+w);
        metadata.mode = chmod!(metadata.mode, a-w);
        if entry_view.file_attributes() & direntry::FILE_ATTRIBUTE_READ_ONLY == 0 {
            metadata.mode |= writable_bits;
        }
        metadata.last_access_at = last_access_at;
        metadata.last_meta_change_at = last_modify_at;
        metadata.last_modify_at = last_modify_at;
        metadata.nr_sectors_allocated =
            Self::regular_file_allocated_sectors(&boot_region, data_length).map_err(Error::from)?;
        metadata.size = data_length;
        Ok(metadata)
    }

    // Directory metadata refresh

    pub(super) fn refresh_directory_metadata_after_namespace_mutation(
        &self,
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        timestamp: Duration,
    ) -> Result<()> {
        if self.metadata.read().type_ != InodeType::Dir {
            return_errno!(Errno::ENOTDIR);
        }

        if self.stream.read().data_length.is_none() {
            let mut metadata = self.metadata.write();
            metadata.last_meta_change_at = timestamp;
            metadata.last_modify_at = timestamp;
            drop(metadata);
            self.mark_metadata_publication_dirty();
            return Ok(());
        }

        let durable_updated = self.rewrite_inode_entry_set(
            InodeRewriteTarget::Directory,
            block_device,
            boot_region,
            |entry_view, source_entry_set| {
                let utc_offset_byte = *source_entry_set
                    .get(direntry::LAST_MODIFIED_UTC_OFFSET_OFFSET)
                    .ok_or(ExfatFsError::InvalidOnDiskLayout)
                    .map_err(Error::from)?;
                let (timestamp_bytes, ten_ms_increment, encoded_utc_offset_byte) =
                    Self::encoded_exfat_timestamp_fields(timestamp, utc_offset_byte)?;
                direntry::republished_entry_set(
                    entry_view,
                    &direntry::FileEntrySetFieldUpdates {
                        last_modified_fields: Some((
                            timestamp_bytes,
                            ten_ms_increment,
                            encoded_utc_offset_byte,
                        )),
                        ..Default::default()
                    },
                )
                .map(Some)
                .map_err(Error::from)
            },
            |metadata| {
                metadata.last_meta_change_at = timestamp;
                metadata.last_modify_at = timestamp;
            },
        )?;
        if durable_updated {
            self.mark_metadata_publication_dirty();
        }
        Ok(())
    }

    // Write helpers

    pub(super) fn rewrite_inode_entry_set(
        &self,
        target: InodeRewriteTarget,
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        rewrite_entry_set_fn: impl FnOnce(FileEntrySetView<'_>, &[u8]) -> Result<Option<Vec<u8>>>,
        update_metadata_fn: impl FnOnce(&mut Metadata),
    ) -> Result<bool> {
        let parent = self.parent.upgrade().ok_or_else(|| {
            Error::with_message(
                Errno::EIO,
                "ordinary exFAT directory parent is not published",
            )
        })?;
        let _directory_guards = match target {
            InodeRewriteTarget::Directory => Some(Self::ordered_directory_write_guards(vec![
                self,
                parent.as_ref(),
            ])),
            InodeRewriteTarget::RegularFile => None,
        };
        let _parent_guard = match target {
            InodeRewriteTarget::Directory => None,
            InodeRewriteTarget::RegularFile => Some(parent.admission.write()),
        };
        let parent_stream = *parent.stream.read();
        let mut directory_bytes =
            Self::read_directory_bytes_for_stream(block_device, boot_region, parent_stream)
                .map_err(Error::from)?;
        let entry_index =
            usize::try_from(self.metadata.read().ino as u32).map_err(|_| Error::new(Errno::EIO))?;
        let entry_view = match direntry::scan_directory_entry(
            parent_stream.data_length.is_none(),
            &directory_bytes,
            entry_index,
        )
        .map_err(Error::from)?
        {
            ScannedDirectoryEntry::File(entry_view) => entry_view,
            _ => return Err(Error::from(ExfatFsError::InvalidOnDiskLayout)),
        };
        let (inode_type, _first_cluster, _data_length, _no_fat_chain) = entry_view
            .child_metadata(boot_region)
            .map_err(Error::from)?;
        match target {
            InodeRewriteTarget::Directory => {
                if entry_view.file_attributes() & direntry::FILE_ATTRIBUTE_DIRECTORY == 0
                    || inode_type != InodeType::Dir
                {
                    return Err(Error::from(ExfatFsError::InvalidOnDiskLayout));
                }
            }
            InodeRewriteTarget::RegularFile => {
                if entry_view.file_attributes() & direntry::FILE_ATTRIBUTE_DIRECTORY != 0
                    || inode_type != InodeType::File
                {
                    return Err(Error::from(ExfatFsError::InvalidOnDiskLayout));
                }
            }
        }

        let slot_range_bytes =
            direntry::slot_range_bytes(entry_view.slot_range()).map_err(Error::from)?;
        let source_entry_set = directory_bytes
            .get(slot_range_bytes.clone())
            .ok_or(ExfatFsError::InvalidOnDiskLayout)
            .map_err(Error::from)?;
        let Some(republished_entry_set) = rewrite_entry_set_fn(entry_view, source_entry_set)?
        else {
            return Ok(false);
        };
        let destination_entry_set = directory_bytes
            .get_mut(slot_range_bytes)
            .ok_or(ExfatFsError::InvalidOnDiskLayout)
            .map_err(Error::from)?;
        destination_entry_set.copy_from_slice(&republished_entry_set);
        Self::write_directory_bytes_for_stream(
            block_device,
            boot_region,
            &directory_bytes,
            parent_stream,
        )
        .map_err(Error::from)?;
        let mut metadata = self.metadata.write();
        update_metadata_fn(&mut metadata);
        Ok(true)
    }
}
