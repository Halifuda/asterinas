// SPDX-License-Identifier: MPL-2.0

//! Implements directory lookup, readdir, and child-inode materialization.
//!
//! Method groups: name lookup, directory-entry scans, readdir emission, and VFS lookup dispatch.

use alloc::string::String;

use aster_block::BlockDevice;

use super::{
    super::{
        boot::BootRegion,
        direntry::{self, DirEntryIssueKind, FileEntrySetView, ScannedDirEntry},
        invalid_on_disk_layout,
    },
    ExfatFs, ExfatInode, UpcaseTable,
    state::InodeStateReadGuard,
};
use crate::{
    fs::{file::InodeType, utils::DirentVisitor, vfs::inode::Inode},
    prelude::*,
};

impl ExfatInode {
    pub(super) fn lookup_child_by_name(
        &self,
        fs: &Arc<ExfatFs>,
        fs_state: &mut super::super::fs::FsState,
        inode_state_guard: &InodeStateReadGuard<'_>,
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        upcase_table: &UpcaseTable,
        lookup_name: &[u16],
        lookup_name_hash: u16,
    ) -> Result<Option<Arc<dyn Inode>>> {
        let cluster_map = inode_state_guard.dir_entry_stream();
        let directory_bytes =
            Self::read_directory_bytes_for_cluster_map(block_device, boot_region, cluster_map)?;
        let Some(entry_view) = Self::locate_named_child_view(
            &directory_bytes,
            cluster_map.data_length.is_none(),
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
        let ino = (u64::from(cluster_map.first_cluster) << 32)
            | u64::from(
                u32::try_from(slot_range.first_entry_index())
                    .map_err(|_| invalid_on_disk_layout())?,
            );
        let valid_data_length = entry_view
            .cluster_map()?
            .valid_data_length
            .ok_or_else(invalid_on_disk_layout)?;
        if valid_data_length > data_length {
            return Err(invalid_on_disk_layout());
        }

        if let Some(child_inode) = ExfatFs::peek_cached_inode(fs_state, ino) {
            let child_inode: Arc<dyn Inode> = child_inode;
            return Ok(Some(child_inode));
        }

        let child_inode = Self::new_child(
            fs,
            self.weak_self(),
            ino,
            inode_type,
            boot_region.cluster_size,
            data_length,
            first_cluster,
            data_length,
            valid_data_length,
            no_fat_chain,
        );
        {
            let unpublished_child_guard = child_inode.inode_state_write_guard();
            child_inode.refresh_cached_metadata_from_entry_view(
                &unpublished_child_guard,
                entry_view,
                boot_region,
            )?;
        }
        if inode_type == InodeType::File {
            child_inode.store_entry_set_location_hint(slot_range)?;
        }
        fs_state
            .inode_cache
            .insert(ino, Arc::downgrade(&child_inode));
        Ok(Some(child_inode))
    }

    pub(super) fn locate_named_child_view<'a>(
        directory_bytes: &'a [u8],
        is_root_directory: bool,
        upcase_table: &UpcaseTable,
        lookup_name: &[u16],
        lookup_name_hash: u16,
    ) -> Result<Option<FileEntrySetView<'a>>> {
        let mut entry_index = 0usize;
        loop {
            match direntry::scan_dir_entry(is_root_directory, directory_bytes, entry_index)? {
                ScannedDirEntry::EndOfDirectory { .. } => return Ok(None),
                ScannedDirEntry::Vacant(slot_range) => {
                    entry_index = slot_range.next_entry_index()?;
                }
                ScannedDirEntry::File(entry_view) => {
                    let candidate_name = entry_view.name()?;
                    if entry_view.stored_name_hash() == lookup_name_hash
                        && upcase_table.names_equal(lookup_name, &candidate_name)
                    {
                        return Ok(Some(entry_view));
                    }
                    entry_index = entry_view.slot_range().next_entry_index()?;
                }
                ScannedDirEntry::Issue { kind, slot_range } => {
                    if kind == DirEntryIssueKind::BenignUnrecognizedEntrySet {
                        entry_index = slot_range.next_entry_index()?;
                        continue;
                    }
                    return Err(invalid_on_disk_layout());
                }
            }
        }
    }

    pub(super) fn readdir_at_impl(
        &self,
        offset: usize,
        visitor: &mut dyn DirentVisitor,
    ) -> Result<usize> {
        let fs = self
            .fs
            .upgrade()
            .ok_or_else(|| Error::with_message(Errno::EIO, "exFAT filesystem is not mounted"))?;
        let block_device = fs.immutable_block_device();
        let boot_region = fs.immutable_boot_region();
        let _fs_state = fs.fs_state.read();
        let inode_state_guard = self.inode_state_read_guard();
        if inode_state_guard.metadata().type_ != InodeType::Dir {
            return_errno!(Errno::ENOTDIR);
        }
        let parent = inode_state_guard.parent();
        drop(inode_state_guard);
        let mut guarded_inodes = vec![self];
        if let Some(parent) = parent.as_ref() {
            guarded_inodes.push(parent.as_ref());
        }
        let inode_guards = Self::directory_read_guards_by_stable_identity(guarded_inodes);
        let inode_state_guard = inode_guards
            .iter()
            .find(|guard| guard.guards_inode(self))
            .ok_or_else(|| Error::new(Errno::EINVAL))?;
        let _allocation_guard = fs.allocation_read_guard()?;
        let directory_ino = inode_state_guard.metadata().ino;
        let cluster_map = inode_state_guard.dir_entry_stream();
        let directory_bytes =
            Self::read_directory_bytes_for_cluster_map(&block_device, &boot_region, cluster_map)?;

        let mut next_offset = offset;
        if next_offset == 0 {
            let dot_next_offset = next_offset
                .checked_add(1)
                .ok_or_else(invalid_on_disk_layout)?;
            if let Err(error) = visitor.visit(".", directory_ino, InodeType::Dir, dot_next_offset) {
                return Err(error);
            }
            next_offset = dot_next_offset;
        }
        if next_offset == 1 {
            let dotdot_next_offset = next_offset
                .checked_add(1)
                .ok_or_else(invalid_on_disk_layout)?;
            let parent_ino = match parent.as_ref() {
                Some(parent) => inode_guards
                    .iter()
                    .find(|guard| guard.guards_inode(parent.as_ref()))
                    .ok_or_else(|| Error::new(Errno::EINVAL))?
                    .metadata()
                    .ino,
                None => directory_ino,
            };
            if let Err(error) =
                visitor.visit("..", parent_ino, InodeType::Dir, dotdot_next_offset)
            {
                if next_offset == offset {
                    return Err(error);
                }
                return Ok(next_offset.saturating_sub(offset));
            }
            next_offset = dotdot_next_offset;
        }

        let mut visible_offset = 2usize;
        let mut entry_index = 0usize;
        loop {
            match direntry::scan_dir_entry(
                cluster_map.data_length.is_none(),
                &directory_bytes,
                entry_index,
            )? {
                ScannedDirEntry::EndOfDirectory { .. } => break,
                ScannedDirEntry::Vacant(slot_range) => {
                    entry_index = slot_range.next_entry_index()?;
                }
                ScannedDirEntry::File(entry_view) => {
                    let candidate_name = entry_view.name()?;
                    let (inode_type, _, _, _) = entry_view.child_metadata(&boot_region)?;
                    let resume_offset = visible_offset
                        .checked_add(1)
                        .ok_or_else(invalid_on_disk_layout)?;

                    if visible_offset >= offset {
                        let entry_name = String::from_utf16(&candidate_name)
                            .map_err(|_| invalid_on_disk_layout())?;
                        let entry_ino = (u64::from(cluster_map.first_cluster) << 32)
                            | u64::from(
                                u32::try_from(entry_view.slot_range().first_entry_index())
                                    .map_err(|_| invalid_on_disk_layout())?,
                            );
                        if let Err(error) =
                            visitor.visit(&entry_name, entry_ino, inode_type, resume_offset)
                        {
                            if next_offset == offset {
                                return Err(error);
                            }
                            return Ok(next_offset.saturating_sub(offset));
                        }
                        next_offset = resume_offset;
                    }
                    visible_offset = resume_offset;
                    entry_index = entry_view.slot_range().next_entry_index()?;
                }
                ScannedDirEntry::Issue { kind, slot_range } => {
                    if kind == DirEntryIssueKind::BenignUnrecognizedEntrySet {
                        entry_index = slot_range.next_entry_index()?;
                        continue;
                    }
                    return Err(invalid_on_disk_layout().into());
                }
            }
        }
        Ok(next_offset.saturating_sub(offset))
    }

    pub(super) fn lookup_impl(&self, name: &str) -> Result<Arc<dyn Inode>> {
        let fs = self
            .fs
            .upgrade()
            .ok_or_else(|| Error::with_message(Errno::EIO, "exFAT filesystem is not mounted"))?;
        let mut fs_state = fs.fs_state.write();
        let mount_state = fs_state.mount_state.as_ref().ok_or_else(super::super::not_mounted)?;
        let inode_state_guard = self.inode_state_read_guard();
        if inode_state_guard.metadata().type_ != InodeType::Dir {
            return_errno!(Errno::ENOTDIR);
        }
        if name == "." {
            let inode: Arc<dyn Inode> = self
                .weak_self()
                .upgrade()
                .ok_or_else(|| Error::with_message(Errno::EIO, "exFAT inode is not mounted"))?;
            return Ok(inode);
        }
        if name == ".." {
            let parent = inode_state_guard.parent().unwrap_or_else(|| {
                self.weak_self()
                    .upgrade()
                    .unwrap_or_else(|| unreachable!("admitted inode must keep self reachable"))
            });
            let parent: Arc<dyn Inode> = parent;
            return Ok(parent);
        }
        let _allocation_guard = fs.allocation_read_guard()?;
        let block_device = fs.immutable_block_device();
        let boot_region = fs.immutable_boot_region();
        let upcase_table = fs_state.upcase_table.as_ref().ok_or_else(super::super::not_mounted)?.clone();
        let lookup_name = Self::validate_name(name, &mount_state.options)?;
        let lookup_name_hash = upcase_table.name_hash(&lookup_name);
        let child_inode = self.lookup_child_by_name(
            &fs,
            &mut fs_state,
            &inode_state_guard,
            &block_device,
            &boot_region,
            &upcase_table,
            &lookup_name,
            lookup_name_hash,
        )?;
        if let Some(child_inode) = child_inode {
            return Ok(child_inode);
        }

        return_errno!(Errno::ENOENT);
    }
}
