// SPDX-License-Identifier: MPL-2.0

#![cfg_attr(
    not(ktest),
    expect(
        dead_code,
        reason = "Read mapping lands before buffered read and page-cache integration."
    )
)]

use aster_block::BlockDevice;

use super::{
    fat::{ClusterId, ExfatChain},
    super_block::ExfatSuperBlock,
};
use crate::prelude::*;

/// Carries the physical placement facts for one logical read offset.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct ExfatReadPlacement {
    pub(super) cluster: ClusterId,
    pub(super) byte_offset_in_cluster: usize,
}

/// Carries the immutable inode facts needed by the read mapper only.
#[derive(Clone, Copy, Debug)]
pub(super) struct ExfatInodeReadView<'a> {
    chain: &'a ExfatChain,
    valid_data_length: usize,
}

impl<'a> ExfatInodeReadView<'a> {
    /// Creates a read-view wrapper around the accepted inode read facts.
    pub(super) fn new(chain: &'a ExfatChain, valid_data_length: usize) -> Self {
        Self {
            chain,
            valid_data_length,
        }
    }
}

/// Maps a logical read offset for an accepted regular-file read view to physical placement.
pub(super) fn map_logical_read_offset(
    block_device: &dyn BlockDevice,
    super_block: &ExfatSuperBlock,
    inode_read_view: ExfatInodeReadView<'_>,
    offset: usize,
) -> Result<Option<ExfatReadPlacement>> {
    if offset >= inode_read_view.valid_data_length {
        return Ok(None);
    }

    let (cluster, byte_offset_in_cluster) =
        inode_read_view
            .chain
            .walk_to_cluster_at_offset(block_device, super_block, offset)?;

    Ok(Some(ExfatReadPlacement {
        cluster,
        byte_offset_in_cluster,
    }))
}

#[cfg(ktest)]
mod tests {
    use aster_block::{
        BlockDevice, BlockDeviceMeta,
        bio::{BioEnqueueError, SubmittedBio},
    };
    use device_id::DeviceId;
    use ostd::prelude::ktest;

    use super::*;
    use crate::fs::fs_impls::exfat_refactor::{
        boot_sector::read_primary_super_block,
        dentry::{ExfatFileDentry, ExfatStreamDentry},
        fat::{ChainMode, ExfatChain},
        inode::{DosTimestamp, ExfatInodeKey, ExfatInodeMeta},
        test_support::{ExfatMemoryDisk, load_exfat_disk},
    };

    #[derive(Debug)]
    struct RejectReadBlockDevice;

    impl BlockDevice for RejectReadBlockDevice {
        fn enqueue(&self, _bio: SubmittedBio) -> core::result::Result<(), BioEnqueueError> {
            panic!("contiguous read mapping must not issue block-device I/O");
        }

        fn metadata(&self) -> BlockDeviceMeta {
            BlockDeviceMeta {
                max_nr_segments_per_bio: usize::MAX,
                nr_sectors: usize::MAX,
            }
        }

        fn name(&self) -> &str {
            "reject-read-device"
        }

        fn id(&self) -> DeviceId {
            DeviceId::null()
        }
    }

    fn sample_file_record(
        attribute: u16,
        valid_data_length: u64,
        data_length: u64,
    ) -> super::super::fileset::ExfatDentrySet {
        super::super::fileset::ExfatDentrySet::from_trusted_metadata(
            ExfatFileDentry {
                dentry_type: 0x85,
                num_secondary: 0,
                checksum: 0,
                attribute,
                reserved1: 0,
                create_time: 0x1234,
                create_date: 0x5678,
                modify_time: 0x9abc,
                modify_date: 0xdef0,
                access_time: 0x1357,
                access_date: 0x2468,
                create_time_cs: 0x2a,
                modify_time_cs: 0x33,
                create_utc_offset: 0x44,
                modify_utc_offset: 0x55,
                access_utc_offset: 0x66,
                reserved2: [0; 7],
            },
            ExfatStreamDentry {
                dentry_type: 0xC0,
                flags: 0,
                reserved1: 0,
                name_len: 0,
                name_hash: 0,
                reserved2: 0,
                valid_size: valid_data_length,
                reserved3: 0,
                start_cluster: 2,
                size: data_length,
            },
            &[0x0041, 0x0042, 0x0043],
            vec![],
        )
        .unwrap()
    }

    fn regular_inode(
        inode_key: ExfatInodeKey,
        chain: ExfatChain,
        valid_data_length: usize,
        data_length: usize,
    ) -> ExfatInodeMeta {
        ExfatInodeMeta::new(
            inode_key,
            &sample_file_record(
                0x0020,
                valid_data_length as u64,
                data_length as u64,
            ),
            chain,
        )
        .unwrap()
    }

    fn write_raw_fat_entry(
        disk: &ExfatMemoryDisk,
        super_block: &ExfatSuperBlock,
        cluster: ClusterId,
        raw_value: ClusterId,
    ) {
        let offset = super_block.fat1_start_sector as usize * super_block.sector_size()
            + cluster as usize * core::mem::size_of::<ClusterId>();
        disk.write_bytes(offset, &raw_value.to_le_bytes());
    }

