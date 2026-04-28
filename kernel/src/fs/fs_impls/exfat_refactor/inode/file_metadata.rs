// SPDX-License-Identifier: MPL-2.0

use super::*;

impl ExfatInode {
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
        let Ok((state_guard, _block_device, boot_region, anomaly, _upcase_table, _options)) =
            fs.admitted_lookup_state()
        else {
            return metadata;
        };
        if anomaly.clear_to_zero || anomaly.media_failure {
            drop(state_guard);
            return metadata;
        }

        let Ok((owner_guard, _stream, data_length, _valid_data_length)) =
            self.admitted_regular_file_stream_snapshot()
        else {
            drop(state_guard);
            return metadata;
        };
        let Ok(allocated_sectors) = Self::regular_file_allocated_sectors(&boot_region, data_length)
        else {
            drop(owner_guard);
            drop(state_guard);
            return metadata;
        };
        metadata.size = data_length;
        metadata.nr_sectors_allocated = allocated_sectors;
        drop(owner_guard);
        drop(state_guard);
        metadata
    }

    pub(super) fn reject_published_identity_change(
        &self,
        matches_requested_fn: impl FnOnce(&Metadata) -> bool,
    ) -> Result<()> {
        let fs = self
            .fs
            .upgrade()
            .ok_or_else(|| Error::with_message(Errno::EIO, "exFAT filesystem is not mounted"))?;
        let (_state_guard, _block_device, _boot_region, anomaly, _upcase_table, _options) =
            fs.admitted_mutation_state().map_err(Error::from)?;
        if anomaly.clear_to_zero || anomaly.media_failure {
            return_errno!(Errno::EIO);
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
        target: RewriteTarget,
        field_kind: TimestampFieldKind,
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
        let Ok((_state_guard, block_device, boot_region, anomaly, _upcase_table, _options)) =
            fs.admitted_mutation_state().map_err(Error::from)
        else {
            return;
        };
        if anomaly.clear_to_zero || anomaly.media_failure {
            return;
        }

        match target {
            RewriteTarget::Directory => {
                if self.stream.read().data_length.is_none() {
                    return;
                }
                if self
                    .rewrite_inode_entry_set(
                        RewriteTarget::Directory,
                        &block_device,
                        &boot_region,
                        |entry_view, source_entry_set| match field_kind {
                            TimestampFieldKind::Accessed => {
                                let utc_offset_byte = *source_entry_set
                                    .get(direntry::LAST_ACCESSED_UTC_OFFSET_OFFSET)
                                    .ok_or(MountVolumeStateError::InvalidOnDiskLayout)
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
                            TimestampFieldKind::Modified => {
                                let utc_offset_byte = *source_entry_set
                                    .get(direntry::LAST_MODIFIED_UTC_OFFSET_OFFSET)
                                    .ok_or(MountVolumeStateError::InvalidOnDiskLayout)
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
                            TimestampFieldKind::Accessed => metadata.last_access_at = time,
                            TimestampFieldKind::Modified => {
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
            RewriteTarget::RegularFile => {
                if self
                    .rewrite_inode_entry_set(
                        RewriteTarget::RegularFile,
                        &block_device,
                        &boot_region,
                        |entry_view, source_entry_set| match field_kind {
                            TimestampFieldKind::Accessed => {
                                let utc_offset_byte = *source_entry_set
                                    .get(direntry::LAST_ACCESSED_UTC_OFFSET_OFFSET)
                                    .ok_or(MountVolumeStateError::InvalidOnDiskLayout)
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
                            TimestampFieldKind::Modified => {
                                let utc_offset_byte = *source_entry_set
                                    .get(direntry::LAST_MODIFIED_UTC_OFFSET_OFFSET)
                                    .ok_or(MountVolumeStateError::InvalidOnDiskLayout)
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
                        TimestampFieldKind::Accessed => metadata.last_access_at = time,
                        TimestampFieldKind::Modified => metadata.last_modify_at = time,
                    }
                    metadata.last_meta_change_at = RealTimeCoarseClock::get().read_time();
                    drop(metadata);
                    self.mark_metadata_publication_dirty();
                }
            }
        }
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

    pub(super) fn set_mode_impl(&self, mode: InodeMode) -> Result<()> {
        if self.metadata.read().type_ == InodeType::Dir {
            let fs = self.fs.upgrade().ok_or_else(|| {
                Error::with_message(Errno::EIO, "exFAT filesystem is not mounted")
            })?;
            let (_state_guard, block_device, boot_region, anomaly, _upcase_table, _options) =
                fs.admitted_mutation_state().map_err(Error::from)?;
            if anomaly.clear_to_zero || anomaly.media_failure {
                return_errno!(Errno::EIO);
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
                RewriteTarget::Directory,
                &block_device,
                &boot_region,
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
        let (_state_guard, block_device, boot_region, anomaly, _upcase_table, _options) =
            fs.admitted_mutation_state().map_err(Error::from)?;
        if anomaly.clear_to_zero || anomaly.media_failure {
            return_errno!(Errno::EIO);
        }

        let requested_writable = mode.intersects(mkmod!(a+w));
        let durable_updated = self.rewrite_inode_entry_set(
            RewriteTarget::RegularFile,
            &block_device,
            &boot_region,
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

    pub(super) fn owner_impl(&self) -> Result<Uid> {
        if self.metadata.read().type_ == InodeType::Dir {
            return self
                .directory_metadata_projection()
                .map(|metadata| metadata.uid);
        }
        Ok(self.metadata_projection().uid)
    }

    pub(super) fn set_owner_impl(&self, uid: Uid) -> Result<()> {
        let inode_type = self.metadata.read().type_;
        if !matches!(inode_type, InodeType::Dir | InodeType::File) {
            self.metadata.write().uid = uid;
            return Ok(());
        }
        self.reject_published_identity_change(|metadata| metadata.uid == uid)
    }

    pub(super) fn group_impl(&self) -> Result<Gid> {
        if self.metadata.read().type_ == InodeType::Dir {
            return self
                .directory_metadata_projection()
                .map(|metadata| metadata.gid);
        }
        Ok(self.metadata_projection().gid)
    }

    pub(super) fn set_group_impl(&self, gid: Gid) -> Result<()> {
        let inode_type = self.metadata.read().type_;
        if !matches!(inode_type, InodeType::Dir | InodeType::File) {
            self.metadata.write().gid = gid;
            return Ok(());
        }
        self.reject_published_identity_change(|metadata| metadata.gid == gid)
    }

    pub(super) fn atime_impl(&self) -> Duration {
        self.metadata_projection().last_access_at
    }

    pub(super) fn set_atime_impl(&self, time: Duration) {
        let inode_type = self.metadata.read().type_;
        match inode_type {
            InodeType::Dir => {
                self.rewrite_timestamp(RewriteTarget::Directory, TimestampFieldKind::Accessed, time)
            }
            InodeType::File => self.rewrite_timestamp(
                RewriteTarget::RegularFile,
                TimestampFieldKind::Accessed,
                time,
            ),
            _ => self.metadata.write().last_access_at = time,
        }
    }

    pub(super) fn mtime_impl(&self) -> Duration {
        self.metadata_projection().last_modify_at
    }

    pub(super) fn set_mtime_impl(&self, time: Duration) {
        let inode_type = self.metadata.read().type_;
        match inode_type {
            InodeType::Dir => self.rewrite_timestamp(
                RewriteTarget::Directory,
                TimestampFieldKind::Modified,
                time,
            ),
            InodeType::File => self.rewrite_timestamp(
                RewriteTarget::RegularFile,
                TimestampFieldKind::Modified,
                time,
            ),
            _ => self.metadata.write().last_modify_at = time,
        }
    }

    pub(super) fn ctime_impl(&self) -> Duration {
        self.metadata_projection().last_meta_change_at
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
            let Ok((_state_guard, _block_device, _boot_region, anomaly, _upcase_table, _options)) =
                fs.admitted_mutation_state().map_err(Error::from)
            else {
                return;
            };
            if anomaly.clear_to_zero || anomaly.media_failure {
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
        let Ok((_state_guard, _block_device, _boot_region, anomaly, _upcase_table, _options)) =
            fs.admitted_mutation_state().map_err(Error::from)
        else {
            return;
        };
        if anomaly.clear_to_zero || anomaly.media_failure {
            return;
        }

        let _owner_guard = self.admission.write();
        self.metadata.write().last_meta_change_at = time;
    }
}
