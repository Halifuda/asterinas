// SPDX-License-Identifier: MPL-2.0
#![short_vis_path::add(overlayfs)]

//! Data-path delegation for overlay inodes.
//!
//! Reads delegate to the current real authority; lower-backed reads pass
//! `O_NOATIME` so a read never updates the lower atime. Writes are
//! upper-backed by construction: the write-capable open path runs the copy-up
//! trigger before the handle is used, so delegation never bypasses the
//! trigger. `O_APPEND` is serialized under the per-inode transaction lock.

use super::{OverlayInode, permission::AccessType};
use crate::{
    fs::{
        file::{AccessMode, PerOpenFileOps, Permission, StatusFlags},
        vfs::inode::{FallocMode, Inode, SymbolicLink},
    },
    prelude::*,
    vm::page_cache::Vmo,
};

impl OverlayInode {
    pub(super) fn read_at_impl(
        &self,
        offset: usize,
        writer: &mut VmWriter,
        status_flags: StatusFlags,
    ) -> Result<usize> {
        let (real, is_lower_backed) = match self.upper.get() {
            Some(upper) => (upper.real_inode(), false),
            None => (
                self.lowers
                    .first()
                    .expect("a real-object stack is never empty")
                    .real_inode(),
                true,
            ),
        };
        let status_flags = if is_lower_backed {
            status_flags | StatusFlags::O_NOATIME
        } else {
            status_flags
        };
        real.read_at(offset, writer, status_flags)
    }

    pub(super) fn write_at_impl(
        &self,
        offset: usize,
        reader: &mut VmReader,
        status_flags: StatusFlags,
    ) -> Result<usize> {
        if status_flags.contains(StatusFlags::O_APPEND) {
            return self.append_write(reader, status_flags);
        }
        let real = self.select_real_inode();
        real.write_at(offset, reader, status_flags)
    }

    pub(super) fn open_impl(
        &self,
        access_mode: AccessMode,
        _status_flags: StatusFlags,
    ) -> Option<Result<Box<dyn PerOpenFileOps>>> {
        if self.type_().is_directory() {
            return None;
        }
        if !access_mode.is_writable() {
            return None;
        }
        let fs = match self.fs_arc() {
            Ok(fs) => fs,
            Err(err) => return Some(Err(err)),
        };
        if fs.policy().is_effective_read_only() {
            return Some(Err(Error::with_message(
                Errno::EROFS,
                "the overlay mount is read-only",
            )));
        }
        match self.copy_up() {
            Ok(()) => None,
            Err(err) => Some(Err(err)),
        }
    }

    pub(super) fn seek_end_impl(&self) -> Option<usize> {
        self.select_real_inode().seek_end()
    }

    // The path-based `truncate()` syscall performs no VFS `MAY_WRITE` check
    // of its own, so this entry runs the uniform mutating admission before
    // any side effect.
    pub(super) fn resize_impl(&self, new_size: usize) -> Result<()> {
        self.check_permission(AccessType::Mutating, Permission::MAY_WRITE)?;
        self.copy_up()?;
        self.select_real_inode().resize(new_size)
    }

    // `fallocate` shares `resize`'s side-effect class, so the uniform
    // mutating admission runs at this entry too rather than on the fd path
    // alone.
    pub(super) fn fallocate_impl(&self, mode: FallocMode, offset: usize, len: usize) -> Result<()> {
        self.check_permission(AccessType::Mutating, Permission::MAY_WRITE)?;
        self.copy_up()?;
        self.select_real_inode().fallocate(mode, offset, len)
    }

    pub(super) fn sync_all_impl(&self) -> Result<()> {
        self.select_real_inode().sync_all()
    }

    pub(super) fn sync_data_impl(&self) -> Result<()> {
        self.select_real_inode().sync_data()
    }

    pub(super) fn read_link_impl(&self) -> Result<SymbolicLink> {
        self.select_real_inode().read_link()
    }

    pub(super) fn page_cache_impl(&self) -> Option<Arc<Vmo>> {
        self.select_real_inode().page_cache()
    }
}
