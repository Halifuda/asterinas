// SPDX-License-Identifier: MPL-2.0

#![cfg_attr(
    not(ktest),
    expect(
        dead_code,
        reason = "Directory record streaming is staged before later directory consumers land."
    )
)]

use aster_block::BlockDevice;

use super::{
    dentry::{DENTRY_SIZE, ExfatDentry, ExfatFileDentry, RawExfatDentry},
    fat::{ChainMode, ClusterId, ExfatChain},
    fileset::ExfatDentrySet,
    io::read_metadata_bytes,
    super_block::ExfatSuperBlock,
};
use crate::{
    fs::file::InodeType,
    prelude::*,
};

const EXFAT_FILE_ATTRIBUTE_DIRECTORY: u16 = 0x10;
const EXFAT_STREAM_FLAG_CONTIGUOUS: u8 = 0x02;

/// Streams directory records in on-disk order.
#[derive(Debug)]
pub(super) struct DirectoryEngine<'a> {
    block_device: &'a dyn BlockDevice,
    super_block: &'a ExfatSuperBlock,
    parent_ino: Option<u64>,
    chain: ExfatChain,
    directory_end_offset: usize,
    cursor: DirectoryCursor,
}

#[derive(Clone, Copy, Debug)]
struct DirectoryCursor {
    byte_offset: usize,
    cluster_offset: usize,
}

/// Carries the trusted location facts for one validated file record.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct DirectoryRecordLocation {
    parent_ino: Option<u64>,
    dentry_set_byte_offset: usize,
    dentry_entry_index: u32,
}

impl DirectoryRecordLocation {
    fn new(parent_ino: Option<u64>, dentry_set_byte_offset: usize, dentry_entry_index: u32) -> Self {
        Self {
            parent_ino,
            dentry_set_byte_offset,
            dentry_entry_index,
        }
    }

    pub(super) fn inode_key_parts(self) -> (Option<u64>, usize, u32) {
        (
            self.parent_ino,
            self.dentry_set_byte_offset,
            self.dentry_entry_index,
        )
    }

    fn stable_inode_number(self) -> u64 {
        let parent_ino = self.parent_ino.unwrap_or(0);
        let byte_offset = u64::try_from(self.dentry_set_byte_offset).unwrap_or(u64::MAX);
        // Keep dirent inode numbers stable across rescans by deriving them only
        // from the validated location facts that also feed `InodeKey`.
        let mixed = parent_ino
            .wrapping_mul(0x9E37_79B1_85EB_CA87)
            ^ byte_offset.rotate_left(17)
            ^ u64::from(self.dentry_entry_index).rotate_left(3)
            ^ 0xA57E_B707_EF32_A31D;

        if mixed <= 1 {
            mixed.wrapping_add(2)
        } else {
            mixed
        }
    }
}

/// Carries one validated file record plus the trusted location facts that identify it.
#[derive(Debug)]
pub(super) struct DirectoryFileRecord {
    dentry_set: ExfatDentrySet,
    location: DirectoryRecordLocation,
}

impl DirectoryFileRecord {
    fn new(dentry_set: ExfatDentrySet, location: DirectoryRecordLocation) -> Self {
        Self { dentry_set, location }
    }

    pub(super) fn dentry_set(&self) -> &ExfatDentrySet {
        &self.dentry_set
    }

    pub(super) fn location(&self) -> DirectoryRecordLocation {
        self.location
    }

    pub(super) fn raw_name_units(&self) -> Vec<u16> {
        self.dentry_set.raw_name_units()
    }

    pub(super) fn inode_type(&self) -> InodeType {
        if self.file_attribute() & EXFAT_FILE_ATTRIBUTE_DIRECTORY != 0 {
            InodeType::Dir
        } else {
            InodeType::File
        }
    }

    pub(super) fn inode_number(&self) -> u64 {
        self.location.stable_inode_number()
    }

    pub(super) fn file_attribute(&self) -> u16 {
        self.dentry_set.file_dentry().attribute
    }

    pub(super) fn start_cluster(&self) -> ClusterId {
        self.dentry_set.stream_dentry().start_cluster
    }

    pub(super) fn chain_mode(&self) -> ChainMode {
        if self.dentry_set.stream_dentry().flags & EXFAT_STREAM_FLAG_CONTIGUOUS != 0 {
            ChainMode::Contiguous
        } else {
            ChainMode::FatBacked
        }
    }

