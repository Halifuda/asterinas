// SPDX-License-Identifier: MPL-2.0

//! The module root of the copy-up authority and file-views subsystem.
//!
//! This module declares the four `copyup/*` submodules and hosts the thin
//! inode-level delegation entries: the per-call delegation helpers
//! (`select_real_inode`, `fs_arc`, `record_copyup_transition`) and the VFS
//! helper bodies called by the canonical `FileOps`/`Inode` trait impls on
//! `OverlayInode`. The real control flow lives in the sibling files:
//! `trigger.rs` (winner/waiter protocol + top-down ancestor walk),
//! `promote.rs` (object-kind promotion body and publication), `workdir.rs`
//! (temp lifecycle).
//!
//! ## Lock contract
//!
//! The delegation entries normally hold no overlay lock beyond the brief
//! `facts` hold inside `select_real_inode`. Exception:
//! `record_copyup_transition` takes the per-object copy-up coordination lock
//! (`copyup_transition`) with `try_lock`.
//!
//! ## Per-call delegation
//!
//! Every call re-resolves the current authority; there is no per-open
//! real-inode view object to reuse across calls.
//!
//! ## References
//!
//! - Linux `ovl_real_file_path` follow-copy-up:
//!   <https://elixir.bootlin.com/linux/latest/source/fs/overlayfs/file.c#L128-L171>

use core::sync::atomic::Ordering;

use self::coordination::{CopyUpPhase, CopyUpTransition};
use crate::{
    fs::{
        file::{AccessMode, PerOpenFileOps, Permission, StatusFlags},
        fs_impls::overlayfs::{AccessType, mount::OverlayFs, projection::OverlayInode},
        vfs::inode::{FallocMode, Inode, SymbolicLink},
    },
    prelude::*,
    vm::page_cache::Vmo,
};

pub(super) mod coordination;

pub(in crate::fs::fs_impls::overlayfs) mod promote;
mod trigger;
mod workdir;

pub(in crate::fs::fs_impls::overlayfs) use workdir::WorkdirTempRequest;

impl OverlayFs {
    /// Returns the next monotonic (non-decreasing), saturating per-mount
    /// workdir temp serial.
    ///
    /// The serial numbers the workdir temp names uniquely, so each staged temp
    /// gets a distinct name; it does not participate in I/O ordering or
    /// mutual exclusion.
    pub(super) fn workdir_temp_serial(&self) -> u64 {
        match self
            .workdir_temp_serial
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
                Some(current.saturating_add(1))
            }) {
            // The closure never returns `None`, so `try_update` always
            // succeeds; this arm is defensive and unreachable.
            Ok(previous) => previous.saturating_add(1),
            Err(_) => u64::MAX,
        }
    }
}

impl OverlayInode {
    /// Resolves the current authority's real inode for one delegated call.
    ///
    /// The `lowers[0]` index is safe by the facts invariant
    /// `upper.is_some() || !lowers.is_empty()`.
    pub(super) fn select_real_inode(&self) -> Arc<dyn Inode> {
        let facts = self.facts_snapshot();
        match facts.upper() {
            Some(upper) => upper.real_inode().clone(),
            None => facts.lowers()[0].real_inode().clone(),
        }
    }

    /// Returns `Err` when the inode does not belong to an overlay filesystem.
    pub(super) fn fs_arc(&self) -> Result<Arc<OverlayFs>> {
        let fs = self.fs();
        Arc::downcast::<OverlayFs>(fs).map_err(|_| {
            Error::with_message(
                Errno::EIO,
                "the inode does not belong to an overlay filesystem",
            )
        })
    }

    /// Records the copy-up transition coordinate at the first positive
    /// binding publication.
    ///
    /// The coordinate is set once — the first positive binding wins; the
    /// non-blocking `try_lock` skips when contended because a transition
    /// already running has already set it.
    pub(super) fn record_copyup_transition(
        &self,
        publication_parent: Arc<OverlayInode>,
        name: &str,
    ) {
        let Some(mut guard) = self.copyup_transition.try_lock() else {
            return;
        };
        if guard.is_some() {
            return;
        }
        *guard = Some(CopyUpTransition {
            publication_parent,
            name: String::from(name),
            phase: CopyUpPhase::Idle,
        });
    }
}

