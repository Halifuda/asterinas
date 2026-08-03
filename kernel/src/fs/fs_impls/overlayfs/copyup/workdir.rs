// SPDX-License-Identifier: MPL-2.0

//! The workdir temporary lifecycle — `P1-34`.
//!
//! This module owns the three frozen workdir temp helpers of the meso-04 spec
//! §4 `copyup/workdir.rs` on [`OverlayFs`] — [`OverlayFs::generate_workdir_temp_name`]
//! (unique naming), [`OverlayFs::create_workdir_temp`] (private staging
//! creation), and [`OverlayFs::cleanup_workdir_temp`] (staging cleanup) — plus
//! the private [`OverlayFs::workdir_root`] resolver that funnels every helper
//! through the mount's pinned workdir claim.
//!
//! The workdir is a private staging area on the upper filesystem, never a
//! layer: temporaries never enter lookup/readdir, unique naming keeps them out
//! of the overlay namespace, and a failure leaves a recorded cleanup
//! obligation, never a visible entry (invariant I7, BC-4 §40/§45.1). A temp
//! handle belongs only to the winner's copy-up transaction (BC-4 §40.2): it is
//! never returned to the VFS, never stored on the inode, and never a
//! page-cache forwarding target. The P1-35 claim guarantees no cross-mount
//! collision (a workdir cannot be claimed by two live mounts), so the
//! composite name needs only per-mount uniqueness.
//!
//! Lock contract (spec §3.0): workdir temp naming is uniqueness-based, not
//! lock-based — no Overlay lock is acquired or held by any method here, and
//! the underlying upper-filesystem calls run against that filesystem's own
//! locking (proven non-re-entrant into Overlay, spec §3.3 Hazard 2). The EROFS
//! gate precedes every workdir/upper side effect (I10): the private
//! [`OverlayFs::workdir_root`] resolver returns `Err(Errno::EROFS)` when no
//! writable claim exists (spec §2 Case 4).
//!
//! Visibility: the three frozen helpers are declared at the overlayfs ceiling
//! (`pub(in crate::fs::fs_impls::overlayfs)`) — the spec's unqualified
//! `pub(super)` read through the dispatch override and the Wave-3 precedent,
//! matching every other spec-`pub(super)` surface of this meso
//! (`coordination.rs`/`trigger.rs`/`mod.rs`); the sibling `copyup/promote.rs`
//! pass consumes them from the same module tree. [`OverlayFs::workdir_root`]
//! is the SINGLE workdir-root claim resolver of the whole overlayfs tree
//! (wave-4 round-2 repair item 5): it is widened to the overlayfs ceiling so
//! `dir/whiteout.rs` can call it, `OverlayInode::workdir_root`
//! (`copyup/promote.rs`) delegates to it, and no inline claims block
//! survives anywhere in the tree.

use crate::{
    fs::{
        file::{InodeMode, InodeType},
        fs_impls::overlayfs::mount::OverlayFs,
        vfs::inode::Inode,
    },
    prelude::*,
};

impl OverlayFs {
    /// Generates a uniquely-named workdir temp name for a copy-up target
    /// (`P1-34`, meso-04 spec §4 `copyup/workdir.rs`).
    ///
    /// The frozen composite is `#{target_name}#{parent_ino}#{serial}`: the
    /// target's publication name, the upper-parent real inode number
    /// ([`Inode::ino`]), and one per-mount saturating workdir serial
    /// ([`OverlayFs::workdir_temp_serial`], the Wave-3
    /// `workdir_temp_serial: AtomicU64` field). Uniqueness is by construction
    /// (target name + upper-parent real ino + per-mount serial); no lock is
    /// held (spec §3.0: naming is uniqueness-based, not lock-based) and the
    /// P1-35 claim guarantees no cross-mount collision.
    pub(in crate::fs::fs_impls::overlayfs) fn generate_workdir_temp_name(
        &self,
        target_name: &str,
        upper_parent: &Arc<dyn Inode>,
    ) -> String {
        let parent_ino = upper_parent.ino();
        let serial = self.workdir_temp_serial();
        format!("#{target_name}#{parent_ino}#{serial}")
    }

    /// Creates a private workdir temp object for copy-up staging (`P1-34`).
    ///
    /// Creates `temp_name` in the workdir root with the given object kind and
    /// mode. The temp handle belongs only to the winner's copy-up transaction
    /// (BC-4 §40.2) and never enters the overlay namespace (invariant I7).
    /// An `EEXIST` from the underlying create is propagated unchanged: the
    /// bounded retry with a fresh serial is caller-side, in the sibling
    /// `copyup/promote.rs` recipe (dispatch override — the spec's in-function
    /// retry loop is re-homed to the caller so this helper stays a
    /// single-attempt primitive).
    pub(in crate::fs::fs_impls::overlayfs) fn create_workdir_temp(
        &self,
        temp_name: &str,
        kind: InodeType,
        mode: InodeMode,
    ) -> Result<Arc<dyn Inode>> {
        self.workdir_root()?.create(temp_name, kind, mode)
    }

    /// Removes a workdir temp object (`P1-34`).
    ///
    /// The recipe calls this best-effort on any pre-publication failure; a
    /// cleanup failure propagates as the recorded P3-09 workdir-cleanup
    /// obligation and never becomes a visible namespace entry (invariant I7,
    /// BC-4 §45.1).
    pub(in crate::fs::fs_impls::overlayfs) fn cleanup_workdir_temp(
        &self,
        temp_name: &str,
    ) -> Result<()> {
        self.workdir_root()?.unlink(temp_name)
    }

    /// Resolves the pinned workdir root inode of this writable mount.
    ///
    /// The single workdir-root claim resolver of the overlayfs tree (wave-4
    /// round-2 repair item 5): every workdir-root consumer — the three
    /// helpers in this file, `OverlayInode::workdir_root`
    /// (`copyup/promote.rs`), the meso-06 dir/ recipes, and the two
    /// `dir/whiteout.rs` sites — funnels through this one entry, so the
    /// claim-resolution shape and the EROFS error text exist exactly once.
    /// The workdir root is reachable via the meso-01 `claims()` seam (spec §2
    /// pre-condition P1-34: "the workdir root and upper parent real inode are
    /// reachable via meso-01 `claims()`"). A missing claim means the mount is
    /// effectively read-only (or the claims were released), so the EROFS gate
    /// fires here — before any workdir/upper side effect (I10, spec §2
    /// Case 4).
    pub(in crate::fs::fs_impls::overlayfs) fn workdir_root(&self) -> Result<Arc<dyn Inode>> {
        let claim = self.claims().ok_or_else(|| {
            Error::with_message(Errno::EROFS, "the overlay mount has no workdir claim")
        })?;
        Ok(claim.workdir_inode().clone())
    }
}
