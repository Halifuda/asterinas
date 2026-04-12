// SPDX-License-Identifier: MPL-2.0
#![expect(
    dead_code,
    reason = "Filesystem-owned allocator is staged before later refactor passes consume it."
)]

use alloc::vec::Vec;
use core::convert::TryFrom;

use aster_block::BlockDevice;

use super::{
    bitmap::AllocationBitmap,
    boot_sector::EXFAT_RESERVED_CLUSTERS,
    fat::{ChainMode, FatValue, write_next_fat_value},
    fs::ExfatFs,
    super_block::ExfatSuperBlock,
};
use crate::prelude::*;

/// Carries the committed allocation facts later namespace and write owners need.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(super) struct AllocationResult {
    pub(super) start_cluster: u32,
    pub(super) cluster_count: u32,
    pub(super) chain_mode: ChainMode,
}

#[derive(Debug)]
pub(super) struct Allocator {
    next_search_cluster: u32,
}

#[derive(Debug)]
struct Reservation {
    start_cluster: u32,
    cluster_count: u32,
    chain_mode: ChainMode,
    clusters: Vec<u32>,
}

impl Reservation {
    fn new(start_cluster: u32, cluster_count: u32, chain_mode: ChainMode, clusters: Vec<u32>) -> Self {
        Self {
            start_cluster,
            cluster_count,
            chain_mode,
            clusters,
        }
    }

    fn start_cluster(&self) -> u32 {
        self.start_cluster
    }

    fn cluster_count(&self) -> u32 {
        self.cluster_count
    }

    fn chain_mode(&self) -> ChainMode {
        self.chain_mode
    }

    fn clusters(&self) -> &[u32] {
        &self.clusters
    }

    fn last_cluster(&self) -> u32 {
        *self
            .clusters
            .last()
            .expect("a reservation always contains at least one cluster")
    }

    fn into_result(&self) -> AllocationResult {
        AllocationResult {
            start_cluster: self.start_cluster,
            cluster_count: self.cluster_count,
            chain_mode: self.chain_mode,
        }
    }
}

impl Allocator {
    /// Creates an allocator with the first search cursor.
    pub(super) fn new(search_cluster: u32) -> Self {
        Self {
            next_search_cluster: search_cluster.max(EXFAT_RESERVED_CLUSTERS),
        }
    }

    /// Searches, reserves, and commits one cluster allocation.
    pub(super) fn allocate(
        &mut self,
        fs: &ExfatFs,
        cluster_count: u32,
    ) -> Result<AllocationResult> {
        if cluster_count == 0 {
            return Err(Error::with_message(
                Errno::EINVAL,
                "allocation request must be positive",
            ));
        }

        let (_, super_block) = fs.file_read_context();
        let bitmap_snapshot = self.bitmap_snapshot(fs)?;
        let reservation = self.reserve(&bitmap_snapshot, cluster_count, super_block)?;
        let committed_snapshot = bitmap_snapshot.reserve_clusters(reservation.clusters())?;
        let next_search_cluster = self.next_search_cursor(super_block, reservation.last_cluster());
        let committed_result = reservation.into_result();

        self.commit(fs, &bitmap_snapshot, committed_snapshot, reservation)?;
        self.next_search_cluster = next_search_cluster;

        Ok(committed_result)
    }

    fn bitmap_snapshot(&self, fs: &ExfatFs) -> Result<AllocationBitmap> {
        let allocation_bitmap = fs.allocation_bitmap();
        let Some(bitmap) = allocation_bitmap.as_ref() else {
            return Err(Error::with_message(
                Errno::EINVAL,
                "allocation bitmap has not been installed",
            ));
        };

        Ok(bitmap.clone())
    }

    fn reserve(
        &self,
        bitmap_snapshot: &AllocationBitmap,
        cluster_count: u32,
        super_block: &ExfatSuperBlock,
    ) -> Result<Reservation> {
        let search_start_cluster = self.next_search_cluster;
        if let Some(start_cluster) =
            bitmap_snapshot.find_contiguous_free_run(search_start_cluster, cluster_count)?
        {
            return self.build_contiguous_reservation(start_cluster, cluster_count, super_block);
        }

        let clusters = bitmap_snapshot.collect_free_clusters(search_start_cluster, cluster_count)?;
        Ok(Reservation::new(
            *clusters
                .first()
                .expect("fragmented reservations always contain at least one cluster"),
            cluster_count,
            ChainMode::FatBacked,
            clusters,
        ))
    }

