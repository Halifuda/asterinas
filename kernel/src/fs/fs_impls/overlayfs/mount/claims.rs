// SPDX-License-Identifier: MPL-2.0

//! Upper/workdir exclusivity claims and the unified 64-bit overlay identity
//! (`P1-35` + `P2-11`).
//!
//! This module implements Scheme A — the inode `Extension` runtime lease — as
//! the frozen claim carrier (spec §3.0): each claimed root inode hosts a
//! VFS-owned `OverlayInuseSlot` (recorded VFS dependency, spec §3.0.5 item 2),
//! and the non-zero unified [`OverlayUuid`] value is both the claim token
//! (per-slot CAS) and, when effective, the overlay UUID persisted as
//! `trusted.overlay.uuid` on the upper root. The upper slot is claimed first
//! and released last; the workdir slot second and released first — enforced
//! structurally by the field declaration order of [`UpperWorkdirClaim`]
//! (`workdir` before `upper`; Rust drops struct fields in declaration order)
//! plus the guard `Drop` order (spec §2 release-order invariant). All claim
//! operations are single-word atomic CASes: non-blocking and safe in `Drop`
//! (Scheme A crash/teardown policy, spec §3.0.4).
//!
//! # Recorded deviation (wave-1 review item 4)
//!
//! The spec §4 listing declares `upper` before `workdir` and asserts that
//! order "guarantees workdir releases before upper". That sentence is
//! inverted: Rust drops struct fields in declaration order, so an `upper`-
//! first declaration releases the upper claim first. The review-corrected
//! declaration order (`workdir` first, `upper` last) is what actually
//! produces the frozen release order (workdir first, upper last) on `Drop`;
//! the frozen invariant itself is unchanged.

use super::{layers::resolve_root_path, options::UuidMode};
use crate::{
    fs::{
        utils::DirentCounter,
        vfs::{
            file_system::FileSystem,
            inode::Inode,
            inode_ext::InodeExt,
            path::Path,
            registry::FsCreationCtx,
            xattr::{XattrName, XattrSetFlags},
        },
    },
    prelude::*,
};

/// The size in bytes of the persisted `trusted.overlay.uuid` value.
///
/// `pub(super)` so the sibling `policy.rs` xattr-capability probe sizes its
/// probe buffer at the persisted value length (wave-1 review item 1: the
/// previous 1-byte probe buffer hit `ERANGE` on backends that fail a short
/// read, deterministically failing `UuidMode::Auto` re-mounts).
pub(super) const OVERLAY_UUID_SIZE: usize = 8;

/// The private xattr name carrying the effective overlay UUID (`P2-11`).
const TRUSTED_OVERLAY_UUID: &str = "trusted.overlay.uuid";

/// The unified 64-bit identity of one writable overlay mount (`P2-11`/`P1-35`).
///
/// The value is never zero. It serves as the claim token for both
/// [`InodeClaimGuard`]s (per-`OverlayInuseSlot` CAS) and, when effective, as
/// the overlay UUID persisted as `trusted.overlay.uuid` and published through
/// `MountPolicy::uuid()`/`SuperBlock::fsid` (spec §3.0 unified-identity
/// invariant).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(in crate::fs::fs_impls::overlayfs) struct OverlayUuid(u64);

impl OverlayUuid {
    /// Creates an [`OverlayUuid`], rejecting the zero value with `EINVAL`.
    pub(super) fn try_new(value: u64) -> Result<Self> {
        if value == 0 {
            return_errno_with_message!(Errno::EINVAL, "the overlay uuid must be non-zero");
        }
        Ok(Self(value))
    }

    /// Returns the raw 64-bit value.
    pub(super) fn value(&self) -> u64 {
        self.0
    }

