// SPDX-License-Identifier: MPL-2.0

//! The overlayfs namespace-mutation and whiteout subsystem.
//!
//! This module hosts the `Inode`-trait entries for directory name-space
//! mutations (create/mknod/link/unlink/rmdir/rename). Each entry resolves a
//! fresh projection of the target name under the parent directory transaction
//! lock and delegates the actual mutation to a per-directory recipe.
//!
//! Key concepts:
//! - **lookup**: the overlay-visible answer for a `(parent, name)` pair — a
//!   positive inode, or a negative reason (`Absent`, `HiddenByWhiteout`,
//!   `HiddenByOpaque`).
//! - **parent directory transaction**: the per-directory `Mutex` guard that
//!   serializes mutation recipes for one parent.
//! - **whiteout**: an upper-layer visibility barrier published when a
//!   lower-backed name is removed; the `whiteout` submodule owns its cache and
//!   publish mechanics.
//! - **entry admission contract**: the six namespace-mutation entries run
//!   `check_permission(Mutating, MAY_WRITE)` (including any required
//!   copy-up promotion) before acquiring the parent directory transaction
//!   lock; rename additionally pre-promotes the source before taking either
//!   parent lock. The recipes therefore assume the caller already admitted
//!   the request and do not re-check permission.
//!
//! ## Structure
//!
//! | Submodule | Responsibility |
//! | --- | --- |
//! | `create` | create-object recipes (absent and over-whiteout branches) |
//! | `link` | hard-link recipe |
//! | `remove` | shared unlink/rmdir recipe and whiteout-publish removal |
//! | `rename` | rename recipe |
//! | `whiteout` | shared whiteout cache and whiteout-publish mechanics |

use self::remove::RemoveKind;
use super::{
    AccessType, Lookup, NegativeLookup, OverlayInode, ReaddirIndex, xattr::set_impure_marker,
};
use crate::{
    fs::{
        file::{InodeMode, InodeType, Permission},
        vfs::inode::{Inode, MknodType, RenameMode},
    },
    prelude::*,
    process::credentials::capabilities::CapSet,
};

pub(super) mod whiteout;

mod create;
mod link;
mod remove;
mod rename;

/// The two parent transaction guards acquired by rename.
type DirLockPair<'a, 'b> = (
    MutexGuard<'a, Option<ReaddirIndex>>,
    Option<MutexGuard<'b, Option<ReaddirIndex>>>,
);

impl OverlayInode {
    // The symlink target is filled by the later `write_link` delegation.
    pub(super) fn create_impl(
        &self,
        name: &str,
        type_: InodeType,
        mode: InodeMode,
    ) -> Result<Arc<dyn Inode>> {
        self.check_permission(AccessType::Mutating, Permission::MAY_WRITE)?;
        let mut dir_guard = self.lock_dir_transaction();
        let projected: Arc<dyn Inode> =
            self.create_object(name, type_, mode, None, &mut dir_guard)?;
        Ok(projected)
    }

    pub(super) fn mknod_impl(
        &self,
        name: &str,
        mode: InodeMode,
        type_: MknodType,
    ) -> Result<Arc<dyn Inode>> {
        if matches!(&type_, MknodType::CharDevice(0)) {
            return_errno_with_message!(
                Errno::EPERM,
                "a raw 0:0 whiteout char device must not be user-creatable"
            );
        }
        self.check_permission(AccessType::Mutating, Permission::MAY_WRITE)?;
        let mut dir_guard = self.lock_dir_transaction();
        let object_type = crate::fs::fs_impls::overlayfs::mknod_object_type(&type_);
        let projected: Arc<dyn Inode> =
            self.create_object(name, object_type, mode, Some(type_), &mut dir_guard)?;
        Ok(projected)
    }

    /// Detects a stale upper-backed name: `current_index` still remembers an
    /// upper-backed inode for `name`, but `fresh_lookup` no longer resolves to
    /// that same object.
    pub(super) fn is_stale_upper(
        &self,
        name: &str,
        fresh_lookup: &Lookup,
        current_index: &Option<ReaddirIndex>,
    ) -> bool {
        let Some(old_inode) = current_index
            .as_ref()
            .and_then(|idx| idx.visible_inode(name))
        else {
            return false;
        };
        if old_inode.upper.get().is_none() {
            return false;
        }
        match fresh_lookup {
            Lookup::Positive(inode) => !Arc::ptr_eq(inode, &old_inode),
            Lookup::Negative(negative) => *negative != NegativeLookup::HiddenByWhiteout,
        }
    }

