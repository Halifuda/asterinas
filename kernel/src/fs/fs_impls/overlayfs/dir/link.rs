// SPDX-License-Identifier: MPL-2.0

//! The link recipe — `P1-28`.
//!
//! This module owns the two frozen recipe helpers of the meso-06 spec §4
//! `dir/link.rs` on [`OverlayInode`] — [`OverlayInode::link_source`] (source
//! authority promotion to the shared upper real inode) and
//! [`OverlayInode::link_over_whiteout`] (workdir hard-link staging plus
//! atomic rename-over replacement of a published whiteout target). The
//! `Inode::link` entry itself lives in the sibling `dir/mod.rs` pass (the
//! thin mutation entries), which composes these helpers with the frozen
//! target publication seam sequence (`BindingCache::insert` + the meso-03
//! `readdir_index_insert` decision seam) under the target parent `DIR`
//! (spec §7.3; BC-6 §60.3).
//!
//! Lock contract (spec §3): neither helper acquires or holds any Overlay
//! lock. They run inside the caller's target-parent `DIR` (Level 2) domain
//! established by `lock_dir_transaction` in `dir/mod.rs`; the source
//! promotion (`ensure_upper_authority`) acquires the per-object `CUL`
//! (Level 3) and `INODE` (Level 4) domains in the frozen
//! `DIR -> CUL -> INODE` order (meso-04-owned, released on publication or
//! return); the underlying upper/workdir operations (`link`/`rename`) may
//! block and run in the sleep-capable domain, never under `WL` (Level 5) or
//! any spin lock (Hazard 2). The workdir temp is private staging (BC-6 §57)
//! and is cleaned best-effort on the rename-over failure path — an explicit
//! fallible operation, never an RAII-durable-rollback (BC-8; "workdir
//! cleanup is an explicit fallible operation, never an
//! RAII-durable-rollback" invariant).
//!
//! Visibility: both helpers are declared `pub(super)` — visible only within
//! the `dir` module tree — because their only consumers are the sibling
//! `dir/mod.rs` mutation entries (spec §1 "Must Remain Internal": nothing in
//! `dir/link.rs` is visible outside `dir` except the frozen `Inode` entries).
//! The spec's unqualified `fn` is read through the dispatch override exactly
//! as the Wave-4 precedent widened `copyup/promote.rs::promote` for its
//! sibling-module consumer.
//!
//! Degradation note (spec §7.3 step 6; stageD §3.2.1; `P2-04`/`P3-01`
//! insertion points): without a `PersistentOriginIndex`, two lower aliases
//! of one lower inode that copy up separately may become two distinct upper
//! inodes; upper-authoritative sources always share one upper inode (the
//! real hard link published here). This pass implements the frozen link
//! surface; the no-index degradation is accepted explicitly, never papered
//! over.

use crate::{
    fs::{
        fs_impls::overlayfs::{copyup::WorkdirTempRequest, projection::OverlayInode},
        vfs::inode::{Inode, RenameMode},
    },
    prelude::*,
};

impl OverlayInode {
    /// Promotes the link source to upper authority and resolves the shared
    /// upper real inode (`P1-28`, meso-06 spec §4 `dir/link.rs`).
    ///
    /// The source branch of the link recipe (spec §7.3 step 3):
    /// `old.ensure_upper_authority()` (meso-04 seam) makes the source
    /// upper-authoritative (idempotent fast path when already upper-backed),
    /// then `old.select_real_inode()` resolves the current authority's real
    /// inode — the single upper real inode that the new target hard link
    /// shares with the source. The caller (the `dir/mod.rs` `Inode::link`
    /// entry) composes this per-branch promotion with the target-parent
    /// promotion in stable object-identity order (spec §3 item 7; meso-04
    /// §3.2 item 6); this helper covers the source branch only.
    ///
    /// Lock contract: runs under the caller's target-parent `DIR`; the
    /// meso-04 promotion acquires `CUL` (Level 3) → `INODE` (Level 4) in the
    /// frozen order and releases them on publication or return. No Overlay
    /// lock is acquired or held by this method itself and none crosses the
    /// return boundary.
    ///
    /// Returns the shared upper real inode on success; propagates any
    /// meso-04 promotion error unchanged (`Err(Errno::ENOENT)` on the
    /// Case-7 defensive guard when no copy-up coordinate is recorded, and
    /// any underlying recipe failure).
    pub(super) fn link_source(&self, old: &Arc<OverlayInode>) -> Result<Arc<dyn Inode>> {
        old.ensure_upper_authority()?;
        Ok(old.select_real_inode())
    }

    /// Replaces a published whiteout target with a hard link to the shared
    /// source upper real inode (`P1-28`, meso-06 spec §4 `dir/link.rs`).
    ///
    /// The target-whiteout leg of the link recipe (spec §7.3 step 4; Linux
    /// `ovl_create_over_whiteout` hardlink leg): the shared source upper
    /// real inode is staged as a private workdir hard link under a unique
    /// temp name (meso-04 `generate_workdir_temp_name`), then atomically
    /// renamed over the whiteout at `name` in the target upper parent with
    /// `RenameMode::Replace`. The whiteout is consumed by the replacement
    /// and never re-cached; the staged hard link becomes the visible upper
    /// object at the target name.
    ///
    /// Workdir temporaries stay private staging (invariant I7, BC-6 §57):
    /// the temp is never a lookup/readdir/`ReaddirIndex` source. On a
    /// rename-over failure the staged hard link is removed best-effort via
    /// meso-04 `cleanup_workdir_temp`; a cleanup failure is the recorded
    /// P3-09 workdir-cleanup obligation and never becomes a visible
    /// namespace entry.
    ///
    /// Lock contract: runs under the caller's target-parent `DIR` (Level 2);
    /// the underlying upper operations (`workdir.link`/`workdir.rename`) may
    /// block and run in the sleep-capable domain, never under `WL` (Level 5)
    /// or any spin lock (Hazard 2). The workdir root resolves through the
    /// single shared resolver `OverlayInode::workdir_root` (wave-4 repair
    /// item 11; I10: no workdir side effect without a writable claim). No
    /// Overlay lock is acquired or held by this method and none crosses the
    /// return boundary.
    pub(super) fn link_over_whiteout(&self, name: &str, upper_real: &Arc<dyn Inode>) -> Result<()> {
        let fs = self.fs_arc()?;
        let upper_parent = self.upper_parent()?;
        let temp = fs.create_workdir_temp(
            name,
            &upper_parent,
            WorkdirTempRequest::Link {
                source: upper_real.clone(),
            },
        )?;
        let workdir = self.workdir_root()?;
        // Step 1 — the hard-link leg: stage the shared source upper real
        // inode as a private workdir hard link under the unique temp name.
        // Step 2 — atomic rename-over: replace the published whiteout at
        // `name` with the staged hard link (`Replace`; the whiteout is
        // consumed, never re-cached). On failure the staged temp is removed
        // best-effort so no workdir residue outlives the recipe.
        if let Err(err) = workdir.rename(temp.name(), &upper_parent, name, RenameMode::Replace) {
            let _ = fs.cleanup_workdir_temp(temp.name());
            return Err(err);
        }
        Ok(())
    }
}
