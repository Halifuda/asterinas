// SPDX-License-Identifier: MPL-2.0

//! The two-stage permission admission pipeline of the `metadata_security`
//! meso (meso-05; `P1-18`).
//!
//! This module hosts the frozen meso-05 spec §4 (`metadata_security/
//! permission.rs`) surface: the single admission entry
//! [`OverlayInode::check_permission`] (the two BC-5 §48-49 stages split into
//! two private helpers — the copy-up sits between the stages, so a fused
//! pipeline is wrong, packet revision 01 override 2), the lock-free local
//! stage [`OverlayInode::check_local_permission`] (EROFS gate for the
//! `Mutating` class + the projected-DAC block), the real-handle stage
//! [`OverlayInode::check_real_permission`] (copy-up promotion via the meso-04
//! authority seam, then the explicit real check under the creator-credential
//! scope). The canonical read-only `Inode::check_permission` forwarder lives
//! in `projection/inode.rs`; it calls this two-parameter inherent admission
//! entry with `AccessType::ReadOnly` and never promotes.
//!
//! Pipeline (frozen, BC-5 §48-49): the local stage always runs first and is
//! entirely lock-free; `default_permissions` skips only the real/creator-
//! credential stage, never the local stage (BC-5 §49; meso-01
//! `MountPolicy::is_default_permissions`); the real stage places the copy-up
//! inside `ensure_upper_authority()` and then evaluates the current real
//! authority (`select_real_inode()`) under the mount's creator-credential
//! scope. The explicit real check is authoritative for entries whose
//! underlying real ops do not self-evaluate (ext2/ramfs metadata setters —
//! §4.0 evidence) and is a benign double evaluation for xattr ops that
//! self-evaluate under the same scope (kept for gate independence).
//!
//! The local DAC block mirrors the VFS default `Inode::check_permission`
//! algorithm (`kernel/src/fs/vfs/fs_apis/inode.rs:556-611`) against the
//! projected `OverlayInode::metadata()` (mode/uid/gid from meso-02), with the
//! `DAC_OVERRIDE` reduction via `lsm_hooks::on_capable`. It is inlined here
//! by the frozen spec (recorded §8 note 3: no reusable kernel helper exists);
//! the `P2-06` protected-state admission is an insertion point (no-op this
//! wave).
//!
//! Lock contract (spec §3): this module acquires no Overlay lock. The local
//! stage is lock-free (brief `INODE` facts snapshot inside `metadata()`,
//! released before any use); the real stage enters the meso-04 authority seam
//! (`DIR -> CUL -> INODE -> WL -> UPPER` frozen order) without holding
//! anything; the creator-credential scope is a task-credential swap, not a
//! lock (P1-19 seam). No Overlay lock crosses the entry boundary, and no
//! `.unwrap()`/`.expect()` is used anywhere in this security gate.

use ostd::task::Task;

use crate::{
    fs::{
        file::Permission,
        fs_impls::overlayfs::{AccessType, projection::OverlayInode},
        vfs::inode::Inode,
    },
    prelude::*,
    process::{Gid, Uid, credentials::capabilities::CapSet, posix_thread::AsPosixThread},
    security::lsm::hooks as lsm_hooks,
};

impl OverlayInode {
    /// The single admission method of this Meso (P1-18 security gate): the
    /// two-stage permission pipeline every projected-object request funnels
    /// through.
    ///
    /// The local stage (lock-free) always runs first and may reject with
    /// `EROFS` (mutating class on an effective read-only mount) or `EACCES`
    /// (projected-DAC demand denied) with no real handle and no copy-up/
    /// workdir/temp/upper side effect (BC-5 §49.1). Unless the mount was
    /// created with `default_permissions`, the real stage then promotes
    /// mutating requests (`ensure_upper_authority()`, meso-04 seam — the
    /// copy-up lives between the stages) and re-evaluates the current real
    /// authority under the creator-credential scope (BC-5 §49.2). The
    /// `default_permissions` skip omits only the real/creator-credential
    /// stage, never the local stage. A real-stage failure propagates as-is
    /// with no invented rollback (meso-04 owns any already-started transition
    /// cleanup). Verdicts are never cached.
    ///
    /// Arity-overload note (frozen): this two-parameter inherent method
    /// coexists with the one-parameter `Inode::check_permission` forwarder
    /// in `projection/inode.rs`; Rust method resolution prefers the inherent
    /// method when the arity matches, so trait callers reach the read-only
    /// forwarder and meso entries call this one.
    pub(in crate::fs::fs_impls::overlayfs) fn check_permission(
        &self,
        access: AccessType,
        perm: Permission,
    ) -> Result<()> {
        self.check_local_permission(access, perm)?;
        if !self.fs_arc()?.policy().is_default_permissions() {
            self.check_real_permission(access, perm)?;
        }
        Ok(())
    }