    /// Generates a fresh non-zero identity from the kernel CSPRNG.
    ///
    /// Generation runs pre-claim and lock-free (spec §3.0.5 item 7); the zero
    /// value has probability `2^-64` and is rejected by [`OverlayUuid::try_new`],
    /// so the loop regenerates (bounded in practice, spec §3.3 Hazard 6).
    ///
    /// `pub(super)` since the sibling `build.rs` also generates the claim
    /// token directly for effective read-only overlays, where nothing is ever
    /// persisted and a fresh in-memory token suffices (wave-1 review round 2,
    /// item 2).
    pub(super) fn generate() -> Self {
        loop {
            let mut bytes = [0u8; OVERLAY_UUID_SIZE];
            crate::util::random::getrandom(&mut bytes);
            let value = u64::from_le_bytes(bytes);
            if let Ok(uuid) = Self::try_new(value) {
                return uuid;
            }
        }
    }

    /// Reads an existing persisted identity from the upper root (`On`/`Auto`).
    ///
    /// Returns `Ok(None)` when no `trusted.overlay.uuid` xattr exists
    /// (`ENODATA`); a malformed value fails closed with `EINVAL`.
    fn read_from_upper(upper_inode: &Arc<dyn Inode>) -> Result<Option<Self>> {
        let name = XattrName::try_from_full_name(TRUSTED_OVERLAY_UUID)
            .ok_or_else(|| Error::with_message(Errno::EINVAL, "invalid overlay uuid xattr name"))?;
        let mut value = [0u8; OVERLAY_UUID_SIZE];
        let mut writer = VmWriter::from(value.as_mut_slice()).to_fallible();
        match upper_inode.get_xattr(name, &mut writer) {
            Ok(written) if written == OVERLAY_UUID_SIZE => {
                Ok(Some(Self::try_new(u64::from_le_bytes(value))?))
            }
            Ok(_) => return_errno_with_message!(
                Errno::EINVAL,
                "the persisted overlay uuid has a malformed value"
            ),
            Err(err) if err.error() == Errno::ENODATA => Ok(None),
            Err(err) => Err(err),
        }
    }

    /// Persists the identity as `trusted.overlay.uuid` on the upper root.
    ///
    /// Uses `XattrSetFlags::CREATE_OR_REPLACE` (P2-11, BC-1 step 9) and is
    /// only called when the identity is effective.
    fn persist_on_upper(&self, upper_inode: &Arc<dyn Inode>) -> Result<()> {
        let name = XattrName::try_from_full_name(TRUSTED_OVERLAY_UUID)
            .ok_or_else(|| Error::with_message(Errno::EINVAL, "invalid overlay uuid xattr name"))?;
        let value = self.value().to_le_bytes();
        let mut reader = VmReader::from(value.as_slice()).to_fallible();
        upper_inode.set_xattr(name, &mut reader, XattrSetFlags::CREATE_OR_REPLACE)
    }
}

/// A runtime lease on one root inode's `OverlayInuseSlot` (`P1-35`).
///
/// The guard pins the claimed inode so the slot cannot be evicted while the
/// claim is held (identity contract, spec §3.0.5 item 3) and holds the unified
/// non-zero token. `Drop` re-resolves the slot from the pinned inode and CASes
/// the token free — atomic, non-blocking, safe in `Drop` (spec §3.0.3 claim
/// guard contract).
#[derive(Debug)]
pub(super) struct InodeClaimGuard {
    /// Pins the claimed inode (keeps the `OverlayInuseSlot` alive).
    inode: Arc<dyn Inode>,
    /// The unified 64-bit claim token / overlay UUID (non-zero invariant).
    token: OverlayUuid,
}

impl InodeClaimGuard {
    /// Claims the inode's `OverlayInuseSlot` with `identity` as the token.
    ///
    /// Returns `EBUSY` when the slot is already claimed by another holder
    /// (spec §3.0.3).
    pub(super) fn try_claim(inode: Arc<dyn Inode>, identity: OverlayUuid) -> Result<Self> {
        inode.overlay_inuse_slot().try_claim(identity.value())?;
        Ok(Self {
            inode,
            token: identity,
        })
    }

    /// Returns the unified token this guard holds.
    #[expect(
        dead_code,
        reason = "frozen accessor (spec §4); consumed by sibling mesos (meso-02+) for claim auditing"
    )]
    pub(super) fn token(&self) -> OverlayUuid {
        self.token
    }
}

