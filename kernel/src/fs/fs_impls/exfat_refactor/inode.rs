// SPDX-License-Identifier: MPL-2.0

#![cfg_attr(
    not(ktest),
    expect(
        dead_code,
        reason = "Inode carrier is staged before open, cache, and data-path integration."
    )
)]

use alloc::sync::{Arc, Weak};
use core::time::Duration;

use super::{
    fat::{ChainMode, ClusterId, ExfatChain},
    fileset::ExfatDentrySet,
    fs::ExfatFs,
};
use crate::{
    fs::{
        file::{InodeMode, InodeType, StatusFlags},
        vfs::{
            file_system::FileSystem,
            inode::{Extension, Inode, InodeIo, Metadata},
        },
    },
    prelude::*,
    process::{Gid, Uid},
};

const SECTOR_SIZE: usize = 512;

/// Carries the VFS-visible exFAT inode metadata snapshot.
pub(super) struct ExfatInode {
    fs: Weak<ExfatFs>,
    metadata: Metadata,
    extension: Extension,
    location: Option<ExfatInodeLocation>,
    file_attribute: u16,
    valid_size: usize,
    start_cluster: ClusterId,
    cluster_count: u32,
    chain_mode: ChainMode,
    allocated_size: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct ExfatInodeLocation {
    parent_ino: Option<u64>,
    dentry_set_byte_offset: usize,
    dentry_entry_index: u32,
}

impl ExfatInodeLocation {
    /// Creates an owner-private location snapshot for later persistence work.
    pub(super) fn new(
        parent_ino: Option<u64>,
        dentry_set_byte_offset: usize,
        dentry_entry_index: u32,
    ) -> Self {
        Self {
            parent_ino,
            dentry_set_byte_offset,
            dentry_entry_index,
        }
    }
}

impl ExfatInode {
    /// Creates an inode carrier from trusted dentry-set, chain, and metadata facts.
    pub(super) fn new(
        fs: Weak<ExfatFs>,
        mut metadata: Metadata,
        dentry_set: &ExfatDentrySet,
        chain: &ExfatChain,
        cluster_size: usize,
        location: Option<ExfatInodeLocation>,
    ) -> Result<Arc<Self>> {
        if cluster_size == 0 {
            return Err(Error::with_message(
                Errno::EINVAL,
                "exFAT inode cluster size must be non-zero",
            ));
        }

        let file_dentry = dentry_set.file_dentry();
        let stream_dentry = dentry_set.stream_dentry();
        let size = usize::try_from(stream_dentry.size)
            .map_err(|_| Error::with_message(Errno::EOVERFLOW, "exFAT inode size overflow"))?;
        let valid_size = usize::try_from(stream_dentry.valid_size).map_err(|_| {
            Error::with_message(Errno::EOVERFLOW, "exFAT inode valid size overflow")
        })?;
        let allocated_size = allocated_size(chain.cluster_count(), cluster_size)?;

        metadata.size = size;
        metadata.optimal_block_size = cluster_size;
        metadata.nr_sectors_allocated = allocated_size.div_ceil(SECTOR_SIZE);

        Ok(Arc::new(Self {
            fs,
            metadata,
            extension: Extension::new(),
            location,
            file_attribute: file_dentry.attribute,
            valid_size,
            start_cluster: chain.current_cluster(),
            cluster_count: chain.cluster_count(),
            chain_mode: chain.mode(),
            allocated_size,
        }))
    }
}

impl InodeIo for ExfatInode {
    fn read_at(
        &self,
        _offset: usize,
        _writer: &mut VmWriter,
        _status_flags: StatusFlags,
    ) -> Result<usize> {
        // Temporary seam: EXR-READ-OPS-25, EXR-WRITE-30, and EXR-PGCACHE-26 will own this path.
        Err(Error::with_message(
            Errno::EOPNOTSUPP,
            "exFAT inode read path is not implemented yet",
        ))
    }

    fn write_at(
        &self,
        _offset: usize,
        _reader: &mut VmReader,
        _status_flags: StatusFlags,
    ) -> Result<usize> {
        // Temporary seam: EXR-READ-OPS-25, EXR-WRITE-30, and EXR-PGCACHE-26 will own this path.
        Err(Error::with_message(
            Errno::EOPNOTSUPP,
            "exFAT inode write path is not implemented yet",
        ))
    }
}

impl Inode for ExfatInode {
    fn size(&self) -> usize {
        self.metadata.size
    }

    fn resize(&self, _new_size: usize) -> Result<()> {
        Err(Error::with_message(
            Errno::EOPNOTSUPP,
            "exFAT inode resize is deferred to write-side ownership",
        ))
    }

