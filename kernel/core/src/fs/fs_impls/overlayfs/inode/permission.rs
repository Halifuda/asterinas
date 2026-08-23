// SPDX-License-Identifier: MPL-2.0
#![short_vis_path::add(overlayfs)]

//! The two-stage permission admission pipeline.
//!
//! This module hosts the single admission entry
//! [`OverlayInode::check_permission`] plus the two stage helpers, and the
//! shared current-credential probes used by the metadata entries.
//!
//! # Stages
//!
//! | Stage | Function |
//! |---|---|
//! | Local | [`OverlayInode::check_local_permission`] (EROFS gate + projected DAC). |
//! | Real | [`OverlayInode::check_real_permission`] (explicit real re-check). |
//!
//! The read-only `Inode::check_permission` forwarder calls this entry
//! with `AccessType::ReadOnly`, never promoting.

/// Classifies a projected overlayfs request as read-only or mutating for
/// permission checks and copy-up triggering.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum AccessType {
    ReadOnly,
    Mutating,
}

use super::OverlayInode;
use crate::{
    fs::{file::Permission, fs_impls::overlayfs::with_current_posix_thread, vfs::inode::Inode},
    prelude::*,
    process::{Gid, Uid, credentials::capabilities::CapSet},
    security::lsm::hooks as lsm_hooks,
};

/// Returns whether the current task holds `cap` in its user namespace
/// (the single shared capability probe of this module). Kernel contexts
/// fail open (`true` — no user to gate); a user context whose
/// thread-local (and thus user namespace) is absent fails closed
/// (`false` — no namespace to scope against).
pub(super) fn current_task_has_capability(cap: CapSet) -> bool {
    let Some(has_cap) = with_current_posix_thread(|task, posix_thread| {
        task.as_thread_local().is_some_and(|thread_local| {
            let user_ns = thread_local.borrow_user_ns();
            lsm_hooks::on_capable(lsm_hooks::CapableContext::new(
                user_ns.as_ref(),
                posix_thread,
                cap,
            ))
            .is_ok()
        })
    }) else {
        // Kernel contexts fail open: there is no user to gate.
        return true;
    };
    has_cap
}

/// Returns the current task's filesystem UID (`None` in a kernel context
/// — no task / no posix thread).
///
/// Callers treat `None` as "not the owner" (the shared kernel-context
/// default applied via `is_some_and`). Consumed by the ownership-sensitive
/// metadata logic and the source-side link admission.
pub(super) fn current_fsuid() -> Option<Uid> {
    let fsuid = with_current_posix_thread(|_, posix_thread| posix_thread.credentials().fsuid())?;
    Some(fsuid)
}

/// Returns whether `gid` equals the current task's filesystem group ID or
/// is in its supplementary group set (kernel contexts report `false`).
///
/// The fsgid disjunct closes the owner-chgrp exemption gap: without it, an owner
/// whose filesystem group ID is the target gid but whose supplementary
/// set omits it would be denied the owner-chgrp exemption.
pub(super) fn current_in_group(gid: Gid) -> bool {
    let Some(in_group) = with_current_posix_thread(|_, posix_thread| {
        let credentials = posix_thread.credentials();
        gid == credentials.fsgid() || credentials.groups().contains(&gid)
    }) else {
        return false;
    };
    in_group
}

impl OverlayInode {
    /// Runs the single admission gate for every projected-object request.
    ///
    /// The gate has three steps:
    /// 1. Lock-free local stage: EROFS gate plus projected DAC check.
    /// 2. Copy-up promotion: only for the `Mutating` class.
    /// 3. Real-handle re-check: unless `default_permissions` is enabled.
    ///
    /// Verdicts are never cached.
    pub(super) fn check_permission(&self, access: AccessType, perm: Permission) -> Result<()> {
        // This two-parameter form shadows the one-parameter
        // `Inode::check_permission` forwarder.
        // Read-only entries never promote, so the forwarder supplies only
        // `AccessType::ReadOnly`.
        self.check_local_permission(access, perm)?;
        if access == AccessType::Mutating {
            self.ensure_upper_authority()?;
        }
        if !self.fs_arc()?.policy().is_default_permissions() {
            self.check_real_permission(perm)?;
        }
        Ok(())
    }

    /// Runs the lock-free local half of the two-stage admission.
    ///
    /// First, the `EROFS` gate runs for the `Mutating` class. It performs no
    /// real-handle lookup, copy-up, or side effect. Then, for both classes,
    /// the projected-DAC block mirrors the VFS default
    /// `Inode::check_permission`, comparing the projected `metadata()` and
    /// the current credentials.
    fn check_local_permission(&self, access: AccessType, mut perm: Permission) -> Result<()> {
        if access == AccessType::Mutating && self.fs_arc()?.policy().is_effective_read_only() {
            return_errno_with_message!(Errno::EROFS, "the overlay mount is read-only");
        }

        // TODO: this block inlines the VFS default `Inode::check_permission`
        // because the VFS exposes no shared mode-DAC evaluator (VFS gap);
        // extract a shared `check_mode_dac` once the VFS interface
        // stabilizes.
        let Some(creds) = with_current_posix_thread(|_, posix_thread| posix_thread.credentials())
        else {
            return Ok(());
        };
        let metadata = self.metadata()?;
        let mode = metadata.mode;

        // With `DAC_OVERRIDE`, read/write DACs are always overridable; the
        // executable DAC is overridable only when at least one exec bit is
        // set.
        let has_dac_override = current_task_has_capability(CapSet::DAC_OVERRIDE);
        if has_dac_override {
            perm -= Permission::MAY_READ | Permission::MAY_WRITE;
            if perm.may_exec() {
                if mode.is_owner_executable()
                    || mode.is_group_executable()
                    || mode.is_other_executable()
                {
                    perm -= Permission::MAY_EXEC;
                } else {
                    return_errno_with_message!(
                        Errno::EACCES,
                        "root execute permission denied: no execute bits set"
                    );
                }
            }
        }

        if metadata.uid == creds.fsuid() {
            if (perm.may_read() && !mode.is_owner_readable())
                || (perm.may_write() && !mode.is_owner_writable())
                || (perm.may_exec() && !mode.is_owner_executable())
            {
                return_errno_with_message!(Errno::EACCES, "owner permission check failed");
            }
        } else if metadata.gid == creds.fsgid() {
            if (perm.may_read() && !mode.is_group_readable())
                || (perm.may_write() && !mode.is_group_writable())
                || (perm.may_exec() && !mode.is_group_executable())
            {
                return_errno_with_message!(Errno::EACCES, "group permission check failed");
            }
        } else if (perm.may_read() && !mode.is_other_readable())
            || (perm.may_write() && !mode.is_other_writable())
            || (perm.may_exec() && !mode.is_other_executable())
        {
            return_errno_with_message!(Errno::EACCES, "other permission check failed");
        }

        // No protected-state check is needed here:
        // protected names (e.g. `protattr`) are already excluded
        // by the xattr classification table.
        Ok(())
    }

    /// Runs the real-handle half of the two-stage permission check:
    /// re-resolves the current real authority per call and evaluates it
    /// directly. The explicit re-check is a benign double evaluation for
    /// xattr ops that self-evaluate under the caller's current credentials;
    /// failures propagate as-is with no invented rollback.
    fn check_real_permission(&self, perm: Permission) -> Result<()> {
        let real = self.select_real_inode();
        real.check_permission(perm)
    }
}
