// SPDX-License-Identifier: MPL-2.0

//! The module root of the `metadata_security_xattr_policy` meso (meso-05).
//!
//! This module declares the three `metadata_security/*` submodules and hosts
//! the thin cross-file seams of the frozen meso-05 spec §4: the single
//! private delegation helper `OverlayInode::delegate_to_real` (shared by the
//! three sibling files) and the cross-meso `OverlayFs::xattr_policy` accessor
//! (meso-02 §3.4 owner-extension rule; the field is the Wave-3 seam in
//! `mount/superblock.rs`, the payload type lands in the sibling `xattr.rs`).
//! The real control flow lives in the sibling files created in parallel from
//! the same frozen spec: `permission.rs` (P1-18 two-stage pipeline),
//! `metadata.rs` (P1-16/P1-17 metadata setters), and `xattr.rs` (P1-33
//! `OverlayXattrPolicy`/`XattrClass` + the xattr entries).
//!
//! Visibility: `xattr` is declared `pub(super)` — read through the spec's
//! overlayfs-ceiling audit as `pub(in crate::fs::fs_impls::overlayfs)` —
//! because the frozen Wave-3 `OverlayFs::xattr_policy` field initialization
//! in `mount/build.rs` names `metadata_security::xattr::OverlayXattrPolicy`
//! from a sibling module; the other two submodules stay private to
//! `metadata_security` (spec §1 "Must Remain Internal": no cross-module
//! consumer names them). The delegation helper is private to the module tree
//! (defined here so `permission.rs`/`metadata.rs`/`xattr.rs` share it); the
//! accessor is published at the same ceiling because the meso-04 copy-time
//! xattr filter (§7 supersession) and this Meso's xattr entries consume it.
//!
//! Lock contract (spec §3): this module acquires no Overlay lock domain.
//! `delegate_to_real` re-resolves the current authority per call through
//! `select_real_inode()` (a brief `INODE` facts snapshot, released before the
//! underlying call) and runs the delegation under the mount's
//! creator-credential scope (`with_creator_credentials_fn`, meso-01 P1-19);
//! no Overlay lock is held across any underlying permission/MAC/xattr
//! callback (BC-5 §52).

use self::xattr::OverlayXattrPolicy;
use super::{mount::OverlayFs, projection::OverlayInode};
use crate::{fs::vfs::inode::Inode, prelude::*};

mod metadata;
mod permission;
pub(super) mod xattr;

impl OverlayInode {
    /// The single private delegation helper of this Meso (spec §4; at most
    /// one, per packet).
    ///
    /// Resolves the current real authority once — a fresh per-call
    /// `select_real_inode()` (BC-5 §49.2), so an fd opened while lower-backed
    /// observes the upper real inode on its next operation after a copy-up —
    /// and runs `operation_fn` under the mount's creator-credential scope
    /// (`with_creator_credentials_fn`, meso-01 P1-19). The returned
    /// `Arc<dyn Inode>` strong pin keeps the resolved real inode alive for
    /// the delegation; no Overlay lock is held across the underlying call
    /// (spec §3). The permission stage has already admitted the operation (or
    /// the entry is a pure read delegation), so the forward runs directly
    /// under the creator-credential scope; for metadata setters whose
    /// underlying real ops do not self-evaluate, `check_real_permission` ran
    /// the explicit real check before this forward (spec §4.0 consequence).
    /// Wave-4 repair item 7: the `#[expect(dead_code)]` marker is removed —
    /// the helper is live code, consumed by every entry in the sibling
    /// `metadata.rs`/`xattr.rs` files in this same tree.
    fn delegate_to_real<T>(
        &self,
        operation_fn: impl FnOnce(&Arc<dyn Inode>) -> Result<T>,
    ) -> Result<T> {
        let fs = self.fs_arc()?;
        let real = self.select_real_inode();
        fs.policy()
            .credential_policy()
            .with_creator_credentials_fn(|| operation_fn(&real))
    }
}

impl OverlayFs {
    /// Returns the immutable xattr classification policy (P1-33).
    ///
    /// The cross-meso owner-extension accessor (meso-02 §3.4 rule) for the
    /// Wave-3 `OverlayFs::xattr_policy` seam: the stateless
    /// [`OverlayXattrPolicy`] (owned once, no lock) is consumed by this
    /// Meso's xattr entries (the `list_xattr` private-name filter) and by
    /// meso-04's copy-time xattr filter (§7 supersession). Wave-4 repair
    /// item 7: the `#[expect(dead_code)]` marker is removed — the accessor
    /// is live code, consumed by the sibling `xattr.rs` entries and the
    /// meso-04 copy-time filter.
    pub(super) fn xattr_policy(&self) -> &OverlayXattrPolicy {
        &self.xattr_policy
    }
}
