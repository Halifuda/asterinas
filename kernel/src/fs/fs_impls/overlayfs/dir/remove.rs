// SPDX-License-Identifier: MPL-2.0

//! The remove recipes — `P1-26`/`P1-27`.
//!
//! This module hosts the single frozen recipe helper of the meso-06 spec §4
//! `dir/remove.rs` on [`OverlayInode`]: [`OverlayInode::remove_target`] — the
//! shared unlink/rmdir recipe parameterized by the closed [`RemoveKind`]
//! vocabulary (wave-4 repair item 12: the former `is_dir: bool` flag is
//! replaced by [`RemoveKind::{Unlink, Rmdir}`] so call sites name the
//! operation) that, under the caller-held parent `DIR`,
//! re-derives the fresh `(parent, name)` projection (Case 10 `ENOENT`),
//! runs the `P1-27` Overlay-visible emptiness gate (meso-03
//! `visible_child_count`) before any upper removal, and then decides
//! **pure-upper direct removal** (upper `unlink`/`rmdir`, no whiteout)
//! versus **lower-backed whiteout publication** (`publish_whiteout` over the
//! removed upper object, plus the `P1-27` clear-empty opaque-temp exchange
//! when the upper directory of a lower-backed directory holds hidden
//! entries that would otherwise leak or resist workdir cleanup). The thin
//! `Inode::unlink`/`Inode::rmdir` entries live in the sibling `dir/mod.rs`
//! pass and delegate into this file; `visible_child_count` is consumed from
//! the landed meso-03 seam, never re-implemented.
//!
//! Lock contract (spec §3/§7.2): the caller (the `dir/mod.rs` entry) holds
//! the parent `DIR` transaction lock and has pinned the mount. This module
//! acquires no Overlay lock of its own beyond the brief `INODE` facts
//! snapshots inside `facts_snapshot`/`select_real_inode` (snapshot-and-
//! release, never held across an underlying call), the brief index `INODE`
//! sections inside the meso-03 `visible_child_count`/`readdir_index_remove`
//! seams, and the meso-04 `CUL` domain entered inside the real stage of
//! `check_permission(AccessType::Mutating, ...)` (stage B promotes the
//! parent under the caller-held `DIR`, meso-04 §3.2 item 7). Upper/workdir
//! physical operations (`unlink`/`rmdir`/`rename`/`set_xattr`) run in the
//! sleep-capable `DIR` domain under the underlying filesystem's own locking;
//! no `WL`/spin domain is entered and no `WL` payload is touched (the
//! whiteout cache and the whiteout publish mechanics are the sibling
//! `dir/whiteout.rs` owner, `P1-25`/`P1-36`). All `DIR`/`CUL`/`INODE`
//! domains are released before any VFS-visible return (§3 outlet); `MOUNT`
//! is never acquired.
//!
//! Visibility: `remove_target` is declared `pub(super)` — visible only
//! within the `dir` module tree — because its only consumer is the sibling
//! `dir/mod.rs` mutation entry (`Inode::unlink`/`Inode::rmdir`), exactly as
//! the Wave-4 precedent widened the `dir/create.rs` recipes for their
//! sibling-module consumer. No `.unwrap()`/`.expect()` appears in any
//! production path; hard invariant failures use the recorded
//! `Error::with_message`/`unreachable!` precedents.

use crate::{
    fs::{
        file::{InodeType, Permission},
        fs_impls::overlayfs::{
            metadata_security::xattr::{OPAQUE_MARKER_VALUE, OPAQUE_XATTR_FULL_NAME},
            AccessType,
            projection::{Binding, BindingKey, HiddenEvidence, NegativeBinding, OverlayInode},
        },
        vfs::{
            inode::{Inode, RenameMode},
            path::is_dot_or_dotdot,
            xattr::{XattrName, XattrSetFlags},
        },
    },
    prelude::*,
};

