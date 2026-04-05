// SPDX-License-Identifier: MPL-2.0

#![cfg_attr(
    not(ktest),
    expect(
        dead_code,
        reason = "Mount bootstrap is staged before later VFS integration work."
    )
)]

use alloc::sync::Arc;

use aster_block::{
    BlockDevice,
    bio::{BioDirection, BioSegment, BioWaiter},
    id::BlockId,
};
use ostd::mm::Segment;

use super::{
    bitmap::ExfatAllocationBitmap,
    fat::{ChainMode, ExfatChain},
    inode::{DosTimestamp, ExfatInodeKey, ExfatInodeMeta, ExfatRegularFileRuntime},
    read::map_logical_read_offset,
    super_block::ExfatSuperBlock,
    sysroot::ExfatSysRootFacts,
    upcase_table::ExfatUpcaseTable,
};
use crate::fs::vfs::page_cache::{CachePage, PageCache, PageCacheBackend};
use crate::prelude::*;

/// Stores the mount-owned shared filesystem state assembled during bootstrap.
pub(super) struct ExfatFs {
    block_device: Arc<dyn BlockDevice>,
    super_block: ExfatSuperBlock,
    upcase_table: ExfatUpcaseTable,
    allocation_bitmap: ExfatAllocationBitmap,
    root_inode: ExfatInodeMeta,
}

/// Implements page-cache backend I/O for one accepted regular file.
struct ExfatRegularFileBackend {
    fs: Arc<ExfatFs>,
    inode_meta: ExfatInodeMeta,
    page_count: usize,
}

impl ExfatRegularFileBackend {
    /// Creates the backend state bound to one mounted filesystem and regular-file shell.
    fn new(fs: Arc<ExfatFs>, inode_meta: ExfatInodeMeta) -> Result<Self> {
        let page_count = inode_meta.regular_file_page_count()?;

        Ok(Self {
            fs,
            inode_meta,
            page_count,
        })
    }

    /// Returns the on-disk byte offset for the start of one cached page.
    fn page_disk_byte_offset(&self, page_index: usize) -> Result<usize> {
        if page_index >= self.page_count {
            return Err(Error::with_message(
                Errno::EINVAL,
                "page index is beyond the backend-visible range",
            ));
        }

        let page_start_offset = page_index
            .checked_mul(PAGE_SIZE)
            .ok_or_else(|| Error::with_message(Errno::EINVAL, "logical page offset overflow"))?;
        let read_view = self.inode_meta.read_view()?;
        let Some(placement) = map_logical_read_offset(
            self.fs.block_device.as_ref(),
            &self.fs.super_block,
            read_view,
            page_start_offset,
        )?
        else {
            return Err(Error::with_message(
                Errno::EINVAL,
                "page index is beyond the backend-visible range",
            ));
        };
        let cluster_start_offset = self
            .fs
            .super_block
            .cluster_to_byte_offset(placement.cluster)?;

        cluster_start_offset
            .checked_add(placement.byte_offset_in_cluster)
            .ok_or_else(|| Error::with_message(Errno::EINVAL, "physical page offset overflow"))
    }
}

impl PageCacheBackend for ExfatRegularFileBackend {
    fn read_page_async(&self, idx: usize, frame: &CachePage) -> Result<BioWaiter> {
        let page_disk_offset = self.page_disk_byte_offset(idx)?;
        let bio_segment = BioSegment::new_from_segment(
            Segment::from(frame.clone()).into(),
            BioDirection::FromDevice,
        );

        let waiter = self
            .fs
            .block_device
            .read_blocks_async(BlockId::from_offset(page_disk_offset), bio_segment)?;

        Ok(waiter)
    }

    fn write_page_async(&self, idx: usize, frame: &CachePage) -> Result<BioWaiter> {
        let page_disk_offset = self.page_disk_byte_offset(idx)?;
        let bio_segment = BioSegment::new_from_segment(
            Segment::from(frame.clone()).into(),
            BioDirection::ToDevice,
        );

        let waiter = self
            .fs
            .block_device
            .write_blocks_async(BlockId::from_offset(page_disk_offset), bio_segment)?;

        Ok(waiter)
    }

    fn npages(&self) -> usize {
        self.page_count
    }
}

