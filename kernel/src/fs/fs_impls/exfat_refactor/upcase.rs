// SPDX-License-Identifier: MPL-2.0

use super::{
    boot::BootRegion,
    fat::{ChainVisitControl, FatReader},
    fs::MountVolumeStateError,
};
use crate::prelude::*;

pub(super) const UPCASE_TABLE_ENTRY_TYPE: u8 = 0x82;

#[derive(Clone)]
pub(super) struct UpcaseTable {
    pub(super) data: Vec<u8>,
}

impl UpcaseTable {
    pub(super) const NAME_MAX: usize = 255;

    pub(super) fn load(
        boot_region: &BootRegion,
        fat_reader: &mut FatReader<'_>,
        upcase: UpcaseRecord,
    ) -> core::result::Result<Self, MountVolumeStateError> {
        boot_region.validate_stream_data(upcase.first_cluster, upcase.data_length)?;
        let data_length = usize::try_from(upcase.data_length)
            .map_err(|_| MountVolumeStateError::InvalidOnDiskLayout)?;
        let mut remaining = data_length;
        let mut table_bytes = Vec::with_capacity(data_length);
        fat_reader.walk_cluster_chain(upcase.first_cluster, |_, cluster_bytes| {
            let bytes_to_copy = remaining.min(cluster_bytes.len());
            table_bytes.extend_from_slice(&cluster_bytes[..bytes_to_copy]);
            remaining -= bytes_to_copy;
            if remaining == 0 {
                return Ok(ChainVisitControl::Stop);
            }
            Ok(ChainVisitControl::Continue)
        })?;
        if remaining != 0 {
            return Err(MountVolumeStateError::InvalidOnDiskLayout);
        }
        if Self::stream_checksum(&table_bytes) != upcase.checksum {
            return Err(MountVolumeStateError::InvalidOnDiskLayout);
        }
        Ok(Self { data: table_bytes })
    }

    fn stream_checksum(bytes: &[u8]) -> u32 {
        let mut checksum = 0u32;
        for byte in bytes {
            checksum = checksum.rotate_right(1).wrapping_add(u32::from(*byte));
        }
        checksum
    }
}

#[derive(Clone, Copy)]
pub(super) struct UpcaseRecord {
    pub(super) checksum: u32,
    pub(super) data_length: u64,
    pub(super) first_cluster: u32,
}

impl UpcaseRecord {
    pub(super) fn parse(entry: &[u8]) -> core::result::Result<Self, MountVolumeStateError> {
        if entry.len() != 32 {
            return Err(MountVolumeStateError::InvalidOnDiskLayout);
        }
        Ok(Self {
            checksum: u32::from_le_bytes([entry[4], entry[5], entry[6], entry[7]]),
            first_cluster: u32::from_le_bytes([entry[20], entry[21], entry[22], entry[23]]),
            data_length: u64::from_le_bytes([
                entry[24], entry[25], entry[26], entry[27], entry[28], entry[29], entry[30],
                entry[31],
            ]),
        })
    }
}