    pub(super) fn cluster_count(&self, cluster_size: usize) -> Result<u32> {
        if cluster_size == 0 {
            return Err(Error::with_message(
                Errno::EINVAL,
                "directory record cluster size must be non-zero",
            ));
        }

        let allocated_size = usize::try_from(self.dentry_set.stream_dentry().size).map_err(|_| {
            Error::with_message(
                Errno::EOVERFLOW,
                "directory record allocated size overflowed usize",
            )
        })?;
        let cluster_count = allocated_size.div_ceil(cluster_size);
        u32::try_from(cluster_count).map_err(|_| {
            Error::with_message(
                Errno::EOVERFLOW,
                "directory record cluster count overflowed u32",
            )
        })
    }
}

/// Describes one logical directory record emitted by the scan service.
#[derive(Debug)]
pub(super) enum DirectoryRecord {
    File(DirectoryFileRecord),
    Singleton(ExfatDentry),
}

impl<'a> DirectoryEngine<'a> {
    /// Creates a read-only directory stream over a validated chain.
    pub(super) fn new(
        block_device: &'a dyn BlockDevice,
        super_block: &'a ExfatSuperBlock,
        parent_ino: Option<u64>,
        chain: ExfatChain,
    ) -> Result<Self> {
        if chain.is_empty() {
            return Err(Error::with_message(
                Errno::EINVAL,
                "directory chain must not be empty",
            ));
        }

        let cluster_count = usize::try_from(chain.cluster_count()).map_err(|_| {
            Error::with_message(Errno::EINVAL, "directory chain size overflows usize")
        })?;
        let directory_end_offset = cluster_count
            .checked_mul(super_block.cluster_size())
            .ok_or_else(|| {
                Error::with_message(Errno::EINVAL, "directory byte size overflows usize")
            })?;

        Ok(Self {
            block_device,
            super_block,
            parent_ino,
            chain,
            directory_end_offset,
            cursor: DirectoryCursor {
                byte_offset: 0,
                cluster_offset: 0,
            },
        })
    }

    /// Returns the next record in on-disk order.
    pub(super) fn next_record(&mut self) -> Result<Option<DirectoryRecord>> {
        loop {
            let Some(dentry) = self.read_next_dentry()? else {
                return Ok(None);
            };

            match dentry {
                ExfatDentry::Deleted(_) => continue,
                ExfatDentry::Unused => return Ok(None),
                dentry if dentry.is_volume_label() => continue,
                ExfatDentry::File(file_dentry) => {
                    let record_start_offset = self
                        .cursor
                        .byte_offset
                        .checked_sub(DENTRY_SIZE)
                        .ok_or_else(|| {
                            Error::with_message(
                                Errno::EINVAL,
                                "directory file record offset underflow",
                            )
                        })?;
                    let record = self.read_file_record(file_dentry, record_start_offset)?;
                    return Ok(Some(DirectoryRecord::File(record)));
                }
                ExfatDentry::Bitmap(bitmap_dentry) => {
                    return Ok(Some(DirectoryRecord::Singleton(ExfatDentry::Bitmap(
                        bitmap_dentry,
                    ))));
                }
                ExfatDentry::Upcase(upcase_dentry) => {
                    return Ok(Some(DirectoryRecord::Singleton(ExfatDentry::Upcase(
                        upcase_dentry,
                    ))));
                }
                _ => {
                    return Err(Error::with_message(
                        Errno::EINVAL,
                        "unexpected top-level directory dentry",
                    ));
                }
            }
        }
    }

    fn read_file_record(
        &mut self,
        file_dentry: ExfatFileDentry,
        record_start_offset: usize,
    ) -> Result<DirectoryFileRecord> {
        let secondary_count = usize::from(file_dentry.num_secondary);
        let mut dentries = Vec::with_capacity(secondary_count + 1);
        dentries.push(ExfatDentry::File(file_dentry));

        for _ in 0..secondary_count {
            let Some(dentry) = self.read_next_dentry()? else {
                return Err(Error::with_message(
                    Errno::EINVAL,
                    "truncated directory file record",
                ));
            };

            if matches!(dentry, ExfatDentry::Deleted(_) | ExfatDentry::Unused) {
                return Err(Error::with_message(
                    Errno::EINVAL,
                    "directory file record was truncated or interrupted",
                ));
            }

            dentries.push(dentry);
        }

        let dentry_entry_index =
            u32::try_from(record_start_offset / DENTRY_SIZE).map_err(|_| {
                Error::with_message(
                    Errno::EOVERFLOW,
                    "directory record entry index overflowed u32",
                )
            })?;
        let location = DirectoryRecordLocation::new(
            self.parent_ino,
            record_start_offset,
            dentry_entry_index,
        );

        Ok(DirectoryFileRecord::new(ExfatDentrySet::new(dentries)?, location))
    }

