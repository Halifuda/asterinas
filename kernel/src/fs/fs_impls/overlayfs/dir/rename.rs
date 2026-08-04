// SPDX-License-Identifier: MPL-2.0

//! The rename recipes of the `namespace_mutation_whiteout` meso (meso-06;
//! `P1-29`/`P1-30`).
//!
//! This module hosts the two frozen rename-family recipe methods on
//! [`OverlayInode`]: the `P1-30` EXDEV gate ([`OverlayInode::cross_device_gate`],
//! the lower-backed/merged-directory cross-directory default) and the upper
//! rename recipe ([`OverlayInode::rename_upper`], same-directory and
//! cross-directory physical moves with the inline dual-parent publication).
//! The thin `Inode`-trait `rename` entry and the two-parent `DIR` inlet live
//! in the sibling `dir/mod.rs` (frozen module layout, spec §4); the entry
//! holds both parent `DIR` domains, runs the mutating admission per affected
//! parent, derives the fresh source projection (Case 10), composes the EXDEV
//! gate for cross-directory moves (spec §7.4 step 3 — before any upper side
//! effect), and delegates the per-branch promotion, the physical upper
//! rename, the source-whiteout compose, the inline dual-parent publication,
//! and the Case-13 reconcile to this file (spec §7.4 steps 4-6).
//!
//! Lock contract (spec §3/§7.4): the caller (the `dir/mod.rs` entry) holds
//! both parent `DIR` transaction locks in stable object-identity order and
//! has pinned the mount; this module acquires no Overlay lock of its own
//! beyond the brief `INODE` facts snapshots inside
//! `facts_snapshot`/`select_real_inode` (snapshot-and-release, never held
//! across an underlying call) and the meso-04 `CUL` domains entered by the
//! per-branch promotions (`ensure_upper_authority` for the source object and
//! the meso-05 real admission stage B — `check_permission(AccessType::
//! Mutating, ...)` — for each parent, under the caller-held `DIR`s, meso-04
//! §3.2 item 7). The underlying upper/workdir operations (`rename`/
//! `lookup`/the whiteout publish) run in the sleep-capable `DIR` domain under
//! the underlying filesystem's own locking; no `WL`/spin domain is entered
//! and no `WL` payload is touched here (the whiteout cache is the sibling
//! `dir/whiteout.rs` owner, `P1-36`). `MOUNT` is never acquired; no Overlay
//! lock crosses the return boundary.
//!
//! Visibility: both helpers are declared `pub(super)` — visible only within
//! the `dir` module tree — because their only consumers are the sibling
//! `dir/mod.rs` `Inode::rename` entry (spec §1 "Must Remain Internal":
//! nothing in `dir/rename.rs` is visible outside `dir` except the frozen
//! `Inode` entries). The spec's unqualified `fn` is read through the dispatch
//! override exactly as the Wave-4 precedent widened `copyup/promote.rs::
//! promote` and the sibling `dir/create.rs::create_object` for their
//! sibling-module consumers.
//!
//! Recorded recipe readings (implementation-time resolutions of the frozen
//! spec text; no signature change):
//!
//! - **Source-whiteout compose (recorded divergence, spec §7.4 step 5;
//!   BC-6 §61; Hazard 6):** Asterinas `RenameMode` has no `RENAME_WHITEOUT`
//!   flag (verified `kernel/src/fs/vfs/fs_apis/inode.rs:753-758`), so when
//!   the moved source had a lower fallback the source-name whiteout is a
//!   composed second upper step — the plain upper `rename`, then
//!   `publish_whiteout` at the old name — inside the same `DIR` domain(s).
//!   The intermediate (the lower name temporarily visible at the old
//!   position) is unobservable under `DIR` and is conservatively reconciled
//!   if the compose fails (Case 13). A whiteout target inverts the compose:
//!   the rename switches whiteouts via `RenameMode::Exchange` (Linux
//!   `ovl_rename_start` "Switch whiteouts"), moving the target whiteout to
//!   the source name, so no composed second step is needed.
//! - **Target lower fallback (BC-6 §61 "target 若有 lower fallback，必须
//!   建立相应 hidden state"):** after the move the moved source's upper
//!   object at the target name IS the target's hidden state — it covers the
//!   target's lower fallback exactly as Linux `ovl_rename_upper` publishes no
//!   target-name whiteout (`dir.c:1135-1339`). A literal target-name whiteout
//!   would hide the moved source and break the rename; the only whiteout this
//!   recipe publishes is the source-name compose. The `Replace`-mode overlay
//!   emptiness gate below is the merged-target check Linux runs instead
//!   (`ovl_check_empty_dir` in `ovl_rename_start`).
//! - **`P2-02` redirect insertion point (spec §2.4/§7.4 step 3):** no
//!   redirect option exists on the mount and no redirect xattr is written
//!   this wave; the EXDEV default is frozen for every cross-directory
//!   lower-backed/merged directory source. Linux also sets an opaque marker
//!   on a pure-upper directory moved into a merged parent
//!   (`ovl_set_opaque_xerr`, `dir.c`); that marker write is not part of the
//!   frozen §7.4 recipe text and is recorded as a Linux-fidelity gap for the
//!   `P2-02` wave alongside the `redirect_max`-style length obligation. The
//!   overlay-level emptiness gate keeps the moved-dir-over-lower-dir case
//!   from producing a wrong visible merge (`ENOTEMPTY`, Case 9 semantics).
//! - **nlink bookkeeping:** Linux's `ovl_nlink_start`/`ovl_drop_nlink`
//!   accounting for replaced targets is not tracked in Asterinas (no overlay
//!   nlink model); the replaced target's upper inode simply loses its
//!   namespace name, matching the `P2-04`/`P3-01` origin/index deferral.
//!
//! No `.unwrap()`/`.expect()` appears in any production path.

