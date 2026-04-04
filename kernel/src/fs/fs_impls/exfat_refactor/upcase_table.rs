// SPDX-License-Identifier: MPL-2.0
#![cfg_attr(
    not(ktest),
    expect(
        dead_code,
        reason = "Upcase-table loading is staged before later consumers are wired."
    )
)]

use core::{convert::TryFrom, mem::size_of};

use aster_block::BlockDevice;

use super::{
    fat::{ChainMode, ExfatChain},
    io::read_metadata_bytes,
    super_block::ExfatSuperBlock,
    sysroot::ExfatSysRootUpcaseDiscovery,
};
use crate::prelude::*;

const MIN_UPCASE_TABLE_BYTES: usize = 128 * size_of::<u16>();

/// Stores the validated, read-only loaded exFAT upcase table.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct ExfatUpcaseTable {
    words: Box<[u16]>,
    byte_size: usize,
    checksum: u32,
}

impl ExfatUpcaseTable {
    /// Loads and validates the on-disk upcase table from the discovered root-entry facts.
    pub(super) fn load(
        block_device: &dyn BlockDevice,
        super_block: &ExfatSuperBlock,
        upcase_facts: &ExfatSysRootUpcaseDiscovery,
    ) -> Result<Self> {
        validate_upcase_table_size(upcase_facts.byte_size)?;

        let cluster_size = super_block.cluster_size();
        if cluster_size == 0 {
            return Err(Error::with_message(
                Errno::EINVAL,
                "upcase table cluster size must not be zero",
            ));
        }

        let cluster_count = upcase_facts.byte_size.div_ceil(cluster_size);
        let cluster_count = u32::try_from(cluster_count).map_err(|_| {
            Error::with_message(Errno::EINVAL, "upcase table spans too many clusters")
        })?;

        let chain = ExfatChain::new(
            block_device,
            super_block,
            upcase_facts.start_cluster,
            Some(cluster_count),
            ChainMode::Contiguous,
        )?;
        let payload_offset = chain.physical_cluster_start_offset(super_block)?;

        let mut payload = vec![0; upcase_facts.byte_size];
        read_metadata_bytes(block_device, payload_offset, &mut payload)?;

        let checksum = checksum32(&payload);
        if checksum != upcase_facts.checksum {
            return Err(Error::with_message(
                Errno::EINVAL,
                "upcase table checksum mismatched",
            ));
        }

        let words = payload
            .chunks_exact(size_of::<u16>())
            .map(|word_bytes| u16::from_le_bytes([word_bytes[0], word_bytes[1]]))
            .collect::<Vec<_>>()
            .into_boxed_slice();

        Ok(Self {
            words,
            byte_size: upcase_facts.byte_size,
            checksum,
        })
    }

    /// Returns the exFAT `NameHash` for logical UTF-16 units after table-backed folding.
    pub(super) fn name_hash(&self, logical_name_units: &[u16]) -> u16 {
        let mut hash = 0u16;
        for &unit in logical_name_units {
            let folded_unit = self.fold_unit(unit);
            for byte in folded_unit.to_le_bytes() {
                hash = hash.rotate_right(1).wrapping_add(u16::from(byte));
            }
        }
        hash
    }

    fn fold_unit(&self, unit: u16) -> u16 {
        self.words.get(usize::from(unit)).copied().unwrap_or(unit)
    }
}

fn validate_upcase_table_size(byte_size: usize) -> Result<()> {
    if byte_size < MIN_UPCASE_TABLE_BYTES {
        return Err(Error::with_message(
            Errno::EINVAL,
            "upcase table is too small",
        ));
    }

    if byte_size % size_of::<u16>() != 0 {
        return Err(Error::with_message(
            Errno::EINVAL,
            "upcase table size must be even",
        ));
    }

    Ok(())
}

fn checksum32(data: &[u8]) -> u32 {
    let mut checksum = 0u32;
    for &value in data {
        checksum = checksum.rotate_right(1).wrapping_add(u32::from(value));
    }
    checksum
}

#[cfg(ktest)]
mod tests {
    use alloc::vec;
    use alloc::vec::Vec;

    use aster_block::{
        bio::{BioEnqueueError, BioType, SubmittedBio},
        BlockDevice, BlockDeviceMeta,
    };
    use device_id::DeviceId;
    use ostd::prelude::ktest;