    fn read_next_dentry(&mut self) -> Result<Option<ExfatDentry>> {
        if self.cursor.byte_offset >= self.directory_end_offset {
            return Ok(None);
        }

        if self.cursor.cluster_offset >= self.super_block.cluster_size() {
            self.advance_cluster()?;
        }

        let cluster_start_offset = self.chain.physical_cluster_start_offset(self.super_block)?;
        let physical_offset = cluster_start_offset
            .checked_add(self.cursor.cluster_offset)
            .ok_or_else(|| Error::with_message(Errno::EINVAL, "directory offset overflow"))?;

        let mut raw_bytes = [0; DENTRY_SIZE];
        read_metadata_bytes(self.block_device, physical_offset, &mut raw_bytes)?;

        self.cursor.byte_offset = self
            .cursor
            .byte_offset
            .checked_add(DENTRY_SIZE)
            .ok_or_else(|| Error::with_message(Errno::EINVAL, "directory cursor overflow"))?;
        self.cursor.cluster_offset += DENTRY_SIZE;

        Ok(Some(ExfatDentry::from(RawExfatDentry::from_bytes(
            &raw_bytes,
        ))))
    }

    fn advance_cluster(&mut self) -> Result<()> {
        self.chain = self.chain.walk(self.block_device, self.super_block, 1)?;
        self.cursor.cluster_offset = 0;
        Ok(())
    }
}

#[cfg(ktest)]
mod tests {
    use alloc::vec;

    use ostd::prelude::ktest;
    use zerocopy::IntoBytes;

    use super::{DirectoryEngine, DirectoryRecord};
    use crate::{
        fs::fs_impls::exfat_refactor::{
            boot_sector::read_primary_super_block,
            dentry::{
                DENTRY_SIZE, ExfatBitmapDentry, ExfatDentry, ExfatStreamDentry, ExfatUpcaseDentry,
                RawExfatDentry,
            },
            fat::{ChainMode, ExfatChain},
            fileset::ExfatDentrySet,
            super_block::ExfatSuperBlock,
            test_support::{ExfatMemoryDisk, load_exfat_disk},
        },
        prelude::{Errno, Pod},
    };

