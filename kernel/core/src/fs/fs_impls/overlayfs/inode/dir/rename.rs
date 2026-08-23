// SPDX-License-Identifier: MPL-2.0

//! The rename recipes: the EXDEV gate ([`OverlayInode::cross_device_gate`])
//! and the upper rename ([`OverlayInode::rename_upper`]).
//!
//! Lock contract: the caller holds both parent directory transaction locks.
//! Permission admission and source promotion are done by the entry before
//! those locks are taken; this module never enters the per-object copy-up
//! coordination lock while holding a parent lock.
//!
//! Notes:
//! - No `RENAME_WHITEOUT`: a source name that still needs a whiteout after
//!   the move is covered by a composed second upper step (rename, then
//!   `publish_whiteout` at the old name); a whiteout target being replaced
//!   inverts via `Exchange`. The VFS interface has no `RENAME_WHITEOUT`, and
//!   both steps run under the same directory transaction domain, so it is
//!   the accepted design rather than a pending TODO.
//! - Redirect is not implemented, so the flat EXDEV default applies.
//! - Target fallback is covered by the moved source: after a successful move,
//!   the target name is backed by the moved source's own upper/lower state,
//!   so no separate target projection or whiteout is needed.
//! - Overlay does not maintain merged `nlink` accounting: `metadata()` reports
//!   the visible source real inode's link count, so lower-layer additional
//!   links are not added into a synthetic overlay count.
//!
//! ## References
//!
//! - <https://elixir.bootlin.com/linux/v6.17/source/fs/overlayfs/copy_up.c#L1295-L1297>
//!   (Linux `ovl_copy_up` pre-rename copy-up)
//! - <https://elixir.bootlin.com/linux/v6.17/source/fs/overlayfs/dir.c#L1080-L1308>
//!   (Linux `ovl_rename` replace gate)
//! - <https://elixir.bootlin.com/linux/v6.17/source/fs/overlayfs/dir.c#L361-L430>
//!   (Linux `ovl_clear_empty` whiteout-residue sweep)

use super::whiteout;
use crate::{
    fs::{
        fs_impls::overlayfs::inode::{
            Lookup, NegativeLookup, OverlayInode, ReaddirIndex, xattr::set_impure_marker,
        },
        vfs::inode::{Inode, RenameMode},
    },
    prelude::*,
};

/// The two parent directory transaction payloads held by a rename.
pub(super) struct RenameLocks<'a> {
    pub(super) self_index: &'a mut Option<ReaddirIndex>,
    pub(super) target_index: Option<&'a mut Option<ReaddirIndex>>,
}

impl OverlayInode {
    /// Returns `Err(EXDEV)` for a cross-directory move of a lower-backed or
    /// merged directory: the `redirect_dir` policy is not implemented, so
    /// the EXDEV default applies (future work is tracked below).
    ///
    /// The gate runs from the fresh source projection before any upper side
    /// effect.
    // TODO(redirect_dir): replace this flat EXDEV default with a
    // redirect-policy probe bounded by the `redirect_max`-style length rule
    // when redirect support is implemented.
    pub(super) fn cross_device_gate(&self, source: &Lookup) -> Result<()> {
        let Lookup::Positive(source_inode) = source else {
            return Ok(());
        };
        if !source_inode.type_().is_directory() {
            return Ok(());
        }
        if source_inode.lowers.is_empty() {
            return Ok(());
        }
        Err(Error::with_message(
            Errno::EXDEV,
            "the overlay cross-directory rename of a lower-backed or merged directory \
             requires the not-yet-implemented redirect_dir policy",
        ))
    }