    fn build_contiguous_reservation(
        &self,
        start_cluster: u32,
        cluster_count: u32,
        super_block: &ExfatSuperBlock,
    ) -> Result<Reservation> {
        let end_cluster = start_cluster
            .checked_add(cluster_count)
            .ok_or_else(|| Error::with_message(Errno::EINVAL, "contiguous allocation overflow"))?;
        if !super_block.is_data_cluster_range(start_cluster..end_cluster) {
            return Err(Error::with_message(
                Errno::EINVAL,
                "contiguous allocation is outside the data region",
            ));
        }

        let mut clusters = Vec::with_capacity(
            usize::try_from(cluster_count).expect("validated allocation request should fit in usize"),
        );
        for cluster in start_cluster..end_cluster {
            clusters.push(cluster);
        }

        Ok(Reservation::new(
            start_cluster,
            cluster_count,
            ChainMode::Contiguous,
            clusters,
        ))
    }

    fn commit(
        &self,
        fs: &ExfatFs,
        bitmap_snapshot: &AllocationBitmap,
        committed_snapshot: AllocationBitmap,
        reservation: Reservation,
    ) -> Result<()> {
        let (block_device, super_block) = fs.file_read_context();

        if matches!(reservation.chain_mode(), ChainMode::FatBacked) {
            if let Err(error) =
                self.commit_fat_chain(block_device, super_block, reservation.clusters())
            {
                return Err(error);
            }
        }

        if let Err(error) = committed_snapshot.write_to_disk(block_device, super_block) {
            if matches!(reservation.chain_mode(), ChainMode::FatBacked) {
                let _ = self.rollback_fat_chain(block_device, super_block, reservation.clusters());
            }
            let _ = bitmap_snapshot.write_to_disk(block_device, super_block);
            return Err(error);
        }

        let mut allocation_bitmap = fs.allocation_bitmap();
        *allocation_bitmap = Some(committed_snapshot);

        Ok(())
    }

    fn commit_fat_chain(
        &self,
        block_device: &dyn BlockDevice,
        super_block: &ExfatSuperBlock,
        clusters: &[u32],
    ) -> Result<()> {
        let mut written_clusters = Vec::with_capacity(clusters.len());
        let result = (|| -> Result<()> {
            for window in clusters.windows(2) {
                let current = window[0];
                let next = window[1];
                write_next_fat_value(block_device, super_block, current, FatValue::Next(next))?;
                written_clusters.push(current);
            }

            let Some(&last_cluster) = clusters.last() else {
                return Err(Error::with_message(
                    Errno::EINVAL,
                    "allocation reservation must not be empty",
                ));
            };
            write_next_fat_value(block_device, super_block, last_cluster, FatValue::EndOfChain)?;
            written_clusters.push(last_cluster);

            Ok(())
        })();

        if result.is_err() {
            let _ = self.rollback_fat_chain(block_device, super_block, &written_clusters);
        }

        result
    }

    fn rollback_fat_chain(
        &self,
        block_device: &dyn BlockDevice,
        super_block: &ExfatSuperBlock,
        clusters: &[u32],
    ) -> Result<()> {
        for &cluster in clusters {
            write_next_fat_value(block_device, super_block, cluster, FatValue::Free)?;
        }

        Ok(())
    }

    fn next_search_cursor(
        &self,
        super_block: &ExfatSuperBlock,
        last_cluster: u32,
    ) -> u32 {
        let data_cluster_end_exclusive = super_block.data_cluster_end_exclusive();
        let next_cluster = last_cluster.saturating_add(1);

        if next_cluster >= data_cluster_end_exclusive {
            EXFAT_RESERVED_CLUSTERS
        } else {
            next_cluster
        }
    }
}

#[cfg(ktest)]
mod tests {
    use alloc::{sync::Arc, vec, vec::Vec};
    use core::fmt::Debug;

    use aster_block::{
        BlockDevice, BlockDeviceMeta,
        bio::{BioEnqueueError, BioType, SubmittedBio},
    };
    use device_id::DeviceId;
    use ostd::prelude::ktest;

    use super::*;
    use crate::fs::fs_impls::exfat_refactor::{
        boot_sector::{EXFAT_RESERVED_CLUSTERS, read_primary_super_block},
        dentry::{DENTRY_SIZE, ExfatBitmapDentry, ExfatDentry, RawExfatDentry},
        fat::{ChainMode, ExfatChain, FatValue, read_next_fat_value},
        fs::ExfatFs,
        io::read_metadata_bytes,
        super_block::ExfatSuperBlock,
        test_support::{ExfatMemoryDisk, load_exfat_disk},
    };

