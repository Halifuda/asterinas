// SPDX-License-Identifier: MPL-2.0

#![short_vis_path::add(overlayfs)]
//! Published mount policy state.
//!
//! A mount policy is the fixed per-mount decision state: whether the mount is
//! effectively read-only or `default_permissions`, the `xino`/UUID modes, the
//! selected private-xattr prefix, and the effective overlay UUID. The
//! upper-filesystem capabilities are measured during mount construction and
//! published here as fixed state; the policy performs no probing itself.

use super::mount::{capabilities::UpperFilesystemCapabilities, inuse::Uuid};
use crate::fs::fs_impls::overlayfs::inode::OverlayXattrPrefix;

/// The UUID/`fsid` policy of an overlay mount.
///
/// The default is [`UuidMode::Auto`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum UuidMode {
    /// The overlay UUID is null and the fsid comes from the topmost underlying fs.
    Off,
    /// Same as [`UuidMode::Off`], plus underlying-layer UUIDs are ignored.
    Null,
    /// The overlay UUID is generated and persisted as the selected
    /// private-prefix uuid record (`trusted.overlay.uuid` by default,
    /// `user.overlay.uuid` in `userxattr` mode).
    On,
    /// Reuse an existing persisted UUID or upgrade to `On`; degrade to `Null`.
    Auto,
}

/// The `xino` mode.
///
/// The default is [`XinoMode::Auto`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in overlayfs) enum XinoMode {
    /// xino encoding disabled; non-directories report the underlying dev/ino.
    Off,
    /// xino enabled when feasible (the default).
    Auto,
    /// xino encoding always enabled.
    On,
}

pub(in overlayfs) struct MountPolicy {
    is_effective_read_only: bool,
    uuid: Option<Uuid>,
    upper_capabilities: Option<UpperFilesystemCapabilities>,
    is_default_permissions: bool,
    xino_mode: XinoMode,
    xattr_prefix: OverlayXattrPrefix,
}

// TODO: Reintroduce a scoped creator-credential switch once the VFS provides a credentials API.

impl MountPolicy {
    /// Assembles the published per-mount policy from the resolved
    /// construction inputs (single caller: `OverlayFs::new`). The options
    /// translation — default resolution and prefix selection — happens at
    /// the construction consumer; the policy publishes effective state only.
    pub(super) fn assemble(
        is_effective_read_only: bool,
        is_default_permissions: bool,
        xino_mode: XinoMode,
        xattr_prefix: OverlayXattrPrefix,
        uuid: Option<Uuid>,
        upper_capabilities: Option<UpperFilesystemCapabilities>,
    ) -> Self {
        Self {
            is_effective_read_only,
            uuid,
            upper_capabilities,
            is_default_permissions,
            xino_mode,
            // Stored for every mount — read-only mounts included — because
            // the passthrough get/list paths need the selected prefix even
            // without an upper.
            xattr_prefix,
        }
    }

    /// Returns whether this mount is effectively read-only.
    pub(in overlayfs) fn is_effective_read_only(&self) -> bool {
        self.is_effective_read_only
    }

    /// Returns whether `default_permissions` was specified for this mount.
    pub(in overlayfs) fn is_default_permissions(&self) -> bool {
        self.is_default_permissions
    }

    pub(super) fn xino_mode(&self) -> XinoMode {
        self.xino_mode
    }

    /// Returns the mount's selected private-xattr prefix (`user.overlay.`
    /// in `userxattr` mode, `trusted.overlay.` by default).
    pub(in overlayfs) fn xattr_prefix(&self) -> OverlayXattrPrefix {
        self.xattr_prefix
    }

    /// Returns the overlay UUID when effective.
    pub(super) fn uuid(&self) -> Option<&Uuid> {
        self.uuid.as_ref()
    }

    /// Returns the post-claim upper-filesystem capabilities.
    pub(in overlayfs) fn upper_capabilities(&self) -> Option<&UpperFilesystemCapabilities> {
        self.upper_capabilities.as_ref()
    }
}
