// SPDX-License-Identifier: MPL-2.0

//! The module root of the `namespace_mutation_whiteout` meso (meso-06).
//!
//! This module declares the five `dir/*` submodules and hosts the thin
//! `Inode`-trait mutation entries of the frozen meso-06 spec §4 — `create` /
//! `mknod` / `write_link` / `link` / `unlink` / `rmdir` / `rename` — plus
//! the four orchestration helpers of the `dir/mod.rs` slice: the one-parent
//! `DIR` inlet ([`OverlayInode::lock_dir_transaction`]), the two-parent
//! `DIR` inlet ([`OverlayInode::lock_parent_dir_transactions`], stable
//! object-identity order, each parent exactly once), the post-promotion
//! upper real parent resolution ([`OverlayInode::upper_parent`]), and the
//! single shared private reconcile entry ([`OverlayInode::invalidate_stale_cache`],
//! Case 13). The real control flow lives in the sibling files created in
//! parallel from the same frozen spec: `create.rs` (P1-23 dispatcher +
//! upper-only/over-whiteout/opaque branches), `remove.rs` (P1-26/27 unlink /
//! rmdir + visible emptiness + clear-empty), `link.rs` (P1-28 source
//! promotion + target-over-whiteout fragments), `rename.rs` (P1-29/30 EXDEV
//! gate + upper rename + dual-parent publication), and `whiteout.rs`
//! (P1-25/36 whiteout cache + representation).
//!
//! Visibility: `whiteout` is declared `pub(super)` — read through the
//! spec's overlayfs-ceiling audit as `pub(in crate::fs::fs_impls::overlayfs)`
//! — because the frozen Wave-3 `OverlayFs::whiteout_cache` field
//! initialization in `mount/build.rs` names `dir::whiteout::WhiteoutCache`
//! from a sibling module (the `copyup::coordination` precedent); the other
//! four submodules stay private to `dir` (spec §1 "Must Remain Internal").
//! The recipe methods the entries delegate to (`create_object`,
//! `remove_target`, `link_source`, `link_over_whiteout`, `cross_device_gate`,
//! `rename_upper`) are consumed at `pub(super)` within `dir`, and the four
//! orchestration helpers are published at the same `pub(super)` so the
//! sibling recipes can compose them (`upper_parent` in the create/remove
//! recipes; `invalidate_stale_cache` on every Case-13 reconcile path).
//!
//! Lock contract (spec §3): every mutation entry establishes the affected
//! parent `DIR` domain(s) first (Level 2, via the two inlets below), then
//! runs the meso-05 `check_permission(AccessType::Mutating, Permission::MAY_WRITE)`
//! admission per affected parent under the held `DIR` — stage A is the
//! EROFS gate, and stage B's `CUL` promotion therefore runs in the frozen
//! `DIR -> CUL -> INODE` order (§3 item 4) — then delegates to the recipe
//! under the same guard(s). All `DIR`/`CUL`/`INODE` domains are released
//! before any VFS-visible return (§3 outlet); `MOUNT` is never acquired and
//! `WL` is never acquired by this module (the `whiteout.rs` slot protocol is
//! the only `WL` holder). No `.unwrap()`/`.expect()` appears in any
//! production path (hard invariant failures use the recorded
//! `unreachable!` precedent of `projection/inode.rs`).

use self::remove::RemoveKind;
use super::{
    projection::{Binding, BindingKey, NegativeBinding, OverlayInode, PositiveBinding},
    AccessType,
};
use crate::{
    fs::{
        file::{InodeMode, InodeType, Permission},
        vfs::inode::{Inode, MknodType, RenameMode},
    },
    prelude::*,
};

pub(super) mod whiteout;

mod create;
mod link;
mod remove;
mod rename;

/// Maps the `mknod` kind request to the overlay-visible object type
/// (`P1-23`/`P1-24`; wave-4 repair item 11).
///
/// The `MknodType` -> `InodeType` classification was inlined at three sites
/// (`dir/mod.rs::mknod`, `dir/create.rs::create_upper_only`, and
/// `dir/create.rs::create_over_whiteout`); this helper is the single
/// mechanical mapping — the priors thin-helper rule is satisfied because the
/// tree's own DRY threshold is three occurrences (the wave-4 review calls
/// the triplication out). `MknodType` has no `InodeType` conversion, so the
/// match is the owned mapping. Whitelist Rule B: three call sites inside the
/// `dir` meso tree.
pub(super) fn mknod_object_type(mknod: &MknodType) -> InodeType {
    match mknod {
        MknodType::NamedPipe => InodeType::NamedPipe,
        MknodType::CharDevice(_) => InodeType::CharDevice,
        MknodType::BlockDevice(_) => InodeType::BlockDevice,
    }
}

