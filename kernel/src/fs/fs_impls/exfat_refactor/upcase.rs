// SPDX-License-Identifier: MPL-2.0

use super::{
    boot::BootRegion,
    fat::{ChainVisitControl, FatReader},
    fs::ExfatFsError,
};
use crate::prelude::*;

pub(super) const UPCASE_TABLE_ENTRY_TYPE: u8 = 0x82;

#[derive(Clone)]
pub(super) struct UpcaseTable {
    mapping: Vec<u16>,
}

impl UpcaseTable {
    pub(super) const NAME_MAX: usize = 255;
    const TABLE_CODE_UNIT_COUNT: usize = u16::MAX as usize + 1;
    const UNCOMPRESSED_TABLE_BYTE_LEN: usize = Self::TABLE_CODE_UNIT_COUNT * 2;
    const MANDATORY_PREFIX_CODE_UNIT_COUNT: u8 = 128;

    pub(super) fn load(
        boot_region: &BootRegion,
        fat_reader: &mut FatReader<'_>,
        upcase: UpcaseRecord,
    ) -> core::result::Result<Self, ExfatFsError> {
        boot_region.validate_stream_data(upcase.first_cluster, upcase.data_length)?;
        let data_length =
            usize::try_from(upcase.data_length).map_err(|_| ExfatFsError::InvalidOnDiskLayout)?;
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
            return Err(ExfatFsError::InvalidOnDiskLayout);
        }
        if Self::stream_checksum(&table_bytes) != upcase.checksum {
            return Err(ExfatFsError::InvalidOnDiskLayout);
        }
        Ok(Self {
            mapping: Self::decode_mapping(&table_bytes)?,
        })
    }

    fn stream_checksum(bytes: &[u8]) -> u32 {
        let mut checksum = 0u32;
        for byte in bytes {
            checksum = checksum.rotate_right(1).wrapping_add(u32::from(*byte));
        }
        checksum
    }

    fn decode_mapping(table_bytes: &[u8]) -> core::result::Result<Vec<u16>, ExfatFsError> {
        let mapping = if table_bytes.len() == Self::UNCOMPRESSED_TABLE_BYTE_LEN {
            let mut mapping = Vec::with_capacity(Self::TABLE_CODE_UNIT_COUNT);
            for word in table_bytes.chunks_exact(2) {
                mapping.push(u16::from_le_bytes([word[0], word[1]]));
            }
            mapping
        } else {
            let mut words = table_bytes.chunks_exact(2);
            if !words.remainder().is_empty() {
                return Err(ExfatFsError::InvalidOnDiskLayout);
            }

            let mut mapping = Vec::with_capacity(Self::TABLE_CODE_UNIT_COUNT);
            while let Some(word) = words.next() {
                let value = u16::from_le_bytes([word[0], word[1]]);
                if value != u16::MAX {
                    if mapping.len() == Self::TABLE_CODE_UNIT_COUNT {
                        return Err(ExfatFsError::InvalidOnDiskLayout);
                    }
                    mapping.push(value);
                    continue;
                }

                let Some(identity_count_word) = words.next() else {
                    if mapping.len() == usize::from(u16::MAX) {
                        mapping.push(u16::MAX);
                        break;
                    }
                    return Err(ExfatFsError::InvalidOnDiskLayout);
                };
                let identity_count =
                    u16::from_le_bytes([identity_count_word[0], identity_count_word[1]]);
                if identity_count == 0 {
                    return Err(ExfatFsError::InvalidOnDiskLayout);
                }

                let run_end = mapping
                    .len()
                    .checked_add(usize::from(identity_count))
                    .ok_or(ExfatFsError::InvalidOnDiskLayout)?;
                if run_end > Self::TABLE_CODE_UNIT_COUNT {
                    return Err(ExfatFsError::InvalidOnDiskLayout);
                }

                for code_unit in mapping.len()..run_end {
                    mapping.push(
                        u16::try_from(code_unit).map_err(|_| ExfatFsError::InvalidOnDiskLayout)?,
                    );
                }
            }

            mapping
        };

        if mapping.len() != Self::TABLE_CODE_UNIT_COUNT {
            return Err(ExfatFsError::InvalidOnDiskLayout);
        }

        for code_unit in 0..Self::MANDATORY_PREFIX_CODE_UNIT_COUNT {
            let expected_mapping = match code_unit {
                b'a'..=b'z' => u16::from(code_unit - b'a' + b'A'),
                _ => u16::from(code_unit),
            };
            if mapping[usize::from(code_unit)] != expected_mapping {
                return Err(ExfatFsError::InvalidOnDiskLayout);
            }
        }

        Ok(mapping)
    }

    pub(super) fn names_equal(&self, left: &[u16], right: &[u16]) -> bool {
        left.len() == right.len()
            && left
                .iter()
                .zip(right.iter())
                .all(|(left_code_unit, right_code_unit)| {
                    self.mapping[usize::from(*left_code_unit)]
                        == self.mapping[usize::from(*right_code_unit)]
                })
    }

    pub(super) fn name_hash(&self, name: &[u16]) -> u16 {
        let mut hash = 0u16;
        for code_unit in name {
            for byte in self.mapping[usize::from(*code_unit)].to_le_bytes() {
                hash = ((hash & 1) << 15) + (hash >> 1) + u16::from(byte);
            }
        }
        hash
    }
}

#[derive(Clone, Copy)]
pub(super) struct UpcaseRecord {
    pub(super) checksum: u32,
    pub(super) data_length: u64,
    pub(super) first_cluster: u32,
}

impl UpcaseRecord {
    pub(super) fn parse(entry: &[u8]) -> core::result::Result<Self, ExfatFsError> {
        if entry.len() != 32 {
            return Err(ExfatFsError::InvalidOnDiskLayout);
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
