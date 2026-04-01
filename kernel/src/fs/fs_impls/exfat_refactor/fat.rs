// SPDX-License-Identifier: MPL-2.0
#![cfg_attr(
    not(ktest),
    expect(
        dead_code,
        reason = "Chain helpers are staged before chain integration."
    )
)]

use core::mem::size_of;

use aster_block::BlockDevice;

use super::{io::read_metadata_bytes, super_block::ExfatSuperBlock};
use crate::prelude::*;

pub(super) type ClusterId = u32;

const FAT_ENTRY_SIZE: u64 = size_of::<u32>() as u64;
const FREE_CLUSTER_VALUE: ClusterId = 0;
const BAD_CLUSTER_VALUE: ClusterId = 0xFFFF_FFF7;
const END_OF_CHAIN_VALUE: ClusterId = 0xFFFF_FFFF;

/// Describes how a cluster chain is traversed.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub(super) enum ChainMode {
    Contiguous,
    FatBacked,
}

/// Stores the current chain position and the remaining cluster count.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub(super) struct ExfatChain {
    current: ClusterId,
    cluster_count: u32,
    mode: ChainMode,
}

impl ExfatChain {
    /// Creates a chain state from a known or counted cluster length.
    pub(super) fn new(
        block_device: &dyn BlockDevice,
        super_block: &ExfatSuperBlock,
        current: ClusterId,
        num_clusters: Option<u32>,
        mode: ChainMode,
    ) -> Result<Self> {
        if current == 0 {
            if matches!(num_clusters, Some(cluster_count) if cluster_count != 0) {
                return Err(Error::with_message(
                    Errno::EINVAL,
                    "empty chain must not have a non-zero cluster count",
                ));
            }

            return Ok(Self {
                current,
                cluster_count: 0,
                mode,
            });
        }

        validate_source_cluster(super_block, current)?;

        let cluster_count = match num_clusters {
            Some(0) => {
                return Err(Error::with_message(
                    Errno::EINVAL,
                    "non-empty chain must have a positive cluster count",
                ));
            }
            Some(cluster_count) => {
                if matches!(mode, ChainMode::Contiguous) {
                    validate_contiguous_chain(super_block, current, cluster_count)?;
                }

                cluster_count
            }
            None => {
                if !matches!(mode, ChainMode::FatBacked) {
                    return Err(Error::with_message(
                        Errno::EINVAL,
                        "unknown-length contiguous chains are unsupported",
                    ));
                }

                count_clusters_from_head(block_device, super_block, current)?
            }
        };

        Ok(Self {
            current,
            cluster_count,
            mode,
        })
    }

    /// Returns the current cluster identifier.
    pub(super) fn current_cluster(&self) -> ClusterId {
        self.current
    }

    /// Returns the remaining cluster count, inclusive of the current cluster.
    pub(super) fn cluster_count(&self) -> u32 {
        self.cluster_count
    }

    /// Returns the chain traversal mode.
    pub(super) fn mode(&self) -> ChainMode {
        self.mode
    }

    /// Returns whether the chain contains no clusters.
    pub(super) fn is_empty(&self) -> bool {
        self.cluster_count == 0
    }

    /// Walks the chain by the given number of cluster steps.
    pub(super) fn walk(
        &self,
        block_device: &dyn BlockDevice,
        super_block: &ExfatSuperBlock,
        steps: u32,
    ) -> Result<Self> {
        if steps >= self.cluster_count {
            return Err(Error::with_message(
                Errno::EINVAL,
                "invalid walking steps for exFAT chain",
            ));
        }

        let destination_cluster = match self.mode {
            ChainMode::Contiguous => {
                let destination = self.current.checked_add(steps).ok_or_else(|| {
                    Error::with_message(Errno::EINVAL, "contiguous chain offset overflow")
                })?;
                if !super_block.is_valid_cluster(destination) {
                    return Err(Error::with_message(
                        Errno::EINVAL,
                        "invalid contiguous chain destination cluster",
                    ));
                }

                destination
            }
            ChainMode::FatBacked => {
                let mut destination = self.current;
                for _ in 0..steps {
                    match read_next_fat_value(block_device, super_block, destination)? {
                        FatValue::Next(next_cluster) => destination = next_cluster,
                        FatValue::Free | FatValue::Bad | FatValue::EndOfChain => {
                            return Err(Error::with_message(
                                Errno::EIO,
                                "malformed FAT chain traversal",
                            ));
                        }
                    }
                }

                destination
            }
        };

        Ok(Self {
            current: destination_cluster,
            cluster_count: self.cluster_count - steps,
            mode: self.mode,
        })
    }