impl Drop for InodeClaimGuard {
    fn drop(&mut self) {
        // Re-resolve the slot from the pinned inode and CAS the token free.
        // The release is non-blocking and fail-safe: a stale/wrong token is a
        // no-op (spec §3.0.3).
        self.inode
            .overlay_inuse_slot()
            .release(self.token.value());
    }
}

/// The claimed upper/workdir pair of a writable overlay mount (`P1-35`/`P2-11`).
///
/// The upper slot is claimed first and released last; the workdir slot second
/// and released first. The field declaration order (`workdir` before `upper`)
/// plus Rust's declaration-order field drops and the guard `Drop` order
/// enforces the release ordering structurally (spec §2 release-order
/// invariant; wave-1 review item 4). `identity` is the unified non-zero value
/// used as the token for both slots and, when effective, persisted as
/// `trusted.overlay.uuid`.
#[derive(Debug)]
pub(in crate::fs::fs_impls::overlayfs) struct UpperWorkdirClaim {
    /// Workdir claim; taken second, released first.
    workdir: InodeClaimGuard,
    /// Upper claim; taken first, released last.
    upper: InodeClaimGuard,
    /// Upper filesystem identity (same-filesystem evidence).
    upper_fs: Arc<dyn FileSystem>,
    /// Unified identity; persisted iff effective (`P2-11`).
    identity: OverlayUuid,
}

impl UpperWorkdirClaim {
    /// Validates the upper/workdir pair structurally (`P0-03`).
    ///
    /// Checks that both roots are directories, that they live on the same
    /// underlying filesystem (`st_dev` evidence), and that the workdir is
    /// neither identical to nor an ancestor/descendant of the upperdir.
    /// Failures map to `ENOTDIR` / `EINVAL` per spec §2 Case 4 (Linux
    /// `ovl_fill_super`).
    pub(super) fn validate_pair(upper: &Path, workdir: &Path) -> Result<()> {
        if !upper.type_().is_directory() {
            return_errno_with_message!(Errno::ENOTDIR, "upperdir is not a directory");
        }
        if !workdir.type_().is_directory() {
            return_errno_with_message!(Errno::ENOTDIR, "workdir is not a directory");
        }
        if upper.metadata().container_dev_id != workdir.metadata().container_dev_id {
            return_errno_with_message!(
                Errno::EINVAL,
                "workdir and upperdir must be on the same underlying filesystem"
            );
        }
        if Arc::ptr_eq(upper.dentry(), workdir.dentry()) {
            return_errno_with_message!(Errno::EINVAL, "workdir must be distinct from upperdir");
        }
        // Alias rejection (wave-1 review `validate-at-boundaries` fix, item
        // 11): two spellings of the same physical directory can produce
        // different `path_name()` strings and different dentry objects
        // (`upperdir=/real/u` with `workdir=/alias/u` where `/alias` is a
        // symlink to `/real`, or a bind-mount alias). The resolved inode
        // objects are the same for the same physical directory, so compare
        // them as well — an aliased same-directory pair must not pass.
        if Arc::ptr_eq(upper.inode(), workdir.inode()) {
            return_errno_with_message!(Errno::EINVAL, "workdir must be distinct from upperdir");
        }

        // Workdir and upperdir must not be each other's ancestor/descendant.
        // The dentry walking APIs are private to `vfs::path`, so the only
        // `pub(in crate::fs)` dentry surface usable from this module is the
        // absolute `path_name()`. Both paths were resolved with
        // `lookup_no_follow` (which follows intermediate symlink components),
        // so these names reflect the resolved hierarchy: symlink aliases of
        // the same tree canonicalize to the same name, and exact aliases are
        // additionally rejected by the inode-identity check above. The
        // remaining exotic case (ancestor/descendant directories reached only
        // through distinct dentry/inode objects) is a recorded limitation —
        // this wave has no VFS canonicalize API.
        let is_same_or_descendant_fn = |candidate: &str, ancestor: &str| -> bool {
            if ancestor == "/" {
                // The filesystem root is an ancestor of every dentry.
                return true;
            }
            candidate == ancestor
                || candidate
                    .strip_prefix(ancestor)
                    .is_some_and(|rest| rest.starts_with('/'))
        };
        let upper_name = upper.dentry().path_name();
        let workdir_name = workdir.dentry().path_name();
        if is_same_or_descendant_fn(&workdir_name, &upper_name)
            || is_same_or_descendant_fn(&upper_name, &workdir_name)
        {
            return_errno_with_message!(
                Errno::EINVAL,
                "workdir must not be an ancestor or descendant of upperdir"
            );
        }
        Ok(())
    }