impl OverlayInode {
    // P1-21/22/23/24: create/mkdir/symlink carry the same VFS entry
    // (`Path::new_fs_child` -> `Dentry::create` -> `Inode::create`, verified
    // `syscall/mkdir.rs:38` and `syscall/symlink.rs:44`); the symlink target
    // is filled by the later `write_link` delegation. Admission under the
    // parent `DIR`; the P1-23 dispatcher (create.rs) re-derives the fresh
    // binding, runs the upper-only/over-whiteout/opaque recipe and its
    // inline publication, and returns the projected `OverlayInode` (Case 1).
    pub(in crate::fs::fs_impls::overlayfs) fn create_impl(
        &self,
        name: &str,
        type_: InodeType,
        mode: InodeMode,
    ) -> Result<Arc<dyn Inode>> {
        // The parent `DIR` domain is established first (frozen §3
        // acquisition order item 1) so the admission's real stage B runs
        // under the held `DIR` (`DIR -> CUL`, item 4); a failed admission
        // produces no upper/workdir/cache side effect (Case 6).
        let _dir_guard = self.lock_dir_transaction();
        self.check_permission(AccessType::Mutating, Permission::MAY_WRITE)?;
        // The P1-23 dispatcher takes the `Inode`-trait create arguments
        // directly (landed `create.rs` resolution of the frozen
        // `InodeKindRequest` parameter — the spec's "carried as the
        // Inode-trait arguments, not a new enum" reading; recorded in the
        // Creator report §5 item 4).
        let projected: Arc<dyn Inode> = self.create_object(name, type_, mode, None)?;
        Ok(projected)
    }

