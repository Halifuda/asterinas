// SPDX-License-Identifier: MPL-2.0

//! The overlay filesystem Macro-Owner carrier and its VFS-facing superblock
//! surface (`P0-05`).
//!
//! This module owns the `OverlayFs` struct (the published mount/layer/policy
//! state), the `FileSystem` trait implementation, and the `MOUNT` level-1
//! lifecycle domain (`MountLifecycle`/`MountPhase`). All fallible mount work
//! happens in `build.rs` (`OverlayFs::new`); the hooks here enter through a
//! pinned `Arc<OverlayFs>` and hold no Overlay lock except the short `MOUNT`
//! transition inside [`OverlayFs::begin_shutdown`].

use super::{
    claims::UpperWorkdirClaim, layers::OverlayLayerStack, policy::MountPolicy, OVERLAY_FS_NAME,
};
use crate::{
    fs::vfs::{
        file_system::{FileSystem, FsEventSubscriberStats, FsFlags, SuperBlock},
        inode::Inode,
    },
    prelude::*,
};

/// The Macro-Owner carrier of the overlay filesystem (mirrors Linux `ovl_fs`).
///
/// `OverlayFs` is the only object that publishes mount/layer/policy state to
/// sibling Mesos. It is created by [`OverlayFs::new`] (in `build.rs`) through
/// the frozen 11-step construction sequence; after publication the layer
/// stack, claims, and policy snapshot are immutable.
///
/// Invariants: `root_inode()` returns the prepared root carrier and performs
/// no fallible work (`P0-05` infallible-root invariant); `claims` is `Some`
/// only for writable mounts and is released exactly once on the final `Drop`
/// (guard `Drop`, atomic non-blocking, no mutex); the `MOUNT` lifecycle is
/// used only for lifecycle transitions and is never held across underlying
/// callbacks.
pub(super) struct OverlayFs {
    pub(super) layer_stack: OverlayLayerStack,
    /// The claimed upper/workdir pair; `Some` only for writable mounts.
    ///
    /// Established single-threaded before publication and released by the
    /// final `Drop` (guard `Drop`, no mutex).
    pub(super) claims: Option<UpperWorkdirClaim>,
    pub(super) policy: MountPolicy,
    /// The reported mount source (`P0-05` show-options surface).
    pub(super) mount_source: String,
    /// The prepared root carrier (provisional cross-meso seam, spec §3.0.5
    /// item 8).
    pub(super) root_inode: Arc<dyn Inode>,
    /// The `MOUNT` level-1 lifecycle domain; phase only, sleep-capable.
    pub(super) lifecycle: Mutex<MountLifecycle>,
    pub(super) fs_event_stats: FsEventSubscriberStats,
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
    #[expect(
        dead_code,
        reason = "frozen published getter (spec §4); consumed by sibling mesos (meso-02+) once they land"
    )]
    pub(super) fn layer_stack(&self) -> &OverlayLayerStack {
        &self.layer_stack
    }

    /// Returns the immutable mount policy snapshot.
    pub(super) fn policy(&self) -> &MountPolicy {
        &self.policy
    }

    /// Returns the claimed upper/workdir pair, if this is a writable mount.
    pub(super) fn claims(&self) -> Option<&UpperWorkdirClaim> {
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
        self.root_inode.clone()
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