    /// Determines the unified identity before the claim step (`P2-11`).
    ///
    /// `On`/`Auto` reuse an existing persisted `trusted.overlay.uuid` when
    /// present; otherwise a fresh non-zero value is generated. `Off`/`Null`
    /// never read and always generate a fresh in-memory-only token (spec §2
    /// Case 10b). The value is determined pre-claim because the token must be
    /// known at claim time (spec §3.0.4 unified-identity ordering).
    pub(super) fn determine_identity(
        upper_inode: &Arc<dyn Inode>,
        uuid_mode: UuidMode,
    ) -> Result<OverlayUuid> {
        match uuid_mode {
            // `On` fails closed: a backend that cannot serve the xattr read
            // also cannot satisfy persistence, so the read error propagates
            // (spec §2 Case 10c).
            UuidMode::On => match OverlayUuid::read_from_upper(upper_inode)? {
                Some(existing) => Ok(existing),
                None => Ok(OverlayUuid::generate()),
            },
            // `Auto` degrades on read unavailability (spec §2 Case 10b/10d):
            // the backend will also fail the post-claim capability probe, so
            // the generated value stays in-memory-only (not effective).
            UuidMode::Auto => match OverlayUuid::read_from_upper(upper_inode) {
                Ok(Some(existing)) => Ok(existing),
                Ok(None) | Err(_) => Ok(OverlayUuid::generate()),
            },
            UuidMode::Off | UuidMode::Null => Ok(OverlayUuid::generate()),
        }
    }

    /// Claims the upper slot first, then the workdir slot (`P1-35`).
    ///
    /// On a workdir conflict the already-taken upper claim is dropped
    /// immediately (rollback of the first claim) and the `EBUSY` propagates —
    /// no partial exclusivity escapes construction (spec §3.0.4 ordering).
    pub(super) fn claim(
        upper_inode: Arc<dyn Inode>,
        workdir_inode: Arc<dyn Inode>,
        upper_fs: Arc<dyn FileSystem>,
        identity: OverlayUuid,
    ) -> Result<Self> {
        let upper = InodeClaimGuard::try_claim(upper_inode, identity)?;
        let workdir = match InodeClaimGuard::try_claim(workdir_inode, identity) {
            Ok(workdir) => workdir,
            Err(err) => {
                // Roll back the first claim before propagating the conflict.
                drop(upper);
                return Err(err);
            }
        };
        Ok(Self {
            workdir,
            upper,
            upper_fs,
            identity,
        })
    }

    /// Prepares the workdir for use (`P0-03`): it must be empty.
    ///
    /// Returns `ENOTEMPTY` when the workdir contains entries (Linux
    /// `ovl_check_empty_dir`); skipped entirely for read-only mounts (the
    /// caller only invokes it for genuinely writable overlays, wave-1 review
    /// item 2).
    pub(super) fn prepare_workdir(&self) -> Result<()> {
        if !self.is_workdir_empty()? {
            return_errno_with_message!(Errno::ENOTEMPTY, "the workdir is not empty");
        }
        Ok(())
    }

    /// Scans the workdir and reports whether it is empty.
    ///
    /// The scan uses a [`DirentCounter`], which excludes `.` and `..`.
    fn is_workdir_empty(&self) -> Result<bool> {
        let mut counter = DirentCounter::new();
        self.workdir.inode.readdir_at(0, &mut counter)?;
        Ok(counter.count() == 0)
    }

    /// Persists the unified identity as `trusted.overlay.uuid` (`P2-11`).
    ///
    /// Called only when the identity is effective (post-claim, construction
    /// step 9). The caller (`build.rs`) maps an `On` persist failure to
    /// `EOPNOTSUPP` fail-closed and an `Auto` persist failure to degrade to
    /// not-effective (spec §3.2 step 9).
    pub(super) fn persist_identity(&self) -> Result<()> {
        self.identity.persist_on_upper(&self.upper.inode)
    }

