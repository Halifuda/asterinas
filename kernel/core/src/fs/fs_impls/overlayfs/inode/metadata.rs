// SPDX-License-Identifier: MPL-2.0
#![short_vis_path::add(overlayfs)]

//! The metadata-mutation entries.
//!
//! This module hosts the six metadata setters: `set_mode`/`set_owner`/
//! `set_group` (chmod/chown) and `set_atime`/`set_mtime`/`set_ctime`
//! (utimes). Every entry admits through the two-stage
//! [`OverlayInode::check_permission`]/`AccessType::Mutating` pipeline — the
//! local permission check, copy-up promotion for the `Mutating` class, and
//! the real-handle re-check (unless `default_permissions`) — then forwards
//! to the real authority via [`OverlayInode::delegate_to_real`].
//!
//! # Ownership gate
//!
//! The pipeline checks mode DAC only: neither the syscall layer nor the
//! pipeline performs an owner/`CAP_FOWNER`/`CAP_CHOWN` pre-check. The
//! ownership-sensitive setters (`set_mode`, `set_owner`, `set_group`)
//! therefore run a local **ownership/capability gate** before the uniform
//! admission and admit with `Permission::empty()`, so ownership/capability
//! alone is authoritative and the chmod-000-then-chmod-644 owner idiom never
//! fails with `EACCES` — a deliberate deviation from a `MAY_WRITE`-only
//! shape.
//!
//! The time setters follow the utimensat disjunction instead: `MAY_WRITE`,
//! or an owner/`CAP_FOWNER` fallback that re-runs admission with
//! `Permission::empty()`; failures are best-effort silent no-ops.

use core::time::Duration;

use super::{
    OverlayInode,
    permission::{AccessType, current_fsuid, current_in_group, current_task_has_capability},
};
use crate::{
    fs::{
        file::{InodeMode, Permission},
        vfs::inode::Inode,
    },
    prelude::*,
    process::{Gid, Uid, credentials::capabilities::CapSet},
};

impl OverlayInode {
    pub(super) fn set_mode_impl(&self, mode: InodeMode) -> Result<()> {
        let metadata = self.metadata()?;
        let is_owner = current_fsuid().is_some_and(|fsuid| fsuid == metadata.uid);
        let has_cap = current_task_has_capability(CapSet::FOWNER);
        if !is_owner && !has_cap {
            return Err(Error::with_message(
                Errno::EPERM,
                "the caller is not the file owner and lacks CAP_FOWNER",
            ));
        }
        // A non-owner without `CAP_FSETID` cannot stamp set-id bits onto the
        // file.
        let has_fsetid = current_task_has_capability(CapSet::FSETID);
        let mut mode = mode;
        if !is_owner && !has_fsetid {
            mode.remove(InodeMode::S_ISUID | InodeMode::S_ISGID);
        }
        self.check_permission(AccessType::Mutating, Permission::empty())?;
        self.delegate_to_real(|real| real.set_mode(mode))
    }

    pub(super) fn set_owner_impl(&self, uid: Uid) -> Result<()> {
        let metadata = self.metadata()?;
        if uid != metadata.uid && !current_task_has_capability(CapSet::CHOWN) {
            return Err(Error::with_message(
                Errno::EPERM,
                "the caller lacks CAP_CHOWN for an ownership change",
            ));
        }
        self.check_permission(AccessType::Mutating, Permission::empty())?;
        self.delegate_to_real(|real| real.set_owner(uid))
    }

    pub(super) fn set_group_impl(&self, gid: Gid) -> Result<()> {
        let metadata = self.metadata()?;
        if gid != metadata.gid {
            let is_owner = current_fsuid().is_some_and(|fsuid| fsuid == metadata.uid);
            let has_cap = current_task_has_capability(CapSet::CHOWN);
            // The owner-chgrp exemption: the owner may change the group to one
            // of its own supplementary groups (kernel contexts default to
            // `false`).
            let in_own_group = current_in_group(gid);
            if !has_cap && !(is_owner && in_own_group) {
                return Err(Error::with_message(
                    Errno::EPERM,
                    "the caller lacks CAP_CHOWN for a group change",
                ));
            }
        }
        self.check_permission(AccessType::Mutating, Permission::empty())?;
        self.delegate_to_real(|real| real.set_group(gid))
    }

    pub(super) fn set_atime_impl(&self, time: Duration) {
        self.best_effort_time_set(|real| real.set_atime(time));
    }

    pub(super) fn set_mtime_impl(&self, time: Duration) {
        self.best_effort_time_set(|real| real.set_mtime(time));
    }

    pub(super) fn set_ctime_impl(&self, time: Duration) {
        self.best_effort_time_set(|real| real.set_ctime(time));
    }
}

impl OverlayInode {
    /// Runs one best-effort time setter (the infallible VFS time-setter surface
    /// makes failures silent no-ops).
    fn best_effort_time_set(&self, operation_fn: impl FnOnce(&Arc<dyn Inode>)) {
        let Some(metadata) = self.metadata().ok() else {
            return;
        };
        let is_owner = current_fsuid().is_some_and(|fsuid| fsuid == metadata.uid);
        let has_cap = current_task_has_capability(CapSet::FOWNER);
        if self
            .check_permission(AccessType::Mutating, Permission::MAY_WRITE)
            .is_err()
        {
            if !is_owner && !has_cap {
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
