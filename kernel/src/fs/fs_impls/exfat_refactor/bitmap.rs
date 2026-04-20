// SPDX-License-Identifier: MPL-2.0

use super::{
    boot::BootRegion,
    fat::{ChainVisitControl, FatReader},
    fs::MountVolumeStateError,
};

pub(super) const ALLOCATION_BITMAP_ENTRY_TYPE: u8 = 0x81;

#[derive(Clone, Copy)]
pub(super) struct AllocationBitmapRecord {
    pub(super) data_length: u64,
    pub(super) first_cluster: u32,
}

impl AllocationBitmapRecord {
    pub(super) fn count_used_clusters(
        self,
        boot_region: &BootRegion,
        fat_reader: &mut FatReader<'_>,
    ) -> core::result::Result<(usize, bool), MountVolumeStateError> {
        boot_region.validate_stream_data(self.first_cluster, self.data_length)?;
        let cluster_count = boot_region.cluster_count_usize()?;
        let required_bytes = cluster_count.div_ceil(8);
        let bitmap_bytes = usize::try_from(self.data_length)
            .map_err(|_| MountVolumeStateError::InvalidOnDiskLayout)?;
        if bitmap_bytes < required_bytes {
            return Err(MountVolumeStateError::InconsistentAccounting);
        }

        let mut bits_remaining = cluster_count;
        let mut bitmap_bytes_remaining = bitmap_bytes;
        let mut used_clusters = 0usize;
        let result = fat_reader.walk_cluster_chain(self.first_cluster, |_, cluster_bytes| {
            let bytes_to_visit = bitmap_bytes_remaining.min(cluster_bytes.len());
            for byte in &cluster_bytes[..bytes_to_visit] {
                if bits_remaining == 0 {
                    if *byte != 0 {
                        return Err(MountVolumeStateError::InconsistentAccounting);
                    }
                    continue;
                }
                let relevant_bits = bits_remaining.min(u8::BITS as usize);
                let mask = if relevant_bits == u8::BITS as usize {
                    u8::MAX
                } else {
                    (1u16
                        .checked_shl(
                            u32::try_from(relevant_bits)
                                .map_err(|_| MountVolumeStateError::InconsistentAccounting)?,
                        )
                        .ok_or(MountVolumeStateError::InconsistentAccounting)?
                        - 1) as u8
                };
                let masked_byte = *byte & mask;
                if masked_byte != *byte && (*byte & !mask) != 0 {
                    return Err(MountVolumeStateError::InconsistentAccounting);
                }
                used_clusters = used_clusters
                    .checked_add(masked_byte.count_ones() as usize)
                    .ok_or(MountVolumeStateError::InconsistentAccounting)?;
                bits_remaining -= relevant_bits;
            }
            bitmap_bytes_remaining -= bytes_to_visit;
            if bitmap_bytes_remaining == 0 {
                return Ok(ChainVisitControl::Stop);
            }
            Ok(ChainVisitControl::Continue)
        });
        match result {
            Ok(()) => (),
            Err(MountVolumeStateError::InvalidOnDiskLayout) => {
                return Err(MountVolumeStateError::InconsistentAccounting);
            }
            Err(error) => return Err(error),
        }
        if bits_remaining != 0 || bitmap_bytes_remaining != 0 {
            return Err(MountVolumeStateError::InconsistentAccounting);
        }

        let counted_percent = if cluster_count == 0 {
            return Err(MountVolumeStateError::InvalidOnDiskLayout);
        } else {
            (used_clusters.saturating_mul(100) + cluster_count / 2) / cluster_count
        };
        let used_clusters_from_recount = match boot_region.percent_in_use {
            0xFF => true,
            percent_in_use => counted_percent != usize::from(percent_in_use),
        };
        Ok((used_clusters, used_clusters_from_recount))
    }

    pub(super) fn parse(entry: &[u8]) -> core::result::Result<Self, MountVolumeStateError> {
        if entry.len() != 32 {
            return Err(MountVolumeStateError::InvalidOnDiskLayout);
        }
        Ok(Self {
            first_cluster: u32::from_le_bytes([entry[20], entry[21], entry[22], entry[23]]),
            data_length: u64::from_le_bytes([
                entry[24], entry[25], entry[26], entry[27], entry[28], entry[29], entry[30],
                entry[31],
            ]),
        })
    }
}
