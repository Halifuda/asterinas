// SPDX-License-Identifier: MPL-2.0

//! The overlay filesystem Macro-Owner carrier and its VFS-facing superblock
//! surface (`P0-05`).
//!
//! This module owns the `OverlayFs` struct (the published mount/layer/policy
//! state plus the meso-02 projection state — `bindings`/`inodes`/`identity`,
//! added under the cross-meso owner-extension rule, meso-02 spec §3.4/§3.5),
//! the `FileSystem` trait implementation, and the `MOUNT` level-1
//! lifecycle domain (`MountLifecycle`/`MountPhase`). All fallible mount work
//! happens in `build.rs` (`OverlayFs::new`); the hooks here enter through a
//! pinned `Arc<OverlayFs>` and hold no Overlay lock except the short `MOUNT`
//! transition inside [`OverlayFs::begin_shutdown`].
//!
//! The wave-2 review item 2 widened the cross-boundary carriers to the
//! documented overlayfs ceiling (`pub(in crate::fs::fs_impls::overlayfs)`):
//! `OverlayFs` itself, plus the canonical `self_weak` reference added by the
//! wave-2 repair item 1 (the root-carrier materialization). The
//! `bindings`/`inodes`/`identity` fields already used that ceiling. Round-2
//! review item 1 widened the consumed accessor methods (`layer_stack()`,
//! `policy()`, `claims()`) to the same ceiling so the `projection` tree can
//! call them (E0624).

use core::sync::atomic::AtomicU64;

use super::{
    OVERLAY_FS_NAME, claims::UpperWorkdirClaim, layers::OverlayLayerStack, policy::MountPolicy,
};
use crate::{
    fs::{
        fs_impls::overlayfs::{
            dir::whiteout::WhiteoutCache,
            metadata_security::xattr::OverlayXattrPolicy,
            projection::{BindingCache, IdentityPolicy, InodeCache},
        },
        pseudofs::AnonDeviceId,
        vfs::{
            file_system::{FileSystem, FsEventSubscriberStats, FsFlags, SuperBlock},
            inode::Inode,
        },
    },
    prelude::*,
};

