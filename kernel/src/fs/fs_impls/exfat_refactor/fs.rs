// SPDX-License-Identifier: MPL-2.0
#![expect(
    dead_code,
    reason = "Filesystem owner is staged before mount integration."
)]

use aster_block::BlockDevice;

use super::{boot_sector::BOOT_SIGNATURE, super_block::ExfatSuperBlock};
use crate::{
    fs::vfs::{
        file_system::{FileSystem, FsEventSubscriberStats, SuperBlock},
        inode::Inode,
    },
    prelude::*,
};

const EXFAT_FS_NAME: &str = "exfat";
const EXFAT_NAME_MAX: usize = 255;

pub(super) struct ExfatFs {
    block_device: Arc<dyn BlockDevice>,
    super_block: ExfatSuperBlock,
    vfs_super_block: SuperBlock,
    fs_event_subscriber_stats: FsEventSubscriberStats,
}

impl ExfatFs {
    pub(super) fn new(
        block_device: Arc<dyn BlockDevice>,
        super_block: ExfatSuperBlock,
    ) -> Result<Self> {
        let sector_count = usize::try_from(super_block.num_sectors).map_err(|_| {
            Error::with_message(
                Errno::EINVAL,
                "exFAT sector count does not fit VFS super block",
            )
        })?;
        let mut vfs_super_block = SuperBlock::new(
            u64::from(BOOT_SIGNATURE),
            super_block.sector_size(),
            EXFAT_NAME_MAX,
            block_device.id(),
        );
        vfs_super_block.blocks = sector_count;

        Ok(Self {
            block_device,
            super_block,
            vfs_super_block,
            fs_event_subscriber_stats: FsEventSubscriberStats::new(),
        })
    }
}

impl FileSystem for ExfatFs {
    fn name(&self) -> &'static str {
        EXFAT_FS_NAME
    }

    fn sync(&self) -> Result<()> {
        // Real flush ordering belongs to EXR-SYNC-31.
        Ok(())
    }

    fn root_inode(&self) -> Arc<dyn Inode> {
        // Temporary seam: EXR-FS-OPEN-22 will install the real root inode after EXR-INODE-CORE-17 lands.
        todo!("EXR-FS-OPEN-22 will install the real root inode")
    }

    fn sb(&self) -> SuperBlock {
        self.vfs_super_block.clone()
    }

    fn fs_event_subscriber_stats(&self) -> &FsEventSubscriberStats {
        &self.fs_event_subscriber_stats
    }
}

#[cfg(ktest)]
mod tests {
    use alloc::sync::Arc;

    use aster_block::BlockDevice;
    use ostd::prelude::ktest;

    use super::{ExfatFs, EXFAT_FS_NAME, EXFAT_NAME_MAX};
    use crate::fs::{
        fs_impls::exfat_refactor::{
            boot_sector::{read_primary_super_block, BOOT_SIGNATURE},
            test_support::load_exfat_disk,
        },
        vfs::file_system::{FileSystem, SuperBlock},
    };

    fn new_exfat_fs() -> ExfatFs {
        let block_device: Arc<dyn BlockDevice> = Arc::new(load_exfat_disk());
        let super_block = read_primary_super_block(block_device.as_ref()).unwrap();

        ExfatFs::new(block_device, super_block).unwrap()
    }

    fn assert_same_super_block(left: &SuperBlock, right: &SuperBlock) {
        assert_eq!(left.magic, right.magic);
        assert_eq!(left.bsize, right.bsize);
        assert_eq!(left.blocks, right.blocks);
        assert_eq!(left.bfree, right.bfree);
        assert_eq!(left.bavail, right.bavail);
        assert_eq!(left.files, right.files);
        assert_eq!(left.ffree, right.ffree);
        assert_eq!(left.fsid, right.fsid);
        assert_eq!(left.namelen, right.namelen);
        assert_eq!(left.frsize, right.frsize);
        assert_eq!(left.flags, right.flags);
        assert_eq!(left.container_dev_id, right.container_dev_id);
    }

    #[ktest]
    fn filesystem_identity_and_super_block_snapshot_are_stable() {
        // Confirms the owner exposes one stable VFS identity and reuses the
        // normalized superblock snapshot rather than rebuilding mount state.
        let fs = new_exfat_fs();
        let filesystem: &dyn FileSystem = &fs;
        let first_super_block = filesystem.sb();
        let second_super_block = filesystem.sb();
        let expected_blocks = usize::try_from(fs.super_block.num_sectors).unwrap();

        assert_eq!(filesystem.name(), EXFAT_FS_NAME);
        assert_eq!(first_super_block.magic, u64::from(BOOT_SIGNATURE));
        assert_eq!(first_super_block.bsize, fs.super_block.sector_size());
        assert_eq!(first_super_block.blocks, expected_blocks);
        assert_eq!(first_super_block.namelen, EXFAT_NAME_MAX);
        assert_eq!(first_super_block.frsize, fs.super_block.sector_size());
        assert_eq!(first_super_block.container_dev_id, fs.block_device.id());
        assert_same_super_block(&first_super_block, &second_super_block);
    }

    #[ktest]
    fn subscriber_stats_and_snapshot_survive_placeholder_sync() {
        // Confirms `sync()` is still a no-op placeholder for owner-visible
        // state, and subscriber stats stay attached to this `ExfatFs` instance.
        let fs = new_exfat_fs();
        let filesystem: &dyn FileSystem = &fs;
        let stats_before_sync = filesystem.fs_event_subscriber_stats();
        let super_block_before_sync = filesystem.sb();

        stats_before_sync.add_subscriber();
        filesystem.sync().unwrap();

        let stats_after_sync = filesystem.fs_event_subscriber_stats();
        let super_block_after_sync = filesystem.sb();

        assert!(core::ptr::eq(stats_before_sync, stats_after_sync));
        assert!(stats_after_sync.has_any_subscribers());
        assert_same_super_block(&super_block_before_sync, &super_block_after_sync);

        stats_after_sync.remove_subscriber();
        assert!(!filesystem.fs_event_subscriber_stats().has_any_subscribers());
    }

    #[ktest]
    #[should_panic(expected = "EXR-FS-OPEN-22 will install the real root inode")]
    fn root_inode_temporary_seam_stays_on_file_system_owner() {
        // Confirms the temporary root handoff remains exposed through the
        // `ExfatFs` FileSystem seam until EXR-FS-OPEN-22 replaces it.
        let fs = new_exfat_fs();
        let filesystem: &dyn FileSystem = &fs;

        let _root_inode = filesystem.root_inode();
    }
}
