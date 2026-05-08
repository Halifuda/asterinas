// SPDX-License-Identifier: MPL-2.0

//! Projects and mutates inode metadata backed by exFAT file-entry sets.
//!
//! Method groups: cached projection, VFS metadata getters, metadata setters, timestamp rewrite,
//! entry-set rewrite, and directory metadata refresh.

use core::time::Duration;

use aster_block::BlockDevice;

use super::{
    super::{
        boot::BootRegion,
        direntry::{self, FileEntrySetView, FileEntryTimestamp, ScannedDirEntry},
        invalid_on_disk_layout,
    },
    ExfatInode, InodeTimestampField,
};
use crate::{
    fs::{
        file::{InodeMode, InodeType, chmod, mkmod},
        vfs::{file_system::FsFlags, inode::Metadata},
    },
    prelude::*,
    process::{Gid, Uid},
    time::clocks::RealTimeCoarseClock,
};

impl ExfatInode {
    // Read projection

    // ---- meta_read (projection + VFS getters) ----

    pub(super) fn metadata_projection(&self) -> Metadata {
        let metadata = *self.metadata.read();
        if metadata.type_ != InodeType::File {
            return metadata;
        }

        *self.metadata.read()
    }

    pub(super) fn metadata_impl(&self) -> Metadata {
        self.metadata_projection()
    }

    pub(super) fn ino_impl(&self) -> u64 {
        self.metadata_projection().ino
    }

    pub(super) fn type_impl(&self) -> InodeType {
        self.metadata_projection().type_
    }

    pub(super) fn mode_impl(&self) -> Result<InodeMode> {
        Ok(self.metadata_projection().mode)
    }

    pub(super) fn owner_impl(&self) -> Result<Uid> {
        Ok(self.metadata_projection().uid)
    }

    pub(super) fn group_impl(&self) -> Result<Gid> {
        Ok(self.metadata_projection().gid)
    }

    pub(super) fn atime_impl(&self) -> Duration {
        self.metadata_projection().last_access_at
    }

    pub(super) fn mtime_impl(&self) -> Duration {
        self.metadata_projection().last_modify_at
    }

    pub(super) fn ctime_impl(&self) -> Duration {
        self.metadata_projection().last_meta_change_at
    }
}

// ---- meta_write (refresh + setters) ----
impl ExfatInode {
    pub(super) fn refresh_cached_metadata_from_entry_view(
        &self,
        entry_view: FileEntrySetView<'_>,
        boot_region: &BootRegion,
    ) -> Result<()> {
        let (inode_type, _first_cluster, data_length, _no_fat_chain) =
            entry_view.child_metadata(boot_region)?;
        let _create_at = Self::decoded_exfat_timestamp(
            entry_view.create_timestamp().timestamp_bytes(),
            entry_view.create_timestamp().ten_ms_increment(),
            entry_view.create_timestamp().utc_offset_byte(),
        )?;
        let last_access_at = Self::decoded_exfat_timestamp(
            entry_view.last_accessed_timestamp().timestamp_bytes(),
            entry_view.last_accessed_timestamp().ten_ms_increment(),
            entry_view.last_accessed_timestamp().utc_offset_byte(),
        )?;
        let last_modify_at = Self::decoded_exfat_timestamp(
            entry_view.last_modified_timestamp().timestamp_bytes(),
            entry_view.last_modified_timestamp().ten_ms_increment(),
            entry_view.last_modified_timestamp().utc_offset_byte(),
        )?;
        let allocated_sectors = Self::regular_file_allocated_sectors(boot_region, data_length)?;
        let mut metadata = self.metadata.write();
        if metadata.type_ != inode_type {
            return Err(invalid_on_disk_layout());
        }
        let writable_bits = metadata.mode & mkmod!(a+w);
        metadata.mode = chmod!(metadata.mode, a-w);
        if !entry_view.is_read_only() {
            metadata.mode |= writable_bits;
        }
        metadata.last_access_at = last_access_at;
        metadata.last_meta_change_at = last_modify_at;
        metadata.last_modify_at = last_modify_at;
        metadata.nr_sectors_allocated = allocated_sectors;
        metadata.size = data_length;
        Ok(())
    }

