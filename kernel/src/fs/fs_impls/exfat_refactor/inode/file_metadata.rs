// SPDX-License-Identifier: MPL-2.0

use core::time::Duration;

use aster_block::BlockDevice;

use super::{
    super::{
        boot::BootRegion,
        direntry::{self, FileEntrySetFieldUpdates},
        fs::ExfatFsError,
    },
    ExfatFs, ExfatInode, InodeRewriteTarget, InodeTimestampField,
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

    pub(super) fn metadata_projection(&self) -> Metadata {
        let mut metadata = *self.metadata.read();
        if metadata.type_ == InodeType::Dir {
            return self.directory_metadata_projection().unwrap_or(metadata);
        }
        if metadata.type_ != InodeType::File {
            return metadata;
        }

        let Some(fs) = self.fs.upgrade() else {
            return metadata;
        };
        let Ok(admission) = fs.admitted_lookup_state() else {
            return metadata;
        };
        if admission.anomaly.clear_to_zero || admission.anomaly.media_failure {
            return metadata;
        }

        let Ok((owner_guard, _stream, data_length, _valid_data_length)) =
            self.admitted_regular_file_stream_snapshot()
        else {
            return metadata;
        };
        let Ok(allocated_sectors) =
            Self::regular_file_allocated_sectors(&admission.boot_region, data_length)
        else {
            drop(owner_guard);
            return metadata;
        };
        metadata.size = data_length;
        metadata.nr_sectors_allocated = allocated_sectors;
        drop(owner_guard);
        metadata
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
        if self.metadata.read().type_ == InodeType::Dir {
            return self
                .directory_metadata_projection()
                .map(|metadata| metadata.mode);
        }
        Ok(self.metadata_projection().mode)
    }

    pub(super) fn owner_impl(&self) -> Result<Uid> {
        if self.metadata.read().type_ == InodeType::Dir {
            return self
                .directory_metadata_projection()
                .map(|metadata| metadata.uid);
        }
        Ok(self.metadata_projection().uid)
    }

    pub(super) fn group_impl(&self) -> Result<Gid> {
        if self.metadata.read().type_ == InodeType::Dir {
            return self
                .directory_metadata_projection()
                .map(|metadata| metadata.gid);
        }
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

    // Write path

    pub(super) fn set_mode_impl(&self, mode: InodeMode) -> Result<()> {
        if self.metadata.read().type_ == InodeType::Dir {
            let fs = self.fs.upgrade().ok_or_else(|| {
                Error::with_message(Errno::EIO, "exFAT filesystem is not mounted")
            })?;
            let admission = fs.admitted_mutation_state().map_err(Error::from)?;
            if admission.forced_shutdown
                || admission.anomaly.clear_to_zero
                || admission.anomaly.media_failure
            {
                return_errno!(Errno::EIO);
            }
            if admission.options.fs_flags.contains(FsFlags::RDONLY) {
                return_errno!(Errno::EROFS);
            }

            let requested_writable = mode.intersects(mkmod!(a+w));
            if self.stream.read().data_length.is_none() {
                let current_writable = self.metadata.read().mode.intersects(mkmod!(a+w));
                if requested_writable == current_writable {
                    return Ok(());
                }
                return_errno!(Errno::EOPNOTSUPP);
            }

            let durable_updated = self.rewrite_inode_entry_set(
                InodeRewriteTarget::Directory,
                &admission.block_device,
                &admission.boot_region,
                |entry_view, _source_entry_set| {
                    let current_attributes = entry_view.file_attributes();
                    let current_writable =
                        current_attributes & direntry::FILE_ATTRIBUTE_READ_ONLY == 0;
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
                    direntry::republished_entry_set(
                        entry_view,
                        &direntry::FileEntrySetFieldUpdates {
                            file_attributes: Some(file_attributes),
                            ..Default::default()
                        },
                    )
                    .map(Some)
                    .map_err(Error::from)
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
                self.mark_metadata_publication_dirty();
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
        let admission = fs.admitted_mutation_state().map_err(Error::from)?;
        if admission.forced_shutdown
            || admission.anomaly.clear_to_zero
            || admission.anomaly.media_failure
        {
            return_errno!(Errno::EIO);
        }
        if admission.options.fs_flags.contains(FsFlags::RDONLY) {
            return_errno!(Errno::EROFS);
        }

        let requested_writable = mode.intersects(mkmod!(a+w));
        let durable_updated = self.rewrite_inode_entry_set(
            InodeRewriteTarget::RegularFile,
            &admission.block_device,
            &admission.boot_region,
            |entry_view, _source_entry_set| {
                if requested_writable
                    == (entry_view.file_attributes() & direntry::FILE_ATTRIBUTE_READ_ONLY != 0)
                {
                    let mut file_attributes = entry_view.file_attributes();
                    if requested_writable {
                        file_attributes &= !direntry::FILE_ATTRIBUTE_READ_ONLY;
                    } else {
                        file_attributes |= direntry::FILE_ATTRIBUTE_READ_ONLY;
                    }
                    return direntry::republished_entry_set(
                        entry_view,
                        &direntry::FileEntrySetFieldUpdates {
                            file_attributes: Some(file_attributes),
                            ..Default::default()
                        },
                    )
                    .map(Some)
                    .map_err(Error::from);
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
            self.mark_metadata_publication_dirty();
        }
        Ok(())
    }

    pub(super) fn set_atime_impl(&self, time: Duration) {
        let inode_type = self.metadata.read().type_;
        match inode_type {
            InodeType::Dir => self.rewrite_timestamp(
                InodeRewriteTarget::Directory,
                InodeTimestampField::Accessed,
                time,
            ),
            InodeType::File => self.rewrite_timestamp(
                InodeRewriteTarget::RegularFile,
                InodeTimestampField::Accessed,
                time,
            ),
            _ => self.metadata.write().last_access_at = time,
        }
    }

    pub(super) fn set_mtime_impl(&self, time: Duration) {
        let inode_type = self.metadata.read().type_;
        match inode_type {
            InodeType::Dir => self.rewrite_timestamp(
                InodeRewriteTarget::Directory,
                InodeTimestampField::Modified,
                time,
            ),
            InodeType::File => self.rewrite_timestamp(
                InodeRewriteTarget::RegularFile,
                InodeTimestampField::Modified,
                time,
            ),
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
            let Ok(admission) = fs.admitted_mutation_state().map_err(Error::from) else {
                return;
            };
            if admission.forced_shutdown
                || admission.anomaly.clear_to_zero
                || admission.anomaly.media_failure
            {
                return;
            }
            if admission.options.fs_flags.contains(FsFlags::RDONLY) {
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
        let Ok(admission) = fs.admitted_mutation_state().map_err(Error::from) else {
            return;
        };
        if admission.forced_shutdown
            || admission.anomaly.clear_to_zero
            || admission.anomaly.media_failure
        {
            return;
        }
        if admission.options.fs_flags.contains(FsFlags::RDONLY) {
            return;
        }

        let _owner_guard = self.admission.write();
        self.metadata.write().last_meta_change_at = time;
    }

    pub(super) fn set_owner_impl(&self, uid: Uid) -> Result<()> {
        let inode_type = self.metadata.read().type_;
        if !matches!(inode_type, InodeType::Dir | InodeType::File) {
            self.metadata.write().uid = uid;
            return Ok(());
        }
        self.reject_published_identity_change(|metadata| metadata.uid == uid)
    }

    pub(super) fn set_group_impl(&self, gid: Gid) -> Result<()> {
        let inode_type = self.metadata.read().type_;
        if !matches!(inode_type, InodeType::Dir | InodeType::File) {
            self.metadata.write().gid = gid;
            return Ok(());
        }
        self.reject_published_identity_change(|metadata| metadata.gid == gid)
    }

    // Write helpers

    pub(super) fn reject_published_identity_change(
        &self,
        matches_requested_fn: impl FnOnce(&Metadata) -> bool,
    ) -> Result<()> {
        let fs = self
            .fs
            .upgrade()
            .ok_or_else(|| Error::with_message(Errno::EIO, "exFAT filesystem is not mounted"))?;
        let admission = fs.admitted_mutation_state().map_err(Error::from)?;
        if admission.forced_shutdown
            || admission.anomaly.clear_to_zero
            || admission.anomaly.media_failure
        {
            return_errno!(Errno::EIO);
        }
        if admission.options.fs_flags.contains(FsFlags::RDONLY) {
            return_errno!(Errno::EROFS);
        }

        let _owner_guard = self.admission.write();
        let metadata = self.metadata.read();
        if matches_requested_fn(&metadata) {
            return Ok(());
        }
        return_errno!(Errno::EPERM);
    }

    pub(super) fn rewrite_timestamp(
        &self,
        target: InodeRewriteTarget,
        field_kind: InodeTimestampField,
        time: Duration,
    ) {
        let Some(fs) = self.fs.upgrade() else {
            return;
        };
        // TODO: These timestamp setters still admit through the generic mounted-mutation gate and
        // reuse the currently stored exFAT UTC-offset byte because `ExfatMountOptions` does not
        // yet own explicit `allow_utime` / timezone policy. Once the later `meso_09`
        // mount-policy follow-up publishes that owner-local policy under `ExfatFs`, remove this
        // seam and route timestamp admission plus UTC-offset selection through that dedicated path.
        let Ok(admission) = fs.admitted_mutation_state().map_err(Error::from) else {
            return;
        };
        if admission.forced_shutdown
            || admission.anomaly.clear_to_zero
            || admission.anomaly.media_failure
        {
            return;
        }
        if admission.options.fs_flags.contains(FsFlags::RDONLY) {
            return;
        }

        match target {
            InodeRewriteTarget::Directory => {
                if self.stream.read().data_length.is_none() {
                    return;
                }
                if self
                    .rewrite_inode_entry_set(
                        InodeRewriteTarget::Directory,
                        &admission.block_device,
                        &admission.boot_region,
                        |entry_view, source_entry_set| match field_kind {
                            InodeTimestampField::Accessed => {
                                let utc_offset_byte = *source_entry_set
                                    .get(direntry::LAST_ACCESSED_UTC_OFFSET_OFFSET)
                                    .ok_or(ExfatFsError::InvalidOnDiskLayout)
                                    .map_err(Error::from)?;
                                let (timestamp_bytes, _ten_ms_increment, encoded_utc_offset_byte) =
                                    Self::encoded_exfat_timestamp_fields(time, utc_offset_byte)?;
                                direntry::republished_entry_set(
                                    entry_view,
                                    &direntry::FileEntrySetFieldUpdates {
                                        last_accessed_fields: Some((
                                            [0, 0, timestamp_bytes[2], timestamp_bytes[3]],
                                            encoded_utc_offset_byte,
                                        )),
                                        ..Default::default()
                                    },
                                )
                                .map(Some)
                                .map_err(Error::from)
                            }
                            InodeTimestampField::Modified => {
                                let utc_offset_byte = *source_entry_set
                                    .get(direntry::LAST_MODIFIED_UTC_OFFSET_OFFSET)
                                    .ok_or(ExfatFsError::InvalidOnDiskLayout)
                                    .map_err(Error::from)?;
                                let (timestamp_bytes, ten_ms_increment, encoded_utc_offset_byte) =
                                    Self::encoded_exfat_timestamp_fields(time, utc_offset_byte)?;
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
                            }
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
                    self.mark_metadata_publication_dirty();
                }
            }
            InodeRewriteTarget::RegularFile => {
                if self
                    .rewrite_inode_entry_set(
                        InodeRewriteTarget::RegularFile,
                        &admission.block_device,
                        &admission.boot_region,
                        |entry_view, source_entry_set| match field_kind {
                            InodeTimestampField::Accessed => {
                                let utc_offset_byte = *source_entry_set
                                    .get(direntry::LAST_ACCESSED_UTC_OFFSET_OFFSET)
                                    .ok_or(ExfatFsError::InvalidOnDiskLayout)
                                    .map_err(Error::from)?;
                                let (timestamp_bytes, _ten_ms_increment, encoded_utc_offset_byte) =
                                    Self::encoded_exfat_timestamp_fields(time, utc_offset_byte)?;
                                direntry::republished_entry_set(
                                    entry_view,
                                    &direntry::FileEntrySetFieldUpdates {
                                        last_accessed_fields: Some((
                                            [0, 0, timestamp_bytes[2], timestamp_bytes[3]],
                                            encoded_utc_offset_byte,
                                        )),
                                        ..Default::default()
                                    },
                                )
                                .map(Some)
                                .map_err(Error::from)
                            }
                            InodeTimestampField::Modified => {
                                let utc_offset_byte = *source_entry_set
                                    .get(direntry::LAST_MODIFIED_UTC_OFFSET_OFFSET)
                                    .ok_or(ExfatFsError::InvalidOnDiskLayout)
                                    .map_err(Error::from)?;
                                let (timestamp_bytes, ten_ms_increment, encoded_utc_offset_byte) =
                                    Self::encoded_exfat_timestamp_fields(time, utc_offset_byte)?;
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
                            }
                        },
                        |_| {},
                    )
                    .is_ok()
                {
                    let mut metadata = self.metadata.write();
                    match field_kind {
                        InodeTimestampField::Accessed => metadata.last_access_at = time,
                        InodeTimestampField::Modified => metadata.last_modify_at = time,
                    }
                    metadata.last_meta_change_at = RealTimeCoarseClock::get().read_time();
                    drop(metadata);
                    self.mark_metadata_publication_dirty();
                }
            }
        }
    }
}