/// The Macro-Owner carrier of the overlay filesystem (mirrors Linux `ovl_fs`).
///
/// `OverlayFs` is the only object that publishes mount/layer/policy state to
/// sibling Mesos. It is created by [`OverlayFs::new`] (in `build.rs`) through
/// the frozen 11-step construction sequence; after publication the layer
/// stack, claims, and policy snapshot are immutable. The meso-02 projection
/// state — `bindings` ([`BindingCache`]), `inodes` ([`InodeCache`]), and
/// `identity` ([`IdentityPolicy`]) — is initialized in the same constructor
/// under the cross-meso owner-extension rule (meso-02 spec §3.4/§3.5) and
/// consumed by the `projection` module.
///
/// Invariants: `root_inode()` returns the prepared root carrier and performs
/// no fallible work (`P0-05` infallible-root invariant); `claims` is `Some`
/// only for writable mounts and is released exactly once on the final `Drop`
/// (guard `Drop`, atomic non-blocking, no mutex); the `MOUNT` lifecycle is
/// used only for lifecycle transitions and is never held across underlying
/// callbacks. The meso-02 `bindings`/`inodes` caches use sleep-capable
/// `RwMutex` internal data locks (not topology levels) and the `identity`
/// policy is immutable after construction.
///
/// The wave-3 seams add the meso-04/05/06 cross-meso carriers —
/// `workdir_temp_serial` (meso-04 P1-34), `xattr_policy` (meso-05 P1-33),
/// `whiteout_cache` (meso-06 P1-36) — as forward references by frozen name;
/// their construction and consumers land with the Wave-4 leaf Creators
/// (`copyup`, `metadata_security`, `dir`) under the same cross-meso
/// owner-extension rule (meso-04 spec §4.1 / meso-05 spec §4 / meso-06 spec §4).
pub(in crate::fs::fs_impls::overlayfs) struct OverlayFs {
    pub(super) layer_stack: OverlayLayerStack,
    /// The claimed upper/workdir pair; `Some` only for writable mounts.
    ///
    /// Established single-threaded before publication and released by the
    /// final `Drop` (guard `Drop`, no mutex).
    pub(super) claims: Option<UpperWorkdirClaim>,
    pub(super) policy: MountPolicy,
    /// The reported mount source (`P0-05` show-options surface).
    pub(super) mount_source: String,
    /// The prepared root carrier (frozen cross-meso seam, spec §3.0.5 item 8;
    /// wave-2 repair item 1 reconciliation).
    ///
    /// The root inode needs the published mount (`fs.layer_stack()` /
    /// `fs.identity()`), but `Weak::upgrade()` is documented-`None` inside
    /// the `Arc::new_cyclic` closure (the strong count stays 0 until the
    /// closure returns). `OverlayFs::new` fills this construction/publication
    /// slot immediately after the `Arc` is published. `root_inode()` only
    /// clones the prepared root; a `None` value for a published mount is a
    /// hard construction invariant failure, never a silent mount-less root.
    pub(super) root_inode: Mutex<Option<Arc<dyn Inode>>>,
    /// The `MOUNT` level-1 lifecycle domain; phase only, sleep-capable.
    pub(super) lifecycle: Mutex<MountLifecycle>,
    pub(super) fs_event_stats: FsEventSubscriberStats,
    /// The canonical weak mount reference (wave-2 repair item 1).
    ///
    /// Established by `Arc::new_cyclic` in `OverlayFs::new` (ramfs
    /// `Arc::new_cyclic` + `Weak<RamFs>` precedent) and consumed by
    /// `projection::project_inode` to stamp created `OverlayInode`s with the
    /// mount's live `Weak` — replacing the root-carrier downcast seam
    /// (wave-2 review `coupling-cohesion` finding). The weak never pins the
    /// mount (B/C-2 lifetime rule).
    pub(in crate::fs::fs_impls::overlayfs) self_weak: Weak<OverlayFs>,
    /// The mount-wide binding cache — the first source for `(parent, name)`
    /// lookup results (`Binding-first` invariant; meso-02 spec §3.4/§4).
    ///
    /// Entries are immutable `Arc<Binding>` snapshots (a positive pins its
    /// inode, a negative pins its barrier); insert/update happen under the
    /// caller's parent `DIR` transaction lock. Not a second layer registry or
    /// identity table.
    pub(in crate::fs::fs_impls::overlayfs) bindings: BindingCache,
    /// The mount-wide inode identity-reuse cache (`P0-16`).
    ///
    /// Maps each `RealObjectKey` to a `Weak<OverlayInode>`; weak values so
    /// the cache never forms an `OverlayFs → OverlayInode → OverlayFs` strong
    /// cycle.
    pub(in crate::fs::fs_impls::overlayfs) inodes: InodeCache,
    /// The immutable dev/ino projection policy (`P2-01`/`P0-12`, including
    /// the `P1-07` lower-id consumption).
    ///
    /// Built once in the extended `OverlayFs::new` (overlay `st_dev` plus
    /// `layer_devs` from the published layer snapshot); the fallback ino
    /// allocator is a saturating `AtomicU64` inside the policy.
    pub(in crate::fs::fs_impls::overlayfs) identity: IdentityPolicy,
    /// The overlay `AnonDeviceId` RAII guard, retained for the mount lifetime
    /// (recorded forward reference from the wave-2 build-extension pass; the
    /// integration-gate widening lands in this repair).
    ///
    /// `IdentityPolicy::overlay_dev_id` copies the device id, so the guard
    /// must live on the published `OverlayFs` (the substrate-idiomatic owner —
    /// every Asterinas pseudo-fs and the legacy overlayfs hold `AnonDeviceId`
    /// on the fs struct) or the minor number could be recycled under a live
    /// mount. The `_`-prefixed name mirrors the sibling pseudo-fs precedent
    /// and suppresses the unused-field lint.
    pub(in crate::fs::fs_impls::overlayfs) _anon_device_id: AnonDeviceId,
    /// The saturating workdir temp-name serial (meso-04 P1-34 seam).
    ///
    /// Unique-naming context for the `copyup` meso's
    /// `OverlayFs::generate_workdir_temp_name` (meso-04 spec §4.1 / P1-34):
    /// the value is saturating-fetched and never gates I/O. The consuming
    /// methods land in Wave 4 (`copyup/mod.rs` + `copyup/workdir.rs`).
    pub(in crate::fs::fs_impls::overlayfs) workdir_temp_serial: AtomicU64,
    /// The immutable xattr classification policy (meso-05 P1-33 seam).
    ///
    /// Owned once here under the cross-meso owner-extension rule (meso-05
    /// spec §4); stateless in this wave, no lock. Forward reference by frozen
    /// name: `OverlayXattrPolicy` lands in Wave 4
    /// (`metadata_security/xattr.rs`); the `OverlayFs::xattr_policy()`
    /// accessor and the construction land with the meso-05 Creator.
    pub(in crate::fs::fs_impls::overlayfs) xattr_policy: OverlayXattrPolicy,
    /// The mount-scoped reusable whiteout cache (meso-06 P1-36 seam; the
    /// `WL` level-5 domain).
    ///
    /// Bounded to one workdir staging slot; `WL` critical sections never
    /// cover BIO/sleep/underlying calls (meso-06 spec §8). Forward reference
    /// by frozen name: `WhiteoutCache` lands in Wave 4 (`dir/whiteout.rs`);
    /// the construction and the short slot protocol land with the meso-06
    /// Creator.
    pub(in crate::fs::fs_impls::overlayfs) whiteout_cache: Mutex<WhiteoutCache>,
}