    struct RejectingWriteDisk {
        inner: ExfatMemoryDisk,
    }

    impl RejectingWriteDisk {
        fn new(inner: ExfatMemoryDisk) -> Self {
            Self { inner }
        }
    }

    impl Debug for RejectingWriteDisk {
        fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            formatter.write_str("RejectingWriteDisk")
        }
    }

    impl BlockDevice for RejectingWriteDisk {
        fn enqueue(&self, bio: SubmittedBio) -> core::result::Result<(), BioEnqueueError> {
            if matches!(bio.type_(), BioType::Write) {
                return Err(BioEnqueueError::Refused);
            }

            self.inner.enqueue(bio)
        }

        fn metadata(&self) -> BlockDeviceMeta {
            self.inner.metadata()
        }

        fn name(&self) -> &str {
            "rejecting-exfat-refactor-test-disk"
        }

        fn id(&self) -> DeviceId {
            self.inner.id()
        }
    }

    fn root_dir_chain(disk: &dyn BlockDevice, super_block: &ExfatSuperBlock) -> ExfatChain {
        ExfatChain::new(
            disk,
            super_block,
            super_block.root_dir,
            None,
            ChainMode::FatBacked,
        )
        .unwrap()
    }

    fn read_chain_bytes(
        disk: &dyn BlockDevice,
        super_block: &ExfatSuperBlock,
        chain: ExfatChain,
        byte_len: usize,
    ) -> Vec<u8> {
        let mut bytes = vec![0; byte_len];
        let mut loaded_chain = chain;
        let mut copied_bytes = 0usize;
        let cluster_size = super_block.cluster_size();

        for cluster_index in 0..chain.cluster_count() {
            let cluster_offset = loaded_chain
                .physical_cluster_start_offset(super_block)
                .unwrap();
            let remaining_bytes = byte_len - copied_bytes;
            let copy_len = remaining_bytes.min(cluster_size);
            read_metadata_bytes(
                disk,
                cluster_offset,
                &mut bytes[copied_bytes..copied_bytes + copy_len],
            )
            .unwrap();
            copied_bytes += copy_len;

            if cluster_index + 1 < chain.cluster_count() {
                loaded_chain = loaded_chain.walk(disk, super_block, 1).unwrap();
            }
        }

        bytes
    }

    fn write_chain_bytes(
        disk: &ExfatMemoryDisk,
        super_block: &ExfatSuperBlock,
        chain: ExfatChain,
        bytes: &[u8],
    ) {
        let mut loaded_chain = chain;
        let mut copied_bytes = 0usize;
        let cluster_size = super_block.cluster_size();

        for cluster_index in 0..chain.cluster_count() {
            let cluster_offset = loaded_chain
                .physical_cluster_start_offset(super_block)
                .unwrap();
            let remaining_bytes = bytes.len() - copied_bytes;
            let copy_len = remaining_bytes.min(cluster_size);
            disk.write_bytes(cluster_offset, &bytes[copied_bytes..copied_bytes + copy_len]);
            copied_bytes += copy_len;

            if cluster_index + 1 < chain.cluster_count() {
                loaded_chain = loaded_chain.walk(disk, super_block, 1).unwrap();
            }
        }
    }

    fn bitmap_fixture(
        disk: &dyn BlockDevice,
        super_block: &ExfatSuperBlock,
    ) -> (ExfatBitmapDentry, ExfatChain, Vec<u8>) {
        let root_chain = root_dir_chain(disk, super_block);
        let root_chain_cluster_count = root_chain.cluster_count();
        let root_bytes = read_chain_bytes(
            disk,
            super_block,
            root_chain,
            usize::try_from(root_chain_cluster_count).unwrap() * super_block.cluster_size(),
        );

        for chunk in root_bytes.chunks_exact(DENTRY_SIZE) {
            if let ExfatDentry::Bitmap(bitmap_dentry) =
                ExfatDentry::from(RawExfatDentry::from_bytes(chunk))
            {
                let bitmap_chain = ExfatChain::new(
                    disk,
                    super_block,
                    bitmap_dentry.start_cluster,
                    None,
                    ChainMode::FatBacked,
                )
                .unwrap();
                let bitmap_bytes = read_chain_bytes(
                    disk,
                    super_block,
                    bitmap_chain,
                    usize::try_from(bitmap_dentry.size).unwrap(),
                );
                return (bitmap_dentry, bitmap_chain, bitmap_bytes);
            }
        }

        panic!("expected bitmap singleton in root directory")
    }

    fn set_cluster_bit(bitmap_bytes: &mut [u8], cluster: u32, allocated: bool) {
        let bit_index = usize::try_from(cluster - EXFAT_RESERVED_CLUSTERS).unwrap();
        let byte_index = bit_index / 8;
        let bit_offset = (bit_index % 8) as u32;
        let mask = 1u8 << bit_offset;

        if allocated {
            bitmap_bytes[byte_index] |= mask;
        } else {
            bitmap_bytes[byte_index] &= !mask;
        }
    }

    fn mark_all_data_clusters(bitmap_bytes: &mut [u8], super_block: &ExfatSuperBlock, allocated: bool) {
        for cluster in EXFAT_RESERVED_CLUSTERS..super_block.data_cluster_end_exclusive() {
            set_cluster_bit(bitmap_bytes, cluster, allocated);
        }
    }

    fn allocator_fs(
        block_device: Arc<dyn BlockDevice>,
        super_block: ExfatSuperBlock,
        bitmap_dentry: ExfatBitmapDentry,
        bitmap_chain: ExfatChain,
    ) -> ExfatFs {
        let fs = ExfatFs::new(block_device, super_block).unwrap();
        fs.load_allocation_bitmap(bitmap_dentry, bitmap_chain).unwrap();
        fs
    }

    fn original_bitmap_snapshot() -> (
        ExfatMemoryDisk,
        ExfatSuperBlock,
        ExfatBitmapDentry,
        ExfatChain,
        Vec<u8>,
        AllocationBitmap,
    ) {
        let disk = load_exfat_disk();
        let super_block = read_primary_super_block(&disk).unwrap();
        let (bitmap_dentry, bitmap_chain, bitmap_bytes) = bitmap_fixture(&disk, &super_block);
        let snapshot = AllocationBitmap::load(&disk, &super_block, bitmap_dentry, bitmap_chain)
            .unwrap();

        (disk, super_block, bitmap_dentry, bitmap_chain, bitmap_bytes, snapshot)
    }

    // Confirms the allocator prefers a contiguous run when one is available and commits it.
    #[ktest]
    fn allocator_prefers_contiguous_free_run_when_available() {
        let (disk, super_block, bitmap_dentry, bitmap_chain, _bitmap_bytes, original_snapshot) =
            original_bitmap_snapshot();
        let contiguous_start = original_snapshot
            .find_contiguous_free_run(EXFAT_RESERVED_CLUSTERS, 3)
            .unwrap()
            .expect("expected a contiguous free run in the fixture");
        let bitmap_snapshot = original_snapshot;

        let block_device: Arc<dyn BlockDevice> = Arc::new(disk);
        let fs = allocator_fs(block_device, super_block, bitmap_dentry, bitmap_chain);
        let used_before = fs.used_cluster_count().unwrap();
        let mut allocator = Allocator::new(EXFAT_RESERVED_CLUSTERS);

        let reservation = allocator
            .reserve(&bitmap_snapshot, 3, &super_block)
            .unwrap();
        assert_eq!(reservation.chain_mode(), ChainMode::Contiguous);
        assert_eq!(reservation.start_cluster(), contiguous_start);
        assert_eq!(reservation.cluster_count(), 3);

        let committed_snapshot = bitmap_snapshot
            .reserve_clusters(reservation.clusters())
            .unwrap();
        allocator
            .commit(&fs, &bitmap_snapshot, committed_snapshot, reservation)
            .unwrap();

        assert_eq!(fs.used_cluster_count().unwrap(), used_before + 3);
        for cluster in contiguous_start..contiguous_start + 3 {
            assert!(fs.cluster_is_allocated(cluster).unwrap());
        }
    }

    // Confirms fragmented allocation is chosen only when contiguous space cannot satisfy the request.
    #[ktest]
    fn allocator_falls_back_to_fragmented_free_clusters_only_when_needed() {
        let (disk, super_block, bitmap_dentry, bitmap_chain, bitmap_bytes, original_snapshot) =
            original_bitmap_snapshot();
        let free_clusters = original_snapshot
            .collect_free_clusters(EXFAT_RESERVED_CLUSTERS, 5)
            .unwrap();
        let fragmented_clusters = [free_clusters[0], free_clusters[2], free_clusters[4]];
        let mut fragmented_bitmap_bytes = bitmap_bytes;
        mark_all_data_clusters(&mut fragmented_bitmap_bytes, &super_block, true);
        for &cluster in &fragmented_clusters {
            set_cluster_bit(&mut fragmented_bitmap_bytes, cluster, false);
        }
        write_chain_bytes(&disk, &super_block, bitmap_chain, &fragmented_bitmap_bytes);
        let bitmap_snapshot =
            AllocationBitmap::load(&disk, &super_block, bitmap_dentry, bitmap_chain).unwrap();

        let block_device: Arc<dyn BlockDevice> = Arc::new(disk);
        let fs = allocator_fs(block_device, super_block, bitmap_dentry, bitmap_chain);
        let used_before = fs.used_cluster_count().unwrap();
        let mut allocator = Allocator::new(EXFAT_RESERVED_CLUSTERS);

        let reservation = allocator
            .reserve(&bitmap_snapshot, 3, &super_block)
            .unwrap();
        assert_eq!(reservation.chain_mode(), ChainMode::FatBacked);
        assert_eq!(reservation.start_cluster(), fragmented_clusters[0]);
        assert_eq!(reservation.cluster_count(), 3);

        let committed_snapshot = bitmap_snapshot
            .reserve_clusters(reservation.clusters())
            .unwrap();
        allocator
            .commit(&fs, &bitmap_snapshot, committed_snapshot, reservation)
            .unwrap();

        assert_eq!(fs.used_cluster_count().unwrap(), used_before + 3);
        let (block_device, super_block) = fs.file_read_context();
        for (index, cluster) in fragmented_clusters.iter().copied().enumerate() {
            assert!(fs.cluster_is_allocated(cluster).unwrap());
            match read_next_fat_value(block_device, super_block, cluster).unwrap() {
                FatValue::Next(next_cluster) if index + 1 < fragmented_clusters.len() => {
                    assert_eq!(next_cluster, fragmented_clusters[index + 1]);
                }
                FatValue::EndOfChain if index + 1 == fragmented_clusters.len() => {}
                other => panic!("unexpected FAT value at cluster {cluster}: {other:?}"),
            }
        }
    }

    // Confirms a commit failure does not publish the reservation or change the bitmap view.
    #[ktest]
    fn allocator_keeps_reservation_private_until_commit_succeeds() {
        let (disk, super_block, bitmap_dentry, bitmap_chain, bitmap_bytes, original_snapshot) =
            original_bitmap_snapshot();
        let contiguous_start = original_snapshot
            .find_contiguous_free_run(EXFAT_RESERVED_CLUSTERS, 3)
            .unwrap()
            .expect("expected a contiguous free run in the fixture");
        let mut contiguous_bitmap_bytes = bitmap_bytes;
        mark_all_data_clusters(&mut contiguous_bitmap_bytes, &super_block, true);
        for cluster in contiguous_start..contiguous_start + 3 {
            set_cluster_bit(&mut contiguous_bitmap_bytes, cluster, false);
        }
        write_chain_bytes(&disk, &super_block, bitmap_chain, &contiguous_bitmap_bytes);
        let bitmap_snapshot =
            AllocationBitmap::load(&disk, &super_block, bitmap_dentry, bitmap_chain).unwrap();

        let block_device: Arc<dyn BlockDevice> = Arc::new(RejectingWriteDisk::new(disk));
        let fs = allocator_fs(block_device, super_block, bitmap_dentry, bitmap_chain);
        let used_before = fs.used_cluster_count().unwrap();
        let mut allocator = Allocator::new(EXFAT_RESERVED_CLUSTERS);
        let search_cursor_before = allocator.next_search_cluster;

        let reservation = allocator
            .reserve(&bitmap_snapshot, 3, &super_block)
            .unwrap();
        let committed_snapshot = bitmap_snapshot
            .reserve_clusters(reservation.clusters())
            .unwrap();
        let _error = allocator
            .commit(&fs, &bitmap_snapshot, committed_snapshot, reservation)
            .unwrap_err();

        assert_eq!(allocator.next_search_cluster, search_cursor_before);
        assert_eq!(fs.used_cluster_count().unwrap(), used_before);
        for cluster in contiguous_start..contiguous_start + 3 {
            assert!(!fs.cluster_is_allocated(cluster).unwrap());
        }
    }
}