    use super::{
        checksum32, ExfatUpcaseTable, MIN_UPCASE_TABLE_BYTES, validate_upcase_table_size,
    };
    use crate::prelude::Errno;
    use crate::fs::fs_impls::exfat_refactor::{
        boot_sector::read_primary_super_block,
        fat::{ChainMode, ExfatChain},
        super_block::ExfatSuperBlock,
        test_support::{load_exfat_disk, ExfatMemoryDisk},
        sysroot::{scan_root_system_entries, ExfatSysRootUpcaseDiscovery},
    };

    const BIO_SECTOR_SIZE: usize = 512;

    #[derive(Debug)]
    struct TruncatedReadDisk {
        inner: ExfatMemoryDisk,
        read_limit: usize,
    }

    impl TruncatedReadDisk {
        fn new(inner: ExfatMemoryDisk, read_limit: usize) -> Self {
            Self { inner, read_limit }
        }
    }

    impl BlockDevice for TruncatedReadDisk {
        fn enqueue(&self, bio: SubmittedBio) -> core::result::Result<(), BioEnqueueError> {
            if matches!(bio.type_(), BioType::Read) {
                let start = bio.sid_range().start.to_raw() as usize * BIO_SECTOR_SIZE;
                let end = bio.sid_range().end.to_raw() as usize * BIO_SECTOR_SIZE;
                if end > self.read_limit {
                    return Err(BioEnqueueError::Refused);
                }
                if start >= self.read_limit {
                    return Err(BioEnqueueError::Refused);
                }
            }

            self.inner.enqueue(bio)
        }

        fn metadata(&self) -> BlockDeviceMeta {
            self.inner.metadata()
        }

        fn name(&self) -> &str {
            self.inner.name()
        }

        fn id(&self) -> DeviceId {
            self.inner.id()
        }
    }

    fn load_upcase_discovery(
        disk: &ExfatMemoryDisk,
        super_block: &ExfatSuperBlock,
    ) -> ExfatSysRootUpcaseDiscovery {
        let root_chain = ExfatChain::new(
            disk,
            super_block,
            super_block.root_dir,
            Some(1),
            ChainMode::Contiguous,
        )
        .unwrap();
        scan_root_system_entries(disk, super_block, root_chain)
            .unwrap()
            .upcase
            .unwrap()
    }

    fn write_payload(disk: &ExfatMemoryDisk, offset: usize, payload: &[u8]) {
        disk.write_bytes(offset, payload);
    }

    fn make_payload(byte_size: usize) -> Vec<u8> {
        let mut payload = vec![0u8; byte_size];
        for (index, byte) in payload.iter_mut().enumerate() {
            *byte = index as u8 ^ 0xA5;
        }
        payload
    }

    fn hash_units(units: &[u16]) -> u16 {
        let mut hash = 0u16;
        for &unit in units {
            for byte in unit.to_le_bytes() {
                hash = hash.rotate_right(1).wrapping_add(u16::from(byte));
            }
        }
        hash
    }

    fn table_with_fold(unit: u16, folded_unit: u16) -> ExfatUpcaseTable {
        let mut words = vec![0u16; usize::from(unit) + 1].into_boxed_slice();
        words[usize::from(unit)] = folded_unit;

        ExfatUpcaseTable {
            words,
            byte_size: (usize::from(unit) + 1) * core::mem::size_of::<u16>(),
            checksum: 0,
        }
    }

    fn load_context() -> (ExfatMemoryDisk, ExfatSuperBlock, ExfatSysRootUpcaseDiscovery) {
        let disk = load_exfat_disk();
        let super_block = read_primary_super_block(&disk).unwrap();
        let upcase = load_upcase_discovery(&disk, &super_block);

        (disk, super_block, upcase)
    }

