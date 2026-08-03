// SPDX-License-Identifier: MPL-2.0

//! Mount resource & policy meso (`mount_resource_policy`, meso-01).
//!
//! Implements the frozen `meso_01_mount_resource_policy_designer_spec.md` boundary:
//! the VFS entry point (`OverlayFsType` implementing `crate::fs::vfs::registry::FsType`),
//! the Macro-Owner carrier (`OverlayFs`), and the published read-only carriers consumed
//! by sibling Mesos (`OverlayLayerStack`/`OverlayLayer`, `MountPolicy`,
//! `CreatorCredentialPolicy`, `UpperFilesystemCapabilities`, `WriteAccessAccounting`,
//! `UpperWorkdirClaim`). All fallible mount work happens inside `FsType::create` →
//! `OverlayFs::new`; the only artifacts that cross this boundary outward are an
//! `Arc<dyn FileSystem>` and the `Errno`-encoded error result (spec §1).

mod build;
mod claims;
mod layers;
mod options;
mod policy;
mod superblock;

pub(super) use claims::{OverlayUuid, UpperWorkdirClaim};
pub(super) use layers::{OverlayLayer, OverlayLayerStack};
pub(super) use options::UuidMode;
pub(super) use policy::{
    CreatorCredentialPolicy, CredentialSource, MountPolicy, UpperFilesystemCapabilities,
    WriteAccessAccounting, WriteAccessGuard,
};
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
/// (`superblock.rs`) — wave-1 review `dry` fix (item 8).
pub(super) const OVERLAY_FS_NAME: &str = "overlay";

/// The VFS entry point of the overlay filesystem (mirrors Linux `ovl_fs_type`).
///
/// Registration is deferred until the overlayfs takeover wave:
/// `overlayfs/mod.rs` still registers `legacy_fs::OverlayFsType`, and this
/// type — with the whole new `mount` path behind it — is unreachable until
/// then (wave-1 review `expect-dead-code` fix, item 8).
#[expect(
    dead_code,
    reason = "registration deferred to the overlayfs takeover wave; legacy_fs::OverlayFsType remains the active entry point"
)]
pub(super) struct OverlayFsType;

impl FsType for OverlayFsType {
    fn name(&self) -> &'static str {
        OVERLAY_FS_NAME
    }

    fn properties(&self) -> FsProperties {
        FsProperties::empty()
    }

    fn create(&self, fs_creation_ctx: &FsCreationCtx) -> Result<Arc<dyn FileSystem>> {
        let overlay_fs = OverlayFs::new(fs_creation_ctx)?;
        Ok(overlay_fs)
    }

    fn sysnode(&self) -> Option<Arc<dyn aster_systree::SysNode>> {
        None
    }
}