/// The `MOUNT` lifecycle state of an [`OverlayFs`].
///
/// Carries only the phase; the claims are intentionally not mutex-guarded
/// (they are released by guard `Drop` on the final `Drop`, never by a
/// lifecycle transition — spec §4 lock-carrier rationale).
#[derive(Debug)]
pub(super) struct MountLifecycle {
    pub(super) phase: MountPhase,
}

/// The `MOUNT` lifecycle phase of an overlay mount.
///
/// `Ready` is the construction-time phase; [`OverlayFs::begin_shutdown`]
/// performs the only transition, `Ready` → `ShuttingDown`. The final release
/// is the last-`Drop` RAII boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum MountPhase {
    /// The mount is live and accepts operations.
    Ready,
    /// The mount is draining; no new operations may start.
    #[expect(
        dead_code,
        reason = "only reachable via the deferred OverlayFs::begin_shutdown teardown path"
    )]
    ShuttingDown,
}

impl OverlayFs {
    /// Transitions the `MOUNT` lifecycle from `Ready` to `ShuttingDown`.
    ///
    /// Returns `EBUSY` if the mount is already shutting down. Claim release
    /// happens only on the final `Drop` (after pinned consumers drain), so no
    /// consumer can observe a half-released claim.
    #[expect(
        dead_code,
        reason = "frozen spec §4 MOUNT lifecycle surface; consumed by the teardown path once the forward VFS seams land (meso-02+ / VFS shutdown)"
    )]
    pub(super) fn begin_shutdown(&self) -> Result<()> {
        let mut lifecycle = self.lifecycle.lock();
        if lifecycle.phase == MountPhase::ShuttingDown {
            return Err(Error::new(Errno::EBUSY));
        }
        lifecycle.phase = MountPhase::ShuttingDown;
        Ok(())
    }

    /// Returns the immutable layer stack.
    ///
    /// Widened to the overlayfs ceiling (round-2 review item 1): the
    /// `projection` tree consumes this accessor from
    /// `OverlayInode::new_root` (E0624 before the widening). The return type
    /// remains `mount`-private this wave (wave-1 `layers.rs` widening is the
    /// recorded integration-gate follow-up).
    pub(in crate::fs::fs_impls::overlayfs) fn layer_stack(&self) -> &OverlayLayerStack {
        &self.layer_stack
    }

    /// Returns the immutable mount policy snapshot.
    ///
    /// Widened to the overlayfs ceiling (round-2 review item 1): consumed by
    /// `OverlayInode::read_only_gate` and `OverlayFs::store_lower_id` from
    /// the `projection` tree.
    pub(in crate::fs::fs_impls::overlayfs) fn policy(&self) -> &MountPolicy {
        &self.policy
    }

    /// Returns the claimed upper/workdir pair, if this is a writable mount.
    ///
    /// Widened to the overlayfs ceiling (round-2 review item 1) to match the
    /// other accessors; no `projection` caller today.
    pub(in crate::fs::fs_impls::overlayfs) fn claims(&self) -> Option<&UpperWorkdirClaim> {
        self.claims.as_ref()
    }

    /// Returns the real filesystem that superblock hooks forward to: the upper
    /// filesystem for writable mounts, otherwise the topmost lower layer.
    ///
    /// `P0-05`: `sync`/`statfs` semantics are forwarded to this filesystem.
    /// The topmost lower is `lowers[0]`, which is guaranteed non-empty by the
    /// `P0-02` invariant — now enforced structurally by the checked
    /// `OverlayLayerStack::assemble` constructor (wave-1 review
    /// `rust-type-invariants` fix, item 8).
    fn selected_real_fs(&self) -> &Arc<dyn FileSystem> {
        match self.claims() {
            Some(claims) => claims.upper_fs(),
            None => &self.layer_stack.lowers[0].fs,
        }
    }
}