/// The remove operation kind of [`OverlayInode::remove_target`] (`P1-26`/
/// `P1-27`; wave-4 repair item 12 — replaces the `is_dir: bool` flag so the
/// call sites in `dir/mod.rs` name the operation they perform instead of
/// `remove_target(name, false)` / `remove_target(name, true)`).
///
/// Closed two-variant set: [`RemoveKind::Unlink`] (type gate `EISDIR` +
/// direct unlink/whiteout publish) and [`RemoveKind::Rmdir`] (emptiness gate
/// + clear-empty opaque exchange). The `remove_target` recipe branches on
/// this closed vocabulary instead of a boolean (the wave-4 review's
/// `no-bool-args` fix).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum RemoveKind {
    /// The `P1-26` unlink operation.
    Unlink,
    /// The `P1-27` rmdir operation.
    Rmdir,
}

impl OverlayInode {
    /// Removes one visible name from this overlay directory (`P1-26`/`P1-27`,
    /// meso-06 spec §4 `dir/remove.rs`).
    ///
    /// The shared unlink/rmdir recipe (spec §7.2):
    ///
    /// 1. **Fresh projection (Case 10):** `lookup_binding` under the
    ///    caller-held parent `DIR` re-derives the current positive/negative
    ///    binding — never a stale VFS dentry. The target must be
    ///    `Positive`; a negative projection (absent or hidden) is
    ///    `Err(ENOENT)`.
    /// 2. **`P1-27` visible-emptiness gate (rmdir only; spec §7.2 step 2):
    ///    `visible_child_count`** (meso-03 seam) counts the Overlay-visible
    ///    children: any visible upper/lower/merged child → `ENOTEMPTY`
    ///    (Case 9); a non-directory target → `ENOTDIR`; a
    ///    `NeedsRebuild`-unresolvable index → conservative `ENOTEMPTY`
    ///    (never an upper-only emptiness guess). Whiteout-hidden children do
    ///    not count (BC-6 §60.2). `unlink` skips this gate.
    /// 3. **Stage B admission:** `check_permission(AccessType::Mutating,
    ///    MAY_WRITE)` promotes this parent to upper authority under the
    ///    caller-held `DIR` (stage A, the EROFS gate, is the entry's
    ///    admission), then `upper_parent()` resolves the promoted upper real
    ///    parent directory.
    /// 4. **Pure-upper target** (upper-backed with no lower fallback and no
    ///    opaque barrier): direct `upper_parent.unlink(name)`/`rmdir(name)`
    ///    (no whiteout); publication inline: `BindingCache::invalidate` +
    ///    `readdir_index_remove` (spec §7.2 step 3; both seams infallible,
    ///    so no Case-13 arm is reachable on this path).
    /// 5. **Upper-over-lower / lower-only / opaque-over-lower target:**
    ///    publication of a whiteout at `(upper_parent, name)` via the
    ///    sibling `publish_whiteout` seam (`P1-25`; `Replace` over a present
    ///    non-dir upper object, `Exchange` + workdir cleanup of the
    ///    displaced dir for a present upper directory, `link` for an absent
    ///    upper name); for a lower-backed **directory** whose upper dir
    ///    holds entries (necessarily whiteouts — the emptiness gate has
    ///    already refused visible children), the clear-empty path
    ///    (spec §7.2a) first replaces the upper dir with a workdir-prepared
    ///    opaque temp dir (atomic `Exchange`), cleans the displaced old
    ///    upper dir in the workdir, and then lets `publish_whiteout`
    ///    exchange the whiteout over the opaque temp. Publication inline:
    ///    `BindingCache::insert` `Negative(HiddenByWhiteout(HiddenEvidence))`
    ///    — the `HiddenEvidence` pin re-observes the published whiteout from
    ///    the upper — + `readdir_index_remove` (spec §7.2 step 4; §5.2).
    ///    The recipe distinguishes the pre-publication failure arm
    ///    (best-effort workdir temp cleanup; lower authority stays valid)
    ///    from the post-physical-success arm (Case 13 conservative
    ///    reconcile), honoring §5.3/§7.2's never-partial contract.
    ///
    /// # Recorded deviations and ambiguity resolutions (Creator report §5)
    ///
    /// - The frozen §7.2a prose "xattrs/metadata copied" is realized as the
    ///   metadata copy (owner/group/mode/times, the meso-04
    ///   `transfer_metadata` precedent) only; the transient opaque temp is
    ///   whiteout-hidden in the same transaction, so the full xattr buffer
    ///   copy is skipped (Linux copies xattrs for permanent-replacement
    ///   rename paths that do not exist in this pass). See the report §5
    ///   item 3.
    /// - A directory target is refused with `Err(EISDIR)` on the `unlink`
    ///   entry (Case 12's `EISDIR` for `P1-26`; the ramfs precedent): the
    ///   Asterinas VFS routes `unlink` on a directory into the fs, so
    ///   without this gate a lower-backed directory would be whiteout-
    ///   hidden instead of refused. See the report §5 item 1.
    /// - An opaque upper directory is classified as lower-backed (its
    ///   `facts.lowers()` is empty by the meso-02 opaque barrier rule, but a
    ///   hidden lower counterpart exists — Linux `ovl_lower_positive`); the
    ///   `is_opaque_directory()` probe extends the pure-upper test so
    ///   rmdir publishes a whiteout instead of exposing the hidden lower
    ///   directory (BC-6 §60.2). See the report §5 item 2.
    pub(super) fn remove_target(&self, name: &str, kind: RemoveKind) -> Result<()> {
        // The mount is pinned by the entry; the parent `DIR` is held.
        let fs = self.fs_arc()?;
        // Step 1 — fresh projection under `DIR` (spec §7.2 step 1; Case 10):
        // the VFS dentry may be stale, the `DIR`-domain projection is
        // authoritative. A negative projection (absent or hidden) is
        // `ENOENT`; the target must be visible.
        let parent_facts = self.facts_snapshot();
        let target_inode = fs
            .lookup_binding(&parent_facts, name)?
            .into_inode()
            .ok_or_else(|| {
                Error::with_message(Errno::ENOENT, "the overlay target does not exist")
            })?;
        let target_facts = target_inode.facts_snapshot();

        if kind == RemoveKind::Rmdir {
            // Step 2 — the `P1-27` visible-emptiness gate (runs before any
            // upper removal; spec §7.2 step 2, Case 9). The meso-03 seam
            // ensures the target index is `Valid` (rebuild under the same
            // `DIR` transaction) and counts the `Visible` entries; `.`/`..`
            // are never entries and whiteout-hidden children do not count.
            match target_inode.visible_child_count(&target_facts) {
                Ok(0) => {}
                Ok(_) => {
                    return Err(Error::with_message(
                        Errno::ENOTEMPTY,
                        "the overlay directory is not empty",
                    ));
                }
                Err(err) if err.error() == Errno::ENOTDIR => {
                    // A non-directory target cannot be rmdir'd.
                    return Err(err);
                }
                Err(_) => {
                    // `NeedsRebuild`-unresolvable: conservative `ENOTEMPTY`
                    // (never an upper-only emptiness guess, Case 9).
                    return Err(Error::with_message(
                        Errno::ENOTEMPTY,
                        "the overlay directory emptiness could not be verified",
                    ));
                }
            }
        } else if target_inode.type_().is_directory() {
            // Defensive type gate (recorded deviation; see the method doc):
            // the Asterinas VFS routes `unlink` on a directory into the fs
            // without refusing it, so every fs gates itself (ramfs
            // precedent). Without this gate a lower-backed directory would
            // be whiteout-hidden instead of refused with `EISDIR` (Case 12's
            // `EISDIR` for `P1-26`).
            return Err(Error::with_message(
                Errno::EISDIR,
                "a directory cannot be unlinked",
            ));
        }

        // Pure-upper vs lower-backed classification (spec §7.2 steps 3/4).
        // An upper object with an empty lower stack is pure-upper ONLY when
        // it is not an opaque directory: an opaque upper directory is a
        // lower-search barrier whose hidden lower counterpart still exists
        // (Linux `ovl_lower_positive`), so removing it must publish a
        // whiteout rather than expose the lower (BC-6 §60.2; recorded
        // ambiguity resolution, report §5 item 2).
        let is_pure_upper = match target_facts.upper() {
            Some(upper_obj) => {
                target_facts.lowers().is_empty() && !upper_obj.is_opaque_directory()?
            }
            None => false,
        };

        // Step 3/4 — stage B admission (promotes the parent under the held
        // `DIR`; stage A is the entry's admission) and the promoted upper
        // real parent (the physical-operation target).
        self.check_permission(AccessType::Mutating, Permission::MAY_WRITE)?;
        let upper_parent = self.upper_parent()?;

        if is_pure_upper {
            // Step 3 — pure-upper direct removal, no whiteout: the name is
            // genuinely gone from the upper namespace. Upper errors
            // propagate as-is (Case 12). Both publication seams are
            // infallible, so no Case-13 arm is structurally reachable here.
            if kind == RemoveKind::Rmdir {
                upper_parent.rmdir(name)?;
            } else {
                upper_parent.unlink(name)?;
            }
            fs.bindings().invalidate(&self.key(), name);
            self.readdir_index_remove(name);
            return Ok(());
        }

        // Step 4 — lower-backed target: preserve the lower result with a
        // published whiteout. `replace_target` tells `publish_whiteout`
        // (sibling `dir/whiteout.rs`, P1-25) the physical shape of the name:
        // `None` (name absent in the upper → link a whiteout at it) vs
        // `Some(type_)` (present upper object → `Replace` non-dir /
        // `Exchange` + displaced-dir cleanup for a dir). The target type for
        // rmdir is always `Dir` (the non-dir type gate above refused
        // rmdir-on-file with `ENOTDIR` and unlink-on-dir with `EISDIR`).
        let target_type = target_inode.type_();
        let replace_target = match target_facts.upper() {
            Some(_) => Some(target_type),
            None => None,
        };
        // The workdir temp name is generated up front (the meso-04 seam is
        // uniqueness-based, not lock-based) so the pre-publication failure
        // arm can best-effort clean the temp regardless of which leg created
        // it; cleaning a never-created name is an ignored `ENOENT`.
        let temp_name = fs.generate_workdir_temp_name(name, &upper_parent);
        // The shared recipe scaffold (wave-4 round-2 repair item 2): the
        // commit marker is flipped at each physical upper commit point and
        // the Case-13 reconcile / pre-publication cleanup classification is
        // owned by `run_recipe` (spec §5.3/§7.2 step 5).
        self.run_recipe(
            &fs,
            Some(&temp_name),
            || self.invalidate_stale_cache(&[(self, name)]),
            |marker| {
                if kind == RemoveKind::Rmdir {
                    // Clear-empty probe (spec §7.2 step 4): the upper directory
                    // of a lower-backed directory may hold entries that the
                    // merged view hides (whiteouts — the emptiness gate has
                    // already refused visible children). Those entries must not
                    // leak and would defeat `publish_whiteout`'s displaced-dir
                    // workdir cleanup (`ENOTEMPTY`), so the §7.2a clear-empty
                    // path replaces the upper dir first.
                    if let Some(upper_obj) = target_facts.upper() {
                        let mut upper_names = Vec::new();
                        upper_obj.real_inode().readdir_at(0, &mut upper_names)?;
                        upper_names.retain(|entry| !is_dot_or_dotdot(entry));
                        if !upper_names.is_empty() {
                            // §7.2a — clear-empty: the upper dir is replaced by
                            // a workdir-prepared opaque temp dir (atomic
                            // exchange); the old upper dir is cleaned up in the
                            // workdir; the whiteout is then published at the
                            // name by the recipe's common publish step below.
                            // The temp is never a visible source (BC-6 §57
                            // item 5).
                            let old_upper_dir = upper_obj.real_inode().clone();
                            let mode = old_upper_dir.mode()?;
                            let temp = fs.create_workdir_temp(&temp_name, InodeType::Dir, mode)?;
                            // The opaque marker is part of the replacement
                            // directory's complete preparation: it keeps the
                            // name a lower-search barrier at every instant of
                            // the swap (crash window included), gated by the
                            // meso-01 private-xattr capability (spec §5.1).
                            let can_store_private_xattr = fs
                                .policy()
                                .upper_capabilities()
                                .is_some_and(|caps| caps.can_store_private_xattr());
                            if !can_store_private_xattr {
                                return Err(Error::with_message(
                                    Errno::EOPNOTSUPP,
                                    "the upper filesystem cannot store the opaque marker \
                                     required for the clear-empty directory exchange",
                                ));
                            }
                            let marker_name = XattrName::try_from_full_name(OPAQUE_XATTR_FULL_NAME)
                                .ok_or_else(|| {
                                    Error::with_message(
                                        Errno::EINVAL,
                                        "invalid overlay opaque marker xattr name",
                                    )
                                })?;
                            let mut marker_reader =
                                VmReader::from(OPAQUE_MARKER_VALUE).to_fallible();
                            temp.set_xattr(
                                marker_name,
                                &mut marker_reader,
                                XattrSetFlags::CREATE_OR_REPLACE,
                            )?;
                            // Metadata copy (spec §7.2a "xattrs/metadata
                            // copied"): owner/group/mode/times from the old
                            // upper dir onto the temp — the meso-04
                            // `transfer_metadata` precedent. The full xattr
                            // buffer copy is a recorded deviation (see the
                            // method doc / report §5 item 3).
                            temp.set_owner(old_upper_dir.owner()?)?;
                            temp.set_group(old_upper_dir.group()?)?;
                            temp.set_mode(old_upper_dir.mode()?)?;
                            temp.set_atime(old_upper_dir.atime());
                            temp.set_mtime(old_upper_dir.mtime());
                            temp.set_ctime(old_upper_dir.ctime());
                            // Atomic exchange: the opaque temp becomes the upper
                            // object at `name` and the old upper dir moves to
                            // the workdir temp name. From this point the visible
                            // upper namespace has changed (Case 13 reconcile
                            // applies on any later failure).
                            // The workdir root resolves through the single shared
                            // resolver (`OverlayInode::workdir_root`, wave-4 repair
                            // item 11) — the inline claims block is deleted.
                            let workdir = self.workdir_root()?;
                            workdir.rename(&temp_name, &upper_parent, name, RenameMode::Exchange)?;
                            marker.commit();
                            // Clean the displaced old upper dir in the workdir:
                            // every remaining entry is a whiteout (the emptiness
                            // gate refused visible children), so unlink each and
                            // rmdir the dir. Best-effort: a cleanup failure is
                            // the recorded P3-09 workdir-cleanup obligation and
                            // never becomes a visible namespace entry — the
                            // whiteout publish below proceeds with the opaque
                            // temp at `name` (BC-6 §57/§60.2).
                            let mut displaced_names = Vec::new();
                            if old_upper_dir.readdir_at(0, &mut displaced_names).is_ok() {
                                for entry in displaced_names {
                                    if !is_dot_or_dotdot(&entry) {
                                        let _ = old_upper_dir.unlink(&entry);
                                    }
                                }
                            }
                            if let Err(cleanup_err) = workdir.rmdir(&temp_name) {
                                warn!(
                                    "overlay clear-empty: workdir cleanup of the displaced \
                                     directory {:?} failed (P3-09 residue, never a visible \
                                     source): {:?}",
                                    temp_name, cleanup_err
                                );
                            }
                        }
                    }
                }
                // The whiteout publish (sibling `dir/whiteout.rs`, P1-25): a
                // present non-dir upper object is replaced (`Replace`), a
                // present dir (the empty upper dir or the opaque temp) is
                // exchanged and its displaced form cleaned in the workdir
                // (`Exchange`), and an absent upper name gets a whiteout linked
                // in (`link`). Marker bytes are written by the sibling owner;
                // no `WL` payload is touched here.
                fs.publish_whiteout(&upper_parent, name, replace_target)?;
                marker.commit();
                // Semantic publication — inline seam composition (revision 01,
                // override 2; spec §7.2 step 4, Case 2): the whiteout is
                // re-observed from the upper (layer 0) so the published
                // `HiddenByWhiteout` binding pins its strong `HiddenEvidence`
                // barrier (BC-2 lifetime rule), then the parent index tombstones
                // the now-hidden name (the `readdir_index_remove` decision seam,
                // §5.2). The re-observation is fallible: on failure the whiteout
                // is already published — Case 13.
                let whiteout_inode = upper_parent.lookup(name)?;
                let evidence = HiddenEvidence::new(0, whiteout_inode);
                fs.bindings().insert(
                    BindingKey::new(self.key(), String::from(name)),
                    Arc::new(Binding::Negative(NegativeBinding::HiddenByWhiteout(evidence))),
                );
                self.readdir_index_remove(name);
                Ok(())
            },
        )
    }
}