    // Write path

    pub(super) fn set_mode_impl(&self, mode: InodeMode) -> Result<()> {
        if self.metadata.read().type_ == InodeType::Dir {
            let fs = self.fs.upgrade().ok_or_else(|| {
                Error::with_message(Errno::EIO, "exFAT filesystem is not mounted")
            })?;
            let mutation_mount_state = fs.mount_state_write_guard()?;
            let block_device = fs.immutable_block_device();
            let boot_region = fs.immutable_boot_region();
            if mutation_mount_state.forced_shutdown
                || mutation_mount_state.flags.clear_to_zero
                || mutation_mount_state.flags.media_failure
            {
                return_errno!(Errno::EIO);
            }
            if mutation_mount_state
                .options
                .fs_flags
                .contains(FsFlags::RDONLY)
            {
                return_errno!(Errno::EROFS);
            }

            let requested_writable = mode.intersects(mkmod!(a+w));
            if self.dir_entry_stream.read().data_length.is_none() {
                let current_writable = self.metadata.read().mode.intersects(mkmod!(a+w));
                if requested_writable == current_writable {
                    return Ok(());
                }
                return_errno!(Errno::EOPNOTSUPP);
            }

            let durable_updated = self.rewrite_inode_entry_set(
                &block_device,
                &boot_region,
                |entry_view| {
                    let current_attributes = entry_view.file_attributes();
                    let current_writable = !entry_view.is_read_only();
                    if requested_writable == current_writable {
                        return Ok(None);
                    }

                    let mut file_attributes =
                        current_attributes | direntry::FILE_ATTRIBUTE_DIRECTORY;
                    if requested_writable {
                        file_attributes &= !direntry::FILE_ATTRIBUTE_READ_ONLY;
                    } else {
                        file_attributes |= direntry::FILE_ATTRIBUTE_READ_ONLY;
                    }
                    let mut mutable_entry_set = entry_view.to_mutable();
                    mutable_entry_set.set_file_attributes(file_attributes);
                    Ok(Some(mutable_entry_set.into_bytes()))
                },
                |metadata| {
                    let writable_bits = metadata.mode & mkmod!(a+w);
                    metadata.mode = chmod!(metadata.mode, a-w);
                    if requested_writable {
                        metadata.mode |= writable_bits;
                    }
                },
            )?;
            if durable_updated {
                let inode_state_guard = self.inode_state.write();
                self.mark_metadata_dirty(&inode_state_guard);
            }
            return Ok(());
        }

        if self.metadata.read().type_ != InodeType::File {
            self.metadata.write().mode = mode;
            return Ok(());
        }

        let fs = self
            .fs
            .upgrade()
            .ok_or_else(|| Error::with_message(Errno::EIO, "exFAT filesystem is not mounted"))?;
        let mutation_mount_state = fs.mount_state_write_guard()?;
        let block_device = fs.immutable_block_device();
        let boot_region = fs.immutable_boot_region();
        if mutation_mount_state.forced_shutdown
            || mutation_mount_state.flags.clear_to_zero
            || mutation_mount_state.flags.media_failure
        {
            return_errno!(Errno::EIO);
        }
        if mutation_mount_state
            .options
            .fs_flags
            .contains(FsFlags::RDONLY)
        {
            return_errno!(Errno::EROFS);
        }

        let requested_writable = mode.intersects(mkmod!(a+w));
        let durable_updated = self.rewrite_inode_entry_set(
            &block_device,
            &boot_region,
            |entry_view| {
                if requested_writable == entry_view.is_read_only() {
                    let mut file_attributes = entry_view.file_attributes();
                    if requested_writable {
                        file_attributes &= !direntry::FILE_ATTRIBUTE_READ_ONLY;
                    } else {
                        file_attributes |= direntry::FILE_ATTRIBUTE_READ_ONLY;
                    }
                    let mut mutable_entry_set = entry_view.to_mutable();
                    mutable_entry_set.set_file_attributes(file_attributes);
                    return Ok(Some(mutable_entry_set.into_bytes()));
                }
                Ok(None)
            },
            |_| {},
        )?;

        let mut metadata = self.metadata.write();
        metadata.mode = chmod!(chmod!(metadata.mode, a-w), u+w);
        if !requested_writable {
            metadata.mode = chmod!(metadata.mode, a-w);
        }
        if durable_updated {
            metadata.last_meta_change_at = RealTimeCoarseClock::get().read_time();
        }
        drop(metadata);
        if durable_updated {
            let inode_state_guard = self.inode_state.write();
            self.mark_metadata_dirty(&inode_state_guard);
        }
        Ok(())
    }