    /// Returns whether the current task holds `cap` in its user namespace
    /// (wave-4 round-4 repair item 3, refined by round-5 repair item 2 —
    /// the single shared capability probe of this Meso).
    ///
    /// A process-global probe, so it is an associated function (no `&self`
    /// receiver). Probes through `lsm_hooks::on_capable` with the current
    /// task's posix thread and user namespace (the
    /// `check_local_permission` machinery). Kernel contexts fail open: with
    /// no current task, or no posix thread (a kernel-internal operation,
    /// not a user process), the probe reports `true` — there is no user to
    /// gate (the `check_local_permission` no-task/no-posix-thread
    /// precedent). A user context whose thread-local (and thus user
    /// namespace) is absent reports `false` — fail-closed, since there is
    /// no namespace against which the capability can be scoped. Whitelist
    /// Rule B: consumed by the permission stage (`check_local_permission`,
    /// DAC_OVERRIDE) and by the metadata ownership gates (`metadata.rs`).
    pub(super) fn current_task_has_capability(cap: CapSet) -> bool {
        let Some(task) = Task::current() else {
            return true;
        };
        let Some(posix_thread) = task.as_posix_thread() else {
            return true;
        };
        task.as_thread_local().is_some_and(|thread_local| {
            let user_ns = thread_local.borrow_user_ns();
            lsm_hooks::on_capable(lsm_hooks::CapableContext::new(
                user_ns.as_ref(),
                posix_thread,
                cap,
            ))
            .is_ok()
        })
    }

    /// Returns the current task's filesystem UID (`None` in a kernel
    /// context — no task / no posix thread; wave-4 round-5 repair item 3).
    ///
    /// Callers treat `None` as "not the owner" (the shared kernel-context
    /// default applied via `is_some_and`). Whitelist Rule B: consumed by
    /// `metadata.rs::caller_owner_facts` and `dir/mod.rs::link`'s
    /// source-side admission.
    pub(in crate::fs::fs_impls::overlayfs) fn current_fsuid() -> Option<Uid> {
        let task = Task::current()?;
        let posix_thread = task.as_posix_thread()?;
        Some(posix_thread.credentials().fsuid())
    }

    /// Returns whether the current task's filesystem group ID or
    /// supplementary group set contains `gid` — Linux `in_group_p` semantics
    /// (`kernel/groups.c` `in_group_p`: `!gid_eq(grp, cred->fsgid)` then
    /// `groups_search(cred->group_info, grp)`; pre-wave5 C4).
    ///
    /// Kernel contexts (no task / no posix thread) report `false` — the
    /// shared kernel-context default, applied in one place. The fsgid
    /// disjunct is the pre-wave5 C4 completion: without it, an owner whose
    /// filesystem group ID (`fsgid`) is the target gid but whose
    /// supplementary set omits it was denied the owner-chgrp exemption.
    /// Whitelist Rule B: consumed
    /// by `metadata.rs::set_group`'s owner-chgrp exemption.
    pub(in crate::fs::fs_impls::overlayfs) fn current_in_group(gid: Gid) -> bool {
        let Some(task) = Task::current() else {
            return false;
        };
        let Some(posix_thread) = task.as_posix_thread() else {
            return false;
        };
        let credentials = posix_thread.credentials();
        gid == credentials.fsgid() || credentials.groups().contains(&gid)
    }