use crate::{
    fs::{
        file::Permission,
        fs_impls::overlayfs::{
            AccessType,
            projection::{
                Binding, BindingKey, HiddenEvidence, NegativeBinding, OverlayInode, PositiveBinding,
            },
        },
        vfs::inode::{Inode, RenameMode},
    },
    prelude::*,
};

impl OverlayInode {
    /// Returns `Err(EXDEV)` for a cross-directory move of a lower-backed or
    /// merged directory when the `redirect_dir` policy is not enabled
    /// (`P1-30`, spec §4 `dir/rename.rs`; Cases 8).
    ///
    /// The frozen gate runs from the fresh source projection **before any
    /// upper side effect** — no parent/source promotion, no workdir temp, no
    /// whiteout, no redirect xattr, no binding/index update (Case 8;
    /// spec §7.4 step 3; BC-6 §61). The caller (`dir/mod.rs` `Inode::rename`)
    /// composes the cross-directory condition ("different parents" — the
    /// frozen signature carries no target, so the same-parent comparison is
    /// the entry's `DIR`-inlet identity check) and invokes this gate only for
    /// a cross-directory move; this method checks the source-object side of
    /// the frozen condition: a lower-backed or merged **directory** source.
    /// "Lower-backed or merged" is exactly `facts.lowers()` non-empty
    /// (a `Merged` directory has upper + lowers; a lower-backed `Single`
    /// directory has `upper == None`, `lowers[0]`; the frozen facts
    /// invariant `upper.is_some() || !lowers.is_empty()` guarantees the
    /// empty-lowers case is genuinely upper-only).
    ///
    /// The `redirect_dir` policy is the `P2-02` insertion point (spec §2.4):
    /// no redirect mount option is published and no redirect xattr is ever
    /// written this wave, so the EXDEV default is frozen (wave-4 repair item
    /// 8 deleted the constant-false `redirect_enabled` guard; the insertion
    /// point is the EXDEV rejection below); when `P2-02` lands, that
    /// rejection becomes the redirect-policy probe bounded by the recorded
    /// `redirect_max`-style length rule (spec §2.4 "No redirect xattr is
    /// written this wave; the EXDEV default is frozen").
    ///
    /// Lock contract: no Overlay lock is acquired or held; the caller holds
    /// both parent `DIR` domains. The `&self` receiver is the frozen
    /// owner shape (the mutated directory is the recipe's natural owner);
    /// the gate's evidence is entirely the caller's fresh `source` binding
    /// (the `link.rs::link_source` receiver precedent).
    pub(super) fn cross_device_gate(&self, source: &Binding) -> Result<()> {
        // Only a directory source can be EXDEV-gated: P1-29 same-directory
        // and P1-30 non-directory moves always proceed. The `into_inode`
        // route is the only overlayfs-visible access to a positive
        // binding's inode payload (the field is projection-private); a
        // negative binding has no inode and never gates.
        let Some(source_inode) = source.clone().into_inode() else {
            return Ok(());
        };
        if !source_inode.type_().is_directory() {
            return Ok(());
        }
        // Lower-backed or merged: a lower object exists under the source
        // name (the empty-lowers case is a pure-upper directory, movable).
        if source_inode.facts_snapshot().lowers().is_empty() {
            return Ok(());
        }
        // The P2-02 redirect policy is an insertion point only: this wave
        // never enables it, so the EXDEV default fires for every
        // cross-directory lower-backed/merged directory source (the entry
        // composes the cross-directory condition). When `P2-02` lands, the
        // redirect policy probe replaces this rejection at this point
        // (wave-4 repair item 8: the constant-false `redirect_enabled`
        // guard is deleted; the insertion point is the EXDEV production
        // below, exactly where the review's comment names it).
        Err(Error::with_message(
            Errno::EXDEV,
            "the overlay cross-directory rename of a lower-backed or merged directory \
             requires the deferred redirect_dir policy",
        ))
    }