    fn make_directory_engine<'a>(
        disk: &'a ExfatMemoryDisk,
        super_block: &'a ExfatSuperBlock,
        cluster_count: u32,
    ) -> DirectoryEngine<'a> {
        let chain = ExfatChain::new(
            disk,
            super_block,
            super_block.root_dir,
            Some(cluster_count),
            ChainMode::Contiguous,
        )
        .unwrap();

        DirectoryEngine::new(disk, super_block, Some(1), chain).unwrap()
    }

    fn write_raw_dentry(
        disk: &ExfatMemoryDisk,
        super_block: &crate::fs::fs_impls::exfat_refactor::super_block::ExfatSuperBlock,
        entry_index: usize,
        dentry: RawExfatDentry,
    ) {
        let cluster_start = super_block
            .cluster_to_byte_offset(super_block.root_dir)
            .unwrap();
        let offset = cluster_start + entry_index * DENTRY_SIZE;
        disk.write_bytes(offset, dentry.as_bytes());
    }

    fn write_entry_sequence(
        disk: &ExfatMemoryDisk,
        super_block: &crate::fs::fs_impls::exfat_refactor::super_block::ExfatSuperBlock,
        start_index: usize,
        entries: &[ExfatDentry],
    ) {
        for (offset, entry) in entries.iter().enumerate() {
            let cluster_start = super_block
                .cluster_to_byte_offset(super_block.root_dir)
                .unwrap();
            let byte_offset = cluster_start + (start_index + offset) * DENTRY_SIZE;
            disk.write_bytes(byte_offset, entry.as_bytes());
        }
    }

    fn write_serialized_bytes(
        disk: &ExfatMemoryDisk,
        super_block: &crate::fs::fs_impls::exfat_refactor::super_block::ExfatSuperBlock,
        start_index: usize,
        bytes: &[u8],
    ) {
        for (offset, chunk) in bytes.chunks_exact(DENTRY_SIZE).enumerate() {
            write_raw_dentry(
                disk,
                super_block,
                start_index + offset,
                RawExfatDentry::from_bytes(chunk),
            );
        }
    }

    fn deleted_dentry() -> RawExfatDentry {
        RawExfatDentry {
            dentry_type: 0x01,
            value: [0; 31],
        }
    }

    fn unused_dentry() -> RawExfatDentry {
        RawExfatDentry {
            dentry_type: 0x00,
            value: [0; 31],
        }
    }

    fn bitmap_dentry(start_cluster: u32, size: u64) -> ExfatDentry {
        ExfatDentry::Bitmap(ExfatBitmapDentry {
            dentry_type: 0x81,
            flags: 0,
            reserved: [0; 18],
            start_cluster,
            size,
        })
    }

    fn upcase_dentry(start_cluster: u32, size: u64) -> ExfatDentry {
        ExfatDentry::Upcase(ExfatUpcaseDentry {
            dentry_type: 0x82,
            reserved1: [0; 3],
            checksum: 0,
            reserved2: [0; 12],
            start_cluster,
            size,
        })
    }

    fn volume_label_dentry() -> RawExfatDentry {
        RawExfatDentry {
            dentry_type: 0x83,
            value: [0; 31],
        }
    }

    // Confirms the stream crosses a cluster boundary without reordering or duplicating records.
    #[ktest]
    fn directory_engine_preserves_order_across_cluster_boundary() {
        let disk = load_exfat_disk();
        let super_block = read_primary_super_block(&disk).unwrap();
        let mut engine = make_directory_engine(&disk, &super_block, 2);
        let cluster_entries = super_block.cluster_size() / DENTRY_SIZE;

        let file_set = ExfatDentrySet::from_trusted_metadata(
            Default::default(),
            ExfatStreamDentry {
                start_cluster: super_block.root_dir,
                size: 0,
                ..Default::default()
            },
            &[b'a' as u16, b'b' as u16, b'c' as u16],
            vec![ExfatDentry::VendorExt(Default::default())],
        )
        .unwrap();
        let file_bytes = file_set.to_le_bytes();
        let file_start = cluster_entries - (file_bytes.len() / DENTRY_SIZE) + 1;

        write_raw_dentry(&disk, &super_block, 0, deleted_dentry());
        write_serialized_bytes(&disk, &super_block, file_start, &file_bytes);
        write_entry_sequence(
            &disk,
            &super_block,
            file_start + file_bytes.len() / DENTRY_SIZE,
            &[upcase_dentry(7, 1234)],
        );
        write_raw_dentry(
            &disk,
            &super_block,
            file_start + file_bytes.len() / DENTRY_SIZE + 1,
            unused_dentry(),
        );

        assert!(matches!(
            engine.next_record().unwrap(),
            Some(DirectoryRecord::File(_))
        ));
        assert!(matches!(
            engine.next_record().unwrap(),
            Some(DirectoryRecord::Singleton(ExfatDentry::Upcase(_)))
        ));
        assert!(engine.next_record().unwrap().is_none());
    }

    // Confirms a valid file record is emitted as a validated `ExfatDentrySet`.
    #[ktest]
    fn directory_engine_emits_validated_file_records() {
        let disk = load_exfat_disk();
        let super_block = read_primary_super_block(&disk).unwrap();
        let mut engine = make_directory_engine(&disk, &super_block, 1);
        let file_set = ExfatDentrySet::from_trusted_metadata(
            Default::default(),
            ExfatStreamDentry {
                start_cluster: super_block.root_dir,
                size: 4096,
                valid_size: 4096,
                ..Default::default()
            },
            &[b'i' as u16, b'n' as u16, b'o' as u16],
            vec![ExfatDentry::GenericSecondary(Default::default())],
        )
        .unwrap();
        let file_bytes = file_set.to_le_bytes();

        write_serialized_bytes(&disk, &super_block, 0, &file_bytes);
        write_raw_dentry(
            &disk,
            &super_block,
            file_bytes.len() / DENTRY_SIZE,
            unused_dentry(),
        );

        let record = engine.next_record().unwrap().unwrap();
        match record {
            DirectoryRecord::File(actual) => {
                assert!(actual.dentry_set().verify_checksum());
                assert_eq!(
                    actual.raw_name_units(),
                    vec![b'i' as u16, b'n' as u16, b'o' as u16]
                );
            }
            DirectoryRecord::Singleton(_) => panic!("expected validated file record"),
        }
        assert!(engine.next_record().unwrap().is_none());
    }

    // Confirms `Bitmap` and `Upcase` entries are surfaced as raw singleton candidates.
    #[ktest]
    fn directory_engine_surfaces_raw_singletons_without_policy() {
        let disk = load_exfat_disk();
        let super_block = read_primary_super_block(&disk).unwrap();
        let mut engine = make_directory_engine(&disk, &super_block, 1);

        write_entry_sequence(
            &disk,
            &super_block,
            0,
            &[bitmap_dentry(11, 8192), upcase_dentry(12, 16384)],
        );
        write_raw_dentry(&disk, &super_block, 2, unused_dentry());

        let first = engine.next_record().unwrap().unwrap();
        let second = engine.next_record().unwrap().unwrap();

        match first {
            DirectoryRecord::Singleton(ExfatDentry::Bitmap(bitmap)) => {
                let start_cluster = bitmap.start_cluster;
                let size = bitmap.size;
                assert_eq!(start_cluster, 11);
                assert_eq!(size, 8192);
            }
            _ => panic!("expected bitmap singleton"),
        }

        match second {
            DirectoryRecord::Singleton(ExfatDentry::Upcase(upcase)) => {
                let start_cluster = upcase.start_cluster;
                let size = upcase.size;
                assert_eq!(start_cluster, 12);
                assert_eq!(size, 16384);
            }
            _ => panic!("expected upcase singleton"),
        }
    }

    // Confirms tombstones are skipped and `Unused` terminates the scan.
    #[ktest]
    fn directory_engine_skips_deleted_and_stops_at_unused() {
        let disk = load_exfat_disk();
        let super_block = read_primary_super_block(&disk).unwrap();
        let mut engine = make_directory_engine(&disk, &super_block, 1);

        write_raw_dentry(&disk, &super_block, 0, deleted_dentry());
        write_entry_sequence(
            &disk,
            &super_block,
            1,
            &[bitmap_dentry(21, 4096), upcase_dentry(22, 4096)],
        );
        write_raw_dentry(&disk, &super_block, 3, unused_dentry());

        let first = engine.next_record().unwrap().unwrap();
        assert!(matches!(
            first,
            DirectoryRecord::Singleton(ExfatDentry::Bitmap(_))
        ));
        assert!(matches!(
            engine.next_record().unwrap(),
            Some(DirectoryRecord::Singleton(ExfatDentry::Upcase(_)))
        ));
        assert!(engine.next_record().unwrap().is_none());
    }

    // Confirms the stream ignores the root volume-label metadata entry while
    // still preserving the explicit singleton candidate boundary.
    #[ktest]
    fn directory_engine_skips_volume_label_and_keeps_system_singletons() {
        let disk = load_exfat_disk();
        let super_block = read_primary_super_block(&disk).unwrap();
        let mut engine = make_directory_engine(&disk, &super_block, 1);

        write_raw_dentry(&disk, &super_block, 0, volume_label_dentry());
        write_entry_sequence(
            &disk,
            &super_block,
            1,
            &[bitmap_dentry(31, 4096), upcase_dentry(32, 8192)],
        );
        write_raw_dentry(&disk, &super_block, 3, unused_dentry());

        assert!(matches!(
            engine.next_record().unwrap(),
            Some(DirectoryRecord::Singleton(ExfatDentry::Bitmap(_)))
        ));
        assert!(matches!(
            engine.next_record().unwrap(),
            Some(DirectoryRecord::Singleton(ExfatDentry::Upcase(_)))
        ));
        assert!(engine.next_record().unwrap().is_none());
    }

    // Confirms unexpected top-level dentries fail instead of surfacing as generic singletons.
    #[ktest]
    fn directory_engine_rejects_unexpected_top_level_dentry() {
        let disk = load_exfat_disk();
        let super_block = read_primary_super_block(&disk).unwrap();
        let mut engine = make_directory_engine(&disk, &super_block, 1);

        write_raw_dentry(
            &disk,
            &super_block,
            0,
            RawExfatDentry {
                dentry_type: 0x80,
                value: [0; 31],
            },
        );
        write_raw_dentry(&disk, &super_block, 1, unused_dentry());

        let error = engine.next_record().unwrap_err();
        assert_eq!(error.error(), Errno::EINVAL);
    }
}