    pub(super) fn set_atime_impl(&self, time: Duration) {
        let inode_type = self.metadata.read().type_;
        match inode_type {
            InodeType::Dir => self.rewrite_timestamp(InodeTimestampField::Accessed, time),
            InodeType::File => self.rewrite_timestamp(InodeTimestampField::Accessed, time),
            _ => self.metadata.write().last_access_at = time,
        }
    }

    pub(super) fn set_mtime_impl(&self, time: Duration) {
        let inode_type = self.metadata.read().type_;
        match inode_type {
            InodeType::Dir => self.rewrite_timestamp(InodeTimestampField::Modified, time),
            InodeType::File => self.rewrite_timestamp(InodeTimestampField::Modified, time),
            _ => self.metadata.write().last_modify_at = time,
        }
    }

    pub(super) fn set_ctime_impl(&self, time: Duration) {
        if self.metadata.read().type_ == InodeType::Dir {
            let Some(fs) = self.fs.upgrade() else {
                return;
            };
            // TODO: Directory `set_ctime()` remains a bounded synthetic no-op until the later
            // `meso_09` mount-policy follow-up gives `ExfatFs` an explicit owner-local
            // `allow_utime` admission path and the broader metadata policy decides whether this
            // VFS-facing request should keep refusing or be absorbed into another durable family.
            let Ok(mutation_mount_state) = fs.mount_state_write_guard() else {
                return;
            };
            if mutation_mount_state.forced_shutdown
                || mutation_mount_state.flags.clear_to_zero
                || mutation_mount_state.flags.media_failure
            {
                return;
            }
            if mutation_mount_state
                .options
                .fs_flags
                .contains(FsFlags::RDONLY)
            {
                return;
            }
            return;
        }

        if self.metadata.read().type_ != InodeType::File {
            self.metadata.write().last_meta_change_at = time;
            return;
        }

        let Some(fs) = self.fs.upgrade() else {
            return;
        };
        // TODO: `set_ctime()` still shares the generic mounted-mutation gate until the later
        // `meso_09` mount-policy follow-up gives `ExfatFs` an explicit owner-local `allow_utime`
        // admission path for timestamp setters. Remove this seam once that dedicated gate exists.
        let Ok(mutation_mount_state) = fs.mount_state_write_guard() else {
            return;
        };
        if mutation_mount_state.forced_shutdown
            || mutation_mount_state.flags.clear_to_zero
            || mutation_mount_state.flags.media_failure
        {
            return;
        }
        if mutation_mount_state
            .options
            .fs_flags
            .contains(FsFlags::RDONLY)
        {
            return;
        }

        let _inode_state_guard = self.inode_state.write();
        self.metadata.write().last_meta_change_at = time;
    }

    pub(super) fn set_owner_impl(&self, uid: Uid) -> Result<()> {
        let inode_type = self.metadata.read().type_;
        if !matches!(inode_type, InodeType::Dir | InodeType::File) {
            self.metadata.write().uid = uid;
            return Ok(());
        }
        self.reject_identity_change(|metadata| metadata.uid == uid)
    }

    pub(super) fn set_group_impl(&self, gid: Gid) -> Result<()> {
        let inode_type = self.metadata.read().type_;
        if !matches!(inode_type, InodeType::Dir | InodeType::File) {
            self.metadata.write().gid = gid;
            return Ok(());
        }
        self.reject_identity_change(|metadata| metadata.gid == gid)
    }

    // Metadata mutation helpers

