// SPDX-License-Identifier: MPL-2.0

//! The create-object recipes of the `namespace_mutation_whiteout` meso
//! (meso-06; `P1-21`/`P1-22`/`P1-23`/`P1-24`).
//!
//! This module hosts the three frozen create-family recipe methods on
//! [`OverlayInode`]: the create-object dispatcher
//! ([`OverlayInode::create_object`], `P1-23`), the upper-only create
//! ([`OverlayInode::create_upper_only`], `P1-21`/`P1-24`), and the
//! create-over-whiteout replacement ([`OverlayInode::create_over_whiteout`],
//! `P1-22`/`P1-24`, including the opaque-directory branch). The thin
//! `Inode`-trait entries (`create`/`mknod`/`write_link`) and the `DIR`
//! transaction helpers live in the sibling `dir/mod.rs` (frozen module
//! layout, spec §4); the recipes compose the frozen owner seams inline —
//! `project_new_upper` + `BindingCache::insert` + `readdir_index_insert`
//! (revision 01, override 2: no meso-06 `publish_*` helper exists).
//!
//! Lock contract (spec §3/§7.1): the caller (the `dir/mod.rs` entry) holds
//! the parent `DIR` transaction lock and has pinned the mount. This module
//! acquires no Overlay lock of its own beyond the brief `INODE` facts
//! snapshots inside `facts_snapshot`/`select_real_inode` (snapshot-and-
//! release, never held across an underlying call) and the meso-04 `CUL`
//! domain entered inside the real stage of `check_permission(
//! AccessType::Mutating, ...)` (stage B promotes the parent under the
//! caller-held `DIR`, meso-04 §3.2 item 7). Upper/workdir physical
//! operations run in the sleep-capable `DIR` domain under the underlying
//! filesystem's own locking; no `WL`/spin domain is entered and no `WL`
//! payload is touched (the whiteout cache is the sibling `dir/whiteout.rs`
//! owner, `P1-36`).
//!
//! Visibility: `create_object` is declared `pub(super)` — read through the
//! dispatch override and the Wave-3 precedent ("the overlayfs ceiling
//! `pub(in crate::fs::fs_impls::overlayfs)` where the spec says `pub(super)`
//! and cross-module reachability requires it") — because the parallel
//! `dir/mod.rs` pass hosts the `Inode`-trait entries that delegate into this
//! file; the two recipe methods stay private to this module exactly as the
//! spec's unqualified `fn` freezes them (their only caller is
//! `create_object` in this file).

use super::mknod_object_type;
use crate::{
    fs::{
        file::{InodeMode, InodeType, Permission},
        fs_impls::overlayfs::{
            AccessType,
            copyup::WorkdirTempRequest,
            metadata_security::xattr::{OPAQUE_MARKER_VALUE, OPAQUE_XATTR_FULL_NAME},
            projection::{
                Binding, BindingKey, NegativeBinding, OverlayInode, OverlayObjectFacts,
                PositiveBinding, PositiveKind, RealObject,
            },
        },
        vfs::{
            inode::{MknodType, RenameMode},
            xattr::{XattrName, XattrSetFlags},
        },
    },
    prelude::*,
};

