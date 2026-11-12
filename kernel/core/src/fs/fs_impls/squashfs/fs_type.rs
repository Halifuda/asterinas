// SPDX-License-Identifier: MPL-2.0

//! The `squashfs` registration entry.
//!
//! [`SquashfsType`] publishes the name `squashfs`,
//! declares that the filesystem needs a disk, and answers a mount request;
//! [`init`] installs it in the registry.

#![expect(unused_variables, reason = "every body here is still `todo!()`")]

use aster_systree::SysNode;

use crate::{
    fs::vfs::{
        file_system::FileSystem,
        registry::{self, FsCreationCtx, FsProperties, FsType},
    },
    prelude::*,
};

/// Users: SquashfsType::name.
pub(super) const SQUASHFS_NAME: &str = "squashfs";

/// Produced by: SQUASHFS_TYPE. Users: registry::register.
pub(super) struct SquashfsType;

/// Users: fs_type::init.
pub(super) static SQUASHFS_TYPE: SquashfsType = SquashfsType;

impl FsType for SquashfsType {
    type Key = ();

    fn name(&self) -> &'static str {
        SQUASHFS_NAME
    }

    fn properties(&self) -> FsProperties {
        FsProperties::NEED_DISK
    }

    fn create(&self, fs_creation_ctx: &mut FsCreationCtx) -> Result<Arc<dyn FileSystem>> {
        // TODO: build the mount from the device once opening an image is implemented.
        Err(Error::with_message(
            Errno::EINVAL,
            "squashfs mounting is not implemented yet",
        ))
    }

    fn sysnode(&self) -> Option<Arc<dyn SysNode>> {
        None
    }
}

/// Caller: fs_impls::init. Callee: registry::register.
pub(in crate::fs::fs_impls) fn init() {
    registry::register(&SQUASHFS_TYPE).unwrap();
}