    /// Walks to the cluster that contains the requested byte offset.
    pub(super) fn walk_to_cluster_at_offset(
        &self,
        block_device: &dyn BlockDevice,
        super_block: &ExfatSuperBlock,
        offset: usize,
    ) -> Result<(Self, usize)> {
        let cluster_size = super_block.cluster_size();
        let steps = offset / cluster_size;
        let intra_cluster_offset = offset % cluster_size;
        let steps = u32::try_from(steps)
            .map_err(|_| Error::with_message(Errno::EINVAL, "invalid walking steps for chain"))?;
        let chain = self.walk(block_device, super_block, steps)?;

        Ok((chain, intra_cluster_offset))
    }

    /// Returns the first byte offset of the current cluster.
    pub(super) fn physical_cluster_start_offset(
        &self,
        super_block: &ExfatSuperBlock,
    ) -> Result<usize> {
        if self.is_empty() {
            return Err(Error::with_message(
                Errno::EINVAL,
                "empty chain has no physical cluster offset",
            ));
        }

        super_block.cluster_to_byte_offset(self.current)
    }
}

fn validate_contiguous_chain(
    super_block: &ExfatSuperBlock,
    current: ClusterId,
    cluster_count: u32,
) -> Result<()> {
    let end_exclusive = current
        .checked_add(cluster_count)
        .ok_or_else(|| Error::with_message(Errno::EINVAL, "contiguous chain range overflow"))?;
    if super_block.is_cluster_range_valid(current..end_exclusive) {
        Ok(())
    } else {
        Err(Error::with_message(
            Errno::EINVAL,
            "invalid contiguous chain range",
        ))
    }
}

fn count_clusters_from_head(
    block_device: &dyn BlockDevice,
    super_block: &ExfatSuperBlock,
    current: ClusterId,
) -> Result<u32> {
    let mut cluster_count = 1u32;
    let mut cluster = current;

    loop {
        match read_next_fat_value(block_device, super_block, cluster)? {
            FatValue::Next(next_cluster) => {
                if cluster_count == super_block.num_clusters {
                    return Err(Error::with_message(
                        Errno::EIO,
                        "missing terminal EndOfChain marker",
                    ));
                }

                cluster = next_cluster;
                cluster_count = cluster_count.checked_add(1).ok_or_else(|| {
                    Error::with_message(Errno::EIO, "missing terminal EndOfChain marker")
                })?;
            }
            FatValue::EndOfChain => return Ok(cluster_count),
            FatValue::Free | FatValue::Bad => {
                return Err(Error::with_message(
                    Errno::EIO,
                    "malformed FAT chain contents",
                ));
            }
        }
    }
}

/// Describes a decoded FAT entry.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub(super) enum FatValue {
    Free,
    Next(ClusterId),
    Bad,
    EndOfChain,
}

impl From<ClusterId> for FatValue {
    fn from(raw_value: ClusterId) -> Self {
        match raw_value {
            FREE_CLUSTER_VALUE => Self::Free,
            BAD_CLUSTER_VALUE => Self::Bad,
            END_OF_CHAIN_VALUE => Self::EndOfChain,
            _ => Self::Next(raw_value),
        }
    }
}

impl From<FatValue> for ClusterId {
    fn from(value: FatValue) -> Self {
        match value {
            FatValue::Free => FREE_CLUSTER_VALUE,
            FatValue::Next(cluster) => cluster,
            FatValue::Bad => BAD_CLUSTER_VALUE,
            FatValue::EndOfChain => END_OF_CHAIN_VALUE,
        }
    }
}

/// Reads and decodes the FAT entry for a validated cluster from the first FAT.
pub(super) fn read_next_fat_value(
    block_device: &dyn BlockDevice,
    super_block: &ExfatSuperBlock,
    cluster: ClusterId,
) -> Result<FatValue> {
    validate_source_cluster(super_block, cluster)?;

    let offset = fat_entry_byte_offset(super_block, cluster)?;
    let mut raw_bytes = [0u8; size_of::<u32>()];
    read_metadata_bytes(block_device, offset, &mut raw_bytes)?;

    let value = FatValue::from(u32::from_le_bytes(raw_bytes));
    match value {
        FatValue::Next(next_cluster) => {
            validate_next_cluster(super_block, next_cluster)?;
            Ok(FatValue::Next(next_cluster))
        }
        other => Ok(other),
    }
}

fn fat_entry_byte_offset(super_block: &ExfatSuperBlock, cluster: ClusterId) -> Result<usize> {
    let fat_start = super_block
        .fat1_start_sector
        .checked_mul(u64::from(super_block.sector_size))
        .ok_or_else(|| Error::with_message(Errno::EINVAL, "fat entry offset overflow"))?;
    let cluster_offset = u64::from(cluster)
        .checked_mul(FAT_ENTRY_SIZE)
        .ok_or_else(|| Error::with_message(Errno::EINVAL, "fat entry offset overflow"))?;
    let byte_offset = fat_start
        .checked_add(cluster_offset)
        .ok_or_else(|| Error::with_message(Errno::EINVAL, "fat entry offset overflow"))?;

    usize::try_from(byte_offset)
        .map_err(|_| Error::with_message(Errno::EINVAL, "fat entry offset overflow"))
}

