// SPDX-License-Identifier: MPL-2.0

//! The overlayfs namespace-mutation and whiteout subsystem.
//!
//! This module hosts thin `Inode`-trait entries for directory name-space
//! mutations (create/mknod/link/unlink/rmdir/rename). Each entry resolves a
//! fresh projection of the target name under the parent directory transaction
//! lock and delegates the actual mutation to a per-directory recipe.
//!
//! Key concepts:
//! - **projection**: the overlay-visible answer for a `(parent, name)` pair —
//!   a positive binding for a visible object, or a negative binding for
//!   absence, opaque hiding, or whiteout hiding.
//! - **parent directory transaction**: the per-directory `Mutex` guard that
//!   serializes mutation recipes for one parent.
//! - **whiteout**: an upper-layer visibility barrier published when a
//!   lower-backed name is removed; the `whiteout` submodule owns its cache and
//!   publish mechanics.
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
    AccessType,
    projection::{Binding, BindingKey, NegativeBinding, OverlayInode, PositiveBinding},
};
use crate::{
    fs::{
        file::{InodeMode, InodeType, Permission},
        vfs::{
            inode::{Inode, MknodType, RenameMode},
            path::Path,
        },
    },
    prelude::*,
    process::credentials::capabilities::CapSet,
};

pub(super) mod whiteout;

mod create;
mod link;
mod remove;
mod rename;

/// Maps the `mknod` kind request to the overlay-visible object type.
///
/// `MknodType` has no `InodeType` conversion, so this match is the only
/// mapping.
pub(super) fn mknod_object_type(mknod: &MknodType) -> InodeType {
    match mknod {
        MknodType::NamedPipe => InodeType::NamedPipe,
        MknodType::CharDevice(_) => InodeType::CharDevice,
        MknodType::BlockDevice(_) => InodeType::BlockDevice,
    }
}

impl OverlayInode {
    // The symlink target is filled by the later `write_link` delegation.
    pub(in crate::fs::fs_impls::overlayfs) fn create_impl(
        &self,
        name: &str,
        type_: InodeType,
        mode: InodeMode,
    ) -> Result<Arc<dyn Inode>> {
        let _dir_guard = self.lock_dir_transaction();
        self.check_permission(AccessType::Mutating, Permission::MAY_WRITE)?;
        let projected: Arc<dyn Inode> = self.create_object(name, type_, mode, None)?;
        Ok(projected)
    }

    pub(in crate::fs::fs_impls::overlayfs) fn mknod_impl(
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
        let _dir_guard = self.lock_dir_transaction();
        self.check_permission(AccessType::Mutating, Permission::MAY_WRITE)?;
        let object_type = mknod_object_type(&type_);
        let projected: Arc<dyn Inode> = self.create_object(name, object_type, mode, Some(type_))?;
        Ok(projected)
    }

    // Thin delegation to the current authority with no promotion: the
    // created symlink is already upper-backed.
    pub(in crate::fs::fs_impls::overlayfs) fn write_link_impl(&self, target: &str) -> Result<()> {
        self.select_real_inode().write_link(target)
    }

    pub(in crate::fs::fs_impls::overlayfs) fn link_impl(
        &self,
        old: &Arc<dyn Inode>,
        name: &str,
    ) -> Result<()> {
        let _dir_guard = self.lock_dir_transaction();
        self.check_permission(AccessType::Mutating, Permission::MAY_WRITE)?;
        let fs = self.fs_arc()?;
        // Fresh target projection under the parent directory transaction
        // lock: a visible target is never silently replaced.
        let binding = fs.lookup_binding(&self.facts_snapshot(), name)?.binding;
        if matches!(&binding, Binding::Positive(_)) {
            return Err(Error::new(Errno::EEXIST));
        }
        let target_is_whiteout = matches!(
            &binding,
            Binding::Negative(NegativeBinding::HiddenByWhiteout(_))
        );
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
            OverlayInode::current_fsuid().is_some_and(|fsuid| fsuid == source_metadata.uid);
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
                && !OverlayInode::current_task_has_capability(CapSet::FOWNER)
            {
                return Err(Error::with_message(
                    Errno::EPERM,
                    "the link source is set-id and the caller lacks CAP_FOWNER",
                ));
            }
        }
        let source_path = self.link_source(&old_overlay)?;
        // A source with lower fallback makes this parent impure: persist the
        // impure marker to the upper parent before either physical-link
        // branch (before committing the link).
        if !old_overlay.facts_snapshot().lowers().is_empty() {
            fs.xattr_policy()
                .set_impure_marker(self.upper_parent_path()?.inode())?;
        }
        if target_is_whiteout {
            self.link_over_whiteout(name, &source_path)?;
        } else {
            self.upper_parent_path()?.link(&source_path, name)?;
        }
        // Inline target publication: the positive binding shares the source
        // `OverlayInode` — inode-cache reuse by `RealObjectKey`, so
        // `project_new_upper` is not needed — and `readdir_index_insert`
        // maintains the target parent index. Both steps are infallible, so
        // no reconcile is needed.
        let key = BindingKey::new(self.key(), String::from(name));
        let binding = Arc::new(Binding::Positive(PositiveBinding::new(old_overlay.clone())));
        fs.bindings().insert(key, binding);
        self.readdir_index_insert(name, old_overlay.clone(), old_overlay.type_());
        Ok(())
    }

    pub(in crate::fs::fs_impls::overlayfs) fn unlink_impl(&self, name: &str) -> Result<()> {
        let _dir_guard = self.lock_dir_transaction();
        self.check_permission(AccessType::Mutating, Permission::MAY_WRITE)?;
        self.remove_target(name, RemoveKind::Unlink)
    }

    pub(in crate::fs::fs_impls::overlayfs) fn rmdir_impl(&self, name: &str) -> Result<()> {
        let _dir_guard = self.lock_dir_transaction();
        self.check_permission(AccessType::Mutating, Permission::MAY_WRITE)?;
        self.remove_target(name, RemoveKind::Rmdir)
    }

    pub(in crate::fs::fs_impls::overlayfs) fn rename_impl(
        &self,
        old_name: &str,
        target: &Arc<dyn Inode>,
        new_name: &str,
        mode: RenameMode,
    ) -> Result<()> {
        // A foreign inode is a defensive error, never a silent cast.
        let target_overlay = Arc::downcast::<OverlayInode>(target.clone()).map_err(|_| {
            Error::with_message(Errno::EIO, "the rename target is not an overlay inode")
        })?;
        let (_source_guard, _target_guard) =
            self.lock_parent_dir_transactions(Some(&target_overlay))?;
        self.check_permission(AccessType::Mutating, Permission::MAY_WRITE)?;
        target_overlay.check_permission(AccessType::Mutating, Permission::MAY_WRITE)?;
        let fs = self.fs_arc()?;
        // Fresh source projection under the parent directory transaction
        // lock: the projection made while holding it is authoritative over a
        // stale VFS dentry.
        let source_binding = fs.lookup_binding(&self.facts_snapshot(), old_name)?.binding;
        let _source_inode = match &source_binding {
            Binding::Positive(positive) => positive.inode(),
            Binding::Negative(_) => return Err(Error::new(Errno::ENOENT)),
        };
        if !core::ptr::addr_eq(core::ptr::from_ref(self), Arc::as_ptr(&target_overlay)) {
            self.cross_device_gate(&source_binding)?;
        }
        // Whether the source name needs a whiteout after the move is decided
        // internally from the fresh source projection.
        self.rename_upper(old_name, &target_overlay, new_name, mode)
    }
}

