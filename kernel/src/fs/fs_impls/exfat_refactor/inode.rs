// SPDX-License-Identifier: MPL-2.0

#![cfg_attr(
    not(ktest),
    expect(
        dead_code,
        reason = "Inode carrier is staged before open, cache, and data-path integration."
    )
)]

use alloc::{
    string::String,
    sync::{Arc, Weak},
    vec::Vec,
};
use core::time::Duration;

use aster_block::{BlockDevice, bio::BioWaiter};
use spin::Once;

use super::{
    directory::{DirectoryFileRecord, DirectoryRecord},
    fat::{ChainMode, ClusterId, ExfatChain},
    fileset::ExfatDentrySet,
    fs::{ExfatFs, EXFAT_NAME_MAX},
    io::read_metadata_bytes,
    super_block::ExfatSuperBlock,
};
use crate::{
    fs::{
        file::{InodeMode, InodeType, StatusFlags},
        utils::DirentVisitor,
        vfs::{
            file_system::FileSystem,
            inode::{Extension, Inode, InodeIo, Metadata},
            page_cache::{CachePage, PageCache, PageCacheBackend},
        },
    },
    prelude::*,
    process::{Gid, Uid},
    vm::vmo::Vmo,
};
use ostd::mm::io::util::HasVmReaderWriter;

const SECTOR_SIZE: usize = 512;
const READ_BOUNCE_BUFFER_SIZE: usize = 4096;

/// Carries the VFS-visible exFAT inode metadata snapshot.
pub(super) struct ExfatInode {
    fs: Weak<ExfatFs>,
    metadata: Metadata,
    extension: Extension,
    page_cache: Once<PageCache>,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PhysicalFileRange {
    cluster: ClusterId,
    physical_byte_offset: usize,
    intra_cluster_offset: usize,
    mappable_byte_count: usize,
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

        let inode = Arc::new(Self {
            fs,
            metadata,
            extension: Extension::new(),
            page_cache: Once::new(),
            location,
            file_attribute: file_dentry.attribute,
            valid_size,
            start_cluster: chain.current_cluster(),
            cluster_count: chain.cluster_count(),
            chain_mode: chain.mode(),
            allocated_size,
        });
        inode.initialize_page_cache()?;
        Ok(inode)
    }

    fn initialize_page_cache(self: &Arc<Self>) -> Result<()> {
        if self.type_() != InodeType::File {
            return Ok(());
        }

        let page_cache = Self::build_page_cache(Arc::downgrade(self), self.size())?;
        self.page_cache.call_once(|| page_cache);
        Ok(())
    }

    fn build_page_cache(backend: Weak<Self>, capacity: usize) -> Result<PageCache> {
        let backend: Weak<dyn PageCacheBackend> = backend;
        PageCache::with_capacity(capacity, backend)
    }

    fn owner_fs(&self) -> Arc<ExfatFs> {
        self.fs
            .upgrade()
            .expect("exFAT inode must not outlive its filesystem owner")
    }

    fn page_count(&self) -> usize {
        if self.type_() != InodeType::File {
            return 0;
        }

        self.size().div_ceil(PAGE_SIZE)
    }

    fn page_offset(&self, page_index: usize) -> Result<usize> {
        if page_index >= self.page_count() {
            return Err(Error::with_message(
                Errno::EINVAL,
                "exFAT page-cache request starts beyond the inode snapshot",
            ));
        }

        page_index.checked_mul(PAGE_SIZE).ok_or_else(|| {
            Error::with_message(
                Errno::EOVERFLOW,
                "exFAT page-cache offset overflowed usize",
            )
        })
    }

    fn fill_cache_page_from_read_owner(&self, page_index: usize, frame: &CachePage) -> Result<()> {
        let page_offset = self.page_offset(page_index)?;
        let mut writer = frame.writer().to_fallible();
        self.read_at(page_offset, &mut writer, StatusFlags::empty())?;

        let remaining = writer.avail();
        if remaining > 0 {
            writer.fill_zeros(remaining).map_err(|(error, _)| error)?;
        }

        Ok(())
    }

    fn mapping_chain(
        &self,
        block_device: &dyn BlockDevice,
        super_block: &ExfatSuperBlock,
    ) -> Result<ExfatChain> {
        ExfatChain::new(
            block_device,
            super_block,
            self.start_cluster,
            Some(self.cluster_count),
            self.chain_mode,
        )
    }

    fn mapping_cluster_size(&self, super_block: &ExfatSuperBlock) -> Result<usize> {
        let cluster_size = super_block.cluster_size();
        if cluster_size != self.metadata.optimal_block_size {
            return Err(Error::with_message(
                Errno::EINVAL,
                "exFAT inode cluster geometry mismatched its metadata snapshot",
            ));
        }

        Ok(cluster_size)
    }

    fn physically_backed_end(&self) -> usize {
        self.size().min(self.valid_size).min(self.allocated_size)
    }

    fn physically_mappable_byte_count(
        &self,
        request_offset: usize,
        request_len: usize,
        intra_cluster_offset: usize,
        cluster_size: usize,
    ) -> Result<usize> {
        let backed_end = self.physically_backed_end();
        let backed_remaining = backed_end.checked_sub(request_offset).ok_or_else(|| {
            Error::with_message(
                Errno::EINVAL,
                "exFAT mapping request starts beyond the physically backed range",
            )
        })?;
        let cluster_remaining =
            cluster_size
                .checked_sub(intra_cluster_offset)
                .ok_or_else(|| {
                    Error::with_message(
                        Errno::EINVAL,
                        "exFAT mapping request exceeds the containing cluster",
                    )
                })?;

        Ok(request_len.min(backed_remaining).min(cluster_remaining))
    }

    fn map_physical_file_range(
        &self,
        block_device: &dyn BlockDevice,
        super_block: &ExfatSuperBlock,
        request_offset: usize,
        request_len: usize,
    ) -> Result<Option<PhysicalFileRange>> {
        if self.type_() != InodeType::File {
            return Err(Error::with_message(
                Errno::EINVAL,
                "exFAT file mapping requires a regular-file inode",
            ));
        }

        if request_len == 0 || request_offset >= self.physically_backed_end() {
            return Ok(None);
        }

        let cluster_size = self.mapping_cluster_size(super_block)?;
        let mapping_chain = self.mapping_chain(block_device, super_block)?;
        let (mapped_chain, intra_cluster_offset) =
            mapping_chain.walk_to_cluster_at_offset(block_device, super_block, request_offset)?;
        let cluster_start_offset = mapped_chain.physical_cluster_start_offset(super_block)?;
        let physical_byte_offset = cluster_start_offset
            .checked_add(intra_cluster_offset)
            .ok_or_else(|| {
                Error::with_message(
                    Errno::EOVERFLOW,
                    "exFAT mapped byte offset overflowed usize",
                )
            })?;
        let mappable_byte_count = self.physically_mappable_byte_count(
            request_offset,
            request_len,
            intra_cluster_offset,
            cluster_size,
        )?;

        Ok(Some(PhysicalFileRange {
            cluster: mapped_chain.current_cluster(),
            physical_byte_offset,
            intra_cluster_offset,
            mappable_byte_count,
        }))
    }

