// SPDX-License-Identifier: MPL-2.0

//! The metadata-mutation entries.
//!
//! This module hosts the six metadata setters: `set_mode`/`set_owner`/
//! `set_group` (chmod/chown) and `set_atime`/`set_mtime`/`set_ctime`
//! (utimes). **Admission** is the two-stage permission-pipeline check
//! through [`OverlayInode::check_permission`]/`AccessType::Mutating`; the
//! **creator-credential scope** is the mount creator's credential set in
//! which each forward resolves its real authority.
//!
//! Every entry admits through the pipeline, then forwards to the real
//! authority in the creator-credential scope via `delegate_to_real`.
//! **Gate** is the mandatory admission check before touching the real
//! inode: the ownership/capability gate when ownership-sensitive, the
//! two-stage `check_permission` pipeline (`EROFS` gate + copy-up
//! promotion), and the real-handle re-check. The real stage already ran
//! inside admission, so the forward neither re-runs nor skips that gate.
//!
//! # Ownership gate
//!
//! The uniform mutating admission is the two-stage
//! [`OverlayInode::check_permission`]/`AccessType::Mutating` pipeline:
//! the local permission check, copy-up promotion for the `Mutating`
//! class, and the real-handle re-check (unless `default_permissions`).
//! Because the real stage runs under root credentials, the
//! ownership-sensitive setters additionally run a local
//! **ownership/capability gate** before that uniform admission; this is
//! a deliberate deviation from a `MAY_WRITE`-only shape.
//!
//! ## Local ownership/capability gate
//!
//! The gate's two inputs are `is_owner` (`fsuid == projected uid`) and
//! `has_cap` (the probed capability; kernel contexts fail open). The
//! chmod/chown syscall layer performs no owner/`CAP_FOWNER`/`CAP_CHOWN`
//! pre-check and the real stage runs under root credentials, so this
//! gate is the last line. The setters admit with `Permission::empty()`
//! (never `MAY_WRITE`), so the gate is authoritative; ownership/
//! capability alone is required — never write access (the
//! chmod-000-then-chmod-644 owner idiom must not fail `EACCES`).
//!
//! ## Time-setter admission
//!
//! The three time setters admit with the utimensat disjunction:
//! `Permission::MAY_WRITE`, or an ownership/`CAP_FOWNER` fallback that
//! re-runs the mutating admission with `Permission::empty()` (so the
//! EROFS gate and copy-up promotion still run on the owner path).
//! They are infallible VFS surfaces, best-effort: a local or real
//! failure is a silent no-op at the overlay boundary. Read-driven
//! atime updates stay with the copy-up module's `O_NOATIME`
//! delegation; this module never models a read as a copy-up trigger.
//!
//! # References
//!
//! - <https://elixir.bootlin.com/linux/v6.16.9/source/fs/attr.c#L161-L226>
//! - <https://elixir.bootlin.com/linux/v6.16.9/source/fs/open.c#L753-L790>
//! - <https://elixir.bootlin.com/linux/v6.16.9/source/kernel/groups.c#L227-L237>

use core::time::Duration;

use crate::{
    fs::{
        file::{InodeMode, Permission},
        fs_impls::overlayfs::{AccessType, projection::OverlayInode},
        vfs::inode::Inode,
    },
    prelude::*,
    process::{Gid, Uid, credentials::capabilities::CapSet},
};

/// The ownership/capability facts of the current caller against one projected
/// owner.
///
/// The `is_owner`/`has_cap` pair is the gate decision shared by the
/// ownership-sensitive setters and the best-effort time-setter gate.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct CallerOwnerFacts {
    /// `fsuid == projected uid`.
    ///
    /// `fsuid` is the filesystem credential used for ownership checks,
    /// not the process's real or effective `uid`; a caller can set `fsuid`
    /// independently, so comparing it preserves Linux ownership semantics.
    /// See [`OverlayInode::current_fsuid`].
    is_owner: bool,
    /// `true` when the caller holds the probed capability in its user
    /// namespace. Kernel contexts fail open (`true` — there is no user to
    /// gate); ordinary user contexts without a thread-local/user namespace
    /// fail closed (`false`). See
    /// [`OverlayInode::current_task_has_capability`].
    has_cap: bool,
}

