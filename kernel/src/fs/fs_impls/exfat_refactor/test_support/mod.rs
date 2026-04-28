// SPDX-License-Identifier: MPL-2.0

mod bitmap;
mod boot_region;
pub(super) mod inode;
mod mount_diagnostics;
mod root_directory;
mod upcase;

use aster_block::BlockDevice;

use super::{
    bitmap::AllocationBitmap,
    boot::{BootRegion, VolumeAnomalyState},
    fs::MountVolumeStateError,
    upcase::UpcaseTable,
};
use crate::prelude::*;

pub(super) struct LoadedMountState {
    pub(super) anomaly: VolumeAnomalyState,
    pub(super) bitmap: AllocationBitmap,
    pub(super) boot_region: BootRegion,
    pub(super) upcase_table: Arc<UpcaseTable>,
    pub(super) used_clusters: usize,
    pub(super) used_clusters_from_recount: bool,
}

pub(super) fn load_validated_mount(
    block_device: &dyn BlockDevice,
) -> core::result::Result<LoadedMountState, MountVolumeStateError> {
    let (boot_region, anomaly, bitmap, upcase_table, used_clusters, used_clusters_from_recount) =
        BootRegion::load_mount_state(block_device)?;
    Ok(LoadedMountState {
        anomaly,
        bitmap,
        boot_region,
        upcase_table,
        used_clusters,
        used_clusters_from_recount,
    })
}

pub(super) fn diagnose_invalid_on_disk_layout_gate(block_device: &dyn BlockDevice) -> &'static str {
    mount_diagnostics::diagnose_invalid_on_disk_layout_gate(block_device)
}
