// SPDX-License-Identifier: MPL-2.0

//! Mount resource and policy: the VFS entry point, the filesystem object,
//! and the mount state of an overlay filesystem.
//!
//! The mount state is the per-mount overlay lifecycle gathered during mount
//! construction: the pinned layer stack, the workdir/upper claims, the
//! creator-credential policy, and the publication of the filesystem object
//! as a VFS-visible `Arc<dyn FileSystem>` self.
//!
//! This module is the VFS entry point ([`OverlayFsType`]) and the top-level
//! overlay filesystem object ([`OverlayFs`]); all fallible mount work happens
//! inside `FsType::create` → `OverlayFs::new`.

mod build;
mod claims;
mod layers;
mod options;
mod policy;
mod superblock;

pub(in overlayfs) use layers::RealPath;
pub(in overlayfs) use options::XinoMode;
use ostd::task::Task;
pub(super) use superblock::OverlayFs;

use crate::{
    fs::vfs::{
        file_system::FileSystem,
        registry::{FsCreationCtx, FsProperties, FsType},
    },
    prelude::*,
    process::posix_thread::{AsPosixThread, PosixThread},
};

pub(super) const OVERLAY_FS_NAME: &str = "overlay";

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

pub(super) fn with_current_posix_thread<T>(
    operation_fn: impl FnOnce(&PosixThread) -> Result<T>,
) -> Result<T> {
    let current_task = Task::current().ok_or_else(|| {
        Error::with_message(Errno::EINVAL, "the overlay mount has no current task")
    })?;
    let posix_thread = current_task.as_posix_thread().ok_or_else(|| {
        Error::with_message(
            Errno::EINVAL,
            "the overlay mount task is not a POSIX thread",
        )
    })?;
    operation_fn(posix_thread)
}