impl OverlayInode {
    // A lower-backed read passes `O_NOATIME` so a read never updates the lower
    // atime; the brief facts snapshots here and inside `select_real_inode` may
    // observe an authority advance between them, which is benign.
    pub(in crate::fs::fs_impls::overlayfs) fn read_at_impl(
        &self,
        offset: usize,
        writer: &mut VmWriter,
        status_flags: StatusFlags,
    ) -> Result<usize> {
        let facts = self.facts_snapshot();
        let is_lower_backed = facts.upper().is_none();
        let real = self.select_real_inode();
        let status_flags = if is_lower_backed {
            status_flags | StatusFlags::O_NOATIME
        } else {
            status_flags
        };
        real.read_at(offset, writer, status_flags)
    }

    // The `O_APPEND` branch serializes `offset := real size` + `write_at`
    // under the `facts` guard (`append_write`) — a bare two-step
    // size-read-then-write would be a TOCTOU where concurrent appends could
    // read the same size and lose an update. Write-capable fds are upper by
    // construction, so delegation never bypasses the trigger.
    pub(in crate::fs::fs_impls::overlayfs) fn write_at_impl(
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
}

impl OverlayInode {
    // The VFS handle uses this inode's own `FileOps`, so the successful path
    // returns `None`; failures surface as `Some(Err)`.
    pub(in crate::fs::fs_impls::overlayfs) fn open_impl(
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
        match self.ensure_upper_authority() {
            Ok(()) => None,
            Err(err) => Some(Err(err)),
        }
    }

    pub(in crate::fs::fs_impls::overlayfs) fn seek_end_impl(&self) -> Option<usize> {
        self.select_real_inode().seek_end()
    }

    // The path-based `truncate()` syscall performs no VFS-level `MAY_WRITE`
    // check of its own, so this entry runs the uniform mutating admission
    // before any side effect, including the copy-up promotion.
    pub(in crate::fs::fs_impls::overlayfs) fn resize_impl(&self, new_size: usize) -> Result<()> {
        if self.fs_arc()?.policy().is_effective_read_only() {
            return_errno_with_message!(Errno::EROFS, "the overlay mount is read-only");
        }
        self.check_permission(AccessType::Mutating, Permission::MAY_WRITE)?;
        self.ensure_upper_authority()?;
        self.select_real_inode().resize(new_size)
    }

    // Mutating admission is duplicated here (not delegated to the fd path):
    // `fallocate` shares `resize`'s side-effect class, so the admission also
    // runs at this entry rather than relying on the fd path alone.
    pub(in crate::fs::fs_impls::overlayfs) fn fallocate_impl(
        &self,
        mode: FallocMode,
        offset: usize,
        len: usize,
    ) -> Result<()> {
        if self.fs_arc()?.policy().is_effective_read_only() {
            return_errno_with_message!(Errno::EROFS, "the overlay mount is read-only");
        }
        self.check_permission(AccessType::Mutating, Permission::MAY_WRITE)?;
        self.ensure_upper_authority()?;
        self.select_real_inode().fallocate(mode, offset, len)
    }

    pub(in crate::fs::fs_impls::overlayfs) fn sync_all_impl(&self) -> Result<()> {
        self.select_real_inode().sync_all()
    }

    pub(in crate::fs::fs_impls::overlayfs) fn sync_data_impl(&self) -> Result<()> {
        self.select_real_inode().sync_data()
    }

    pub(in crate::fs::fs_impls::overlayfs) fn read_link_impl(&self) -> Result<SymbolicLink> {
        self.select_real_inode().read_link()
    }

    pub(in crate::fs::fs_impls::overlayfs) fn page_cache_impl(&self) -> Option<Arc<Vmo>> {
        self.select_real_inode().page_cache()
    }
}
