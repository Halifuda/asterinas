// SPDX-License-Identifier: MPL-2.0

//! The overlay filesystem object and its VFS-facing superblock surface.
//!
//! `OverlayFs` is the per-mount overlay filesystem object that owns the
//! layer stack, claims, policy, and projection state; the `FileSystem`
//! impl forwards the superblock surface to the underlying real filesystem.

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

/// The top-level overlay filesystem object.
///
/// Created by [`OverlayFs::new`]; owns the layer stack, claims, policy,
/// projection state.
pub(in overlayfs) struct OverlayFs {
    pub(super) layer_stack: OverlayLayerStack,
    /// The claimed upper/workdir pair; `Some` only for writable mounts.
    ///
    /// Established single-threaded before publication and released by the
    /// final `Drop` (guard `Drop`). The claim additionally pins the prepared
    /// workdir staging workspace inode (`<workdir>/work`) once
    /// `prepare_workdir` completes.
    pub(super) claims: Option<UpperWorkdirClaim>,
    pub(super) policy: MountPolicy,
    pub(super) mount_source: String,
    /// The prepared root inode.
    ///
    /// `root_inode()` only clones the prepared root; a `None` value for a
    /// published mount is a hard construction invariant failure, never a
    /// silent mount-less root.
    pub(super) root_inode: Mutex<Option<Arc<dyn Inode>>>,
    pub(super) lifecycle: Mutex<MountLifecycle>,
    pub(super) fs_event_stats: FsEventSubscriberStats,
    /// The canonical weak mount reference.
    pub(in overlayfs) self_weak: Weak<OverlayFs>,
    /// The mount-wide binding cache — the first source for `(parent, name)`
    /// lookup results.
    ///
    /// A positive binding pins its inode, a negative one pins its barrier;
    /// insert/update happen under the caller's parent directory transaction
    /// lock.
    pub(in overlayfs) bindings: BindingCache,
    /// The mount-wide inode identity-reuse cache.
    ///
    /// Maps each `RealObjectKey` to a `Weak<OverlayInode>`.
    pub(in overlayfs) inodes: InodeCache,
    /// The dev/ino projection policy.
    pub(in overlayfs) identity: IdentityPolicy,
    /// The overlay `AnonDeviceId` RAII guard, retained for the mount lifetime.
    ///
    /// `IdentityPolicy::overlay_dev_id` copies the device id, so the guard
    /// must live on the published `OverlayFs` (the substrate-idiomatic owner —
    /// every Asterinas pseudo-fs and the legacy overlayfs hold `AnonDeviceId`
    /// on the fs struct) or the minor number could be recycled under a live
    /// mount. The `_`-prefixed name mirrors the sibling pseudo-fs precedent
    /// and suppresses the unused-field lint.
    pub(in overlayfs) _anon_device_id: AnonDeviceId,
    /// The saturating workdir temp-name serial.
    pub(in overlayfs) workdir_temp_serial: AtomicU64,
    /// The xattr classification policy.
    ///
    /// Owned once here; stateless.
    pub(in overlayfs) xattr_policy: OverlayXattrPolicy,
    /// The mount-scoped reusable whiteout cache.
    ///
    /// Bounded to one workdir staging slot.
    pub(in overlayfs) whiteout_cache: Mutex<WhiteoutCache>,
}

/// The mount lifecycle state of an [`OverlayFs`].
#[derive(Debug)]
pub(super) struct MountLifecycle {
    pub(super) phase: MountPhase,
}

/// The mount lifecycle phase of an overlay mount.
///
/// `Ready` is the construction-time phase; [`OverlayFs::begin_shutdown`]
/// performs the only transition, `Ready` → `ShuttingDown`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum MountPhase {
    /// The mount is live and accepts operations.
    Ready,
    /// The mount is draining; no new operations may start.
    #[expect(dead_code, reason = "the VFS exposes no filesystem shutdown callback")]
    ShuttingDown,
}

impl OverlayFs {
    /// Transitions the mount lifecycle from `Ready` to `ShuttingDown`.
    ///
    /// Returns `EBUSY` if the mount is already shutting down.
    // TODO: Invoke this from the VFS unmount/shutdown callback before detach.
    #[expect(dead_code, reason = "the VFS exposes no filesystem shutdown callback")]
    pub(super) fn begin_shutdown(&self) -> Result<()> {
        let mut lifecycle = self.lifecycle.lock();
        if lifecycle.phase == MountPhase::ShuttingDown {
            return Err(Error::new(Errno::EBUSY));
        }
        lifecycle.phase = MountPhase::ShuttingDown;
        Ok(())
    }

    /// Returns the layer stack of this mount.
    pub(in overlayfs) fn layer_stack(&self) -> &OverlayLayerStack {
        &self.layer_stack
    }

    /// Returns the mount policy.
    pub(in overlayfs) fn policy(&self) -> &MountPolicy {
        &self.policy
    }

    pub(in overlayfs) fn claims(&self) -> Option<&UpperWorkdirClaim> {
        self.claims.as_ref()
    }

    /// Returns the real filesystem that superblock hooks forward to: the upper
    /// filesystem for writable mounts, otherwise the topmost lower layer.
    ///
    /// `sync`/`statfs` semantics are forwarded to this filesystem.
    fn selected_real_fs(&self) -> &Arc<dyn FileSystem> {
        self.layer_stack
            .upper
            .as_ref()
            .map_or(&self.layer_stack.lowers[0].fs, |upper| &upper.fs)
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
            None => unreachable!(
                "OverlayFs::new materializes the root inode before publication; \
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