    // Thin delegation to the current authority with no promotion: the
    // created symlink is already upper-backed.
    pub(super) fn write_link_impl(&self, target: &str) -> Result<()> {
        self.select_real_inode().write_link(target)
    }

    pub(super) fn link_impl(&self, old: &Arc<dyn Inode>, name: &str) -> Result<()> {
        // Admission and source promotion run before the parent transaction
        // lock so no lock is ever held while copy-up takes a CUL.
        self.check_permission(AccessType::Mutating, Permission::MAY_WRITE)?;
        // The source must be an Overlay inode (the VFS passes an inode of
        // this filesystem); a foreign inode is a defensive error, never a
        // silent cast.
        let old_overlay = Arc::downcast::<OverlayInode>(old.clone()).map_err(|_| {
            Error::with_message(Errno::EIO, "the link source is not an overlay inode")
        })?;
        // The `link` syscall performs no source check of its own (VFS gap),
        // so this source-side check is required; the base permission check
        // still remains authoritative.
        let source_metadata = old_overlay.metadata()?;
        let source_owned =
            super::permission::current_fsuid().is_some_and(|fsuid| fsuid == source_metadata.uid);
        // Non-owner source checks: when there is no current task/posix thread
        // (kernel-internal context), `current_fsuid()` is `None` and the
        // permission probe permits the operation (no user credential exists
        // to be checked); in a user context, the source-side checks below
        // run in addition to the parent's base permission check.
        if !source_owned {
            if old_overlay
                .check_permission(
                    AccessType::ReadOnly,
                    Permission::MAY_READ | Permission::MAY_WRITE,
                )
                .is_err()
            {
                return Err(Error::with_message(
                    Errno::EPERM,
                    "the link source is not accessible to the caller",
                ));
            }
            if old_overlay.type_() != InodeType::File {
                return Err(Error::with_message(
                    Errno::EPERM,
                    "the link source is not a regular file",
                ));
            }
            if (source_metadata.mode.has_set_uid()
                || (source_metadata.mode.has_set_gid()
                    && source_metadata.mode.is_group_executable()))
                && !super::permission::current_task_has_capability(CapSet::FOWNER)
            {
                return Err(Error::with_message(
                    Errno::EPERM,
                    "the link source is set-id and the caller lacks CAP_FOWNER",
                ));
            }
        }
        // Source promotion is also part of the pre-lock admission: the shared
        // upper path must be ready before the parent lock serializes the
        // target-name mutation.
        let source_path = self.link_source(&old_overlay)?;
        let fs = self.fs_arc()?;
        let mut dir_guard = self.lock_dir_transaction();
        // Fresh target projection under the parent directory transaction
        // lock: link expects a negative target; a fresh positive means that
        // expectation became stale, so surface ESTALE.
        let target_lookup = fs.lookup(self, name)?;
        if matches!(target_lookup, Lookup::Positive(_)) {
            return Err(Error::new(Errno::ESTALE));
        }
        let target_is_whiteout = matches!(
            target_lookup,
            Lookup::Negative(NegativeLookup::HiddenByWhiteout)
        );
        // A source with lower fallback makes this parent impure: persist the
        // impure marker to the upper parent before either physical-link
        // branch (before committing the link).
        if !old_overlay.lowers.is_empty() {
            set_impure_marker(self.upper_parent_path()?.inode())?;
        }
        if target_is_whiteout {
            self.link_over_whiteout(name, &source_path)?;
        } else {
            self.upper_parent_path()?.link(&source_path, name)?;
        }
        // The new name shares the source `OverlayInode`; only the readdir
        // index needs maintenance.
        self.readdir_index_insert(
            name,
            old_overlay.clone(),
            old_overlay.type_(),
            &mut dir_guard,
        );
        Ok(())
    }