    /// Runs the upper rename recipe — per-branch promotion, the physical
    /// upper rename (same-directory and cross-directory), the source-whiteout
    /// compose, and the inline dual-parent publication (`P1-29`/`P1-30`,
    /// spec §4 `dir/rename.rs`; Cases 4/5/10/12/13).
    ///
    /// The caller (the `dir/mod.rs` `Inode::rename` entry) holds both parent
    /// `DIR` domains and has run the mutating admission per affected parent
    /// and the `P1-30` EXDEV gate. "Source has a lower fallback" (spec §7.4
    /// step 5) is derived inside this recipe from the freshly projected
    /// source facts — the entry passes no boolean (wave-4 round-2 repair
    /// item 4). This recipe then:
    ///
    /// 1. **Re-derives the fresh source and target projections under `DIR`**
    ///    (spec §7.4 step 2; the same binding-cache-first evidence the entry
    ///    used — a negative source is `ENOENT`, Case 10, and a visible
    ///    target under `NoReplace` is `EEXIST` — and the `Replace`-mode
    ///    merged-target emptiness gate, Linux `ovl_check_empty_dir`).
    /// 2. **Promotes each branch in stable object-identity order** (spec
    ///    §7.4 step 4; meso-04 §3.2 item 6): the source object
    ///    (`ensure_upper_authority`), then the source parent, then the target
    ///    parent (meso-05 stage B via `check_permission(AccessType::Mutating,
    ///    ...)`); each branch's scope is decided under its own `CUL`, and
    ///    the entry's earlier admission makes these idempotent no-ops in the
    ///    ordinary path (the `create.rs::create_upper_only` precedent).
    /// 3. **Performs the physical upper rename** — same-directory
    ///    `upper_parent.rename(old, upper_parent, new, ...)`, cross-directory
    ///    `upper_parent.rename(old, target_upper_parent, new, ...)` — with
    ///    the frozen `RenameMode` (Replace/NoReplace/Exchange per `mode`)
    ///    and the whiteout-target adjustments of Linux `ovl_rename_start`
    ///    (consume/replace a whiteout marker; switch whiteouts via
    ///    `Exchange` when the source has a lower fallback), and then the
    ///    source-whiteout compose when the moved source had a lower fallback
    ///    (Asterinas has no `RENAME_WHITEOUT`; the compose is a second upper
    ///    step inside the same `DIR` domain(s), spec §7.4 step 5).
    /// 4. **Publishes inline** (revision 01, override 2 — the former
    ///    `publish_rename` helper is dissolved, spec §7.4 step 6): the source
    ///    binding (`BindingCache::invalidate`, or a
    ///    `Negative(HiddenByWhiteout)` insert when a source whiteout was
    ///    published, pinning the whiteout's real inode via `HiddenEvidence`
    ///    with layer index 0), the target binding (`BindingCache::insert`
    ///    positive, sharing the moved source `OverlayInode` with the kind
    ///    derived from the source inode's own facts — `lookup_binding`
    ///    derives the per-name kind from the projected facts, so the
    ///    published binding mirrors the object's classification), and
    ///    `invalidate_readdir_index` on both affected parents (same parent
    ///    once; rename is a reordering operation, frozen §5.2 conservative
    ///    floor).
    ///
    /// Any failure after the physical upper rename committed — the
    /// source-whiteout compose or the hidden-binding evidence re-lookup —
    /// triggers the Case-13 conservative reconcile of the whole affected set
    /// as a unit (spec §5.3/§7.4 step 6) before the error is returned.
    ///
    /// Lock contract: runs under the caller's two parent `DIR` domains
    /// (Level 2); the promotions enter the per-branch `CUL` (Level 3) →
    /// `INODE` (Level 4) domains in the frozen order; the underlying upper
    /// operations may block and run in the sleep-capable domain, never under
    /// `WL` (Level 5) or any spin lock (Hazard 2). No Overlay lock is
    /// acquired or held by this method and none crosses the return boundary.
    pub(super) fn rename_upper(
        &self,
        old_name: &str,
        target: &Arc<OverlayInode>,
        new_name: &str,
        mode: RenameMode,
    ) -> Result<()> {
        let fs = self.fs_arc()?;

        // Fresh source and target projections under the caller-held `DIR`
        // domain(s) (spec §7.4 step 2; never from a stale VFS dentry). The
        // source must be visible (Case 10 — the `DIR`-domain projection is
        // authoritative over the VFS dentry that may have triggered the
        // call).
        let source_binding = fs.lookup_binding(&self.facts_snapshot(), old_name)?;
        let source_inode = source_binding.clone().into_inode().ok_or_else(|| {
            Error::with_message(
                Errno::ENOENT,
                "the rename source is not visible under the parent DIR",
            )
        })?;
        // "Source has a lower fallback" decides whether the source name gets
        // a whiteout after the move (spec §7.4 step 5). Wave-4 round-2
        // repair item 4: the signal is derived HERE from the freshly
        // projected source facts — the entry no longer passes a bare bool —
        // and `lowers` is retained across copy-up, so the value is stable
        // through the per-branch promotion below.
        let source_has_lower = !source_inode.facts_snapshot().lowers().is_empty();
        let target_binding = fs.lookup_binding(&target.facts_snapshot(), new_name)?;
        let target_is_whiteout = matches!(
            &target_binding,
            Binding::Negative(NegativeBinding::HiddenByWhiteout(_))
        );
        let target_is_positive = matches!(&target_binding, Binding::Positive(_));

        // A visible target under `NoReplace` is `EEXIST` (the upper rename's
        // NOREPLACE only observes the upper namespace — a lower-visible name
        // must still fail, the Linux `ovl_copy_up(new)` equivalence); the
        // fresh projection is authoritative and no upper side effect runs.
        if mode == RenameMode::NoReplace && target_is_positive {
            return Err(Error::with_message(
                Errno::EEXIST,
                "the rename target already exists and is visible",
            ));
        }

        // `Replace` over a visible lower-backed directory target requires the
        // merged target directory to be Overlay-visible-empty before the move
        // (Linux `ovl_check_empty_dir` in `ovl_rename_start`; the upper
        // rename only sees the upper dir). The meso-03 `visible_child_count`
        // seam counts visible children (whiteout-hidden children do not
        // count, BC-6 §60.2); a pure-upper target defers to the upper
        // rename's own emptiness enforcement (Case 12).
        if mode == RenameMode::Replace
            && target_is_positive
            && let Some(target_object) = target_binding.clone().into_inode()
            && target_object.type_().is_directory()
        {
            let target_facts = target_object.facts_snapshot();
            if !target_facts.lowers().is_empty()
                && target_object.visible_child_count(&target_facts)? != 0
            {
                return Err(Error::with_message(
                    Errno::ENOTEMPTY,
                    "the overlay rename target directory is not empty",
                ));
            }
        }

        // Per-branch promotion in stable object-identity order (spec §7.4
        // step 4; meso-04 §3.2 item 6): the source object first, then the
        // source parent, then the target parent. Each branch's scope is
        // decided under its own `CUL`; `ensure_upper_authority` and the
        // meso-05 stage-B promotion are idempotent fast paths when the
        // branch is already upper-backed (the entry's admission already
        // promoted both parents, so these are no-ops in the ordinary path).
        source_inode.ensure_upper_authority()?;
        self.check_permission(AccessType::Mutating, Permission::MAY_WRITE)?;
        target.check_permission(AccessType::Mutating, Permission::MAY_WRITE)?;

        // The promoted upper real parents — the physical-operation targets
        // (post-promotion `select_real_inode`, `dir/mod.rs::upper_parent`).
        let upper_parent = self.upper_parent()?;
        let target_upper_parent = target.upper_parent()?;

        // The shared recipe scaffold (wave-4 round-2 repair item 2): the
        // commit marker is flipped at the physical upper rename and the
        // Case-13 reconcile classification is owned by `run_recipe`; the
        // plain upper rename stages no workdir temp, so `temp_name` is
        // `None` (a pre-commit failure has nothing to clean).
        self.run_recipe(
            &fs,
            None,
            || self.invalidate_stale_cache(&[(target.as_ref(), new_name), (self, old_name)]),
            |marker| {
                // Whiteout-target adjustments (Linux `ovl_rename_start`): a
                // whiteout is a negative name — never a visible NOREPLACE
                // failure and never an ordinary rename target — so it is always
                // replaced or switched: a source with a lower fallback switches
                // whiteouts via `Exchange` (the whiteout lands at the source
                // name, so the composed second step is not needed); any other
                // source consumes the marker with a plain replace (spec §7.4
                // step 5 "target whiteout -> consume/replace per §7.1
                // semantics"). A caller-requested `Exchange` is preserved.
                let effective_mode = match mode {
                    RenameMode::Exchange => RenameMode::Exchange,
                    _ if target_is_whiteout && source_has_lower => RenameMode::Exchange,
                    _ if target_is_whiteout => RenameMode::Replace,
                    _ => mode,
                };
                let same_parent = self.key() == target.key();
                // The physical upper rename (spec §7.4 step 5): same-directory
                // against the single upper parent, cross-directory against the
                // promoted target upper parent.
                if same_parent {
                    upper_parent.rename(old_name, &upper_parent, new_name, effective_mode)?;
                } else {
                    upper_parent.rename(old_name, &target_upper_parent, new_name, effective_mode)?;
                }
                marker.commit();
                // Source-whiteout compose (recorded divergence, spec §7.4 step
                // 5; Hazard 6): when the moved source had a lower fallback and
                // the move vacated the source name without leaving a cover
                // (neither a switched whiteout nor a caller-requested
                // exchange), the source-name whiteout is the composed second
                // upper step inside the same `DIR` domain(s). The intermediate
                // (the lower name temporarily visible at the old position) is
                // unobservable under `DIR` and is conservatively reconciled if
                // the compose fails (Case 13).
                let mut source_whiteout_published = false;
                if source_has_lower && !target_is_whiteout && mode != RenameMode::Exchange {
                    fs.publish_whiteout(&upper_parent, old_name, None)?;
                    source_whiteout_published = true;
                }
                // Dual-parent publication (INLINE, spec §7.4 step 6; revision 01
                // override 2 — the `publish_rename` helper is dissolved).
                // Source binding: a published whiteout is inserted as the hidden
                // barrier binding (pinning the whiteout's real inode via
                // `HiddenEvidence`, layer index 0 = upper); a plain move leaves
                // the old name vacated, so the stale positive binding is
                // invalidated and the next lookup re-derives from upper truth.
                if source_whiteout_published {
                    let whiteout_real = upper_parent.lookup(old_name)?;
                    fs.bindings().insert(
                        BindingKey::new(self.key(), String::from(old_name)),
                        Arc::new(Binding::Negative(NegativeBinding::HiddenByWhiteout(
                            HiddenEvidence::new(0, whiteout_real),
                        ))),
                    );
                } else {
                    fs.bindings().invalidate(&self.key(), old_name);
                }
                // Target binding: the moved source is now the visible object at
                // the target name (Cases 4/5); its classification remains in
                // the source inode's own facts, so the published binding has no
                // stale per-name classification snapshot.
                fs.bindings().insert(
                    BindingKey::new(target.key(), String::from(new_name)),
                    Arc::new(Binding::Positive(PositiveBinding::new(source_inode.clone()))),
                );
                // Rename reorders the visible sequence; the frozen §5.2 rule is
                // the conservative invalidate on every affected parent (same
                // parent once).
                self.invalidate_readdir_index();
                if !same_parent {
                    target.invalidate_readdir_index();
                }
                Ok(())
            },
        )
    }
}