    // P1-24: a raw `0:0` whiteout char device is refused before any
    // admission or side effect (Case 11; Linux `ovl_mknod`, dir.c:746-753);
    // every other `mknod` kind funnels into the same P1-23 dispatcher with
    // the `MknodType` carried in the request.
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
        // The dispatcher takes the `Inode`-trait create arguments directly
        // (landed `create.rs` resolution of the frozen `InodeKindRequest`
        // parameter, Creator report §5 item 4): the mechanical
        // `MknodType` -> `InodeType` classification is the shared
        // `mknod_object_type` mapping (wave-4 repair item 11) and the
        // `MknodType` itself is the `mknod_type` leg (the create.rs recipes
        // derive their own object-type classification from `mknod_type`
        // when it is `Some`).
        let object_type = mknod_object_type(&type_);
        let projected: Arc<dyn Inode> = self.create_object(name, object_type, mode, Some(type_))?;
        Ok(projected)
    }

    // P1-24 completion: the upper symlink was created by `create` (the
    // VFS-wide create-then-`write_link` two-step, syscall/symlink.rs:44-45 —
    // the same window ramfs accepts). Thin delegation to the current
    // authority with no promotion: the created symlink is already
    // upper-backed (spec §4; not a redesign of meso-04's `read_link`).
    pub(in crate::fs::fs_impls::overlayfs) fn write_link_impl(&self, target: &str) -> Result<()> {
        self.select_real_inode().write_link(target)
    }

    // P1-28: link. Only the target parent's `DIR` is acquired (the source is
    // an object, not a namespace of this mutation; its promotion is
    // `CUL`-serialized by meso-04, spec §7.3 step 1). The two frozen `link.rs`
    // fragments — `link_source` (source promotion to the shared upper real
    // inode) and `link_over_whiteout` (workdir hard link + rename-over) —
    // are composed here with the fresh target projection and the inline
    // target publication (Cases 2/3/7/10; the publication seams are
    // infallible, so Case 13 is unreachable on this path — recorded in the
    // Creator report §5 item 7).
    pub(in crate::fs::fs_impls::overlayfs) fn link_impl(
        &self,
        old: &Arc<dyn Inode>,
        name: &str,
    ) -> Result<()> {
        let _dir_guard = self.lock_dir_transaction();
        self.check_permission(AccessType::Mutating, Permission::MAY_WRITE)?;
        let fs = self.fs_arc()?;
        // Fresh target projection under `DIR` (never from a stale VFS
        // dentry): a visible target is never silently replaced (Case 7).
        let binding = fs.lookup_binding(&self.facts_snapshot(), name)?;
        if matches!(&binding, Binding::Positive(_)) {
            return Err(Error::new(Errno::EEXIST));
        }
        let target_is_whiteout =
            matches!(&binding, Binding::Negative(NegativeBinding::HiddenByWhiteout(_)));
        // The source must be an Overlay inode (the VFS passes an inode of
        // this filesystem); a foreign inode is a defensive error, never a
        // silent cast.
        let old_overlay = Arc::downcast::<OverlayInode>(old.clone()).map_err(|_| {
            Error::with_message(Errno::EIO, "the link source is not an overlay inode")
        })?;
        // Source-side admission (wave-4 repair item 5; Linux `may_linkat`
        // under `fs.protected_hardlinks`): the `link` syscall performs no
        // source check of its own (VFS gap), so before the source promotion
        // trigger runs, the caller must either own the source or hold
        // write-DAC on it. The meso-05 read-only admission surface (the
        // 1-param `check_permission` leg — never promotes) evaluated against
        // the source's projected metadata mirrors the Linux owner-or-write
        // check; this gates `link_source`'s copy-up, so an inaccessible
        // source is refused with `EPERM` and never forced to the upper
        // layer.
        let source_metadata = old_overlay.metadata();
        // The owner probe runs through the shared `current_fsuid()`
        // accessor (wave-4 round-5 repair item 3): the kernel-context
        // default — no task / no posix thread means "not the owner" — is
        // handled in one place (`permission.rs`), and the borrow-lifetime
        // trap of binding the owned `CurrentTask` locally is gone.
        let source_owned = OverlayInode::current_fsuid()
            .is_some_and(|fsuid| fsuid == source_metadata.uid);
        if !source_owned && old_overlay.check_permission(AccessType::ReadOnly, Permission::MAY_WRITE).is_err() {
            return Err(Error::with_message(
                Errno::EPERM,
                "the link source is not accessible to the caller",
            ));
        }
        // Source promotion (link.rs): `ensure_upper_authority` then the
        // shared upper real inode. Without an origin/index, two lower
        // aliases of one lower inode may copy up separately to distinct
        // upper inodes (recorded degradation, P2-04/P3-01 insertion points);
        // upper-authoritative sources always share one upper inode (real
        // hard link), spec §7.3 step 6.
        let upper_real = self.link_source(&old_overlay)?;
        if target_is_whiteout {
            // Target hidden by a whiteout: workdir hard link + rename-over
            // the whiteout (Linux `ovl_create_over_whiteout` hardlink leg).
            self.link_over_whiteout(name, &upper_real)?;
        } else {
            // Absent (or opaque-hidden) target: direct upper hard link.
            self.upper_parent()?.link(&upper_real, name)?;
        }
        // Inline target publication (revision-01 override 2): the positive
        // binding shares the source `OverlayInode` — inode-cache reuse by
        // `RealObjectKey`, so `project_new_upper` is not needed (spec §7.3
        // step 5) — and the meso-03 decision seam maintains the target
        // parent index (Valid + upper-only rule, spec §5.2). Both seams are
        // infallible; they run under the held `DIR` before release.
        let key = BindingKey::new(self.key(), String::from(name));
        let binding = Arc::new(Binding::Positive(PositiveBinding::new(old_overlay.clone())));
        fs.bindings().insert(key, binding);
        self.readdir_index_insert(name, old_overlay.clone(), old_overlay.type_());
        Ok(())
    }

    // P1-26: unlink. The `remove.rs` recipe owns the fresh target projection
    // (Case 10 ENOENT), the pure-upper direct unlink vs lower-backed
    // whiteout publish, and the inline publication (BindingCache invalidate /
    // HiddenByWhiteout insert + `readdir_index_remove`), Cases 2/10/13.
    pub(in crate::fs::fs_impls::overlayfs) fn unlink_impl(&self, name: &str) -> Result<()> {
        let _dir_guard = self.lock_dir_transaction();
        self.check_permission(AccessType::Mutating, Permission::MAY_WRITE)?;
        self.remove_target(name, RemoveKind::Unlink)
    }

    // P1-27: rmdir. Same admission + single-parent `DIR` shape as `unlink`;
    // `remove_target(name, RemoveKind::Rmdir)` runs the Overlay-visible
    // emptiness gate (meso-03 `visible_child_count`; whiteout-hidden children
    // do not count) before any upper removal and takes the clear-empty path
    // for lower-backed directories with upper children (Cases 2/9/10/13).
    // The operation kind is the closed `RemoveKind` vocabulary (wave-4
    // repair item 12 — no `is_dir` boolean at the call sites).
    pub(in crate::fs::fs_impls::overlayfs) fn rmdir_impl(&self, name: &str) -> Result<()> {
        let _dir_guard = self.lock_dir_transaction();
        self.check_permission(AccessType::Mutating, Permission::MAY_WRITE)?;
        self.remove_target(name, RemoveKind::Rmdir)
    }

    // P1-29/30: rename. Two-parent `DIR` acquisition (stable object-identity
    // order, each parent exactly once), mutating admission per affected
    // parent, the fresh source projection (Case 10), the P1-30 EXDEV gate
    // before any upper side effect, and then the upper rename recipe
    // (`rename_upper`, rename.rs) which owns per-branch promotion, the
    // physical upper rename (+ source-whiteout compose), the dual-parent
    // inline publication, and the Case-13 reconcile on failure.
    pub(in crate::fs::fs_impls::overlayfs) fn rename_impl(
        &self,
        old_name: &str,
        target: &Arc<dyn Inode>,
        new_name: &str,
        mode: RenameMode,
    ) -> Result<()> {
        // The destination parent must be an Overlay inode (the VFS passes
        // the destination directory of this filesystem); a foreign inode is
        // a defensive error, never a silent cast.
        let target_overlay = Arc::downcast::<OverlayInode>(target.clone()).map_err(|_| {
            Error::with_message(Errno::EIO, "the rename target is not an overlay inode")
        })?;
        // Two-parent `DIR` acquisition in stable object-identity order
        // (Hazard 1; spec §3 item 1): the same-parent case returns one guard
        // (each parent exactly once).
        let (_source_guard, _target_guard) =
            self.lock_parent_dir_transactions(Some(&target_overlay))?;
        // Mutating admission per affected parent under the held `DIR`s
        // (stage A EROFS gate; stage B `CUL` in the frozen order), spec §3
        // item 4.
        self.check_permission(AccessType::Mutating, Permission::MAY_WRITE)?;
        target_overlay.check_permission(AccessType::Mutating, Permission::MAY_WRITE)?;
        let fs = self.fs_arc()?;
        // Fresh source projection under `DIR`: the `DIR`-domain projection is
        // authoritative over a stale VFS dentry; a negative source is ENOENT
        // (Case 10).
        let source_binding = fs.lookup_binding(&self.facts_snapshot(), old_name)?;
        let _source_inode = match &source_binding {
            Binding::Positive(positive) => positive.inode(),
            Binding::Negative(_) => return Err(Error::new(Errno::ENOENT)),
        };
        // EXDEV gate (P1-30) before any upper side effect: only a
        // cross-directory move of a lower-backed/merged directory hits the
        // frozen EXDEV default (P2-02 redirect is an insertion point only,
        // spec §7.4 step 3). The same-parent comparison is the carrier
        // address identity of the two `DIR` inlets.
        if !core::ptr::addr_eq(core::ptr::from_ref(self), Arc::as_ptr(&target_overlay)) {
            self.cross_device_gate(&source_binding)?;
        }
        // "Source has a lower fallback" decides whether the source name gets
        // a whiteout after the move (spec §7.4 step 5); rename.rs derives it
        // internally from the fresh source projection (wave-4 round-2 repair
        // item 4 — no bare boolean crosses the entry boundary).
        self.rename_upper(old_name, &target_overlay, new_name, mode)
    }
}