    /// Runs the upper rename recipe under the caller's two parent directory
    /// transaction guards.
    ///
    /// Any failure after the physical upper rename committed triggers the
    /// conservative reconcile of the whole affected set as a unit before the
    /// error is returned.
    pub(super) fn rename_upper(
        &self,
        old_name: &str,
        target: &Arc<OverlayInode>,
        new_name: &str,
        mode: RenameMode,
        source_lookup: &Lookup,
        mut locks: RenameLocks<'_>,
    ) -> Result<()> {
        let fs = self.fs_arc()?;

        let source_inode = match source_lookup {
            Lookup::Positive(inode) => inode.clone(),
            Lookup::Negative(_) => {
                return Err(Error::with_message(
                    Errno::ENOENT,
                    "the rename source is not visible under the parent DIR",
                ));
            }
        };
        let source_has_lower = !source_inode.lowers.is_empty();
        let target_lookup = fs.lookup(target, new_name)?;
        let target_is_whiteout = matches!(
            &target_lookup,
            Lookup::Negative(NegativeLookup::HiddenByWhiteout)
        );
        let target_is_positive = matches!(&target_lookup, Lookup::Positive(_));

        // A visible target under `NoReplace` is `EEXIST`: the upper rename's
        // `NOREPLACE` only observes the upper namespace, so a lower-visible
        // name must still fail.
        if mode == RenameMode::NoReplace && target_is_positive {
            return Err(Error::with_message(
                Errno::EEXIST,
                "the rename target already exists and is visible",
            ));
        }

        // `Replace` over a visible lower-backed directory target requires the
        // merged target directory to be overlay-visible-empty before the move
        // (whiteout-hidden children do not count; a pure-upper target defers
        // to the upper rename's own emptiness enforcement). The gate records
        // the fresh target facts so the target's physical whiteout-residue
        // sweep can run after the per-branch promotions.
        let gate_target_facts = if mode == RenameMode::Replace
            && target_is_positive
            && let Lookup::Positive(target_object) = &target_lookup
            && target_object.type_().is_directory()
        {
            let target_facts = target_object.real_object_stack();
            if !target_facts.lowers.is_empty() && target_object.visible_child_count()? != 0 {
                return Err(Error::with_message(
                    Errno::ENOTEMPTY,
                    "the overlay rename target directory is not empty",
                ));
            }
            Some(target_facts)
        } else {
            None
        };

        let upper_parent_path = self.upper_parent_path()?;
        let target_upper_parent_path = target.upper_parent_path()?;

        // When the `Replace` gate passed for a directory target with a
        // physical upper copy, sweep the target's physical whiteout residue
        // before the physical rename. Strict and pre-commit: a failure aborts
        // before the inlined recipe, whose pre-commit cleanup is a no-op
        // because no workdir temp is staged.
        if let Some(target_upper_dir) = gate_target_facts
            .as_ref()
            .and_then(|target_facts| target_facts.upper.as_ref())
        {
            whiteout::cleanup_upper_whiteouts(&target_upper_dir.real_path()?)?;
        }

        // A cross-directory move of a source with lower fallback makes the
        // target parent impure: persist the impure marker before the
        // physical rename (before committing the rename).
        let same_parent = self.key() == target.key();
        if !same_parent && source_has_lower {
            set_impure_marker(target_upper_parent_path.inode())?;
        }

        let mut committed = false;
        let result: Result<()> = (|| {
            // A whiteout target is always replaced or switched (never a
            // visible `NOREPLACE` failure): a lower-backed source
            // switches via `Exchange` (the whiteout lands at the source
            // name, avoiding a composed second step), any other source
            // consumes it with a plain `Replace`, and a caller-requested
            // `Exchange` is preserved.
            let effective_mode = match mode {
                RenameMode::Exchange => RenameMode::Exchange,
                _ if target_is_whiteout && source_has_lower => RenameMode::Exchange,
                _ if target_is_whiteout => RenameMode::Replace,
                _ => mode,
            };
            if same_parent {
                upper_parent_path.rename(old_name, &upper_parent_path, new_name, effective_mode)?;
            } else {
                upper_parent_path.rename(
                    old_name,
                    &target_upper_parent_path,
                    new_name,
                    effective_mode,
                )?;
            }
            committed = true;
            if source_has_lower && !target_is_whiteout && mode != RenameMode::Exchange {
                fs.publish_whiteout(&upper_parent_path, old_name, None)?;
            }
            // Rename reorders the visible sequence; the conservative rule
            // invalidates on every affected parent (same parent once).
            self.finish_whiteout_index(None, locks.self_index);
            if !same_parent {
                let Some(index) = locks.target_index.as_mut() else {
                    unreachable!("a different rename target parent has a lock");
                };
                target.invalidate_readdir_index(index);
            }
            Ok(())
        })();
        match result {
            Ok(()) => {}
            Err(err) => {
                if committed {
                    if same_parent {
                        target.invalidate_readdir_index(locks.self_index);
                        self.invalidate_readdir_index(locks.self_index);
                    } else {
                        let Some(index) = locks.target_index.as_mut() else {
                            unreachable!("a different rename target parent has a lock");
                        };
                        target.invalidate_readdir_index(index);
                        self.invalidate_readdir_index(locks.self_index);
                    }
                }
                return Err(err);
            }
        }
        // A cross-directory rename may have restored purity in the source or
        // target parent (the overwrite-of-origin-target case can clear the
        // target's last origin-bearing entry) — refresh both markers
        // best-effort (the mutation already committed; a refresh failure
        // never fails the rename).
        if !same_parent {
            self.refresh_impure_marker_best_effort(locks.self_index, "rename: source parent");
            let Some(index) = locks.target_index.as_mut() else {
                unreachable!("a different rename target parent has a lock");
            };
            target.refresh_impure_marker_best_effort(index, "rename: target parent");
        }
        Ok(())
    }
}