    /// PRIVATE STAGE A — the lock-free local half of the two-stage check.
    ///
    /// For the `Mutating` class, the `EROFS` gate (`MountPolicy::
    /// is_effective_read_only`, P0-18) runs first — before the DAC block —
    /// so a read-only mount fails with no real handle, no copy-up, and no
    /// workdir/temp/upper side effect (BC-5 §49.1). The projected-DAC block
    /// then mirrors the VFS default `Inode::check_permission` algorithm
    /// (`inode.rs:556-611`, inlined per spec §8 note 3) against the projected
    /// `metadata()` (meso-02 mode/uid/gid) and the current task's credentials
    /// (`fsuid`/`fsgid`), with the `DAC_OVERRIDE` reduction via
    /// `lsm_hooks::on_capable`. `Permission::empty()` passes trivially. The
    /// `P2-06` protected-state admission is an insertion point (no-op this
    /// wave).
    fn check_local_permission(&self, access: AccessType, mut perm: Permission) -> Result<()> {
        // EROFS gate (P0-18): the mutating class on an effective read-only
        // mount fails before the DAC block and before any authority side
        // effect (BC-5 §49.1).
        if access == AccessType::Mutating && self.fs_arc()?.policy().is_effective_read_only() {
            return_errno_with_message!(Errno::EROFS, "the overlay mount is read-only");
        }

        // Projected-DAC block (the `inode.rs:556-611` mirror, inlined — spec
        // §8 note 3). No task / no posix thread / no thread-local: the kernel
        // context is not a user process, so there is no DAC demand to check
        // (mirror's `Option`-based guards, fail-open for non-user contexts;
        // the DAC_OVERRIDE probe is fail-closed when the thread-local is
        // absent — no `.unwrap()`/`.expect()` anywhere in this gate).
        let Some(task) = Task::current() else {
            return Ok(());
        };
        let Some(posix_thread) = task.as_posix_thread() else {
            return Ok(());
        };

        let creds = posix_thread.credentials();
        let metadata = self.metadata();
        let mode = metadata.mode;

        // With DAC_OVERRIDE, read/write DACs are always overridable; the
        // executable DAC is overridable only when at least one exec bit is
        // set (the VFS reduction, `inode.rs:569-583`). The probe runs
        // through the shared user-namespace capability helper (wave-4
        // round-4 repair item 3): at this point the task and posix thread
        // are known to exist, so the helper's kernel-context fail-open arm
        // is unreachable here and the thread-local-absent case stays
        // fail-closed — identical to the previous inline probe.
        let has_dac_override = Self::current_task_has_capability(CapSet::DAC_OVERRIDE);
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

        // Owner / group / other mode-DAC checks against the projected
        // metadata (the `inode.rs:585-607` mirror).
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

        // P2-06 protected-state admission seam: insertion point (no-op this
        // wave; the `protattr` record is already in the xattr known-private
        // table).
        Ok(())
    }

    /// PRIVATE STAGE B — the real-handle half of the two-stage check.
    ///
    /// For the `Mutating` class the copy-up lives here, between the two
    /// stages (packet revision 01 override 2): `ensure_upper_authority()`
    /// (meso-04 seam) promotes the object first, then the current real
    /// authority is re-resolved per call (`select_real_inode()`, BC-5 §49.2)
    /// and evaluated under the mount's creator-credential scope
    /// (`with_creator_credentials_fn`, meso-01 P1-19). The explicit real
    /// stage is authoritative for entries whose underlying ops do not
    /// self-evaluate (metadata setters — §4.0 evidence) and a benign double
    /// evaluation for xattr ops that self-evaluate under the same scope.
    /// A failure propagates as-is with no invented rollback (meso-04 owns any
    /// already-started transition cleanup/reconcile).
    fn check_real_permission(&self, access: AccessType, perm: Permission) -> Result<()> {
        if access == AccessType::Mutating {
            self.ensure_upper_authority()?;
        }
        let fs = self.fs_arc()?;
        let real = self.select_real_inode();
        fs.policy()
            .credential_policy()
            .with_creator_credentials_fn(|| real.check_permission(perm))
    }
}