    fn read_visible_byte_count(&self, offset: usize, writer: &VmWriter) -> Result<usize> {
        let request_end = offset.checked_add(writer.avail()).ok_or_else(|| {
            Error::with_message(Errno::EOVERFLOW, "exFAT read request overflowed usize")
        })?;

        Ok(self.size().min(request_end).saturating_sub(offset))
    }

    fn copy_physical_file_range(
        &self,
        block_device: &dyn BlockDevice,
        file_range: PhysicalFileRange,
        request_len: usize,
        writer: &mut VmWriter,
    ) -> Result<usize> {
        let copy_len = request_len
            .min(file_range.mappable_byte_count)
            .min(writer.avail());
        if copy_len == 0 {
            return Ok(0);
        }

        let mut scratch = vec![0; READ_BOUNCE_BUFFER_SIZE.min(copy_len)];
        let mut copied = 0usize;

        while copied < copy_len {
            let remaining = copy_len - copied;
            let chunk_len = scratch.len().min(remaining);
            let chunk_offset = file_range
                .physical_byte_offset
                .checked_add(copied)
                .ok_or_else(|| {
                    Error::with_message(
                        Errno::EOVERFLOW,
                        "exFAT physical read offset overflowed usize",
                    )
                })?;

            read_metadata_bytes(block_device, chunk_offset, &mut scratch[..chunk_len])?;
            writer
                .write_fallible(&mut VmReader::from(&scratch[..chunk_len]))
                .map_err(|(error, _)| error)?;
            copied = copied.checked_add(chunk_len).ok_or_else(|| {
                Error::with_message(Errno::EOVERFLOW, "exFAT read copy length overflowed usize")
            })?;
        }

        Ok(copied)
    }

    fn zero_fill_valid_size_gap(
        &self,
        offset: usize,
        request_len: usize,
        writer: &mut VmWriter,
    ) -> Result<usize> {
        if request_len == 0 || offset < self.valid_size || offset >= self.size() {
            return Ok(0);
        }

        let zero_len = self
            .size()
            .checked_sub(offset)
            .ok_or_else(|| Error::with_message(Errno::EINVAL, "exFAT zero-fill offset underflow"))?
            .min(request_len)
            .min(writer.avail());

        Ok(writer.fill_zeros(zero_len).map_err(|(error, _)| error)?)
    }

    fn ensure_directory(&self) -> Result<()> {
        if self.type_() != InodeType::Dir {
            return Err(Error::new(Errno::ENOTDIR));
        }

        Ok(())
    }

    fn record_name_matches(
        fs: &ExfatFs,
        lookup_folded_name: &[u16],
        lookup_name_hash: u16,
        file_record: &DirectoryFileRecord,
    ) -> Result<bool> {
        let record_name_units = file_record.raw_name_units();
        let record_folded_name = fs.fold_utf16(&record_name_units)?;
        if fs.name_hash_from_folded_utf16(&record_folded_name)? != lookup_name_hash {
            return Ok(false);
        }

        Ok(record_folded_name == lookup_folded_name)
    }

    fn visible_record_name(file_record: &DirectoryFileRecord) -> Result<String> {
        String::from_utf16(&file_record.raw_name_units()).map_err(|_| {
            Error::with_message(Errno::EINVAL, "directory record name is not valid UTF-16")
        })
    }
}