impl ExfatFs {
    /// Mounts a refactored exFAT filesystem from already-validated bootstrap inputs.
    pub(super) fn mount(
        block_device: Arc<dyn BlockDevice>,
        super_block: &ExfatSuperBlock,
        root_facts: ExfatSysRootFacts,
    ) -> Result<Self> {
        let Some(upcase_facts) = root_facts.upcase else {
            return Err(Error::with_message(
                Errno::EINVAL,
                "missing root upcase entry",
            ));
        };
        let Some(bitmap_facts) = root_facts.bitmap else {
            return Err(Error::with_message(
                Errno::EINVAL,
                "missing root bitmap entry",
            ));
        };

        let upcase_table =
            ExfatUpcaseTable::load(block_device.as_ref(), super_block, &upcase_facts)?;
        let allocation_bitmap =
            ExfatAllocationBitmap::load(block_device.as_ref(), super_block, &bitmap_facts)?;
        let root_chain = ExfatChain::new(
            block_device.as_ref(),
            super_block,
            super_block.root_dir,
            None,
            ChainMode::FatBacked,
        )?;
        let root_size = root_chain.byte_len(super_block)?;
        let zero_timestamp = DosTimestamp {
            time: 0,
            date: 0,
            increment_10ms: 0,
            utc_offset: 0,
        };
        let root_inode = ExfatInodeMeta::new_root(
            ExfatInodeKey::root(),
            root_chain,
            root_size,
            root_size,
            zero_timestamp,
            zero_timestamp,
            zero_timestamp,
        )?;

        Ok(Self {
            block_device,
            super_block: *super_block,
            upcase_table,
            allocation_bitmap,
            root_inode,
        })
    }

    /// Attaches regular-file page-cache runtime state for an accepted inode shell.
    pub(super) fn attach_regular_file_runtime(
        self: &Arc<Self>,
        inode_meta: ExfatInodeMeta,
    ) -> Result<ExfatRegularFileRuntime> {
        // Cache capacity follows visible length so the backend-visible range and cache range stay aligned.
        let cache_capacity = inode_meta.regular_file_cache_capacity()?;
        let backend: Arc<dyn PageCacheBackend> =
            Arc::new(ExfatRegularFileBackend::new(self.clone(), inode_meta)?);
        let page_cache = PageCache::with_capacity(cache_capacity, Arc::downgrade(&backend))?;

        Ok(ExfatRegularFileRuntime::new(page_cache, backend))
    }
}

#[cfg(ktest)]
mod tests {
    use alloc::{sync::Arc, vec::Vec};
    use core::sync::atomic::{AtomicUsize, Ordering};

    use ostd::{mm::VmIo, prelude::ktest};

    use super::*;
    use crate::fs::fs_impls::exfat_refactor::{
        bitmap::ExfatAllocationBitmap,
        boot_sector::read_primary_super_block,
        dentry::{ExfatFileDentry, ExfatStreamDentry},
        fat::{ChainMode, ClusterId, ExfatChain},
        fileset::ExfatDentrySet,
        sysroot::{ExfatSysRootFacts, scan_root_system_entries},
        test_support::{ExfatMemoryDisk, load_exfat_disk},
    };

    #[derive(Debug)]
    struct CountingBlockDevice {
        inner: Arc<ExfatMemoryDisk>,
        enqueue_count: AtomicUsize,
    }

    impl CountingBlockDevice {
        fn new(inner: Arc<ExfatMemoryDisk>) -> Self {
            Self {
                inner,
                enqueue_count: AtomicUsize::new(0),
            }
        }

        fn enqueue_count(&self) -> usize {
            self.enqueue_count.load(Ordering::Relaxed)
        }
    }

    impl BlockDevice for CountingBlockDevice {
        fn enqueue(
            &self,
            bio: aster_block::bio::SubmittedBio,
        ) -> core::result::Result<(), aster_block::bio::BioEnqueueError> {
            self.enqueue_count.fetch_add(1, Ordering::Relaxed);
            self.inner.enqueue(bio)
        }

        fn metadata(&self) -> aster_block::BlockDeviceMeta {
            self.inner.metadata()
        }

        fn name(&self) -> &str {
            "counting-exfat-refactor-test-disk"
        }

        fn id(&self) -> device_id::DeviceId {
            self.inner.id()
        }
    }

    fn zero_timestamp() -> DosTimestamp {
        DosTimestamp {
            time: 0,
            date: 0,
            increment_10ms: 0,
            utc_offset: 0,
        }
    }

    fn load_mount_inputs() -> (Arc<ExfatMemoryDisk>, ExfatSuperBlock, ExfatSysRootFacts) {
        let disk = Arc::new(load_exfat_disk());
        let super_block = read_primary_super_block(disk.as_ref()).unwrap();
        let root_chain = ExfatChain::new(
            disk.as_ref(),
            &super_block,
            super_block.root_dir,
            Some(1),
            ChainMode::Contiguous,
        )
        .unwrap();
        let root_facts = scan_root_system_entries(disk.as_ref(), &super_block, root_chain).unwrap();

        (disk, super_block, root_facts)
    }