    fn metadata(&self) -> Metadata {
        self.metadata
    }

    fn ino(&self) -> u64 {
        self.metadata.ino
    }

    fn type_(&self) -> InodeType {
        self.metadata.type_
    }

    fn mode(&self) -> Result<InodeMode> {
        Ok(self.metadata.mode)
    }

    fn set_mode(&self, _mode: InodeMode) -> Result<()> {
        Err(Error::with_message(
            Errno::EOPNOTSUPP,
            "exFAT inode mode updates are deferred to write-side ownership",
        ))
    }

    fn owner(&self) -> Result<Uid> {
        Ok(self.metadata.uid)
    }

    fn set_owner(&self, _uid: Uid) -> Result<()> {
        Err(Error::with_message(
            Errno::EOPNOTSUPP,
            "exFAT inode owner updates are deferred to write-side ownership",
        ))
    }

    fn group(&self) -> Result<Gid> {
        Ok(self.metadata.gid)
    }

    fn set_group(&self, _gid: Gid) -> Result<()> {
        Err(Error::with_message(
            Errno::EOPNOTSUPP,
            "exFAT inode group updates are deferred to write-side ownership",
        ))
    }

    fn atime(&self) -> Duration {
        self.metadata.last_access_at
    }

    fn set_atime(&self, _time: Duration) {
        // Temporary seam: EXR-WRITE-30 and EXR-SYNC-31 will own timestamp persistence.
    }

    fn mtime(&self) -> Duration {
        self.metadata.last_modify_at
    }

    fn set_mtime(&self, _time: Duration) {
        // Temporary seam: EXR-WRITE-30 and EXR-SYNC-31 will own timestamp persistence.
    }

    fn ctime(&self) -> Duration {
        self.metadata.last_meta_change_at
    }

    fn set_ctime(&self, _time: Duration) {
        // Temporary seam: EXR-WRITE-30 and EXR-SYNC-31 will own timestamp persistence.
    }

    fn fs(&self) -> Arc<dyn FileSystem> {
        let fs: Arc<dyn FileSystem> = self
            .fs
            .upgrade()
            .expect("exFAT inode must not outlive its filesystem owner");
        fs
    }

    fn extension(&self) -> &Extension {
        &self.extension
    }
}

fn allocated_size(cluster_count: u32, cluster_size: usize) -> Result<usize> {
    cluster_size
        .checked_mul(cluster_count as usize)
        .ok_or_else(|| Error::with_message(Errno::EOVERFLOW, "exFAT inode allocation overflow"))
}

#[cfg(ktest)]
mod tests {
    use aster_block::BlockDevice;
    use ostd::prelude::ktest;

    use super::*;
    use crate::fs::fs_impls::exfat_refactor::{
        boot_sector::read_primary_super_block,
        dentry::{ExfatFileDentry, ExfatStreamDentry},
        test_support::load_exfat_disk,
    };

    fn assert_eopnotsupp<T>(result: Result<T>) {
        match result {
            Ok(_) => panic!("temporary inode seam should reject"),
            Err(error) => assert_eq!(error.error(), Errno::EOPNOTSUPP),
        }
    }

    fn trusted_dentry_set(
        file_size: u64,
        valid_size: u64,
        file_attribute: u16,
        start_cluster: ClusterId,
    ) -> ExfatDentrySet {
        let mut file_dentry = ExfatFileDentry::default();
        file_dentry.attribute = file_attribute;

        let mut stream_dentry = ExfatStreamDentry::default();
        stream_dentry.valid_size = valid_size;
        stream_dentry.start_cluster = start_cluster;
        stream_dentry.size = file_size;

        ExfatDentrySet::from_trusted_metadata(
            file_dentry,
            stream_dentry,
            &[b'i' as u16, b'n' as u16, b'o' as u16],
            Vec::new(),
        )
        .expect("trusted inode dentry set should validate")
    }