    pub(super) fn reject_identity_change(
        &self,
        matches_requested_fn: impl FnOnce(&Metadata) -> bool,
    ) -> Result<()> {
        let fs = self
            .fs
            .upgrade()
            .ok_or_else(|| Error::with_message(Errno::EIO, "exFAT filesystem is not mounted"))?;
        let mutation_mount_state = fs.mount_state_write_guard()?;
        if mutation_mount_state.forced_shutdown
            || mutation_mount_state.flags.clear_to_zero
            || mutation_mount_state.flags.media_failure
        {
            return_errno!(Errno::EIO);
        }
        if mutation_mount_state
            .options
            .fs_flags
            .contains(FsFlags::RDONLY)
        {
            return_errno!(Errno::EROFS);
        }

        let _inode_state_guard = self.inode_state.write();
        let metadata = self.metadata.read();
        if matches_requested_fn(&metadata) {
            return Ok(());
        }
        return_errno!(Errno::EPERM);
    }
}

// ---- entry_rewrite (timestamp + directory metadata refresh) ----
impl ExfatInode {
    pub(super) fn rewrite_timestamp(&self, field_kind: InodeTimestampField, time: Duration) {
        let Some(fs) = self.fs.upgrade() else {
            return;
        };
        // TODO: These timestamp setters still admit through the generic mounted-mutation gate and
        // reuse the currently stored exFAT UTC-offset byte because `MountOptions` does not
        // yet own explicit `allow_utime` / timezone policy. Once the later `meso_09`
        // mount-policy follow-up exposes that owner-local policy under `ExfatFs`, remove this
        // seam and route timestamp admission plus UTC-offset selection through that dedicated path.
        let Ok(mutation_mount_state) = fs.mount_state_write_guard() else {
            return;
        };
        let block_device = fs.immutable_block_device();
        let boot_region = fs.immutable_boot_region();
        if mutation_mount_state.forced_shutdown
            || mutation_mount_state.flags.clear_to_zero
            || mutation_mount_state.flags.media_failure
        {
            return;
        }
        if mutation_mount_state
            .options
            .fs_flags
            .contains(FsFlags::RDONLY)
        {
            return;
        }

        if self.metadata.read().type_ == InodeType::Dir
            && self.dir_entry_stream.read().data_length.is_none()
        {
            return;
        }

        if self
            .rewrite_inode_entry_set(
                &block_device,
                &boot_region,
                |entry_view| {
                    let mut mutable_entry_set = entry_view.to_mutable();
                    match field_kind {
                        InodeTimestampField::Accessed => {
                            let (timestamp_bytes, _ten_ms_increment, encoded_utc_offset_byte) =
                                Self::encoded_exfat_timestamp_fields(
                                    time,
                                    entry_view.last_accessed_timestamp().utc_offset_byte(),
                                )?;
                            mutable_entry_set.set_last_accessed_timestamp(FileEntryTimestamp::new(
                                [0, 0, timestamp_bytes[2], timestamp_bytes[3]],
                                None,
                                encoded_utc_offset_byte,
                            ));
                        }
                        InodeTimestampField::Modified => {
                            let (timestamp_bytes, ten_ms_increment, encoded_utc_offset_byte) =
                                Self::encoded_exfat_timestamp_fields(
                                    time,
                                    entry_view.last_modified_timestamp().utc_offset_byte(),
                                )?;
                            mutable_entry_set.set_last_modified_timestamp(FileEntryTimestamp::new(
                                timestamp_bytes,
                                Some(ten_ms_increment),
                                encoded_utc_offset_byte,
                            ));
                        }
                    }
                    Ok(Some(mutable_entry_set.into_bytes()))
                },
                |metadata| match field_kind {
                    InodeTimestampField::Accessed => metadata.last_access_at = time,
                    InodeTimestampField::Modified => {
                        metadata.last_meta_change_at = time;
                        metadata.last_modify_at = time;
                    }
                },
            )
            .is_ok_and(|updated| updated)
        {
            let inode_state_guard = self.inode_state.write();
            self.mark_metadata_dirty(&inode_state_guard);
        }
    }