impl OverlayInode {
    /// Dispatches one create-family request (create/mkdir/mknod/symlink)
    /// from the fresh `(parent, name)` projection under the parent `DIR`
    /// (`P1-23`).
    ///
    /// The decision uses current BindingCache/barrier evidence via
    /// `lookup_binding` — never the stale VFS negative dentry that may have
    /// triggered the call (spec §6, packet override 4):
    ///
    /// - `Negative(Absent)` / `Negative(HiddenByOpaque(_))` → upper-only
    ///   create (`create_upper_only`), no workdir, no opaque marker;
    /// - `Negative(HiddenByWhiteout(_))` → create-over-whiteout
    ///   (`create_over_whiteout`), workdir temp + atomic replace (+ the
    ///   opaque branch when the requested kind is `Dir`);
    /// - `Positive(_)` → `Err(EEXIST)` — a visible lower/merged target is
    ///   never silently replaced (BC-6 §59, Case 7).
    ///
    /// # Frozen-signature resolution (recorded)
    ///
    /// The frozen spec §4 helper signature names `InodeKindRequest` as the
    /// kind carrier with the note "kind = {File, Dir, SymLink, Socket} + mode
    /// (+ mknod type) — carried as the Inode-trait arguments, not a new
    /// enum". `InodeKindRequest` is not a real type anywhere in the tree and
    /// this Meso declares no new enum, so the pass carries the `Inode`-trait
    /// create arguments directly: `type_` + `mode` (the `create` entry
    /// shape) plus `mknod_type: Option<MknodType>` (the `mknod` entry shape;
    /// `Some` selects the mknod recipe at the upper call). The `dir/mod.rs`
    /// `mknod` entry applies the frozen Case-11 gate (`CharDevice(0)` →
    /// `EPERM`) before delegating (spec §2, Case 11).
    pub(super) fn create_object(
        &self,
        name: &str,
        type_: InodeType,
        mode: InodeMode,
        mknod_type: Option<MknodType>,
    ) -> Result<Arc<OverlayInode>> {
        let fs = self.fs_arc()?;
        let parent_facts = self.facts_snapshot();
        let binding = fs.lookup_binding(&parent_facts, name)?;
        match binding {
            Binding::Negative(NegativeBinding::Absent)
            | Binding::Negative(NegativeBinding::HiddenByOpaque(_)) => {
                self.create_upper_only(name, type_, mode, mknod_type)
            }
            Binding::Negative(NegativeBinding::HiddenByWhiteout(_)) => {
                self.create_over_whiteout(name, type_, mode, mknod_type)
            }
            Binding::Positive(_) => Err(Error::with_message(
                Errno::EEXIST,
                "the overlay target already exists and is visible",
            )),
        }
    }

    /// Creates a genuinely absent object directly in the upper parent
    /// (`P1-21`/`P1-24`; spec §6 Upper-only branch, §7.1 step 3).
    ///
    /// Runs the meso-05 real admission stage (`check_permission(
    /// AccessType::Mutating, MAY_WRITE)`) — which promotes this parent to
    /// upper authority under the caller-held `DIR` — then performs the upper
    /// `create`/`mknod` directly (no workdir) and publishes the result
    /// inline. A plain-absent or opaque-hidden target never creates opaque
    /// (BC-6 §59). The publication seams are infallible, so no Case-13 arm
    /// is structurally reachable in this recipe; the post-physical failure
    /// reconcile lives in [`OverlayInode::create_over_whiteout`] (the one
    /// create-family recipe with a fallible step after the upper commit).
    fn create_upper_only(
        &self,
        name: &str,
        type_: InodeType,
        mode: InodeMode,
        mknod_type: Option<MknodType>,
    ) -> Result<Arc<OverlayInode>> {
        // Stage B of the meso-05 admission (spec §7.1 step 3): promotes the
        // parent under the caller-held DIR; stage A (the EROFS gate + local
        // DAC) is the entry's admission — revision 01 deletes the
        // self-declared read_only_gate (override 4), so no EROFS check is
        // duplicated here.
        self.check_permission(AccessType::Mutating, Permission::MAY_WRITE)?;
        let fs = self.fs_arc()?;
        let upper_parent = self.upper_parent()?;
        // The overlay-visible object kind of the request (used by the index
        // seam below). Shared mechanical mapping (wave-4 repair item 11 —
        // the `MknodType` -> `InodeType` classification is the single
        // `mknod_object_type` helper in `dir/mod.rs`, consumed by all three
        // sites); the `None` leg keeps the plain `create` object type.
        let object_type = mknod_type.as_ref().map(mknod_object_type).unwrap_or(type_);
        // Upper physical operation: direct create/mknod in the upper parent.
        let new_upper = match mknod_type {
            Some(mknod) => upper_parent.mknod(name, mode, mknod)?,
            None => upper_parent.create(name, type_, mode)?,
        };
        // Semantic publication — inline seam composition (revision 01,
        // override 2; spec §5.1/§5.2, Case 1): the new upper object's facts,
        // the projected OverlayInode, the positive binding, and the index.
        let upper_layer = fs.layer_stack().upper.as_ref().ok_or_else(|| {
            Error::with_message(Errno::EROFS, "the overlay mount has no upper layer")
        })?;
        let new_facts = OverlayObjectFacts::try_new(
            PositiveKind::Single,
            Some(RealObject::new(
                0,
                new_upper,
                upper_layer.fsid,
                upper_layer.container_dev_id,
            )),
            Vec::new(),
        )
        .ok_or_else(|| {
            Error::with_message(
                Errno::EIO,
                "the new upper object facts are not constructible",
            )
        })?;
        let inode = fs.project_new_upper(&new_facts);
        fs.bindings().insert(
            BindingKey::new(self.key(), String::from(name)),
            Arc::new(Binding::Positive(PositiveBinding::new(inode.clone()))),
        );
        self.readdir_index_insert(name, inode.clone(), object_type);
        Ok(inode)
    }

