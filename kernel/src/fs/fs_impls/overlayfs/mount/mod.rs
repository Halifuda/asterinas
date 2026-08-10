// SPDX-License-Identifier: MPL-2.0

//! Mount resource and policy: the VFS entry point, the filesystem carrier,
//! and the read-only carriers published to the rest of the overlayfs
//! implementation.
//!
//! This module provides the VFS entry point ([`OverlayFsType`] implementing
//! `crate::fs::vfs::registry::FsType`), the macro-level carrier
//! ([`OverlayFs`]), and the read-only carriers consumed by sibling modules
//! (`OverlayLayerStack`/`OverlayLayer`/`RealPath`, `MountPolicy`,
//! `CreatorCredentialPolicy`, `UpperFilesystemCapabilities`,
//! `WriteAccessAccounting`, `UpperWorkdirClaim`). All fallible mount work
//! happens inside `FsType::create` → `OverlayFs::new`; the only values that
//! cross this module boundary outward are an `Arc<dyn FileSystem>` and an
//! `Errno`-encoded error result.

mod build;
mod claims;
mod layers;
mod options;
mod policy;
mod superblock;

pub(in crate::fs::fs_impls::overlayfs) use layers::RealPath;
pub(in crate::fs::fs_impls::overlayfs) use options::XinoMode;
pub(super) use superblock::OverlayFs;

use crate::{
    fs::vfs::{
        file_system::FileSystem,
        registry::{FsCreationCtx, FsProperties, FsType},
    },
    prelude::*,
};

/// The external-facing filesystem name of overlayfs (mirrors Linux
/// `ovl_fs_type`).
///
/// Single representation of the `"overlay"` name used by the VFS entry point
/// ([`FsType::name`]), the reported mount-source default (`build.rs`), and
/// [`FileSystem::name`](crate::fs::vfs::file_system::FileSystem::name)
/// (`superblock.rs`).
pub(super) const OVERLAY_FS_NAME: &str = "overlay";

/// The VFS entry point of the overlay filesystem (mirrors Linux `ovl_fs_type`).
///
/// Registered by [`super::init`] as the active overlay filesystem entry point.
pub(super) struct OverlayFsType;

impl FsType for OverlayFsType {
    type Key = ();

    fn name(&self) -> &'static str {
        OVERLAY_FS_NAME
    }

    fn properties(&self) -> FsProperties {
        FsProperties::empty()
    }

    fn create(&self, fs_creation_ctx: &mut FsCreationCtx) -> Result<Arc<dyn FileSystem>> {
        let overlay_fs = OverlayFs::new(fs_creation_ctx)?;
        Ok(overlay_fs)
    }

    fn sysnode(&self) -> Option<Arc<dyn aster_systree::SysNode>> {
        None
    }
}
