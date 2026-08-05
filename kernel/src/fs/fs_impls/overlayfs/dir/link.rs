// SPDX-License-Identifier: MPL-2.0

//! The link recipe.
//!
//! This module owns the two recipe helpers on [`OverlayInode`] —
//! [`OverlayInode::link_source`] (source authority promotion to the shared
//! upper real inode) and [`OverlayInode::link_over_whiteout`] (workdir
//! hard-link staging plus atomic rename-over replacement of a published
//! whiteout target). The `Inode::link` entry itself lives in the sibling
//! `dir/mod.rs` (the thin mutation entries), which composes these helpers
//! with the target publication sequence (`BindingCache::insert` +
//! `readdir_index_insert`) under the target parent `DIR`.
//!
//! Lock contract: neither helper acquires or holds any Overlay lock. They run
//! inside the caller's target-parent `DIR` domain established by
//! `lock_dir_transaction` in `dir/mod.rs`; the source promotion
//! (`ensure_upper_authority`) acquires the per-object `CUL` and `INODE`
//! domains in the `DIR -> CUL -> INODE` order (released on publication or
//! return); the underlying upper/workdir operations (`link`/`rename`) may
//! block and run in the sleep-capable domain, never under `WL` or any spin
//! lock. The workdir temp is private staging and is cleaned best-effort on
//! the rename-over failure path — an explicit fallible operation, never an
//! RAII-durable-rollback.
//!
//!
//! Degradation note: without a persistent origin index, two lower aliases of
//! one lower inode that copy up separately may become two distinct upper
//! inodes; upper-authoritative sources always share one upper inode (the real
//! hard link published here). The no-index degradation is accepted
//! explicitly, never papered over.

use crate::{
    fs::{
        fs_impls::overlayfs::{copyup::WorkdirTempRequest, projection::OverlayInode},
        vfs::inode::{Inode, RenameMode},
    },
    prelude::*,
};

impl OverlayInode {
    /// Promotes the link source to upper authority and resolves the shared
    /// upper real inode.
    ///
    /// The source branch of the link recipe: `old.ensure_upper_authority()`
    /// makes the source upper-authoritative (idempotent fast path when
    /// already upper-backed), then `old.select_real_inode()` resolves the
    /// current authority's real inode — the single upper real inode that the
    /// new target hard link shares with the source. The caller (the
    /// `dir/mod.rs` `Inode::link` entry) composes this per-branch promotion
    /// with the target-parent promotion in stable object-identity order; this
    /// helper covers the source branch only.
    ///
    /// Lock contract: runs under the caller's target-parent `DIR`; the
    /// promotion acquires `CUL` → `INODE` in order and releases them on
    /// publication or return. No Overlay lock is acquired or held by this
    /// method itself and none crosses the return boundary.
    ///
    /// Returns the shared upper real inode on success; propagates any
    /// promotion error unchanged (`Err(Errno::ENOENT)` on the defensive guard
    /// when no copy-up coordinate is recorded, and any underlying recipe
    /// failure).
    pub(super) fn link_source(&self, old: &Arc<OverlayInode>) -> Result<Arc<dyn Inode>> {
        old.ensure_upper_authority()?;
        Ok(old.select_real_inode())
    }

    /// Replaces a published whiteout target with a hard link to the shared
    /// source upper real inode.
    ///
    /// The target-whiteout leg of the link recipe (Linux
    /// `ovl_create_over_whiteout` hardlink leg): the shared source upper real
    /// inode is staged as a private workdir hard link under a unique temp
    /// name (`generate_workdir_temp_name`), then atomically renamed over the
    /// whiteout at `name` in the target upper parent with
    /// `RenameMode::Replace`. The whiteout is consumed by the replacement and
    /// never re-cached; the staged hard link becomes the visible upper object
    /// at the target name.
    ///
    /// Workdir temporaries stay private staging: the temp is never a
    /// lookup/readdir/`ReaddirIndex` source. On a rename-over failure the
    /// staged hard link is removed best-effort via
    /// `cleanup_workdir_temp`; a cleanup failure is a known workdir-cleanup
    /// debt and never becomes a visible namespace entry.
    ///
    /// Lock contract: runs under the caller's target-parent `DIR`; the
    /// underlying upper operations (`workdir.link`/`workdir.rename`) may
    /// block and run in the sleep-capable domain, never under `WL` or any
    /// spin lock. The workdir root resolves through the single shared
    /// resolver `OverlayInode::workdir_root` (no workdir side effect without
    /// a writable claim). No Overlay lock is acquired or held by this method
    /// and none crosses the return boundary.
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