    fn sample_file_record(
        attribute: u16,
        valid_data_length: u64,
        data_length: u64,
    ) -> ExfatDentrySet {
        ExfatDentrySet::from_trusted_metadata(
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

    fn regular_inode_meta(
        inode_key_offset: usize,
        chain: ExfatChain,
        start_cluster: ClusterId,
        valid_data_length: usize,
        data_length: usize,
    ) -> ExfatInodeMeta {
        let inode_key = ExfatInodeKey::from_cluster_and_offset(start_cluster, inode_key_offset).unwrap();
        let file_record = sample_file_record(
            0x0020,
            u64::try_from(valid_data_length).unwrap(),
            u64::try_from(data_length).unwrap(),
        );

        ExfatInodeMeta::new(inode_key, &file_record, chain).unwrap()
    }

    fn mount_fs_with_disk() -> (Arc<ExfatMemoryDisk>, ExfatSuperBlock, Arc<ExfatFs>) {
        let (disk, super_block, root_facts) = load_mount_inputs();
        let mount_device: Arc<dyn BlockDevice> = disk.clone();
        let fs = Arc::new(ExfatFs::mount(mount_device, &super_block, root_facts).unwrap());

        (disk, super_block, fs)
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
        chain_clusters: &[ClusterId],
    ) {
        for window in chain_clusters.windows(2) {
            write_raw_fat_entry(disk, super_block, window[0], window[1]);
        }
        let tail_cluster = *chain_clusters.last().unwrap();
        write_raw_fat_entry(disk, super_block, tail_cluster, u32::MAX);
    }

    fn first_in_range_page_index(super_block: &ExfatSuperBlock) -> usize {
        super_block.cluster_size().div_ceil(PAGE_SIZE)
    }

    #[ktest]
    fn mount_happy_path_publishes_complete_shared_state() {
        let (disk, super_block, root_facts) = load_mount_inputs();
        let expected_upcase =
            ExfatUpcaseTable::load(disk.as_ref(), &super_block, &root_facts.upcase.unwrap())
                .unwrap();
        let expected_bitmap =
            ExfatAllocationBitmap::load(disk.as_ref(), &super_block, &root_facts.bitmap.unwrap())
                .unwrap();
        let expected_root_chain = ExfatChain::new(
            disk.as_ref(),
            &super_block,
            super_block.root_dir,
            None,
            ChainMode::FatBacked,
        )
        .unwrap();
        let expected_root_size = expected_root_chain.byte_len(&super_block).unwrap();
        let zero_timestamp = zero_timestamp();
        let expected_root = ExfatInodeMeta::new_root(
            ExfatInodeKey::root(),
            expected_root_chain,
            expected_root_size,
            expected_root_size,
            zero_timestamp,
            zero_timestamp,
            zero_timestamp,
        )
        .unwrap();
        let mount_device: Arc<dyn BlockDevice> = disk.clone();

        let mounted = ExfatFs::mount(mount_device, &super_block, root_facts).unwrap();

        assert_eq!(mounted.block_device.name(), "exfat-refactor-test-disk");
        assert_eq!(mounted.super_block.root_dir, super_block.root_dir);
        assert_eq!(mounted.upcase_table, expected_upcase);
        assert_eq!(mounted.allocation_bitmap, expected_bitmap);
        assert_eq!(mounted.root_inode, expected_root);
    }

    #[ktest]
    fn mount_rejects_missing_root_discovery_facts() {
        let (disk, super_block, root_facts) = load_mount_inputs();
        let mount_device: Arc<dyn BlockDevice> = disk.clone();
        let mut missing_bitmap = root_facts;
        missing_bitmap.bitmap = None;

        let missing_bitmap_error =
            match ExfatFs::mount(mount_device.clone(), &super_block, missing_bitmap) {
                Ok(_) => panic!("mount should reject missing bitmap facts"),
                Err(error) => error,
            };

        assert_eq!(missing_bitmap_error.error(), Errno::EINVAL);

        let mut missing_upcase = root_facts;
        missing_upcase.upcase = None;

        let missing_upcase_error = match ExfatFs::mount(mount_device, &super_block, missing_upcase)
        {
            Ok(_) => panic!("mount should reject missing upcase facts"),
            Err(error) => error,
        };

        assert_eq!(missing_upcase_error.error(), Errno::EINVAL);
    }

    #[ktest]
    fn mount_root_seed_uses_synthetic_root_constructor() {
        let (disk, super_block, root_facts) = load_mount_inputs();
        let root_chain = ExfatChain::new(
            disk.as_ref(),
            &super_block,
            super_block.root_dir,
            None,
            ChainMode::FatBacked,
        )
        .unwrap();
        let root_size = root_chain.byte_len(&super_block).unwrap();
        let zero_timestamp = zero_timestamp();
        let expected_root = ExfatInodeMeta::new_root(
            ExfatInodeKey::root(),
            root_chain,
            root_size,
            root_size,
            zero_timestamp,
            zero_timestamp,
            zero_timestamp,
        )
        .unwrap();
        let mount_device: Arc<dyn BlockDevice> = disk;

        let mounted = ExfatFs::mount(mount_device, &super_block, root_facts).unwrap();

        assert_eq!(mounted.root_inode, expected_root);
    }

    #[ktest]
    fn mount_failure_is_atomic_when_loader_rejects_dependency() {
        let (disk, super_block, mut root_facts) = load_mount_inputs();
        let mut upcase = root_facts.upcase.unwrap();
        upcase.checksum ^= 1;
        root_facts.upcase = Some(upcase);
        let mount_device: Arc<dyn BlockDevice> = disk;

        let error = match ExfatFs::mount(mount_device, &super_block, root_facts) {
            Ok(_) => panic!("mount should not publish partial state on loader failure"),
            Err(error) => error,
        };

        assert_eq!(error.error(), Errno::EINVAL);
    }

    #[ktest]
    fn backend_page_count_tracks_visible_length() {
        // Confirms the backend page-count contract follows visible length, not allocated length.
        let (_, super_block, fs) = mount_fs_with_disk();
        let cluster_size = super_block.cluster_size();
        let chain = ExfatChain::new(
            fs.block_device.as_ref(),
            &super_block,
            super_block.root_dir,
            Some(8),
            ChainMode::Contiguous,
        )
        .unwrap();

        let visible_short = PAGE_SIZE + 123;
        let allocated_long = cluster_size * 8;
        let short_inode =
            regular_inode_meta(0x140, chain, super_block.root_dir, visible_short, allocated_long);
        let short_runtime = fs.attach_regular_file_runtime(short_inode).unwrap();
        assert_eq!(short_runtime.backend_page_count(), visible_short.div_ceil(PAGE_SIZE));

        let visible_aligned = PAGE_SIZE * 3;
        let aligned_inode = regular_inode_meta(
            0x180,
            chain,
            super_block.root_dir,
            visible_aligned,
            allocated_long,
        );
        let aligned_runtime = fs.attach_regular_file_runtime(aligned_inode).unwrap();
        assert_eq!(
            aligned_runtime.backend_page_count(),
            visible_aligned.div_ceil(PAGE_SIZE)
        );
    }

    #[ktest]
    fn contiguous_page_read_uses_mapping_boundary() {
        // Confirms contiguous page reads resolve through the EXR-READ-11A mapper.
        let (disk, super_block, fs) = mount_fs_with_disk();
        let page_index = first_in_range_page_index(&super_block);
        let page_start_offset = page_index * PAGE_SIZE;
        let chain = ExfatChain::new(
            disk.as_ref(),
            &super_block,
            super_block.root_dir,
            Some(8),
            ChainMode::Contiguous,
        )
        .unwrap();
        let inode_meta = regular_inode_meta(
            0x1c0,
            chain,
            super_block.root_dir,
            page_start_offset + 1,
            page_start_offset + PAGE_SIZE,
        );
        let backend = ExfatRegularFileBackend::new(fs, inode_meta).unwrap();
        let read_view = backend.inode_meta.read_view().unwrap();
        let expected_placement =
            map_logical_read_offset(disk.as_ref(), &super_block, read_view, page_start_offset)
                .unwrap()
                .unwrap();
        let expected_offset = super_block
            .cluster_to_byte_offset(expected_placement.cluster)
            .unwrap()
            + expected_placement.byte_offset_in_cluster;

        assert_eq!(backend.page_disk_byte_offset(page_index).unwrap(), expected_offset);
    }

    #[ktest]
    fn fat_backed_page_read_uses_mapping_boundary() {
        // Confirms FAT-backed page reads also resolve through EXR-READ-11A mapping facts.
        let (disk, super_block, fs) = mount_fs_with_disk();
        let page_index = first_in_range_page_index(&super_block);
        let page_start_offset = page_index * PAGE_SIZE;
        let cluster_size = super_block.cluster_size();
        let traversed_steps = page_start_offset / cluster_size;
        let cluster_count = traversed_steps + 2;
        let head_cluster = 2;
        let mut chain_clusters = Vec::with_capacity(cluster_count);
        for index in 0..cluster_count {
            let index = u32::try_from(index).unwrap();
            chain_clusters.push(head_cluster + index * 2);
        }
        for &cluster in &chain_clusters {
            assert!(super_block.is_data_cluster_id(cluster));
        }
        write_fat_chain(disk.as_ref(), &super_block, &chain_clusters);
        let chain = ExfatChain::new(
            disk.as_ref(),
            &super_block,
            head_cluster,
            Some(u32::try_from(cluster_count).unwrap()),
            ChainMode::FatBacked,
        )
        .unwrap();
        let inode_meta = regular_inode_meta(
            0x200,
            chain,
            head_cluster,
            page_start_offset + 1,
            page_start_offset + PAGE_SIZE,
        );
        let backend = ExfatRegularFileBackend::new(fs, inode_meta).unwrap();
        let read_view = backend.inode_meta.read_view().unwrap();
        let expected_placement =
            map_logical_read_offset(disk.as_ref(), &super_block, read_view, page_start_offset)
                .unwrap()
                .unwrap();
        let expected_offset = super_block
            .cluster_to_byte_offset(expected_placement.cluster)
            .unwrap()
            + expected_placement.byte_offset_in_cluster;
        let arithmetic_cluster = head_cluster + u32::try_from(traversed_steps).unwrap();

        assert_eq!(
            expected_placement.cluster,
            chain_clusters[traversed_steps]
        );
        assert_ne!(expected_placement.cluster, arithmetic_cluster);
        assert_eq!(backend.page_disk_byte_offset(page_index).unwrap(), expected_offset);
    }

    #[ktest]
    fn out_of_range_pages_stay_zero_backed() {
        // Confirms pages beyond backend range are zero-backed without backend disk I/O.
        let (disk, super_block, root_facts) = load_mount_inputs();
        let counting_device = Arc::new(CountingBlockDevice::new(disk.clone()));
        let mount_device: Arc<dyn BlockDevice> = counting_device.clone();
        let fs = Arc::new(ExfatFs::mount(mount_device, &super_block, root_facts).unwrap());
        let chain = ExfatChain::new(
            disk.as_ref(),
            &super_block,
            super_block.root_dir,
            Some(4),
            ChainMode::Contiguous,
        )
        .unwrap();
        let inode_meta = regular_inode_meta(
            0x240,
            chain,
            super_block.root_dir,
            PAGE_SIZE / 2,
            PAGE_SIZE * 2,
        );
        let runtime = fs.attach_regular_file_runtime(inode_meta).unwrap();
        let out_of_range_index = runtime.backend_page_count();
        let baseline_enqueues = counting_device.enqueue_count();

        runtime
            .page_cache()
            .resize((out_of_range_index + 1) * PAGE_SIZE)
            .unwrap();
        let mut page_data = [0xA5u8; 64];
        runtime
            .page_cache()
            .pages()
            .read_bytes(out_of_range_index * PAGE_SIZE, &mut page_data)
            .unwrap();

        assert!(page_data.iter().all(|&byte| byte == 0));
        assert_eq!(counting_device.enqueue_count(), baseline_enqueues);
    }

    #[ktest]
    fn backend_contract_stays_out_of_buffered_read() {
        // Confirms page-cache backend reads succeed without any buffered `read_at` path.
        let (disk, super_block, fs) = mount_fs_with_disk();
        let chain = ExfatChain::new(
            disk.as_ref(),
            &super_block,
            super_block.root_dir,
            Some(4),
            ChainMode::Contiguous,
        )
        .unwrap();
        let inode_meta = regular_inode_meta(
            0x280,
            chain,
            super_block.root_dir,
            PAGE_SIZE,
            PAGE_SIZE * 4,
        );
        let read_view = inode_meta.read_view().unwrap();
        let placement = map_logical_read_offset(disk.as_ref(), &super_block, read_view, 0)
            .unwrap()
            .unwrap();
        let disk_offset = super_block
            .cluster_to_byte_offset(placement.cluster)
            .unwrap()
            + placement.byte_offset_in_cluster;
        let expected_data = [0x3Cu8; 64];
        disk.write_bytes(disk_offset, &expected_data);
        let runtime = fs.attach_regular_file_runtime(inode_meta).unwrap();
        let mut read_data = [0u8; 64];

        runtime.page_cache().pages().read_bytes(0, &mut read_data).unwrap();

        assert_eq!(runtime.backend_page_count(), 1);
        assert_eq!(read_data, expected_data);
    }
}