    /// Replaces a whiteout-hidden name with a completely prepared private
    /// workdir temp, then publishes it (`P1-22`/`P1-24`; spec §6
    /// Over-whiteout branch, §7.1 step 4).
    ///
    /// The replacement object is prepared in the workdir (never visible as a
    /// lookup/readdir source), the opaque marker is applied to a `Dir` temp
    /// **before** the atomic swap (the opaque record is part of the
    /// replacement object's complete publication — BC-6 §59, spec §6 opaque
    /// branch), and the whiteout is consumed atomically: `Replace` for
    /// non-directories, `Exchange` + workdir unlink of the displaced whiteout
    /// for directories. A `SymLink` temp's target is filled later by the
    /// VFS-wide `write_link` two-step (spec §11 item 4). Publication is the
    /// same inline seam sequence as [`OverlayInode::create_upper_only`].
    ///
    /// Failure handling (spec §7.1 step 5): any failure before the atomic
    /// upper commit best-effort-cleans the temp; a failure after the commit
    /// (the only fallible step there is the directory `Exchange`-leg unlink
    /// of the displaced whiteout) reconciles the affected `(parent, name)` =
    /// `(self, name)` projection as a unit (Case 13, §5.3).
    ///
    /// # Recorded resolution (reconcile arm, wave-4 repair item 10)
    ///
    /// The shared reconcile entry `invalidate_stale_cache` now accepts
    /// `&OverlayInode` parents (not `&Arc<OverlayInode>`), so this
    /// one-parent `&self` recipe calls the entry directly with its own
    /// `(self, name)` pair — the former inline two-seam composition
    /// (`BindingCache::invalidate` + `invalidate_readdir_index`) is deleted
    /// and the entry is the single Case-13 reconcile shape consumed by every
    /// recipe. This arm covers only the one affected pair of this recipe and
    /// never a partial sequence.
    ///
    /// The shared workdir-temp request carries a borrowed [`MknodType`] for
    /// the special-object leg. Its retry owner recreates the VFS value for
    /// each attempt, so device identity survives an `EEXIST` retry without a
    /// caller-local staging operation.
    fn create_over_whiteout(
        &self,
        name: &str,
        type_: InodeType,
        mode: InodeMode,
        mknod_type: Option<MknodType>,
    ) -> Result<Arc<OverlayInode>> {
        // Stage B of the meso-05 admission (promotes the parent under the
        // caller-held DIR; stage A is the entry's admission — revision 01,
        // no self-declared read_only_gate, override 4).
        self.check_permission(AccessType::Mutating, Permission::MAY_WRITE)?;
        let fs = self.fs_arc()?;
        let upper_parent = self.upper_parent()?;
        // Shared mechanical kind mapping (wave-4 repair item 11; consumed by
        // the opaque branch and the index seam). Computed before
        // `mknod_type` is consumed by the temp creation below.
        let object_type = mknod_type.as_ref().map(mknod_object_type).unwrap_or(type_);
        // Private staging: the temp is never a lookup/readdir/ReaddirIndex
        // source (BC-6 §57). The typed request selects either the `mknod` or
        // create operation while the shared owner performs every retry.
        let temp = match &mknod_type {
            Some(node) => fs.create_workdir_temp(
                name,
                &upper_parent,
                WorkdirTempRequest::Mknod { mode, node },
            )?,
            None => fs.create_workdir_temp(
                name,
                &upper_parent,
                WorkdirTempRequest::Create { kind: type_, mode },
            )?,
        };
        let (temp_name, temp) = temp.into_parts();
        let workdir = self.workdir_root()?;
        // The shared recipe scaffold (wave-4 round-2 repair item 2): the
        // commit marker is flipped at the physical upper commit point and the
        // Case-13 reconcile / pre-publication cleanup classification is owned
        // by `run_recipe` (spec §7.1 step 5).
        self.run_recipe(
            &fs,
            Some(&temp_name),
            || self.invalidate_stale_cache(&[(self, name)]),
            |marker| {
                if object_type == InodeType::Dir {
                    // Opaque branch (P1-22/P1-23): the opaque record is part of
                    // the replacement directory's complete publication; the
                    // marker write is gated by the meso-01 private-xattr
                    // capability (spec §5.1) and runs on the temp before the
                    // atomic swap — the whiteout is never deleted first (BC-6
                    // §59).
                    let can_store_private_xattr = fs
                        .policy()
                        .upper_capabilities()
                        .is_some_and(|caps| caps.can_store_private_xattr());
                    if !can_store_private_xattr {
                        return Err(Error::with_message(
                            Errno::EOPNOTSUPP,
                            "the upper filesystem cannot store the opaque marker \
                             required for a directory over a whiteout",
                        ));
                    }
                    let marker_name = XattrName::try_from_full_name(OPAQUE_XATTR_FULL_NAME)
                        .ok_or_else(|| {
                            Error::with_message(
                                Errno::EINVAL,
                                "invalid overlay opaque marker xattr name",
                            )
                        })?;
                    let mut marker_reader = VmReader::from(OPAQUE_MARKER_VALUE).to_fallible();
                    temp.set_xattr(
                        marker_name,
                        &mut marker_reader,
                        XattrSetFlags::CREATE_OR_REPLACE,
                    )?;
                }
                // Atomic replacement over the whiteout: `Replace` for non-dirs;
                // for dirs `Exchange` (the displaced whiteout lands in the
                // workdir) then the workdir unlink removes it.
                if object_type.is_directory() {
                    workdir.rename(&temp_name, &upper_parent, name, RenameMode::Exchange)?;
                    marker.commit();
                    workdir.unlink(&temp_name)?;
                } else {
                    workdir.rename(&temp_name, &upper_parent, name, RenameMode::Replace)?;
                    marker.commit();
                }
                // Semantic publication — inline seam composition (revision 01,
                // override 2; Case 1). The temp handle is the object now
                // published at `(upper_parent, name)` (inode identity is stable
                // across the rename), so it is the new upper real object.
                let upper_layer = fs.layer_stack().upper.as_ref().ok_or_else(|| {
                    Error::with_message(Errno::EROFS, "the overlay mount has no upper layer")
                })?;
                let new_facts = OverlayObjectFacts::try_new(
                    PositiveKind::Single,
                    Some(RealObject::new(
                        0,
                        temp,
                        upper_layer.fsid,
                        upper_layer.container_dev_id,
                    )),
                    Vec::new(),
                )
                .ok_or_else(|| {
                    Error::with_message(
                        Errno::EIO,
                        "the new upper object facts are not constructible",
                    )
                })?;
                let inode = fs.project_new_upper(&new_facts);
                fs.bindings().insert(
                    BindingKey::new(self.key(), String::from(name)),
                    Arc::new(Binding::Positive(PositiveBinding::new(inode.clone()))),
                );
                self.readdir_index_insert(name, inode.clone(), object_type);
                Ok(inode)
            },
        )
    }
}