    /// Reports whether both slots are still owned by this claim's token.
    ///
    /// Returns `false` if either slot is claimed by a different token (spec
    /// §2 exclusivity invariant).
    #[expect(
        dead_code,
        reason = "frozen UpperWorkdirClaim accessor (spec §4); consumed by sibling mesos once they land"
    )]
    pub(in crate::fs::fs_impls::overlayfs) fn has_exclusive_claim(&self) -> bool {
        let upper_owned = self
            .upper
            .inode
            .overlay_inuse_slot()
            .is_claimed_by(self.identity.value());
        let workdir_owned = self
            .workdir
            .inode
            .overlay_inuse_slot()
            .is_claimed_by(self.identity.value());
        upper_owned && workdir_owned
    }

    /// Returns the pinned upper root inode.
    #[expect(
        dead_code,
        reason = "frozen UpperWorkdirClaim accessor (spec §4); consumed by sibling mesos once they land"
    )]
    pub(in crate::fs::fs_impls::overlayfs) fn upper_inode(&self) -> &Arc<dyn Inode> {
        &self.upper.inode
    }

    /// Returns the pinned workdir root inode.
    pub(in crate::fs::fs_impls::overlayfs) fn workdir_inode(&self) -> &Arc<dyn Inode> {
        &self.workdir.inode
    }

    /// Returns the upper filesystem identity.
    pub(in crate::fs::fs_impls::overlayfs) fn upper_fs(&self) -> &Arc<dyn FileSystem> {
        &self.upper_fs
    }

    /// Returns the unified identity of this claim.
    #[expect(
        dead_code,
        reason = "frozen UpperWorkdirClaim accessor (spec §4); consumed by sibling mesos once they land"
    )]
    pub(in crate::fs::fs_impls::overlayfs) fn identity(&self) -> OverlayUuid {
        self.identity
    }
}

/// Probes that a root path resolves to a backend-instance-stable inode
/// (`P1-35` pre-claim evidence; heuristic, spec §3.0.2).
///
/// Resolving the same root path twice must yield `Arc::ptr_eq`-equal inodes,
/// proving the backend's inode cache is instance-stable for pinned roots. In
/// addition, both resolutions must be the same instance as `pinned_inode` —
/// the layer-pinned object that step 6 actually claims — so the checked
/// object and the used object are the same (wave-1 review item 6, TOCTOU
/// check/use alignment). This is a heuristic; the durable guarantee is the
/// backend identity contract (spec §3.0.5 item 3). A failing backend fails
/// closed with `EOPNOTSUPP` (spec §2 Case 3).
///
/// # Visibility note (recorded deviation)
///
/// Declared `pub(super)` (visible within the `mount` module tree only) instead
/// of strictly module-private because the frozen construction step 3 (spec
/// §3.2) invokes the probe for both roots from `OverlayFs::new` in the sibling
/// `build.rs`; a module-private item would be unreachable from there. The probe
/// is not re-exported by `mount/mod.rs`, so it stays internal to the mount
/// meso, preserving the spec §4 visibility-audit intent.
pub(super) fn verify_inode_instance_stability(
    fs_creation_ctx: &FsCreationCtx,
    raw_path: &str,
    pinned_inode: &Arc<dyn Inode>,
) -> Result<()> {
    // Both resolutions go through the shared `resolve_root_path` helper
    // (wave-1 review `dry` fix, item 8); each resolution is compared both to
    // the other and to the layer-pinned inode that is claimed downstream.
    let first = resolve_root_path(fs_creation_ctx, raw_path)?.inode().clone();
    let second = resolve_root_path(fs_creation_ctx, raw_path)?.inode().clone();
    if !Arc::ptr_eq(&first, &second) || !Arc::ptr_eq(&first, pinned_inode) {
        return_errno_with_message!(
            Errno::EOPNOTSUPP,
            "the underlying filesystem does not provide instance-stable inodes for pinned roots"
        );
    }
    Ok(())
}