impl InodeIo for ExfatInode {
    fn read_at(
        &self,
        offset: usize,
        writer: &mut VmWriter,
        _status_flags: StatusFlags,
    ) -> Result<usize> {
        if self.type_() != InodeType::File {
            return Err(Error::with_message(
                Errno::EISDIR,
                "exFAT buffered reads require a regular-file inode",
            ));
        }

        if !writer.has_avail() || offset >= self.size() {
            return Ok(0);
        }

        let fs = self.owner_fs();
        let (block_device, super_block) = fs.file_read_context();
        let mut logical_offset = offset;
        let mut visible_byte_count = 0usize;
        let visible_request_len = self.read_visible_byte_count(offset, writer)?;

        while writer.has_avail() {
            let remaining_visible = visible_request_len
                .checked_sub(visible_byte_count)
                .ok_or_else(|| {
                    Error::with_message(
                        Errno::EINVAL,
                        "exFAT read loop advanced past the visible request",
                    )
                })?;
            if remaining_visible == 0 {
                break;
            }

            let Some(file_range) = self.map_physical_file_range(
                block_device,
                super_block,
                logical_offset,
                remaining_visible,
            )?
            else {
                break;
            };
            let copied =
                self.copy_physical_file_range(block_device, file_range, remaining_visible, writer)?;
            if copied == 0 {
                break;
            }

            logical_offset = logical_offset.checked_add(copied).ok_or_else(|| {
                Error::with_message(
                    Errno::EOVERFLOW,
                    "exFAT logical read offset overflowed usize",
                )
            })?;
            visible_byte_count = visible_byte_count.checked_add(copied).ok_or_else(|| {
                Error::with_message(Errno::EOVERFLOW, "exFAT read length overflowed usize")
            })?;
        }

        let remaining_visible = visible_request_len
            .checked_sub(visible_byte_count)
            .ok_or_else(|| {
                Error::with_message(
                    Errno::EINVAL,
                    "exFAT zero-fill exceeded the visible read request",
                )
            })?;
        let zero_filled =
            self.zero_fill_valid_size_gap(logical_offset, remaining_visible, writer)?;

        visible_byte_count
            .checked_add(zero_filled)
            .ok_or_else(|| Error::with_message(Errno::EOVERFLOW, "exFAT read length overflowed"))
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

impl PageCacheBackend for ExfatInode {
    fn read_page_async(&self, idx: usize, frame: &CachePage) -> Result<BioWaiter> {
        self.fill_cache_page_from_read_owner(idx, frame)?;
        Ok(BioWaiter::new())
    }

    fn write_page_async(&self, _idx: usize, _frame: &CachePage) -> Result<BioWaiter> {
        Err(Error::with_message(
            Errno::EOPNOTSUPP,
            "exFAT page-cache writeback is deferred to EXR-WRITE-30 and EXR-SYNC-31",
        ))
    }

    fn npages(&self) -> usize {
        self.page_count()
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
        let fs: Arc<dyn FileSystem> = self.owner_fs();
        fs
    }

    fn page_cache(&self) -> Option<Arc<Vmo>> {
        self.page_cache.get().map(|page_cache| page_cache.pages().clone())
    }

    fn extension(&self) -> &Extension {
        &self.extension
    }

    fn readdir_at(&self, offset: usize, visitor: &mut dyn DirentVisitor) -> Result<usize> {
        self.ensure_directory()?;

        let fs = self.owner_fs();
        let mut directory_stream = fs.directory_stream(
            Some(self.ino()),
            self.start_cluster,
            self.cluster_count,
            self.chain_mode,
        )?;
        let mut logical_offset = 0usize;
        let mut emitted_count = 0usize;

        while let Some(record) = directory_stream.next_record()? {
            let DirectoryRecord::File(file_record) = record else {
                continue;
            };

            if logical_offset < offset {
                logical_offset = logical_offset.checked_add(1).ok_or_else(|| {
                    Error::with_message(
                        Errno::EOVERFLOW,
                        "directory logical offset overflowed usize",
                    )
                })?;
                continue;
            }

            let name = Self::visible_record_name(&file_record)?;
            let emitted_offset = logical_offset.checked_add(1).ok_or_else(|| {
                Error::with_message(
                    Errno::EOVERFLOW,
                    "directory logical offset overflowed usize",
                )
            })?;
            match visitor.visit(
                &name,
                file_record.inode_number(),
                file_record.inode_type(),
                emitted_offset,
            ) {
                Ok(()) => {
                    logical_offset = emitted_offset;
                    emitted_count = emitted_count.checked_add(1).ok_or_else(|| {
                        Error::with_message(
                            Errno::EOVERFLOW,
                            "directory emitted count overflowed usize",
                        )
                    })?;
                }
                Err(error) => {
                    if emitted_count == 0 {
                        return Err(error);
                    }
                    break;
                }
            }
        }

        Ok(emitted_count)
    }

    fn lookup(&self, name: &str) -> Result<Arc<dyn Inode>> {
        self.ensure_directory()?;

        let lookup_name_units: Vec<u16> = name.encode_utf16().collect();
        if lookup_name_units.len() > EXFAT_NAME_MAX {
            return Err(Error::new(Errno::ENAMETOOLONG));
        }

        let fs = self.owner_fs();
        let lookup_folded_name = fs.fold_utf16(&lookup_name_units)?;
        let lookup_name_hash = fs.name_hash_from_folded_utf16(&lookup_folded_name)?;
        let mut directory_stream = fs.directory_stream(
            Some(self.ino()),
            self.start_cluster,
            self.cluster_count,
            self.chain_mode,
        )?;

        while let Some(record) = directory_stream.next_record()? {
            let DirectoryRecord::File(file_record) = record else {
                continue;
            };

            if Self::record_name_matches(
                fs.as_ref(),
                &lookup_folded_name,
                lookup_name_hash,
                &file_record,
            )? {
                let inode = fs.resolve_or_publish_child_inode(&file_record)?;
                let inode: Arc<dyn Inode> = inode;
                return Ok(inode);
            }
        }

        Err(Error::new(Errno::ENOENT))
    }
}

fn allocated_size(cluster_count: u32, cluster_size: usize) -> Result<usize> {
    cluster_size
        .checked_mul(cluster_count as usize)
        .ok_or_else(|| Error::with_message(Errno::EOVERFLOW, "exFAT inode allocation overflow"))
}

#[cfg(ktest)]
mod tests {
    use alloc::{string::String, sync::Arc, vec, vec::Vec};

    use aster_block::BlockDevice;
    use ostd::prelude::ktest;

    use super::*;
    use crate::fs::{
        fs_impls::exfat_refactor::{
            boot_sector::read_primary_super_block,
            dentry::{
                ExfatDentry, ExfatFileDentry, ExfatStreamDentry, ExfatUpcaseDentry, RawExfatDentry,
                DENTRY_SIZE,
            },
            fileset::ExfatDentrySet,
            io::read_metadata_bytes,
            test_support::{load_exfat_disk, ExfatMemoryDisk},
        },
        utils::DirentVisitor,
    };
    use crate::vm::vmo::CommitFlags;
    use ostd::mm::VmIo;

    const UPCASE_TABLE_UNIT_COUNT: usize = 0x1_0000;
    const UPCASE_TABLE_IDENTITY_RUN_MARKER: u16 = 0xFFFF;

    fn assert_eopnotsupp<T>(result: Result<T>) {
        match result {
            Ok(_) => panic!("temporary inode seam should reject"),
            Err(error) => assert_eq!(error.error(), Errno::EOPNOTSUPP),
        }
    }

    fn mandatory_upcase_unit(unit: u16) -> u16 {
        match unit {
            0x61..=0x7A => unit - 0x20,
            _ => unit,
        }
    }

    fn table_checksum(raw_table_bytes: &[u8]) -> u32 {
        raw_table_bytes.iter().fold(0u32, |checksum, byte| {
            checksum.rotate_right(1).wrapping_add(u32::from(*byte))
        })
    }

    fn valid_upcase_fixture() -> (ExfatUpcaseDentry, Vec<u8>) {
        let mut raw_table_bytes = Vec::with_capacity(130 * 2);
        let identity_count = u16::try_from(UPCASE_TABLE_UNIT_COUNT - 128).unwrap();

        for unit in 0u16..128 {
            raw_table_bytes.extend_from_slice(&mandatory_upcase_unit(unit).to_le_bytes());
        }
        raw_table_bytes.extend_from_slice(&UPCASE_TABLE_IDENTITY_RUN_MARKER.to_le_bytes());
        raw_table_bytes.extend_from_slice(&identity_count.to_le_bytes());

        let upcase_dentry = ExfatUpcaseDentry {
            dentry_type: 0x82,
            reserved1: [0; 3],
            checksum: table_checksum(&raw_table_bytes),
            reserved2: [0; 12],
            start_cluster: 7,
            size: raw_table_bytes.len() as u64,
        };

        (upcase_dentry, raw_table_bytes)
    }

    fn write_upcase_prerequisite(
        disk: &ExfatMemoryDisk,
        super_block: &crate::fs::fs_impls::exfat_refactor::super_block::ExfatSuperBlock,
        mut upcase_dentry: ExfatUpcaseDentry,
        raw_table_bytes: &[u8],
    ) {
        let root_dir_offset = super_block
            .cluster_to_byte_offset(super_block.root_dir)
            .unwrap();
        let (upcase_entry_index, existing_upcase_dentry) =
            first_existing_upcase_root_entry(disk, super_block);
        let upcase_entry_offset = root_dir_offset + upcase_entry_index * DENTRY_SIZE;
        upcase_dentry.start_cluster = existing_upcase_dentry.start_cluster;

        disk.write_bytes(upcase_entry_offset, upcase_dentry.as_bytes());
        disk.write_bytes(
            super_block
                .cluster_to_byte_offset(upcase_dentry.start_cluster)
                .unwrap(),
            raw_table_bytes,
        );
    }

    fn first_existing_upcase_root_entry(
        disk: &ExfatMemoryDisk,
        super_block: &crate::fs::fs_impls::exfat_refactor::super_block::ExfatSuperBlock,
    ) -> (usize, ExfatUpcaseDentry) {
        let root_dir_offset = super_block
            .cluster_to_byte_offset(super_block.root_dir)
            .unwrap();
        let cluster_size = super_block.cluster_size();
        let entry_count = cluster_size / DENTRY_SIZE;

        for entry_index in 0..entry_count {
            let mut raw_bytes = [0; DENTRY_SIZE];
            read_metadata_bytes(
                disk,
                root_dir_offset + entry_index * DENTRY_SIZE,
                &mut raw_bytes,
            )
            .unwrap();
            match ExfatDentry::from(RawExfatDentry::from_bytes(&raw_bytes)) {
                ExfatDentry::Upcase(upcase_dentry) => return (entry_index, upcase_dentry),
                _ => {}
            }
        }

        panic!("expected an existing upcase slot in the root directory");
    }

    fn first_root_system_entry_end(
        disk: &ExfatMemoryDisk,
        super_block: &crate::fs::fs_impls::exfat_refactor::super_block::ExfatSuperBlock,
    ) -> usize {
        let root_dir_offset = super_block
            .cluster_to_byte_offset(super_block.root_dir)
            .unwrap();
        let cluster_size = super_block.cluster_size();
        let entry_count = cluster_size / DENTRY_SIZE;
        let mut bitmap_entry_index = None;
        let mut upcase_entry_index = None;

        for entry_index in 0..entry_count {
            let mut raw_bytes = [0; DENTRY_SIZE];
            read_metadata_bytes(
                disk,
                root_dir_offset + entry_index * DENTRY_SIZE,
                &mut raw_bytes,
            )
            .unwrap();

            match ExfatDentry::from(RawExfatDentry::from_bytes(&raw_bytes)) {
                ExfatDentry::Bitmap(_) => bitmap_entry_index = Some(entry_index),
                ExfatDentry::Upcase(_) => upcase_entry_index = Some(entry_index),
                _ => {}
            }

            if bitmap_entry_index.is_some() && upcase_entry_index.is_some() {
                break;
            }
        }

        let bitmap_entry_index = bitmap_entry_index.expect("expected bitmap slot in root");
        let upcase_entry_index = upcase_entry_index.expect("expected upcase slot in root");
        bitmap_entry_index.max(upcase_entry_index) + 1
    }

    fn file_record(
        name: &str,
        file_attribute: u16,
        start_cluster: u32,
        size: u64,
    ) -> ExfatDentrySet {
        let mut file_dentry = ExfatFileDentry::default();
        file_dentry.attribute = file_attribute;

        let mut stream_dentry = ExfatStreamDentry::default();
        stream_dentry.valid_size = size;
        stream_dentry.start_cluster = start_cluster;
        stream_dentry.size = size;

        let name_units = name.encode_utf16().collect::<Vec<_>>();
        ExfatDentrySet::from_trusted_metadata(file_dentry, stream_dentry, &name_units, Vec::new())
            .expect("trusted file record should validate")
    }

    fn write_file_records(
        disk: &ExfatMemoryDisk,
        super_block: &crate::fs::fs_impls::exfat_refactor::super_block::ExfatSuperBlock,
        start_index: usize,
        records: &[ExfatDentrySet],
    ) {
        let root_dir_offset = super_block
            .cluster_to_byte_offset(super_block.root_dir)
            .unwrap();
        let mut next_index = start_index;

        for record in records {
            let bytes = record.to_le_bytes();
            disk.write_bytes(root_dir_offset + next_index * DENTRY_SIZE, &bytes);
            next_index += bytes.len() / DENTRY_SIZE;
        }

        disk.write_bytes(
            root_dir_offset + next_index * DENTRY_SIZE,
            ExfatDentry::Unused.as_bytes(),
        );
    }

    fn clear_root_visible_entries_after_singletons(
        disk: &ExfatMemoryDisk,
        super_block: &crate::fs::fs_impls::exfat_refactor::super_block::ExfatSuperBlock,
        start_index: usize,
    ) {
        let root_dir_offset = super_block
            .cluster_to_byte_offset(super_block.root_dir)
            .unwrap();
        disk.write_bytes(
            root_dir_offset + start_index * DENTRY_SIZE,
            ExfatDentry::Unused.as_bytes(),
        );
    }

    fn prepared_directory_root(
        record_specs: &[(&str, u16)],
    ) -> (Arc<ExfatFs>, Arc<ExfatInode>, usize) {
        let disk = load_exfat_disk();
        let super_block = read_primary_super_block(&disk).unwrap();
        let (upcase_dentry, raw_table_bytes) = valid_upcase_fixture();
        write_upcase_prerequisite(&disk, &super_block, upcase_dentry, &raw_table_bytes);

        let insertion_index = first_root_system_entry_end(&disk, &super_block);
        let cluster_size = super_block.cluster_size();
        let root_start_cluster = super_block.root_dir;
        let records = record_specs
            .iter()
            .map(|(name, file_attribute)| {
                file_record(
                    name,
                    *file_attribute,
                    root_start_cluster,
                    cluster_size as u64,
                )
            })
            .collect::<Vec<_>>();
        write_file_records(&disk, &super_block, insertion_index, &records);

        let block_device: Arc<dyn BlockDevice> = Arc::new(disk);
        let fs = Arc::new(ExfatFs::new(block_device.clone(), super_block).unwrap());
        fs.install_upcase_table(upcase_dentry, &raw_table_bytes)
            .unwrap();

        let chain = ExfatChain::new(
            block_device.as_ref(),
            &super_block,
            super_block.root_dir,
            Some(1),
            ChainMode::Contiguous,
        )
        .unwrap();
        let root_dentry_set = trusted_dentry_set(0, 0, 0x10, chain.current_cluster());
        let metadata = Metadata {
            ino: 1,
            size: 0,
            optimal_block_size: cluster_size,
            nr_sectors_allocated: 0,
            last_access_at: Duration::ZERO,
            last_modify_at: Duration::ZERO,
            last_meta_change_at: Duration::ZERO,
            type_: InodeType::Dir,
            mode: InodeMode::S_IRUSR,
            nr_hard_links: 1,
            uid: Uid::new(0),
            gid: Gid::new(0),
            container_dev_id: block_device.id(),
            self_dev_id: None,
        };
        let root_inode = ExfatInode::new(
            Arc::downgrade(&fs),
            metadata,
            &root_dentry_set,
            &chain,
            cluster_size,
            Some(ExfatInodeLocation::new(None, 0, 0)),
        )
        .unwrap();

        (fs, root_inode, insertion_index)
    }

    fn prepared_clean_directory_root(
        record_specs: &[(&str, u16)],
    ) -> (Arc<ExfatFs>, Arc<ExfatInode>) {
        let disk = load_exfat_disk();
        let super_block = read_primary_super_block(&disk).unwrap();
        let (upcase_dentry, raw_table_bytes) = valid_upcase_fixture();
        write_upcase_prerequisite(&disk, &super_block, upcase_dentry, &raw_table_bytes);

        let insertion_index = first_root_system_entry_end(&disk, &super_block);
        clear_root_visible_entries_after_singletons(&disk, &super_block, insertion_index);

        let cluster_size = super_block.cluster_size();
        let root_start_cluster = super_block.root_dir;
        let records = record_specs
            .iter()
            .map(|(name, file_attribute)| {
                file_record(
                    name,
                    *file_attribute,
                    root_start_cluster,
                    cluster_size as u64,
                )
            })
            .collect::<Vec<_>>();
        write_file_records(&disk, &super_block, insertion_index, &records);

        let block_device: Arc<dyn BlockDevice> = Arc::new(disk);
        let fs = Arc::new(ExfatFs::new(block_device.clone(), super_block).unwrap());
        fs.install_upcase_table(upcase_dentry, &raw_table_bytes)
            .unwrap();

        let chain = ExfatChain::new(
            block_device.as_ref(),
            &super_block,
            super_block.root_dir,
            Some(1),
            ChainMode::Contiguous,
        )
        .unwrap();
        let root_dentry_set = trusted_dentry_set(0, 0, 0x10, chain.current_cluster());
        let metadata = Metadata {
            ino: 1,
            size: 0,
            optimal_block_size: cluster_size,
            nr_sectors_allocated: 0,
            last_access_at: Duration::ZERO,
            last_modify_at: Duration::ZERO,
            last_meta_change_at: Duration::ZERO,
            type_: InodeType::Dir,
            mode: InodeMode::S_IRUSR,
            nr_hard_links: 1,
            uid: Uid::new(0),
            gid: Gid::new(0),
            container_dev_id: block_device.id(),
            self_dev_id: None,
        };
        let root_inode = ExfatInode::new(
            Arc::downgrade(&fs),
            metadata,
            &root_dentry_set,
            &chain,
            cluster_size,
            Some(ExfatInodeLocation::new(None, 0, 0)),
        )
        .unwrap();

        (fs, root_inode)
    }

    struct CapturingDirentVisitor {
        limit: Option<usize>,
        entries: Vec<(String, u64, InodeType, usize)>,
    }

    impl CapturingDirentVisitor {
        fn unlimited() -> Self {
            Self {
                limit: None,
                entries: Vec::new(),
            }
        }

        fn with_limit(limit: usize) -> Self {
            Self {
                limit: Some(limit),
                entries: Vec::new(),
            }
        }
    }

    impl DirentVisitor for CapturingDirentVisitor {
        fn visit(&mut self, name: &str, ino: u64, type_: InodeType, offset: usize) -> Result<()> {
            if self.limit.is_some_and(|limit| self.entries.len() == limit) {
                return Err(Error::new(Errno::EINTR));
            }

            self.entries.push((String::from(name), ino, type_, offset));
            Ok(())
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

    fn prepared_mapping_context() -> (Arc<dyn BlockDevice>, ExfatSuperBlock, Arc<ExfatFs>) {
        let block_device: Arc<dyn BlockDevice> = Arc::new(load_exfat_disk());
        let super_block = read_primary_super_block(block_device.as_ref()).unwrap();
        let fs = Arc::new(ExfatFs::new(block_device.clone(), super_block).unwrap());

        (block_device, super_block, fs)
    }

    fn mapping_test_inode(
        fs: &Arc<ExfatFs>,
        block_device: &Arc<dyn BlockDevice>,
        super_block: &ExfatSuperBlock,
        start_cluster: ClusterId,
        cluster_count: u32,
        file_size: u64,
        valid_size: u64,
    ) -> Arc<ExfatInode> {
        let cluster_size = super_block.cluster_size();
        let chain = ExfatChain::new(
            block_device.as_ref(),
            super_block,
            start_cluster,
            Some(cluster_count),
            ChainMode::Contiguous,
        )
        .unwrap();
        let metadata = Metadata {
            ino: 64,
            size: 0,
            optimal_block_size: SECTOR_SIZE,
            nr_sectors_allocated: 0,
            last_access_at: Duration::ZERO,
            last_modify_at: Duration::ZERO,
            last_meta_change_at: Duration::ZERO,
            type_: InodeType::File,
            mode: InodeMode::S_IRUSR,
            nr_hard_links: 1,
            uid: Uid::new(0),
            gid: Gid::new(0),
            container_dev_id: block_device.id(),
            self_dev_id: None,
        };
        let dentry_set = trusted_dentry_set(file_size, valid_size, 0x20, start_cluster);

        ExfatInode::new(
            Arc::downgrade(fs),
            metadata,
            &dentry_set,
            &chain,
            cluster_size,
            None,
        )
        .unwrap()
    }

    fn prepared_buffered_read_context() -> (Arc<ExfatMemoryDisk>, ExfatSuperBlock, Arc<ExfatFs>) {
        let disk = Arc::new(load_exfat_disk());
        let super_block = read_primary_super_block(disk.as_ref()).unwrap();
        let block_device: Arc<dyn BlockDevice> = disk.clone();
        let fs = Arc::new(ExfatFs::new(block_device, super_block).unwrap());

        (disk, super_block, fs)
    }

    fn buffered_read_test_inode(
        fs: &Arc<ExfatFs>,
        disk: &Arc<ExfatMemoryDisk>,
        super_block: &ExfatSuperBlock,
        start_cluster: ClusterId,
        cluster_count: u32,
        file_size: u64,
        valid_size: u64,
    ) -> Arc<ExfatInode> {
        let block_device: Arc<dyn BlockDevice> = disk.clone();
        mapping_test_inode(
            fs,
            &block_device,
            super_block,
            start_cluster,
            cluster_count,
            file_size,
            valid_size,
        )
    }

    fn patterned_file_bytes(len: usize) -> Vec<u8> {
        (0..len).map(|index| (index % 251) as u8).collect()
    }

    fn write_contiguous_file_bytes(
        disk: &ExfatMemoryDisk,
        super_block: &ExfatSuperBlock,
        start_cluster: ClusterId,
        file_bytes: &[u8],
    ) {
        let file_byte_offset = super_block.cluster_to_byte_offset(start_cluster).unwrap();
        disk.write_bytes(file_byte_offset, file_bytes);
    }

    fn committed_page_bytes(page_cache: &Arc<Vmo>, page_index: usize) -> Vec<u8> {
        let frame = page_cache
            .commit_on(page_index, CommitFlags::empty())
            .unwrap();
        let mut bytes = vec![0xA5; PAGE_SIZE];
        frame.read_bytes(0, &mut bytes).unwrap();
        bytes
    }

    // Confirms copied metadata, weak FS owner recovery, and buffered read ownership.
    #[ktest]
    fn inode_carrier_snapshots_metadata_and_exercises_buffered_read() {
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
        let block_device = disk.clone();
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
        let mut expected_read = [0u8; 4];
        read_metadata_bytes(
            block_device.as_ref(),
            super_block
                .cluster_to_byte_offset(chain.current_cluster())
                .unwrap(),
            &mut expected_read,
        )
        .unwrap();
        let read_len = inode
            .read_at(0, &mut read_writer, StatusFlags::empty())
            .unwrap();
        assert_eq!(read_len, expected_read.len());
        assert_eq!(read_buffer, expected_read);

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

    // Confirms regular-file snapshots own their page-cache attachment and size facts.
    #[ktest]
    fn inode_page_cache_attachment_stays_inode_local_for_regular_file_snapshot() {
        let (disk, super_block, fs) = prepared_buffered_read_context();
        let file_size = PAGE_SIZE + 64;
        let file_bytes = patterned_file_bytes(file_size);
        let inode_a = buffered_read_test_inode(
            &fs,
            &disk,
            &super_block,
            super_block.root_dir,
            2,
            file_size as u64,
            file_size as u64,
        );
        let inode_b = buffered_read_test_inode(
            &fs,
            &disk,
            &super_block,
            super_block.root_dir,
            2,
            file_size as u64,
            file_size as u64,
        );
        write_contiguous_file_bytes(
            disk.as_ref(),
            &super_block,
            super_block.root_dir,
            &file_bytes,
        );

        let page_cache_a = inode_a.page_cache().expect("regular files own a page cache");
        let page_cache_b = inode_b.page_cache().expect("regular files own a page cache");
        let expected_cache_size = file_size.div_ceil(PAGE_SIZE) * PAGE_SIZE;

        assert_eq!(page_cache_a.size(), expected_cache_size);
        assert_eq!(page_cache_a.size(), page_cache_b.size());
        assert_eq!(
            inode_a.page_cache.get().unwrap().backend().npages(),
            inode_a.size().div_ceil(PAGE_SIZE)
        );
        assert!(!Arc::ptr_eq(&page_cache_a, &page_cache_b));
    }

    // Confirms cache misses are filled through the inode owner for backed bytes.
    #[ktest]
    fn inode_page_cache_backend_fills_backed_bytes_through_inode_owner() {
        let (disk, super_block, fs) = prepared_buffered_read_context();
        let file_size = PAGE_SIZE + 64;
        let file_bytes = patterned_file_bytes(file_size);
        let inode = buffered_read_test_inode(
            &fs,
            &disk,
            &super_block,
            super_block.root_dir,
            2,
            file_size as u64,
            file_size as u64,
        );
        write_contiguous_file_bytes(
            disk.as_ref(),
            &super_block,
            super_block.root_dir,
            &file_bytes,
        );

        let page_cache = inode.page_cache().expect("regular files own a page cache");
        let page_bytes = committed_page_bytes(&page_cache, 0);

        assert_eq!(&page_bytes[..PAGE_SIZE], &file_bytes[..PAGE_SIZE]);
        assert_eq!(inode.page_cache.get().unwrap().backend().npages(), 2);
    }

    // Confirms cache-visible data keeps the valid-size gap and EOF zero-fill rules.
    #[ktest]
    fn inode_page_cache_preserves_valid_size_gap_and_eof_zero_fill() {
        let (disk, super_block, fs) = prepared_buffered_read_context();
        let file_size = PAGE_SIZE + 64;
        let valid_size = PAGE_SIZE + 16;
        let file_bytes = patterned_file_bytes(valid_size);
        let inode = buffered_read_test_inode(
            &fs,
            &disk,
            &super_block,
            super_block.root_dir,
            2,
            file_size as u64,
            valid_size as u64,
        );
        write_contiguous_file_bytes(
            disk.as_ref(),
            &super_block,
            super_block.root_dir,
            &file_bytes,
        );

        let page_cache = inode.page_cache().expect("regular files own a page cache");
        let page_bytes = committed_page_bytes(&page_cache, 1);

        assert_eq!(&page_bytes[..16], &file_bytes[PAGE_SIZE..valid_size]);
        assert_eq!(&page_bytes[16..64], &[0; 48]);
        assert_eq!(&page_bytes[64..], &[0; PAGE_SIZE - 64]);
    }

    // Confirms repeated cache-backed commits stay stable on one inode snapshot.
    #[ktest]
    fn inode_page_cache_repeated_commit_reads_are_stable_on_one_snapshot() {
        let (disk, super_block, fs) = prepared_buffered_read_context();
        let file_size = PAGE_SIZE + 64;
        let valid_size = PAGE_SIZE + 16;
        let file_bytes = patterned_file_bytes(valid_size);
        let inode = buffered_read_test_inode(
            &fs,
            &disk,
            &super_block,
            super_block.root_dir,
            2,
            file_size as u64,
            valid_size as u64,
        );
        write_contiguous_file_bytes(
            disk.as_ref(),
            &super_block,
            super_block.root_dir,
            &file_bytes,
        );

        let page_cache = inode.page_cache().expect("regular files own a page cache");
        let first_page_bytes = committed_page_bytes(&page_cache, 1);
        let second_page_bytes = committed_page_bytes(&page_cache, 1);

        assert_eq!(first_page_bytes, second_page_bytes);
        assert_eq!(&first_page_bytes[..16], &file_bytes[PAGE_SIZE..valid_size]);
        assert_eq!(&first_page_bytes[16..64], &[0; 48]);
    }

    // Confirms logical offsets resolve to the expected cluster position and byte offset.
    #[ktest]
    fn file_mapping_translates_logical_offsets_to_expected_physical_ranges() {
        let (block_device, super_block, fs) = prepared_mapping_context();
        let cluster_size = super_block.cluster_size();
        let cluster_size_u64 = u64::try_from(cluster_size).unwrap();
        let start_cluster = super_block.root_dir;
        let inode = mapping_test_inode(
            &fs,
            &block_device,
            &super_block,
            start_cluster,
            3,
            cluster_size_u64 * 3,
            cluster_size_u64 * 3,
        );
        let snapshot_before = (
            inode.metadata().size,
            inode.valid_size,
            inode.start_cluster,
            inode.cluster_count,
            inode.allocated_size,
        );

        let boundary_range = inode
            .map_physical_file_range(
                block_device.as_ref(),
                &super_block,
                cluster_size,
                cluster_size,
            )
            .unwrap()
            .unwrap();
        let mid_cluster_range = inode
            .map_physical_file_range(block_device.as_ref(), &super_block, cluster_size + 17, 99)
            .unwrap()
            .unwrap();

        assert_eq!(
            boundary_range,
            PhysicalFileRange {
                cluster: start_cluster + 1,
                physical_byte_offset: super_block
                    .cluster_to_byte_offset(start_cluster + 1)
                    .unwrap(),
                intra_cluster_offset: 0,
                mappable_byte_count: cluster_size,
            }
        );
        assert_eq!(
            mid_cluster_range,
            PhysicalFileRange {
                cluster: start_cluster + 1,
                physical_byte_offset: super_block
                    .cluster_to_byte_offset(start_cluster + 1)
                    .unwrap()
                    + 17,
                intra_cluster_offset: 17,
                mappable_byte_count: 99,
            }
        );
        assert_eq!(
            (
                inode.metadata().size,
                inode.valid_size,
                inode.start_cluster,
                inode.cluster_count,
                inode.allocated_size,
            ),
            snapshot_before
        );
    }

    // Confirms the mapped span stops at size, valid-size, and cluster boundaries.
    #[ktest]
    fn file_mapping_mappable_span_respects_size_facts_and_cluster_geometry() {
        let (block_device, super_block, fs) = prepared_mapping_context();
        let cluster_size = super_block.cluster_size();
        let cluster_size_u64 = u64::try_from(cluster_size).unwrap();
        let start_cluster = super_block.root_dir;

        let size_limited = mapping_test_inode(
            &fs,
            &block_device,
            &super_block,
            start_cluster,
            3,
            cluster_size_u64 + 40,
            cluster_size_u64 * 3,
        );
        let valid_size_limited = mapping_test_inode(
            &fs,
            &block_device,
            &super_block,
            start_cluster,
            3,
            cluster_size_u64 * 3,
            cluster_size_u64 + 24,
        );
        let cluster_limited = mapping_test_inode(
            &fs,
            &block_device,
            &super_block,
            start_cluster,
            3,
            cluster_size_u64 * 3,
            cluster_size_u64 * 3,
        );

        let size_bound_range = size_limited
            .map_physical_file_range(block_device.as_ref(), &super_block, cluster_size, 128)
            .unwrap()
            .unwrap();
        let valid_size_bound_range = valid_size_limited
            .map_physical_file_range(block_device.as_ref(), &super_block, cluster_size, 128)
            .unwrap()
            .unwrap();
        let cluster_bound_range = cluster_limited
            .map_physical_file_range(
                block_device.as_ref(),
                &super_block,
                cluster_size * 2 - 13,
                256,
            )
            .unwrap()
            .unwrap();

        assert_eq!(size_bound_range.cluster, start_cluster + 1);
        assert_eq!(size_bound_range.intra_cluster_offset, 0);
        assert_eq!(size_bound_range.mappable_byte_count, 40);
        assert_eq!(valid_size_bound_range.cluster, start_cluster + 1);
        assert_eq!(valid_size_bound_range.intra_cluster_offset, 0);
        assert_eq!(valid_size_bound_range.mappable_byte_count, 24);
        assert_eq!(cluster_bound_range.cluster, start_cluster + 1);
        assert_eq!(cluster_bound_range.intra_cluster_offset, cluster_size - 13);
        assert_eq!(cluster_bound_range.mappable_byte_count, 13);
    }

    // Confirms repeated translation calls return the same result for one inode snapshot.
    #[ktest]
    fn file_mapping_repeated_calls_are_stable_on_one_snapshot() {
        let (block_device, super_block, fs) = prepared_mapping_context();
        let cluster_size = super_block.cluster_size();
        let cluster_size_u64 = u64::try_from(cluster_size).unwrap();
        let inode = mapping_test_inode(
            &fs,
            &block_device,
            &super_block,
            super_block.root_dir,
            3,
            cluster_size_u64 * 3,
            cluster_size_u64 * 3,
        );

        let first_range = inode
            .map_physical_file_range(block_device.as_ref(), &super_block, cluster_size + 33, 64)
            .unwrap();
        let second_range = inode
            .map_physical_file_range(block_device.as_ref(), &super_block, cluster_size + 33, 64)
            .unwrap();

        assert_eq!(first_range, second_range);
    }

    // Confirms empty and fully unbacked requests stay explicit instead of inventing read policy.
    #[ktest]
    fn file_mapping_empty_or_unbacked_requests_stay_explicit() {
        let (block_device, super_block, fs) = prepared_mapping_context();
        let cluster_size = super_block.cluster_size();
        let cluster_size_u64 = u64::try_from(cluster_size).unwrap();
        let inode = mapping_test_inode(
            &fs,
            &block_device,
            &super_block,
            super_block.root_dir,
            2,
            cluster_size_u64 * 3,
            cluster_size_u64 * 3,
        );

        let zero_len_range = inode
            .map_physical_file_range(block_device.as_ref(), &super_block, cluster_size, 0)
            .unwrap();
        let beyond_allocated_range = inode
            .map_physical_file_range(block_device.as_ref(), &super_block, cluster_size * 2, 1)
            .unwrap();

        assert_eq!(zero_len_range, None);
        assert_eq!(beyond_allocated_range, None);
    }

    // Confirms buffered reads copy backed bytes and stop at logical EOF.
    #[ktest]
    fn file_buffered_read_copies_backed_bytes_and_truncates_at_eof() {
        let (disk, super_block, fs) = prepared_buffered_read_context();
        let file_bytes = patterned_file_bytes(96);
        let inode =
            buffered_read_test_inode(&fs, &disk, &super_block, super_block.root_dir, 1, 96, 96);
        write_contiguous_file_bytes(
            disk.as_ref(),
            &super_block,
            super_block.root_dir,
            &file_bytes,
        );

        let mut read_buffer = [0xCC; 32];
        let mut read_writer = VmWriter::from(read_buffer.as_mut_slice()).to_fallible();
        let read_len = inode
            .read_at(80, &mut read_writer, StatusFlags::empty())
            .unwrap();

        assert_eq!(read_len, 16);
        assert_eq!(&read_buffer[..16], &file_bytes[80..96]);
        assert_eq!(&read_buffer[16..], &[0xCC; 16]);
    }

    // Confirms reads crossing valid_size return data first, then zeros only up to EOF.
    #[ktest]
    fn file_buffered_read_zero_fills_from_valid_size_to_logical_eof() {
        let (disk, super_block, fs) = prepared_buffered_read_context();
        let file_bytes = patterned_file_bytes(40);
        let inode =
            buffered_read_test_inode(&fs, &disk, &super_block, super_block.root_dir, 1, 64, 40);
        write_contiguous_file_bytes(
            disk.as_ref(),
            &super_block,
            super_block.root_dir,
            &file_bytes,
        );

        let mut read_buffer = [0xA5; 40];
        let mut read_writer = VmWriter::from(read_buffer.as_mut_slice()).to_fallible();
        let read_len = inode
            .read_at(32, &mut read_writer, StatusFlags::empty())
            .unwrap();

        assert_eq!(read_len, 32);
        assert_eq!(&read_buffer[..8], &file_bytes[32..40]);
        assert_eq!(&read_buffer[8..32], &[0; 24]);
        assert_eq!(&read_buffer[32..], &[0xA5; 8]);
    }

    // Confirms reads starting at or past EOF return zero and leave caller bytes unchanged.
    #[ktest]
    fn file_buffered_read_at_or_beyond_eof_returns_zero_without_mutation() {
        let (disk, super_block, fs) = prepared_buffered_read_context();
        let file_bytes = patterned_file_bytes(48);
        let inode =
            buffered_read_test_inode(&fs, &disk, &super_block, super_block.root_dir, 1, 48, 48);
        write_contiguous_file_bytes(
            disk.as_ref(),
            &super_block,
            super_block.root_dir,
            &file_bytes,
        );

        let mut eof_buffer = [0x5A; 12];
        let mut eof_writer = VmWriter::from(eof_buffer.as_mut_slice()).to_fallible();
        let eof_read_len = inode
            .read_at(48, &mut eof_writer, StatusFlags::empty())
            .unwrap();

        let mut past_eof_buffer = [0x5A; 12];
        let mut past_eof_writer = VmWriter::from(past_eof_buffer.as_mut_slice()).to_fallible();
        let past_eof_read_len = inode
            .read_at(53, &mut past_eof_writer, StatusFlags::empty())
            .unwrap();

        assert_eq!(eof_read_len, 0);
        assert_eq!(past_eof_read_len, 0);
        assert_eq!(eof_buffer, [0x5A; 12]);
        assert_eq!(past_eof_buffer, [0x5A; 12]);
    }

    // Confirms repeated reads on one inode snapshot stay deterministic across data and zero-fill.
    #[ktest]
    fn file_buffered_read_repeated_calls_are_stable_on_one_snapshot() {
        let (disk, super_block, fs) = prepared_buffered_read_context();
        let cluster_size = super_block.cluster_size();
        let valid_size = cluster_size + 32;
        let file_size = cluster_size + 96;
        let file_bytes = patterned_file_bytes(valid_size);
        let inode = buffered_read_test_inode(
            &fs,
            &disk,
            &super_block,
            super_block.root_dir,
            2,
            file_size as u64,
            valid_size as u64,
        );
        write_contiguous_file_bytes(
            disk.as_ref(),
            &super_block,
            super_block.root_dir,
            &file_bytes,
        );

        let mut first_buffer = [0x7E; 96];
        let mut first_writer = VmWriter::from(first_buffer.as_mut_slice()).to_fallible();
        let first_read_len = inode
            .read_at(cluster_size - 24, &mut first_writer, StatusFlags::empty())
            .unwrap();

        let mut second_buffer = [0x7E; 96];
        let mut second_writer = VmWriter::from(second_buffer.as_mut_slice()).to_fallible();
        let second_read_len = inode
            .read_at(cluster_size - 24, &mut second_writer, StatusFlags::empty())
            .unwrap();

        let mut expected_buffer = [0x7E; 96];
        expected_buffer[..56].copy_from_slice(&file_bytes[cluster_size - 24..valid_size]);
        expected_buffer[56..].fill(0);

        assert_eq!(first_read_len, 96);
        assert_eq!(second_read_len, first_read_len);
        assert_eq!(first_buffer, expected_buffer);
        assert_eq!(second_buffer, expected_buffer);
    }

    // Confirms lookup folds names through the installed table, derives the key from
    // trusted location facts, and reuses one canonical opened child handle.
    #[ktest]
    fn lookup_reuses_the_canonical_child_handle_for_case_equivalent_names() {
        let (fs, root, insertion_index) = prepared_directory_root(&[("readme", 0x20)]);
        assert_eq!(fs.opened_inode_count(), 0);

        let mut directory_stream = fs
            .directory_stream(
                Some(root.ino()),
                root.start_cluster,
                root.cluster_count,
                root.chain_mode,
            )
            .unwrap();
        let file_record = loop {
            match directory_stream.next_record().unwrap() {
                Some(DirectoryRecord::File(file_record)) => break file_record,
                Some(DirectoryRecord::Singleton(_)) => continue,
                None => panic!("expected one visible file record"),
            }
        };

        let expected_location = (
            Some(root.ino()),
            insertion_index * DENTRY_SIZE,
            u32::try_from(insertion_index).unwrap(),
        );
        assert_eq!(file_record.location().inode_key_parts(), expected_location);

        let canonical_child = fs.resolve_or_publish_child_inode(&file_record).unwrap();
        let first_lookup = root.lookup("README").unwrap();
        let second_lookup = root.lookup("readme").unwrap();
        let canonical_child_as_inode: Arc<dyn Inode> = canonical_child.clone();

        assert_eq!(fs.opened_inode_count(), 1);
        assert!(Arc::ptr_eq(&first_lookup, &second_lookup));
        assert!(Arc::ptr_eq(&first_lookup, &canonical_child_as_inode));
    }

    // Confirms a miss stays read-only and leaves the opened-child table unchanged.
    #[ktest]
    fn lookup_miss_does_not_publish_a_synthetic_child_handle() {
        let (fs, root, _) = prepared_directory_root(&[("present", 0x20)]);
        assert_eq!(fs.opened_inode_count(), 0);

        let miss_error = root.lookup("absent").unwrap_err();
        assert_eq!(miss_error.error(), Errno::ENOENT);
        assert_eq!(fs.opened_inode_count(), 0);

        let present = root.lookup("present").unwrap();
        let reopened = root.lookup("PRESENT").unwrap();
        assert_eq!(fs.opened_inode_count(), 1);
        assert!(Arc::ptr_eq(&present, &reopened));
    }

    // Confirms readdir emits visible entries in order and hides the root singletons.
    #[ktest]
    fn readdir_emits_visible_entries_in_stable_order() {
        let (fs, root) = prepared_clean_directory_root(&[("alpha", 0x20), ("beta", 0x20)]);
        let _fs = fs;
        let mut visitor = CapturingDirentVisitor::unlimited();

        let read_count = root.readdir_at(0, &mut visitor).unwrap();
        let names = visitor
            .entries
            .iter()
            .map(|(name, _, _, _)| name.clone())
            .collect::<Vec<_>>();
        let offsets = visitor
            .entries
            .iter()
            .map(|(_, _, _, offset)| *offset)
            .collect::<Vec<_>>();

        assert_eq!(names, vec![String::from("alpha"), String::from("beta")]);
        assert_eq!(offsets, vec![1, 2]);
        assert_eq!(read_count, 2);
    }

    // Confirms readdir resumes from the returned offset and reproduces the same prefix.
    #[ktest]
    fn readdir_continuation_remains_stable_across_repeated_calls() {
        let (fs, root) =
            prepared_clean_directory_root(&[("alpha", 0x20), ("beta", 0x20), ("gamma", 0x20)]);
        let _fs = fs;

        let mut first_page = CapturingDirentVisitor::with_limit(2);
        let mut first_offset = 0usize;
        let first_read_count = root.readdir_at(first_offset, &mut first_page).unwrap();
        first_offset += first_read_count;
        let first_names = first_page
            .entries
            .iter()
            .map(|(name, _, _, _)| name.clone())
            .collect::<Vec<_>>();

        let mut resumed_page = CapturingDirentVisitor::unlimited();
        let mut resumed_offset = first_offset;
        let resumed_read_count = root.readdir_at(resumed_offset, &mut resumed_page).unwrap();
        resumed_offset += resumed_read_count;
        let resumed_names = resumed_page
            .entries
            .iter()
            .map(|(name, _, _, _)| name.clone())
            .collect::<Vec<_>>();

        let mut repeated_first_page = CapturingDirentVisitor::with_limit(2);
        let mut repeated_first_offset = 0usize;
        let repeated_first_read_count = root.readdir_at(0, &mut repeated_first_page).unwrap();
        repeated_first_offset += repeated_first_read_count;
        let repeated_first_names = repeated_first_page
            .entries
            .iter()
            .map(|(name, _, _, _)| name.clone())
            .collect::<Vec<_>>();

        assert_eq!(
            first_names,
            vec![String::from("alpha"), String::from("beta")]
        );
        assert_eq!(first_read_count, 2);
        assert_eq!(first_offset, 2);
        assert_eq!(resumed_names, vec![String::from("gamma")]);
        assert_eq!(resumed_read_count, 1);
        assert_eq!(resumed_offset, 3);
        assert_eq!(repeated_first_names, first_names);
        assert_eq!(repeated_first_read_count, first_read_count);
        assert_eq!(repeated_first_offset, first_offset);
    }
}