fn validate_source_cluster(super_block: &ExfatSuperBlock, cluster: ClusterId) -> Result<()> {
    if super_block.is_valid_cluster(cluster) {
        Ok(())
    } else {
        Err(Error::with_message(
            Errno::EINVAL,
            "invalid data-region cluster",
        ))
    }
}

fn validate_next_cluster(super_block: &ExfatSuperBlock, cluster: ClusterId) -> Result<()> {
    if super_block.is_valid_cluster(cluster) {
        Ok(())
    } else {
        Err(Error::with_message(
            Errno::EINVAL,
            "invalid decoded FAT next-cluster target",
        ))
    }
}

#[cfg(ktest)]
mod tests {
    use ostd::prelude::ktest;

    use super::{read_next_fat_value, ChainMode, ClusterId, ExfatChain, FatValue};
    use crate::fs::fs_impls::exfat_refactor::{
        boot_sector::read_primary_super_block, io::read_metadata_bytes,
        super_block::ExfatSuperBlock, test_support::load_exfat_disk,
    };

    fn read_raw_fat_entry(
        disk: &dyn aster_block::BlockDevice,
        super_block: &ExfatSuperBlock,
        cluster: ClusterId,
    ) -> ClusterId {
        let offset = super_block.fat1_start_sector as usize * super_block.sector_size()
            + cluster as usize * core::mem::size_of::<ClusterId>();
        let mut raw_bytes = [0u8; core::mem::size_of::<ClusterId>()];
        read_metadata_bytes(disk, offset, &mut raw_bytes).unwrap();
        ClusterId::from_le_bytes(raw_bytes)
    }

    fn write_raw_fat_entry(
        disk: &crate::fs::fs_impls::exfat_refactor::test_support::ExfatMemoryDisk,
        super_block: &ExfatSuperBlock,
        cluster: ClusterId,
        raw_value: ClusterId,
    ) {
        let offset = super_block.fat1_start_sector as usize * super_block.sector_size()
            + cluster as usize * core::mem::size_of::<ClusterId>();
        disk.write_bytes(offset, &raw_value.to_le_bytes());
    }

    fn write_fat_chain(
        disk: &crate::fs::fs_impls::exfat_refactor::test_support::ExfatMemoryDisk,
        super_block: &ExfatSuperBlock,
        start: ClusterId,
        next: ClusterId,
    ) {
        write_raw_fat_entry(disk, super_block, start, next);
        write_raw_fat_entry(disk, super_block, next, u32::MAX);
    }

    #[ktest]
    fn fat_value_preserves_special_markers_and_next_clusters() {
        // Confirms the raw decoder and reverse conversion keep the special FAT
        // markers distinct from ordinary successor cluster values.
        let next_cluster = 7;

        assert_eq!(FatValue::from(0), FatValue::Free);
        assert_eq!(FatValue::from(0xFFFF_FFF7), FatValue::Bad);
        assert_eq!(FatValue::from(0xFFFF_FFFF), FatValue::EndOfChain);
        assert_eq!(FatValue::from(next_cluster), FatValue::Next(next_cluster));
        assert_eq!(u32::from(FatValue::Free), 0);
        assert_eq!(u32::from(FatValue::Bad), 0xFFFF_FFF7);
        assert_eq!(u32::from(FatValue::EndOfChain), 0xFFFF_FFFF);
        assert_eq!(u32::from(FatValue::Next(next_cluster)), next_cluster);
    }

    #[ktest]
    fn read_next_fat_value_decodes_embedded_image_entry() {
        // Confirms the helper reads the on-disk FAT entry from the embedded
        // exFAT image and decodes it the same way as a direct raw-byte read.
        let disk = load_exfat_disk();
        let super_block = read_primary_super_block(&disk).unwrap();
        let cluster = super_block.root_dir;
        let expected = FatValue::from(read_raw_fat_entry(&disk, &super_block, cluster));

        assert_eq!(
            read_next_fat_value(&disk, &super_block, cluster).unwrap(),
            expected
        );
    }

    #[ktest]
    fn read_next_fat_value_rejects_invalid_source_cluster() {
        // Confirms reserved cluster identifiers fail before the helper reaches
        // the block-device read stage.
        let disk = load_exfat_disk();
        let super_block = read_primary_super_block(&disk).unwrap();

        assert!(read_next_fat_value(&disk, &super_block, 1).is_err());
    }