impl OverlayInode {
    pub(in crate::fs::fs_impls::overlayfs) fn set_mode_impl(&self, mode: InodeMode) -> Result<()> {
        let metadata = self.metadata()?;
        let facts = self.caller_owner_facts(metadata.uid, CapSet::FOWNER);
        if !facts.is_owner && !facts.has_cap {
            return Err(Error::with_message(
                Errno::EPERM,
                "the caller is not the file owner and lacks CAP_FOWNER",
            ));
        }
        // A non-owner without `CAP_FSETID` cannot stamp set-id bits onto the
        // file.
        let has_fsetid = self
            .caller_owner_facts(metadata.uid, CapSet::FSETID)
            .has_cap;
        let mut mode = mode;
        if !facts.is_owner && !has_fsetid {
            mode.remove(InodeMode::S_ISUID | InodeMode::S_ISGID);
        }
        self.check_permission(AccessType::Mutating, Permission::empty())?;
        self.delegate_to_real(|real| real.set_mode(mode))
    }

    pub(in crate::fs::fs_impls::overlayfs) fn set_owner_impl(&self, uid: Uid) -> Result<()> {
        let metadata = self.metadata()?;
        if uid != metadata.uid && !self.caller_owner_facts(metadata.uid, CapSet::CHOWN).has_cap {
            return Err(Error::with_message(
                Errno::EPERM,
                "the caller lacks CAP_CHOWN for an ownership change",
            ));
        }
        self.check_permission(AccessType::Mutating, Permission::empty())?;
        self.delegate_to_real(|real| real.set_owner(uid))
    }

    pub(in crate::fs::fs_impls::overlayfs) fn set_group_impl(&self, gid: Gid) -> Result<()> {
        let metadata = self.metadata()?;
        if gid != metadata.gid {
            let facts = self.caller_owner_facts(metadata.uid, CapSet::CHOWN);
            // The owner-chgrp exemption: the owner may change the group to one
            // of its own supplementary groups (kernel contexts default to
            // `false`).
            let in_own_group = OverlayInode::current_in_group(gid);
            if !facts.has_cap && !(facts.is_owner && in_own_group) {
                return Err(Error::with_message(
                    Errno::EPERM,
                    "the caller lacks CAP_CHOWN for a group change",
                ));
            }
        }
        self.check_permission(AccessType::Mutating, Permission::empty())?;
        self.delegate_to_real(|real| real.set_group(gid))
    }

    pub(in crate::fs::fs_impls::overlayfs) fn set_atime_impl(&self, time: Duration) {
        self.best_effort_time_set(|real| real.set_atime(time));
    }

    pub(in crate::fs::fs_impls::overlayfs) fn set_mtime_impl(&self, time: Duration) {
        self.best_effort_time_set(|real| real.set_mtime(time));
    }

    pub(in crate::fs::fs_impls::overlayfs) fn set_ctime_impl(&self, time: Duration) {
        self.best_effort_time_set(|real| real.set_ctime(time));
    }
}

impl OverlayInode {
    /// Resolves the caller's ownership/capability facts against the given
    /// projected owner.
    ///
    /// Consumed by the ownership-sensitive setters and the best-effort
    /// time-setter gate.
    fn caller_owner_facts(&self, projected_uid: Uid, cap: CapSet) -> CallerOwnerFacts {
        let is_owner = OverlayInode::current_fsuid().is_some_and(|fsuid| fsuid == projected_uid);
        CallerOwnerFacts {
            is_owner,
            has_cap: OverlayInode::current_task_has_capability(cap),
        }
    }

    /// Runs one best-effort time setter (the infallible VFS time-setter surface
    /// makes failures silent no-ops).
    fn best_effort_time_set(&self, operation_fn: impl FnOnce(&Arc<dyn Inode>)) {
        let Some(metadata) = self.metadata().ok() else {
            return;
        };
        let facts = self.caller_owner_facts(metadata.uid, CapSet::FOWNER);
        if self
            .check_permission(AccessType::Mutating, Permission::MAY_WRITE)
            .is_err()
        {
            if !facts.is_owner && !facts.has_cap {
                return;
            }
            if self
                .check_permission(AccessType::Mutating, Permission::empty())
                .is_err()
            {
                return;
            }
        }
        let _ = self.delegate_to_real(|real| {
            operation_fn(real);
            Ok(())
        });
    }
}
