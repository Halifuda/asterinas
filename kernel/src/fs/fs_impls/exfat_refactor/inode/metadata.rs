// SPDX-License-Identifier: MPL-2.0

//! Projects and mutates inode metadata backed by exFAT file-entry sets.
//!
//! Method groups: cached projection, VFS metadata getters, metadata setters, timestamp rewrite,
//! entry-set rewrite, and directory metadata refresh.

use core::{cell::Cell, time::Duration};

use super::{
    super::{
        boot::BootRegion,
        dir_entry_format::{self as direntry, FileEntrySetView, FileEntryTimestamp},
        fs::{ExfatFs, FsState},
        invalid_on_disk_layout,
    },
    ExfatInode, InodeTimestampField,
    state::InodeStateWriteGuard,
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

// ---- meta_write (refresh + setters) ----
impl ExfatInode {
    pub(super) fn prepare_directory_metadata_refresh_with_guards(
        &self,
        self_inode_state_guard: &InodeStateWriteGuard<'_>,
        parent_inode_state_guard: &InodeStateWriteGuard<'_>,
        boot_region: &BootRegion,
        timestamp: Duration,
    ) -> Result<
        Option<(
            direntry::DirEntrySlotRange,
            Vec<u8>,
            Vec<u8>,
            Vec<(usize, bool)>,
        )>,
    > {
        self.prepare_rewritten_entry_set_write_with_guard(
            self_inode_state_guard,
            parent_inode_state_guard,
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
        )
    }

    pub(super) fn refresh_cached_metadata_from_entry_view(
        &self,
        inode_state_guard: &InodeStateWriteGuard<'_>,
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
        inode_state_guard.with_metadata_mut(|metadata| {
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
        })?;
        Ok(())
    }

    // Write path

    pub(super) fn set_mode_impl(&self, mode: InodeMode) -> Result<()> {
        let fs = self
            .fs
            .upgrade()
            .ok_or_else(|| Error::with_message(Errno::EIO, "exFAT filesystem is not mounted"))?;
        let mut fs_state = fs.fs_state.write();
        let mount_state = fs_state
            .mount_state
            .as_ref()
            .ok_or_else(super::super::not_mounted)?;
        if mount_state.forced_shutdown
            || mount_state.volume_flags.clear_to_zero
            || mount_state.volume_flags.media_failure
        {
            return_errno!(Errno::EIO);
        }
        if mount_state.options.fs_flags.contains(FsFlags::RDONLY) {
            return_errno!(Errno::EROFS);
        }

        let (discovered_type, parent) = {
            let inode_state_guard = self.inode_state_read_guard();
            (
                inode_state_guard.metadata().type_,
                inode_state_guard.parent(),
            )
        };
        let mut guarded_inodes = vec![self];
        if matches!(discovered_type, InodeType::Dir | InodeType::File)
            && let Some(parent) = parent.as_ref()
        {
            guarded_inodes.push(parent.as_ref());
        }
        let inode_guards = Self::directory_write_guards_by_ino(guarded_inodes);
        let self_inode_state_guard = inode_guards
            .iter()
            .find(|guard| guard.guards_inode(self))
            .ok_or_else(|| Error::new(Errno::EINVAL))?;
        let inode_type = self_inode_state_guard.metadata().type_;
        if !matches!(inode_type, InodeType::Dir | InodeType::File) {
            self_inode_state_guard.with_metadata_mut(|metadata| metadata.mode = mode);
            return Ok(());
        }

        let requested_writable = mode.intersects(mkmod!(a+w));
        let current_writable = self_inode_state_guard
            .metadata()
            .mode
            .intersects(mkmod!(a+w));
        if inode_type == InodeType::Dir
            && self_inode_state_guard
                .dir_entry_stream()
                .data_length
                .is_none()
        {
            if requested_writable == current_writable {
                return Ok(());
            }
            return_errno!(Errno::EOPNOTSUPP);
        }
        if requested_writable == current_writable {
            return Ok(());
        }
        let parent = parent.as_ref().ok_or_else(|| Error::new(Errno::EIO))?;
        if !self_inode_state_guard
            .parent()
            .is_some_and(|admitted_parent| Arc::ptr_eq(&admitted_parent, parent))
        {
            return_errno!(Errno::EIO);
        }
        let parent_inode_state_guard = inode_guards
            .iter()
            .find(|guard| guard.guards_inode(parent.as_ref()))
            .ok_or_else(|| Error::new(Errno::EINVAL))?;
        let boot_region = fs.immutable_boot_region();
        let _allocation_guard = fs.allocation_read_guard()?;
        let update_result = (|| {
            fs.publish_dirty_admission(&mut fs_state)?;

            self.rewrite_inode_entry_set_with_guards(
                &mut fs_state,
                self_inode_state_guard,
                parent_inode_state_guard,
                &boot_region,
                |entry_view| {
                    if requested_writable != entry_view.is_read_only() {
                        return Ok(None);
                    }
                    let mut file_attributes = entry_view.file_attributes();
                    if inode_type == InodeType::Dir {
                        file_attributes |= direntry::FILE_ATTRIBUTE_DIRECTORY;
                    }
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
                    if inode_type == InodeType::Dir {
                        let writable_bits = metadata.mode & mkmod!(a+w);
                        metadata.mode = chmod!(metadata.mode, a-w);
                        if requested_writable {
                            metadata.mode |= writable_bits;
                        }
                    } else {
                        metadata.mode = chmod!(metadata.mode, a-w);
                        if requested_writable {
                            metadata.mode |= mkmod!(u+w);
                        }
                    }
                },
            )
        })();
        if update_result.is_err() {
            ExfatFs::mark_mount_dirty_after_failure(&mut fs_state);
        }
        let durable_updated = update_result?;
        if durable_updated {
            self_inode_state_guard.with_metadata_mut(|metadata| {
                metadata.last_meta_change_at = RealTimeCoarseClock::get().read_time();
            });
            self.mark_metadata_dirty(self_inode_state_guard);
        }
        Ok(())
    }

    pub(super) fn set_atime_impl(&self, time: Duration) {
        self.rewrite_timestamp(InodeTimestampField::Accessed, time);
    }

    pub(super) fn set_mtime_impl(&self, time: Duration) {
        self.rewrite_timestamp(InodeTimestampField::Modified, time);
    }

    pub(super) fn set_ctime_impl(&self, time: Duration) {
        let Some(fs) = self.fs.upgrade() else {
            return;
        };
        let fs_state = fs.fs_state.read();
        let Some(mount_state) = fs_state.mount_state.as_ref() else {
            return;
        };
        if mount_state.forced_shutdown
            || mount_state.volume_flags.clear_to_zero
            || mount_state.volume_flags.media_failure
        {
            return;
        }
        if mount_state.options.fs_flags.contains(FsFlags::RDONLY) {
            return;
        }

        let inode_state_guard = self.inode_state_write_guard();
        if inode_state_guard.metadata().type_ != InodeType::Dir {
            inode_state_guard.with_metadata_mut(|metadata| metadata.last_meta_change_at = time);
        }
    }

    pub(super) fn set_owner_impl(&self, uid: Uid) -> Result<()> {
        let fs = self
            .fs
            .upgrade()
            .ok_or_else(|| Error::with_message(Errno::EIO, "exFAT filesystem is not mounted"))?;
        let fs_state = fs.fs_state.read();
        let mount_state = fs_state
            .mount_state
            .as_ref()
            .ok_or_else(super::super::not_mounted)?;
        if mount_state.forced_shutdown
            || mount_state.volume_flags.clear_to_zero
            || mount_state.volume_flags.media_failure
        {
            return_errno!(Errno::EIO);
        }
        if mount_state.options.fs_flags.contains(FsFlags::RDONLY) {
            return_errno!(Errno::EROFS);
        }
        let inode_state_guard = self.inode_state_write_guard();
        if !matches!(
            inode_state_guard.metadata().type_,
            InodeType::Dir | InodeType::File
        ) {
            inode_state_guard.with_metadata_mut(|metadata| metadata.uid = uid);
            return Ok(());
        }
        if inode_state_guard.metadata().uid == uid {
            return Ok(());
        }
        return_errno!(Errno::EPERM);
    }

    pub(super) fn set_group_impl(&self, gid: Gid) -> Result<()> {
        let fs = self
            .fs
            .upgrade()
            .ok_or_else(|| Error::with_message(Errno::EIO, "exFAT filesystem is not mounted"))?;
        let fs_state = fs.fs_state.read();
        let mount_state = fs_state
            .mount_state
            .as_ref()
            .ok_or_else(super::super::not_mounted)?;
        if mount_state.forced_shutdown
            || mount_state.volume_flags.clear_to_zero
            || mount_state.volume_flags.media_failure
        {
            return_errno!(Errno::EIO);
        }
        if mount_state.options.fs_flags.contains(FsFlags::RDONLY) {
            return_errno!(Errno::EROFS);
        }

        let inode_state_guard = self.inode_state_write_guard();
        if !matches!(
            inode_state_guard.metadata().type_,
            InodeType::Dir | InodeType::File
        ) {
            inode_state_guard.with_metadata_mut(|metadata| metadata.gid = gid);
            return Ok(());
        }
        if inode_state_guard.metadata().gid == gid {
            return Ok(());
        }
        return_errno!(Errno::EPERM);
    }
}

// ---- entry_rewrite (timestamp + directory metadata refresh) ----
impl ExfatInode {
    fn rewrite_timestamp(&self, field_kind: InodeTimestampField, time: Duration) {
        let Some(fs) = self.fs.upgrade() else {
            return;
        };
        // TODO: These timestamp setters still admit through the generic mounted-mutation gate and
        // reuse the currently stored exFAT UTC-offset byte because `MountOptions` does not
        // yet own explicit `allow_utime` / timezone policy. Once the later `meso_09`
        // mount-policy follow-up exposes that owner-local policy under `ExfatFs`, remove this
        // seam and route timestamp admission plus UTC-offset selection through that dedicated path.
        let mut fs_state = fs.fs_state.write();
        let Some(mount_state) = fs_state.mount_state.as_ref() else {
            return;
        };
        let boot_region = fs.immutable_boot_region();
        if mount_state.forced_shutdown
            || mount_state.volume_flags.clear_to_zero
            || mount_state.volume_flags.media_failure
        {
            return;
        }
        if mount_state.options.fs_flags.contains(FsFlags::RDONLY) {
            return;
        }

        let (discovered_type, is_root_directory, parent) = {
            let inode_state_guard = self.inode_state_read_guard();
            (
                inode_state_guard.metadata().type_,
                inode_state_guard.dir_entry_stream().data_length.is_none(),
                inode_state_guard.parent(),
            )
        };
        if !matches!(discovered_type, InodeType::Dir | InodeType::File) {
            let inode_state_guard = self.inode_state_write_guard();
            match field_kind {
                InodeTimestampField::Accessed => {
                    inode_state_guard.with_metadata_mut(|metadata| metadata.last_access_at = time)
                }
                InodeTimestampField::Modified => {
                    inode_state_guard.with_metadata_mut(|metadata| metadata.last_modify_at = time)
                }
            }
            return;
        }
        if discovered_type == InodeType::Dir && is_root_directory {
            let inode_state_guard = self.inode_state_write_guard();
            if inode_state_guard.metadata().type_ == InodeType::Dir
                && inode_state_guard.dir_entry_stream().data_length.is_none()
            {
                return;
            }
        }
        let Some(parent) = parent else {
            return;
        };
        let inode_guards = Self::directory_write_guards_by_ino(vec![self, parent.as_ref()]);
        let Some(self_inode_state_guard) =
            inode_guards.iter().find(|guard| guard.guards_inode(self))
        else {
            return;
        };
        if !matches!(
            self_inode_state_guard.metadata().type_,
            InodeType::Dir | InodeType::File
        ) || !self_inode_state_guard
            .parent()
            .is_some_and(|admitted_parent| Arc::ptr_eq(&admitted_parent, &parent))
        {
            return;
        }
        let Some(parent_inode_state_guard) = inode_guards
            .iter()
            .find(|guard| guard.guards_inode(parent.as_ref()))
        else {
            return;
        };
        let Ok(_allocation_guard) = fs.allocation_read_guard() else {
            return;
        };

        let normalized_time = Cell::new(None);
        let rewrite_result = (|| {
            fs.publish_dirty_admission(&mut fs_state)?;

            self.rewrite_inode_entry_set_with_guards(
                &mut fs_state,
                self_inode_state_guard,
                parent_inode_state_guard,
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
                            let normalized_timestamp = Self::decoded_exfat_timestamp(
                                timestamp_bytes,
                                None,
                                encoded_utc_offset_byte,
                            )?;
                            normalized_time.set(Some(normalized_timestamp));
                            mutable_entry_set.set_last_accessed_timestamp(FileEntryTimestamp::new(
                                timestamp_bytes,
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
                            let normalized_timestamp = Self::decoded_exfat_timestamp(
                                timestamp_bytes,
                                Some(ten_ms_increment),
                                encoded_utc_offset_byte,
                            )?;
                            normalized_time.set(Some(normalized_timestamp));
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
                    InodeTimestampField::Accessed => {
                        metadata.last_access_at = normalized_time.get().unwrap_or(time);
                    }
                    InodeTimestampField::Modified => {
                        metadata.last_meta_change_at = time;
                        metadata.last_modify_at = normalized_time.get().unwrap_or(time);
                    }
                },
            )
        })();
        if rewrite_result.is_err() {
            ExfatFs::mark_mount_dirty_after_failure(&mut fs_state);
        }
        if rewrite_result.is_ok_and(|updated| updated) {
            self.mark_metadata_dirty(self_inode_state_guard);
        }
    }

    pub(super) fn refresh_directory_metadata_after_namespace_mutation_with_guards(
        &self,
        fs_state: &mut FsState,
        boot_region: &BootRegion,
        timestamp: Duration,
        self_inode_state_guard: &InodeStateWriteGuard<'_>,
        parent_inode_state_guard: Option<&InodeStateWriteGuard<'_>>,
        prepared_entry_set_write: Option<(
            direntry::DirEntrySlotRange,
            Vec<u8>,
            Vec<u8>,
            Vec<(usize, bool)>,
        )>,
        namespace_stage_exposed: bool,
    ) -> Result<()> {
        if self_inode_state_guard.metadata().type_ != InodeType::Dir {
            return_errno!(Errno::ENOTDIR);
        }

        if self_inode_state_guard
            .dir_entry_stream()
            .data_length
            .is_none()
        {
            self_inode_state_guard.with_metadata_mut(|metadata| {
                metadata.last_meta_change_at = timestamp;
                metadata.last_modify_at = timestamp;
            });
            self.mark_metadata_dirty(self_inode_state_guard);
            return Ok(());
        }

        let parent_inode_state_guard = parent_inode_state_guard.ok_or_else(|| {
            Error::with_message(
                Errno::EINVAL,
                "ordinary exFAT directory refresh requires parent write-guard proof",
            )
        })?;
        let classified_update = if let Some(prepared_entry_set_write) = prepared_entry_set_write {
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
                !namespace_stage_exposed,
            )
        } else {
            self.rewrite_validated_entry_set_with_guard_classified(
                fs_state,
                self_inode_state_guard,
                parent_inode_state_guard,
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
                !namespace_stage_exposed,
            )
        };
        match classified_update {
            Ok(Ok(durable_updated)) => {
                if durable_updated {
                    self_inode_state_guard.with_metadata_mut(|metadata| {
                        metadata.last_meta_change_at = timestamp;
                        metadata.last_modify_at = timestamp;
                    });
                    self.mark_metadata_dirty(self_inode_state_guard);
                }
                Ok(())
            }
            Ok(Err(error)) => {
                self_inode_state_guard.with_metadata_mut(|metadata| {
                    metadata.last_meta_change_at = timestamp;
                    metadata.last_modify_at = timestamp;
                });
                self.mark_metadata_dirty(self_inode_state_guard);
                Err(error)
            }
            Err(error) => Err(error),
        }
    }

    fn rewrite_inode_entry_set_with_guards(
        &self,
        fs_state: &mut FsState,
        self_inode_state_guard: &InodeStateWriteGuard<'_>,
        parent_inode_state_guard: &InodeStateWriteGuard<'_>,
        boot_region: &BootRegion,
        rewrite_entry_set_fn: impl FnOnce(FileEntrySetView<'_>) -> Result<Option<Vec<u8>>>,
        update_metadata_fn: impl FnOnce(&mut Metadata),
    ) -> Result<bool> {
        let classified_update = self.rewrite_validated_entry_set_with_guard_classified(
            fs_state,
            self_inode_state_guard,
            parent_inode_state_guard,
            boot_region,
            rewrite_entry_set_fn,
            true,
        );
        match classified_update {
            Ok(Ok(durable_updated)) => {
                if durable_updated {
                    self_inode_state_guard.with_metadata_mut(update_metadata_fn);
                }
                Ok(durable_updated)
            }
            Ok(Err(error)) => {
                self_inode_state_guard.with_metadata_mut(update_metadata_fn);
                Err(error)
            }
            Err(error) => Err(error),
        }
    }

    pub(super) fn publish_live_regular_file_entry_set(
        &self,
        fs_state: &mut FsState,
        self_inode_state_guard: &InodeStateWriteGuard<'_>,
        parent_inode_state_guard: &InodeStateWriteGuard<'_>,
        boot_region: &BootRegion,
    ) -> Result<bool> {
        if self_inode_state_guard.metadata().type_ != InodeType::File {
            return_errno!(Errno::EOPNOTSUPP);
        }

        let cluster_map = self_inode_state_guard.dir_entry_stream();
        let last_modify_at = self_inode_state_guard.metadata().last_modify_at;
        let durable_updated = self.rewrite_inode_entry_set_with_guards(
            fs_state,
            self_inode_state_guard,
            parent_inode_state_guard,
            boot_region,
            |entry_view| {
                let (inode_type, _first_cluster, _data_length, _no_fat_chain) =
                    entry_view.child_metadata(boot_region)?;
                if inode_type != InodeType::File || entry_view.is_directory() {
                    return Err(invalid_on_disk_layout());
                }

                let (timestamp_bytes, hundredths_increment, encoded_utc_offset_byte) =
                    Self::encoded_exfat_timestamp_fields(
                        last_modify_at,
                        entry_view.last_modified_timestamp().utc_offset_byte(),
                    )?;
                let mut mutable_entry_set = entry_view.to_mutable();
                mutable_entry_set.set_cluster_map(&cluster_map)?;
                mutable_entry_set.set_last_modified_timestamp(FileEntryTimestamp::new(
                    timestamp_bytes,
                    Some(hundredths_increment),
                    encoded_utc_offset_byte,
                ));
                Ok(Some(mutable_entry_set.into_bytes()))
            },
            |_| {},
        )?;
        Ok(durable_updated)
    }
}