    pub(super) fn unlink_impl(&self, name: &str) -> Result<()> {
        self.check_permission(AccessType::Mutating, Permission::MAY_WRITE)?;
        let mut dir_guard = self.lock_dir_transaction();
        self.remove_target(name, RemoveKind::Unlink, &mut dir_guard)
    }

    pub(super) fn rmdir_impl(&self, name: &str) -> Result<()> {
        self.check_permission(AccessType::Mutating, Permission::MAY_WRITE)?;
        let mut dir_guard = self.lock_dir_transaction();
        self.remove_target(name, RemoveKind::Rmdir, &mut dir_guard)
    }

    pub(super) fn rename_impl(
        &self,
        old_name: &str,
        old_inode: &Arc<dyn Inode>,
        new_dir_inode: &Arc<dyn Inode>,
        new_name: &str,
        replaced_inode: Option<&Arc<dyn Inode>>,
        mode: RenameMode,
    ) -> Result<()> {
        // A foreign inode is a defensive error, never a silent cast.
        let source_overlay = Arc::downcast::<OverlayInode>(old_inode.clone()).map_err(|_| {
            Error::with_message(Errno::EIO, "the rename source is not an overlay inode")
        })?;
        let target_overlay =
            Arc::downcast::<OverlayInode>(new_dir_inode.clone()).map_err(|_| {
                Error::with_message(Errno::EIO, "the rename target is not an overlay inode")
            })?;
        // Both parent admission checks run before any parent lock is taken:
        // each `Mutating` check promotes that directory to upper authority
        // without holding the transaction lock.
        self.check_permission(AccessType::Mutating, Permission::MAY_WRITE)?;
        target_overlay.check_permission(AccessType::Mutating, Permission::MAY_WRITE)?;
        // The VFS-provided source inode is used for the pre-lock source
        // promotion and the EXDEV gate; `rename_upper` performs a fresh
        // liveness recheck after the parent locks are taken.
        source_overlay.copy_up()?;
        if !core::ptr::addr_eq(core::ptr::from_ref(self), Arc::as_ptr(&target_overlay)) {
            self.cross_device_gate(&source_overlay)?;
        }
        let (mut source_guard, mut target_guard) =
            self.lock_parent_dir_transactions(Some(&target_overlay))?;
        self.rename_upper(
            old_name,
            &source_overlay,
            &target_overlay,
            new_name,
            replaced_inode,
            mode,
            rename::RenameLocks {
                self_index: &mut source_guard,
                target_index: target_guard.as_deref_mut(),
            },
        )
    }
}

impl OverlayInode {
    /// Returns the per-inode transaction guard for this directory.
    ///
    /// The payload is `Some(ReaddirIndex)` for directories; non-directories
    /// still carry the lock as a plain serialization token.
    fn lock_dir_transaction(&self) -> MutexGuard<'_, Option<ReaddirIndex>> {
        self.lock.lock()
    }

    /// Acquires the two affected parent directory transaction guards in
    /// stable object-identity order, each parent exactly once.
    ///
    /// `RealObjectKey` is not orderable, so the parents are ordered by
    /// `Arc::as_ptr`; the same-inode case acquires the single guard once.
    fn lock_parent_dir_transactions<'a, 'b>(
        &'a self,
        other: Option<&'b Arc<OverlayInode>>,
    ) -> Result<DirLockPair<'a, 'b>> {
        if self.lock.lock().is_none() {
            return Err(Error::with_message(
                Errno::ENOTDIR,
                "the source parent is not an overlay directory",
            ));
        }
        let Some(other) = other else {
            return Ok((self.lock.lock(), None));
        };
        if other.lock.lock().is_none() {
            return Err(Error::with_message(
                Errno::ENOTDIR,
                "the target parent is not an overlay directory",
            ));
        }
        let self_addr = core::ptr::from_ref(self);
        let other_addr = Arc::as_ptr(other);
        if core::ptr::addr_eq(self_addr, other_addr) {
            return Ok((self.lock.lock(), None));
        }
        if self_addr < other_addr {
            let self_guard = self.lock.lock();
            let other_guard = other.lock.lock();
            Ok((self_guard, Some(other_guard)))
        } else {
            let other_guard = other.lock.lock();
            let self_guard = self.lock.lock();
            Ok((self_guard, Some(other_guard)))
        }
    }
}