    fn write_fat_chain(
        disk: &ExfatMemoryDisk,
        super_block: &ExfatSuperBlock,
        head_cluster: ClusterId,
        tail_clusters: &[ClusterId],
    ) {
        let mut current_cluster = head_cluster;
        for &next_cluster in tail_clusters {
            write_raw_fat_entry(disk, super_block, current_cluster, next_cluster);
            current_cluster = next_cluster;
        }
        write_raw_fat_entry(disk, super_block, current_cluster, u32::MAX);
    }

    fn zero_timestamp() -> DosTimestamp {
        DosTimestamp {
            time: 0,
            date: 0,
            increment_10ms: 0,
            utc_offset: 0,
        }
    }

    #[ktest]
    fn contiguous_offset_maps_without_fat_reads() {
        // Confirms contiguous placement is arithmetic-only and does not touch the FAT path.
        let disk = load_exfat_disk();
        let super_block = read_primary_super_block(&disk).unwrap();
        let cluster_size = super_block.cluster_size();
        let head_cluster = super_block.root_dir;
        let chain = ExfatChain::new(
            &RejectReadBlockDevice,
            &super_block,
            head_cluster,
            Some(3),
            ChainMode::Contiguous,
        )
        .unwrap();
        let inode = regular_inode(
            ExfatInodeKey::from_cluster_and_offset(head_cluster, 0x40).unwrap(),
            chain,
            cluster_size * 3,
            cluster_size * 3,
        );

        let placement = map_logical_read_offset(
            &RejectReadBlockDevice,
            &super_block,
            inode.read_view().unwrap(),
            cluster_size + 13,
        )
        .unwrap()
        .unwrap();

        assert_eq!(
            placement,
            ExfatReadPlacement {
                cluster: head_cluster + 1,
                byte_offset_in_cluster: 13,
            }
        );
    }

    #[ktest]
    fn fat_backed_offset_maps_through_chain() {
        // Confirms FAT-backed placement walks the accepted chain instead of using arithmetic.
        let disk = load_exfat_disk();
        let super_block = read_primary_super_block(&disk).unwrap();
        let cluster_size = super_block.cluster_size();
        let head_cluster = super_block.root_dir;
        let middle_cluster = head_cluster + 1;
        let tail_cluster = head_cluster + 2;
        write_fat_chain(&disk, &super_block, head_cluster, &[middle_cluster, tail_cluster]);
        let chain = ExfatChain::new(
            &disk,
            &super_block,
            head_cluster,
            Some(3),
            ChainMode::FatBacked,
        )
        .unwrap();
        let inode = regular_inode(
            ExfatInodeKey::from_cluster_and_offset(head_cluster, 0x80).unwrap(),
            chain,
            cluster_size * 3,
            cluster_size * 3,
        );

        let placement = map_logical_read_offset(
            &disk,
            &super_block,
            inode.read_view().unwrap(),
            cluster_size * 2 + 17,
        )
        .unwrap()
        .unwrap();

        assert_eq!(
            placement,
            ExfatReadPlacement {
                cluster: tail_cluster,
                byte_offset_in_cluster: 17,
            }
        );
    }

    #[ktest]
    fn offset_at_valid_data_end_returns_none() {
        // Confirms the mapper uses valid-data length rather than allocated size as the EOF limit.
        let disk = load_exfat_disk();
        let super_block = read_primary_super_block(&disk).unwrap();
        let cluster_size = super_block.cluster_size();
        let head_cluster = super_block.root_dir;
        let chain = ExfatChain::new(
            &disk,
            &super_block,
            head_cluster,
            Some(3),
            ChainMode::Contiguous,
        )
        .unwrap();
        let inode = regular_inode(
            ExfatInodeKey::from_cluster_and_offset(head_cluster, 0xC0).unwrap(),
            chain,
            cluster_size,
            cluster_size * 3,
        );

        assert_eq!(
            map_logical_read_offset(&disk, &super_block, inode.read_view().unwrap(), cluster_size)
                .unwrap(),
            None
        );
        assert_eq!(
            map_logical_read_offset(
                &disk,
                &super_block,
                inode.read_view().unwrap(),
                cluster_size + 1,
            )
            .unwrap(),
            None
        );
    }

    #[ktest]
    fn non_regular_file_is_rejected() {
        // Confirms directory shells fail before the mapper publishes any placement result.
        let disk = load_exfat_disk();
        let super_block = read_primary_super_block(&disk).unwrap();
        let cluster_size = super_block.cluster_size();
        let head_cluster = super_block.root_dir;
        let chain = ExfatChain::new(
            &disk,
            &super_block,
            head_cluster,
            Some(1),
            ChainMode::Contiguous,
        )
        .unwrap();
        let timestamp = zero_timestamp();
        let directory_inode = ExfatInodeMeta::new_root(
            ExfatInodeKey::root(),
            chain,
            cluster_size,
            cluster_size,
            timestamp,
            timestamp,
            timestamp,
        )
        .unwrap();

        let error = directory_inode
            .read_view()
            .expect_err("directory shells must not cross the read-mapping boundary");

        assert_eq!(error.error(), Errno::EISDIR);
    }
}