    // Confirms copied metadata, weak FS owner recovery, and staged seam rejections.
    #[ktest]
    fn inode_carrier_snapshots_metadata_and_rejects_temporary_seams() {
        let disk = Arc::new(load_exfat_disk());
        let super_block = read_primary_super_block(disk.as_ref()).unwrap();
        let chain = ExfatChain::new(
            disk.as_ref(),
            &super_block,
            super_block.root_dir,
            Some(1),
            ChainMode::Contiguous,
        )
        .unwrap();
        let container_dev_id = disk.id();
        let fs = Arc::new(ExfatFs::new(disk, super_block).unwrap());

        let file_size = 1234u64;
        let valid_size = 1200u64;
        let file_attribute = 0x20u16;
        let cluster_size = super_block.cluster_size();
        let mut dentry_set = trusted_dentry_set(
            file_size,
            valid_size,
            file_attribute,
            chain.current_cluster(),
        );

        let mode = InodeMode::S_IRUSR | InodeMode::S_IWUSR | InodeMode::S_IRGRP;
        let uid = Uid::new(1000);
        let gid = Gid::new(1001);
        let atime = Duration::from_secs(10);
        let mtime = Duration::from_secs(20);
        let ctime = Duration::from_secs(30);
        let metadata = Metadata {
            ino: 42,
            size: 0,
            optimal_block_size: SECTOR_SIZE,
            nr_sectors_allocated: 0,
            last_access_at: atime,
            last_modify_at: mtime,
            last_meta_change_at: ctime,
            type_: InodeType::File,
            mode,
            nr_hard_links: 1,
            uid,
            gid,
            container_dev_id,
            self_dev_id: None,
        };

        let location = ExfatInodeLocation::new(Some(7), 4096, 3);
        let inode = ExfatInode::new(
            Arc::downgrade(&fs),
            metadata,
            &dentry_set,
            &chain,
            cluster_size,
            Some(location),
        )
        .unwrap();

        let mut changed_stream = dentry_set.stream_dentry();
        changed_stream.valid_size = 0;
        changed_stream.size = 0;
        dentry_set.set_stream_dentry(changed_stream);

        assert_eq!(Arc::strong_count(&fs), 1);
        assert_eq!(inode.ino(), 42);
        assert_eq!(inode.size(), file_size as usize);
        assert_eq!(inode.type_(), InodeType::File);
        assert_eq!(inode.mode().unwrap(), mode);
        assert_eq!(inode.owner().unwrap(), uid);
        assert_eq!(inode.group().unwrap(), gid);
        assert_eq!(inode.atime(), atime);
        assert_eq!(inode.mtime(), mtime);
        assert_eq!(inode.ctime(), ctime);

        let metadata = inode.metadata();
        assert_eq!(metadata.ino, inode.ino());
        assert_eq!(metadata.size, inode.size());
        assert_eq!(metadata.type_, inode.type_());
        assert_eq!(metadata.mode, inode.mode().unwrap());
        assert_eq!(metadata.uid, inode.owner().unwrap());
        assert_eq!(metadata.gid, inode.group().unwrap());
        assert_eq!(metadata.last_access_at, inode.atime());
        assert_eq!(metadata.last_modify_at, inode.mtime());
        assert_eq!(metadata.last_meta_change_at, inode.ctime());
        assert_eq!(metadata.optimal_block_size, cluster_size);
        assert_eq!(
            metadata.nr_sectors_allocated,
            cluster_size.div_ceil(SECTOR_SIZE)
        );

        assert_eq!(inode.location, Some(location));
        assert_eq!(inode.file_attribute, file_attribute);
        assert_eq!(inode.valid_size, valid_size as usize);
        assert_eq!(inode.start_cluster, chain.current_cluster());
        assert_eq!(inode.cluster_count, chain.cluster_count());
        assert_eq!(inode.chain_mode, ChainMode::Contiguous);
        assert_eq!(inode.allocated_size, cluster_size);

        let upgraded_fs = inode.fs();
        let expected_fs: Arc<dyn FileSystem> = fs.clone();
        assert!(Arc::ptr_eq(&upgraded_fs, &expected_fs));

        let mut read_buffer = [0u8; 4];
        let mut read_writer = VmWriter::from(read_buffer.as_mut_slice()).to_fallible();
        assert_eopnotsupp(inode.read_at(0, &mut read_writer, StatusFlags::empty()));

        let write_buffer = [1u8; 4];
        let mut write_reader = VmReader::from(write_buffer.as_slice()).to_fallible();
        assert_eopnotsupp(inode.write_at(0, &mut write_reader, StatusFlags::empty()));
        assert_eopnotsupp(inode.resize(2048));
        assert_eopnotsupp(inode.set_mode(InodeMode::S_IRUSR));
        assert_eopnotsupp(inode.set_owner(Uid::new(2000)));
        assert_eopnotsupp(inode.set_group(Gid::new(2001)));

        let metadata_after_rejections = inode.metadata();
        assert_eq!(metadata_after_rejections.size, file_size as usize);
        assert_eq!(metadata_after_rejections.mode, mode);
        assert_eq!(metadata_after_rejections.uid, uid);
        assert_eq!(metadata_after_rejections.gid, gid);
    }
}