impl OverlayInode {
    /// Returns the payload-less parent `DIR` transaction guard of this
    /// directory.
    ///
    /// The one-parent `DIR` inlet (revision-01 name, spec §4; formerly
    /// `acquire_dir`). `self.dir()` is `Some` exactly for directory
    /// carriers, and every mutation entry of this module is a child-name
    /// operation that the VFS routes on directory inodes (the same
    /// `Some`-invariant `lookup` relies on), so the `None` arm is a hard
    /// invariant failure — never a silent guard-less mutation, and never a
    /// `.unwrap()`/`.expect()` (the recorded `unreachable!` precedent of
    /// `projection/inode.rs`).
    pub(super) fn lock_dir_transaction(&self) -> MutexGuard<'_, ()> {
        match self.dir() {
            Some(dir) => dir.lock(),
            None => unreachable!(
                "mutation entries run on overlay directories only; the VFS routes child-name \
                 operations on directory inodes"
            ),
        }
    }

    /// Acquires the two affected parent `DIR` domains in stable
    /// object-identity order (Hazard 1), each parent exactly once.
    ///
    /// The two-parent `DIR` inlet (revision-01 name, spec §4; formerly
    /// `acquire_parent_dirs`). The frozen ordering key `RealObjectKey`
    /// lexicographic `(fsid, real_ino)` is not currently publishable — the
    /// landed `RealObjectKey` derives no `Ord` and its fields are
    /// `projection`-private — so this helper applies the spec's blessed
    /// alternative ("`Arc::as_ptr` ordering, the meso-04 example, is an
    /// acceptable equivalent", §3 Hazard 1): the two parents are ordered by
    /// their carrier address, `core::ptr::from_ref(self)` being exactly the
    /// address `Arc::as_ptr` returns for the same carrier. The inode cache
    /// (`get_or_create` by `RealObjectKey`) guarantees one carrier per
    /// logical directory, so the address is a stable per-directory identity
    /// and the same-carrier case (a same-directory rename) acquires the
    /// single `DIR` once. The guards are returned as the frozen anonymous
    /// tuple `(self_guard, other_guard)` — a local return shape, not a named
    /// coordination carrier (spec §4); the elided `'_` lifetimes are written
    /// as explicit `'a`/`'b` because the two guards borrow from two distinct
    /// inputs (recorded realization, Creator report §5 item 6).
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
        // Single-parent operation: the first element is this parent's guard.
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
        // Stable object-identity order by carrier address; the same-carrier
        // case acquires the single `DIR` once (each parent exactly once).
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

    /// Returns the promoted upper real parent directory of this directory.
    ///
    /// The physical-operation target of every recipe (KEPT unchanged,
    /// override 7): after the mutating admission's stage B promotion
    /// (meso-04), `select_real_inode()` resolves the upper real inode. The
    /// `Result` return is the frozen signature; the body is a single
    /// infallible resolution (the promotion side effect already ran).
    pub(super) fn upper_parent(&self) -> Result<Arc<dyn Inode>> {
        Ok(self.select_real_inode())
    }

    /// Conservatively invalidates the stale projection of the affected
    /// `(parent, name)` pairs after a physical upper success whose semantic
    /// publication failed (Case 13).
    ///
    /// The SINGLE shared private reconcile entry (revision-01 name, spec §4;
    /// formerly `conservative_invalidate`; wave-4 repair item 10 — every
    /// Case-13 recipe calls this entry instead of inlining its own shape).
    /// A physical upper operation has committed but a
    /// BindingCache/barrier/index publication step failed, so the cached
    /// projection is stale; for each affected `(parent, name)` the
    /// mount-wide binding entry is invalidated (`BindingCache::invalidate`,
    /// keyed by `parent.key()`) and the parent's readdir index is marked
    /// `NeedsRebuild` (`invalidate_readdir_index`), so the next
    /// lookup/readdir re-derives from upper truth. The parents are plain
    /// `&OverlayInode` handles, so the one-parent `&self` recipes pass their
    /// own `(self, name)` pair directly and Arc-carrying recipes pass
    /// `(arc.as_ref(), name)` — no inlined two-seam arm survives at any call
    /// site. Works for one- and two-parent operations and never claims a
    /// partial or stronger transaction (BC-6 §63; §5.3). The mount upgrade
    /// is best-effort: on a dying mount there is no live cache to reconcile
    /// (no `.unwrap()`/`.expect()`).
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