impl OverlayInode {
    /// Returns the parent directory transaction guard
    /// (`MutexGuard<'_, ()>`) of this directory.
    pub(super) fn lock_dir_transaction(&self) -> MutexGuard<'_, ()> {
        match self.dir() {
            Some(dir) => dir.lock(),
            None => unreachable!(
                "mutation entries run on overlay directories only; the VFS routes child-name \
                 operations on directory inodes"
            ),
        }
    }

    /// Acquires the two affected parent directory transaction guards in
    /// stable object-identity order, each parent exactly once.
    ///
    /// `RealObjectKey` is not orderable, so the parents are ordered by
    /// `Arc::as_ptr`; the same-inode case acquires the single guard once.
    pub(super) fn lock_parent_dir_transactions<'a, 'b>(
        &'a self,
        other: Option<&'b Arc<OverlayInode>>,
    ) -> Result<(MutexGuard<'a, ()>, Option<MutexGuard<'b, ()>>)> {
        let self_dir = match self.dir() {
            Some(dir) => dir,
            None => {
                return Err(Error::with_message(
                    Errno::ENOTDIR,
                    "the source parent is not an overlay directory",
                ));
            }
        };
        let Some(other) = other else {
            return Ok((self_dir.lock(), None));
        };
        let other_dir = match other.dir() {
            Some(dir) => dir,
            None => {
                return Err(Error::with_message(
                    Errno::ENOTDIR,
                    "the target parent is not an overlay directory",
                ));
            }
        };
        let self_addr = core::ptr::from_ref(self);
        let other_addr = Arc::as_ptr(other);
        if core::ptr::addr_eq(self_addr, other_addr) {
            return Ok((self_dir.lock(), None));
        }
        if self_addr < other_addr {
            let self_guard = self_dir.lock();
            let other_guard = other_dir.lock();
            Ok((self_guard, Some(other_guard)))
        } else {
            let other_guard = other_dir.lock();
            let self_guard = self_dir.lock();
            Ok((self_guard, Some(other_guard)))
        }
    }

    /// Returns the dentry-anchored path of the promoted upper real parent
    /// directory.
    ///
    /// After promotion the facts guarantee an upper object that is always
    /// dentry-anchored, so the checked `real_path()` accessor succeeds;
    /// `EROFS`/`EIO` propagate when that guarantee does not hold.
    pub(super) fn upper_parent_path(&self) -> Result<Path> {
        let facts = self.facts_snapshot();
        let upper = facts.upper().ok_or_else(|| {
            Error::with_message(Errno::EROFS, "the overlay object has no upper real parent")
        })?;
        upper.real_path()
    }

    /// Reconciles the affected `(parent, name)` projections as a unit after
    /// a physical upper success whose semantic publication failed;
    /// best-effort, supports one- and two-parent operations.
    pub(super) fn invalidate_stale_cache(&self, affected: &[(&OverlayInode, &str)]) {
        let Ok(fs) = self.fs_arc() else {
            return;
        };
        for (parent, name) in affected {
            fs.bindings().invalidate(&parent.key(), name);
            parent.invalidate_readdir_index();
        }
    }
}
