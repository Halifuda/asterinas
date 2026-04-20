// SPDX-License-Identifier: MPL-2.0

mod bitmap;
mod boot_region;
mod mount_diagnostics;
mod root_directory;
mod upcase;

use super::{
    boot::ValidatedMount,
    fs::MountVolumeStateError,
};
use aster_block::BlockDevice;

pub(super) fn load_validated_mount(
    block_device: &dyn BlockDevice,
) -> core::result::Result<ValidatedMount, MountVolumeStateError> {
    ValidatedMount::load(block_device)
}

pub(super) fn diagnose_invalid_on_disk_layout_gate(block_device: &dyn BlockDevice) -> &'static str {
    mount_diagnostics::diagnose_invalid_on_disk_layout_gate(block_device)
}