    #[ktest]
    fn read_next_fat_value_rejects_invalid_next_cluster_target() {
        // Confirms the helper rejects a decoded next-cluster value that points
        // outside the valid data-region cluster range.
        let disk = load_exfat_disk();
        let super_block = read_primary_super_block(&disk).unwrap();
        let cluster = super_block.root_dir;

        write_raw_fat_entry(&disk, &super_block, cluster, 1);

        assert!(read_next_fat_value(&disk, &super_block, cluster).is_err());
    }

    #[ktest]
    fn exfat_chain_accepts_empty_chain_without_fat_reads() {
        // Confirms empty chains are represented explicitly and do not need FAT
        // traversal to become observable.
        let disk = load_exfat_disk();
        let super_block = read_primary_super_block(&disk).unwrap();

        let chain = ExfatChain::new(&disk, &super_block, 0, Some(0), ChainMode::FatBacked).unwrap();

        assert!(chain.is_empty());
        assert_eq!(chain.current_cluster(), 0);
        assert_eq!(chain.cluster_count(), 0);
        assert_eq!(chain.mode(), ChainMode::FatBacked);
        assert!(chain.physical_cluster_start_offset(&super_block).is_err());
    }

    #[ktest]
    fn exfat_chain_walks_contiguous_chain_and_reports_offsets() {
        // Confirms contiguous traversal uses arithmetic only and preserves the
        // intra-cluster byte offset when mapping a byte position.
        let disk = load_exfat_disk();
        let super_block = read_primary_super_block(&disk).unwrap();
        let start_cluster = super_block.root_dir;
        let next_cluster = start_cluster + 1;
        let chain = ExfatChain::new(
            &disk,
            &super_block,
            start_cluster,
            Some(2),
            ChainMode::Contiguous,
        )
        .unwrap();

        let walked = chain.walk(&disk, &super_block, 1).unwrap();
        let (offset_chain, offset_in_cluster) = chain
            .walk_to_cluster_at_offset(&disk, &super_block, super_block.cluster_size() + 13)
            .unwrap();

        assert_eq!(chain.current_cluster(), start_cluster);
        assert_eq!(chain.cluster_count(), 2);
        assert_eq!(chain.mode(), ChainMode::Contiguous);
        assert_eq!(
            chain.physical_cluster_start_offset(&super_block).unwrap(),
            super_block.cluster_to_byte_offset(start_cluster).unwrap()
        );
        assert_eq!(walked.current_cluster(), next_cluster);
        assert_eq!(walked.cluster_count(), 1);
        assert_eq!(offset_chain.current_cluster(), next_cluster);
        assert_eq!(offset_in_cluster, 13);
    }

    #[ktest]
    fn exfat_chain_counts_and_walks_unknown_length_fat_chain() {
        // Confirms FAT-backed chains can be counted from the head and walked
        // after the count is inferred from the on-disk FAT entries.
        let disk = load_exfat_disk();
        let super_block = read_primary_super_block(&disk).unwrap();
        let start_cluster = super_block.root_dir;
        let next_cluster = start_cluster + 1;

        write_fat_chain(&disk, &super_block, start_cluster, next_cluster);

        let chain = ExfatChain::new(
            &disk,
            &super_block,
            start_cluster,
            None,
            ChainMode::FatBacked,
        )
        .unwrap();
        let walked = chain.walk(&disk, &super_block, 1).unwrap();
        let (offset_chain, offset_in_cluster) = chain
            .walk_to_cluster_at_offset(&disk, &super_block, super_block.cluster_size() + 7)
            .unwrap();

        assert_eq!(chain.current_cluster(), start_cluster);
        assert_eq!(chain.cluster_count(), 2);
        assert_eq!(chain.mode(), ChainMode::FatBacked);
        assert_eq!(walked.current_cluster(), next_cluster);
        assert_eq!(walked.cluster_count(), 1);
        assert_eq!(offset_chain.current_cluster(), next_cluster);
        assert_eq!(offset_in_cluster, 7);
    }

    #[ktest]
    fn exfat_chain_rejects_invalid_step_counts() {
        // Confirms walking past the end of the chain is rejected before any
        // caller can observe a wrapped or truncated destination.
        let disk = load_exfat_disk();
        let super_block = read_primary_super_block(&disk).unwrap();
        let start_cluster = super_block.root_dir;
        let chain = ExfatChain::new(
            &disk,
            &super_block,
            start_cluster,
            Some(1),
            ChainMode::Contiguous,
        )
        .unwrap();

        assert!(chain.walk(&disk, &super_block, 1).is_err());
        assert!(chain
            .walk_to_cluster_at_offset(&disk, &super_block, super_block.cluster_size())
            .is_err());
    }
}