    // Confirms the loader accepts a real discovery record and preserves the
    // full table bytes past the legacy 128-entry prefix boundary.
    #[ktest]
    fn valid_load_preserves_full_table_surface() {
        let (disk, super_block, mut upcase) = load_context();
        let start_cluster = upcase.start_cluster;
        let byte_size = upcase.byte_size;
        let payload = make_payload(byte_size);
        let checksum = checksum32(&payload);
        let payload_offset = super_block.cluster_to_byte_offset(start_cluster).unwrap();

        write_payload(&disk, payload_offset, &payload);
        upcase.checksum = checksum;

        let table = ExfatUpcaseTable::load(&disk, &super_block, &upcase).unwrap();

        assert_eq!(table.byte_size, byte_size);
        assert_eq!(table.checksum, checksum);
        assert_eq!(
            table.words.len(),
            byte_size / core::mem::size_of::<u16>()
        );
        assert_eq!(table.words[0], u16::from_le_bytes([payload[0], payload[1]]));
        assert_eq!(
            table.words[128],
            u16::from_le_bytes([payload[256], payload[257]])
        );
        assert_eq!(
            table.words[table.words.len() - 1],
            u16::from_le_bytes([
                payload[payload.len() - 2],
                payload[payload.len() - 1],
            ])
        );
    }

    // Confirms the loader refuses a table whose preserved checksum fact does
    // not match the on-disk payload.
    #[ktest]
    fn checksum_mismatch_is_rejected() {
        let (disk, super_block, mut upcase) = load_context();
        let start_cluster = upcase.start_cluster;
        let byte_size = upcase.byte_size;
        let payload = make_payload(byte_size);
        let payload_offset = super_block.cluster_to_byte_offset(start_cluster).unwrap();

        write_payload(&disk, payload_offset, &payload);
        upcase.checksum = checksum32(&payload) ^ 0x1;

        let error = ExfatUpcaseTable::load(&disk, &super_block, &upcase).unwrap_err();

        assert_eq!(error.error(), Errno::EINVAL);
    }

    // Confirms malformed discovery facts are rejected before they can become a
    // canonical table value.
    #[ktest]
    fn malformed_discovery_facts_are_rejected() {
        let (disk, super_block, upcase) = load_context();
        let payload = make_payload(super_block.cluster_size());
        let payload_offset = super_block.cluster_to_byte_offset(super_block.root_dir).unwrap();

        write_payload(&disk, payload_offset, &payload);

        let mut invalid_cluster = upcase;
        invalid_cluster.start_cluster = 1;
        invalid_cluster.checksum = checksum32(&payload);

        let mut invalid_size = upcase;
        invalid_size.byte_size = MIN_UPCASE_TABLE_BYTES + 1;
        invalid_size.checksum = checksum32(&payload);

        let invalid_cluster_error = ExfatUpcaseTable::load(&disk, &super_block, &invalid_cluster)
            .unwrap_err();
        let invalid_size_error = ExfatUpcaseTable::load(&disk, &super_block, &invalid_size)
            .unwrap_err();

        assert_eq!(invalid_cluster_error.error(), Errno::EINVAL);
        assert_eq!(invalid_size_error.error(), Errno::EINVAL);
        assert!(validate_upcase_table_size(MIN_UPCASE_TABLE_BYTES - 2).is_err());
    }

    // Confirms the loader rejects a payload that cannot be fully read from the
    // underlying device even when the discovery facts themselves are valid.
    #[ktest]
    fn truncated_payload_is_rejected() {
        let (disk, super_block, mut upcase) = load_context();
        let start_cluster = upcase.start_cluster;
        let byte_size = super_block.cluster_size() * 2;
        let payload = make_payload(super_block.cluster_size());
        let payload_offset = super_block.cluster_to_byte_offset(start_cluster).unwrap();
        let read_limit = payload_offset + super_block.cluster_size();

        write_payload(&disk, payload_offset, &payload);
        upcase.byte_size = byte_size;
        upcase.checksum = checksum32(&payload);
        let truncated_disk = TruncatedReadDisk::new(disk, read_limit);

        let error = ExfatUpcaseTable::load(&truncated_disk, &super_block, &upcase).unwrap_err();

        assert_eq!(error.error(), Errno::EINVAL);
    }

    // Confirms the canonical hash service folds a later table entry before hashing.
    #[ktest]
    fn name_hash_uses_folded_units_from_full_table_surface() {
        let table = table_with_fold(0x0120, 0x0041);
        let logical_name_units = [0x0120];

        assert_eq!(table.name_hash(&logical_name_units), hash_units(&[0x0041]));
        assert_ne!(table.name_hash(&logical_name_units), hash_units(&logical_name_units));
    }
}
