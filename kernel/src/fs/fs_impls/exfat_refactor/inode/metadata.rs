// SPDX-License-Identifier: MPL-2.0

//! Projects and mutates inode metadata backed by exFAT file-entry sets.
//!
//! Method groups: cached projection, VFS metadata getters, metadata setters, timestamp rewrite,
//! entry-set rewrite, and directory metadata refresh.

use core::{cell::Cell, time::Duration};

use aster_block::BlockDevice;

use super::{
    super::{
        boot::BootRegion,
        direntry::{self, FileEntrySetView, FileEntryTimestamp},
        invalid_on_disk_layout, not_mounted,
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

impl ExfatInode {
    // Read projection

    // ---- meta_read (projection + VFS getters) ----

    pub(super) fn metadata_projection(&self) -> Metadata {
        self.inode_state_read_guard().metadata()
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
        self.inode_state_write_guard()
            .with_metadata_mut(|metadata| {
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
        let inode_type = self.metadata_projection().type_;
        if inode_type == InodeType::Dir {
            let fs = self.fs.upgrade().ok_or_else(|| {
                Error::with_message(Errno::EIO, "exFAT filesystem is not mounted")
            })?;
            let mut mutation_mount_state = fs.mount_state_write_guard()?;
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
            let (is_root_directory, current_writable) = {
                let inode_state_guard = self.inode_state_read_guard();
                (
                    inode_state_guard.dir_entry_stream().data_length.is_none(),
                    inode_state_guard.metadata().mode.intersects(mkmod!(a+w)),
                )
            };
            if is_root_directory {
                if requested_writable == current_writable {
                    return Ok(());
                }
                return_errno!(Errno::EOPNOTSUPP);
            }
            if requested_writable == current_writable {
                return Ok(());
            }

            let update_result = (|| {
                let mount_state = mutation_mount_state
                    .state_guard
                    .as_mut()
                    .ok_or_else(not_mounted)?;
                fs.publish_dirty_admission(mount_state)?;

                self.rewrite_inode_entry_set(
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
                )
            })();
            if update_result.is_err() {
                if let Some(mount_state) = mutation_mount_state.state_guard.as_mut() {
                    mount_state.volume_flags.volume_dirty = true;
                    mount_state.dirty_bracket_opened_by_mount = false;
                }
            }
            let durable_updated = update_result?;
            if durable_updated {
                let inode_state_guard = self.inode_state_write_guard();
                self.mark_metadata_dirty(&inode_state_guard);
            }
            return Ok(());
        }

        if inode_type != InodeType::File {
            self.inode_state_write_guard()
                .with_metadata_mut(|metadata| metadata.mode = mode);
            return Ok(());
        }

        let fs = self
            .fs
            .upgrade()
            .ok_or_else(|| Error::with_message(Errno::EIO, "exFAT filesystem is not mounted"))?;
        let mut mutation_mount_state = fs.mount_state_write_guard()?;
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
        let current_writable = self.metadata_projection().mode.intersects(mkmod!(a+w));
        if requested_writable == current_writable {
            return Ok(());
        }
        let update_result = (|| {
            let mount_state = mutation_mount_state
                .state_guard
                .as_mut()
                .ok_or_else(not_mounted)?;
            fs.publish_dirty_admission(mount_state)?;

            self.rewrite_inode_entry_set(
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
            )
        })();
        if update_result.is_err() {
            if let Some(mount_state) = mutation_mount_state.state_guard.as_mut() {
                mount_state.volume_flags.volume_dirty = true;
                mount_state.dirty_bracket_opened_by_mount = false;
            }
        }
        let durable_updated = update_result?;

        self.inode_state_write_guard()
            .with_metadata_mut(|metadata| {
                metadata.mode = chmod!(chmod!(metadata.mode, a-w), u+w);
                if !requested_writable {
                    metadata.mode = chmod!(metadata.mode, a-w);
                }
                if durable_updated {
                    metadata.last_meta_change_at = RealTimeCoarseClock::get().read_time();
                }
            });
        if durable_updated {
            let inode_state_guard = self.inode_state_write_guard();
            self.mark_metadata_dirty(&inode_state_guard);
        }
        Ok(())
    }

    pub(super) fn set_atime_impl(&self, time: Duration) {
        let inode_type = self.metadata_projection().type_;
        match inode_type {
            InodeType::Dir => self.rewrite_timestamp(InodeTimestampField::Accessed, time),
            InodeType::File => self.rewrite_timestamp(InodeTimestampField::Accessed, time),
            _ => self
                .inode_state_write_guard()
                .with_metadata_mut(|metadata| metadata.last_access_at = time),
        }
    }

    pub(super) fn set_mtime_impl(&self, time: Duration) {
        let inode_type = self.metadata_projection().type_;
        match inode_type {
            InodeType::Dir => self.rewrite_timestamp(InodeTimestampField::Modified, time),
            InodeType::File => self.rewrite_timestamp(InodeTimestampField::Modified, time),
            _ => self
                .inode_state_write_guard()
                .with_metadata_mut(|metadata| metadata.last_modify_at = time),
        }
    }

    pub(super) fn set_ctime_impl(&self, time: Duration) {
        let inode_type = self.metadata_projection().type_;
        if inode_type == InodeType::Dir {
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

        if inode_type != InodeType::File {
            self.inode_state_write_guard()
                .with_metadata_mut(|metadata| metadata.last_meta_change_at = time);
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

        let inode_state_guard = self.inode_state_write_guard();
        inode_state_guard.with_metadata_mut(|metadata| metadata.last_meta_change_at = time);
    }

    pub(super) fn set_owner_impl(&self, uid: Uid) -> Result<()> {
        let inode_type = self.metadata_projection().type_;
        if !matches!(inode_type, InodeType::Dir | InodeType::File) {
            self.inode_state_write_guard()
                .with_metadata_mut(|metadata| metadata.uid = uid);
            return Ok(());
        }
        self.reject_identity_change(|metadata| metadata.uid == uid)
    }

    pub(super) fn set_group_impl(&self, gid: Gid) -> Result<()> {
        let inode_type = self.metadata_projection().type_;
        if !matches!(inode_type, InodeType::Dir | InodeType::File) {
            self.inode_state_write_guard()
                .with_metadata_mut(|metadata| metadata.gid = gid);
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

        let metadata = self.inode_state_read_guard().metadata();
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
        let Ok(mut mutation_mount_state) = fs.mount_state_write_guard() else {
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

        let inode_state_guard = self.inode_state_read_guard();
        if inode_state_guard.metadata().type_ == InodeType::Dir
            && inode_state_guard.dir_entry_stream().data_length.is_none()
        {
            return;
        }

        let normalized_time = Cell::new(None);
        let rewrite_result = (|| {
            let mount_state = mutation_mount_state
                .state_guard
                .as_mut()
                .ok_or_else(not_mounted)?;
            fs.publish_dirty_admission(mount_state)?;

            self.rewrite_inode_entry_set(
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
            if let Some(mount_state) = mutation_mount_state.state_guard.as_mut() {
                mount_state.volume_flags.volume_dirty = true;
                mount_state.dirty_bracket_opened_by_mount = false;
            }
        }
        if rewrite_result.is_ok_and(|updated| updated) {
            let inode_state_guard = self.inode_state_write_guard();
            self.mark_metadata_dirty(&inode_state_guard);
        }
    }

    pub(super) fn refresh_directory_metadata_after_namespace_mutation_with_guards(
        &self,
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        timestamp: Duration,
        self_inode_state_guard: &InodeStateWriteGuard<'_>,
        parent_inode_state_guard: Option<&InodeStateWriteGuard<'_>>,
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
        let durable_updated = self.rewrite_inode_entry_set_with_guards(
            self_inode_state_guard,
            parent_inode_state_guard,
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
            self.mark_metadata_dirty(self_inode_state_guard);
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
        let parent = {
            let self_inode_state_guard = self.inode_state_read_guard();
            self_inode_state_guard.parent().ok_or_else(|| {
                Error::with_message(Errno::EIO, "ordinary exFAT directory parent is not mounted")
            })?
        };
        let directory_guards = Self::directory_write_guards_by_ino(vec![self, parent.as_ref()]);
        let guard_for_inode = |inode: &ExfatInode| {
            directory_guards
                .iter()
                .find(|guard| guard.guards_inode(inode))
                .ok_or_else(|| Error::new(Errno::EINVAL))
        };
        let self_inode_state_guard = guard_for_inode(self)?;
        let parent_inode_state_guard = guard_for_inode(parent.as_ref())?;
        self.rewrite_inode_entry_set_with_guards(
            self_inode_state_guard,
            parent_inode_state_guard,
            block_device,
            boot_region,
            rewrite_entry_set_fn,
            update_metadata_fn,
        )
    }

    fn rewrite_inode_entry_set_with_guards(
        &self,
        self_inode_state_guard: &InodeStateWriteGuard<'_>,
        parent_inode_state_guard: &InodeStateWriteGuard<'_>,
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        rewrite_entry_set_fn: impl FnOnce(FileEntrySetView<'_>) -> Result<Option<Vec<u8>>>,
        update_metadata_fn: impl FnOnce(&mut Metadata),
    ) -> Result<bool> {
        let durable_updated = self.rewrite_validated_entry_set_with_guard(
            self_inode_state_guard,
            parent_inode_state_guard,
            block_device,
            boot_region,
            rewrite_entry_set_fn,
        )?;
        if durable_updated {
            self_inode_state_guard.with_metadata_mut(update_metadata_fn);
        }
        Ok(durable_updated)
    }

    pub(super) fn publish_live_regular_file_entry_set(
        &self,
        self_inode_state_guard: &InodeStateWriteGuard<'_>,
        parent_inode_state_guard: &InodeStateWriteGuard<'_>,
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
    ) -> Result<bool> {
        if self_inode_state_guard.metadata().type_ != InodeType::File {
            return_errno!(Errno::EOPNOTSUPP);
        }

        let cluster_map = self_inode_state_guard.dir_entry_stream();
        let last_modify_at = self_inode_state_guard.metadata().last_modify_at;
        let durable_updated = match self.rewrite_inode_entry_set_with_guards(
            self_inode_state_guard,
            parent_inode_state_guard,
            block_device,
            boot_region,
            |entry_view| {
                let (inode_type, _first_cluster, _data_length, _no_fat_chain) =
                    entry_view.child_metadata(boot_region)?;
                if inode_type != InodeType::File || entry_view.is_directory() {
                    return Err(Error::from(invalid_on_disk_layout()));
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
        ) {
            Ok(durable_updated) => durable_updated,
            Err(error) => return Err(error),
        };
        Ok(durable_updated)
    }
}