impl FileSystem for OverlayFs {
    fn name(&self) -> &'static str {
        OVERLAY_FS_NAME
    }

    fn source(&self) -> Option<&str> {
        Some(self.mount_source.as_str())
    }

    fn sync(&self) -> Result<()> {
        self.selected_real_fs().sync()
    }

    fn root_inode(&self) -> Arc<dyn Inode> {
        let root_inode = self.root_inode.lock();
        match root_inode.as_ref() {
            Some(root) => root.clone(),
            // `OverlayFs::new` fills the slot right after publishing the
            // `Arc`, so a published mount always carries its root; a missing
            // slot is a construction-order violation, never a runtime
            // condition (hard invariant, no `.unwrap()`/`.expect()`).
            None => unreachable!(
                "OverlayFs::new materializes the root carrier before publication; \
                 a published overlay mount always has its root slot set"
            ),
        }
    }

    fn sb(&self) -> SuperBlock {
        let mut super_block = self.selected_real_fs().sb();
        if let Some(uuid) = self.policy().uuid() {
            super_block.fsid = uuid.value();
        }
        super_block
    }

    fn flags(&self) -> FsFlags {
        if self.policy().is_effective_read_only() {
            FsFlags::RDONLY
        } else {
            FsFlags::empty()
        }
    }

    fn set_fs_flags(&self, flags: FsFlags, _data: Option<&str>, _ctx: &Context) -> Result<()> {
        // The effective read-only state is frozen at mount time and only
        // reported by `flags()`; full remount semantics are a recorded
        // insertion point under `P0-05`, so any delta is rejected instead of
        // being silently accepted.
        let current_flags = self.flags();
        if current_flags.contains(FsFlags::RDONLY) && !flags.contains(FsFlags::RDONLY) {
            return Err(Error::new(Errno::EROFS));
        }
        if flags != current_flags {
            return Err(Error::with_message(
                Errno::EINVAL,
                "unsupported overlayfs remount delta",
            ));
        }
        Ok(())
    }

    fn fs_event_subscriber_stats(&self) -> &FsEventSubscriberStats {
        &self.fs_event_stats
    }
}