    pub(super) fn refresh_directory_metadata_after_namespace_mutation(
        &self,
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        timestamp: Duration,
    ) -> Result<()> {
        if self.metadata.read().type_ != InodeType::Dir {
            return_errno!(Errno::ENOTDIR);
        }

        if self.dir_entry_stream.read().data_length.is_none() {
            let mut metadata = self.metadata.write();
            metadata.last_meta_change_at = timestamp;
            metadata.last_modify_at = timestamp;
            drop(metadata);
            let inode_state_guard = self.inode_state.write();
            self.mark_metadata_dirty(&inode_state_guard);
            return Ok(());
        }

        let durable_updated = self.rewrite_inode_entry_set(
            block_device,
            boot_region,
            |entry_view| {
                let (timestamp_bytes, ten_ms_increment, encoded_utc_offset_byte) =
                    Self::encoded_exfat_timestamp_fields(
                        timestamp,
                        entry_view.last_modified_timestamp().utc_offset_byte(),
                    )?;
                let mut mutable_entry_set = entry_view.to_mutable();
                mutable_entry_set.set_last_modified_timestamp(FileEntryTimestamp::new(
                    timestamp_bytes,
                    Some(ten_ms_increment),
                    encoded_utc_offset_byte,
                ));
                Ok(Some(mutable_entry_set.into_bytes()))
            },
            |metadata| {
                metadata.last_meta_change_at = timestamp;
                metadata.last_modify_at = timestamp;
            },
        )?;
        if durable_updated {
            let inode_state_guard = self.inode_state.write();
            self.mark_metadata_dirty(&inode_state_guard);
        }
        Ok(())
    }

    pub(super) fn rewrite_inode_entry_set(
        &self,
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        rewrite_entry_set_fn: impl FnOnce(FileEntrySetView<'_>) -> Result<Option<Vec<u8>>>,
        update_metadata_fn: impl FnOnce(&mut Metadata),
    ) -> Result<bool> {
        let parent = self.parent.upgrade().ok_or_else(|| {
            Error::with_message(Errno::EIO, "ordinary exFAT directory parent is not mounted")
        })?;
        let is_dir = self.metadata.read().type_ == InodeType::Dir;
        let _directory_guards = if is_dir {
            Some(Self::directory_write_guards_by_ino(vec![
                self,
                parent.as_ref(),
            ]))
        } else {
            None
        };
        let _parent_guard = if is_dir {
            None
        } else {
            Some(parent.inode_state.write())
        };
        let parent_cluster_map = *parent.dir_entry_stream.read();
        let mut directory_bytes = Self::read_directory_bytes_for_cluster_map(
            block_device,
            boot_region,
            parent_cluster_map,
        )?;
        let entry_index =
            usize::try_from(self.metadata.read().ino as u32).map_err(|_| Error::new(Errno::EIO))?;
        let entry_view = match direntry::scan_dir_entry(
            parent_cluster_map.data_length.is_none(),
            &directory_bytes,
            entry_index,
        )? {
            ScannedDirEntry::File(entry_view) => entry_view,
            _ => return Err(Error::from(invalid_on_disk_layout())),
        };
        let (inode_type, _first_cluster, _data_length, _no_fat_chain) =
            entry_view.child_metadata(boot_region)?;
        match inode_type {
            InodeType::Dir => {
                if !entry_view.is_directory() {
                    return Err(Error::from(invalid_on_disk_layout()));
                }
            }
            InodeType::File => {
                if entry_view.is_directory() {
                    return Err(Error::from(invalid_on_disk_layout()));
                }
            }
            _ => return Err(Error::from(invalid_on_disk_layout())),
        }

        let slot_range_bytes = direntry::slot_range_bytes(entry_view.slot_range())?;
        let Some(updated_entry_set_bytes) = rewrite_entry_set_fn(entry_view)? else {
            return Ok(false);
        };
        let destination_entry_set = directory_bytes
            .get_mut(slot_range_bytes)
            .ok_or(invalid_on_disk_layout())?;
        destination_entry_set.copy_from_slice(&updated_entry_set_bytes);
        Self::write_directory_bytes_for_cluster_map(
            block_device,
            boot_region,
            &directory_bytes,
            parent_cluster_map,
        )?;
        let mut metadata = self.metadata.write();
        update_metadata_fn(&mut metadata);
        Ok(true)
    }
}
