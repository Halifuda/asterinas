// SPDX-License-Identifier: MPL-2.0

use alloc::{string::String, vec, vec::Vec};
use core::time::Duration;

use aster_block::BlockDevice;
use ostd::mm::VmIo;

use super::{
    boot::BootRegion,
    fat::{ChainVisitControl, FatReader},
    fs::{ExfatFs, MountVolumeStateError},
    upcase::UpcaseTable,
};
use crate::{
    fs::{
        file::{AccessMode, FileIo, InodeMode, InodeType, StatusFlags, mkmod},
        utils::DirentVisitor,
        vfs::{
            file_system::FileSystem,
            inode::{Extension, FallocMode, Inode, Metadata, MknodType, SymbolicLink},
        },
    },
    prelude::*,
    process::{Gid, Uid},
    vm::vmo::Vmo,
};

const END_OF_DIRECTORY_ENTRY_TYPE: u8 = 0x00;
const ALLOCATION_BITMAP_ENTRY_TYPE: u8 = 0x81;
const UPCASE_TABLE_ENTRY_TYPE: u8 = 0x82;
const VOLUME_LABEL_ENTRY_TYPE: u8 = 0x83;
const FILE_DIRECTORY_ENTRY_TYPE: u8 = 0x85;
const STREAM_EXTENSION_ENTRY_TYPE: u8 = 0xC0;
const FILE_NAME_ENTRY_TYPE: u8 = 0xC1;
const DIRECTORY_ENTRY_SIZE: usize = 32;
const ENTRY_TYPE_IMPORTANCE_BIT: u8 = 0x20;
const ENTRY_TYPE_CATEGORY_BIT: u8 = 0x40;
const ENTRY_TYPE_IN_USE_BIT: u8 = 0x80;
const FILE_ATTRIBUTE_DIRECTORY: u16 = 0x0010;

pub(super) struct ExfatInode {
    data_length: Option<usize>,
    extension: Extension,
    first_cluster: u32,
    fs: Weak<ExfatFs>,
    metadata: RwLock<Metadata>,
    no_fat_chain: bool,
    this: Weak<Self>,
}

impl ExfatInode {
    fn new(
        fs: &Arc<ExfatFs>,
        metadata: Metadata,
        first_cluster: u32,
        data_length: Option<usize>,
        no_fat_chain: bool,
    ) -> Arc<Self> {
        Arc::new_cyclic(|weak_self| Self {
            data_length,
            extension: Extension::new(),
            first_cluster,
            fs: Arc::downgrade(fs),
            metadata: RwLock::new(metadata),
            no_fat_chain,
            this: weak_self.clone(),
        })
    }

    pub(super) fn new_root(
        fs: &Arc<ExfatFs>,
        root_cluster: u32,
        cluster_size: usize,
    ) -> Arc<Self> {
        let mut metadata = Metadata::new_dir(
            u64::from(root_cluster),
            mkmod!(u+rwx, g+rx, o+rx),
            cluster_size,
            fs.container_device_id(),
        );
        metadata.size = cluster_size;
        Self::new(fs, metadata, root_cluster, None, false)
    }

    fn new_child(
        fs: &Arc<ExfatFs>,
        ino: u64,
        inode_type: InodeType,
        cluster_size: usize,
        size: usize,
        first_cluster: u32,
        data_length: usize,
        no_fat_chain: bool,
    ) -> Arc<Self> {
        let mut metadata = match inode_type {
            InodeType::Dir => Metadata::new_dir(
                ino,
                mkmod!(u+rwx, g+rx, o+rx),
                cluster_size,
                fs.container_device_id(),
            ),
            _ => Metadata::new_file(
                ino,
                mkmod!(u+rw, g+r, o+r),
                cluster_size,
                fs.container_device_id(),
            ),
        };
        metadata.size = size;
        Self::new(fs, metadata, first_cluster, Some(data_length), no_fat_chain)
    }

    fn read_directory_bytes(
        &self,
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
    ) -> core::result::Result<Vec<u8>, MountVolumeStateError> {
        let Some(data_length) = self.data_length else {
            let mut directory_bytes = Vec::new();
            let mut fat_reader = FatReader::new(block_device.as_ref(), boot_region);
            fat_reader.walk_cluster_chain(self.first_cluster, |_, cluster_bytes| {
                directory_bytes.extend_from_slice(cluster_bytes);
                Ok(ChainVisitControl::Continue)
            })?;
            return Ok(directory_bytes);
        };

        if data_length == 0 {
            if self.first_cluster != 0 {
                return Err(MountVolumeStateError::InvalidOnDiskLayout);
            }
            return Ok(Vec::new());
        }
        if data_length % DIRECTORY_ENTRY_SIZE != 0 {
            return Err(MountVolumeStateError::InvalidOnDiskLayout);
        }

        let data_length_u64 =
            u64::try_from(data_length).map_err(|_| MountVolumeStateError::InvalidOnDiskLayout)?;
        boot_region.validate_stream_data(self.first_cluster, data_length_u64)?;

        if self.no_fat_chain {
            let cluster_count = data_length.div_ceil(boot_region.cluster_size);
            let mut directory_bytes = Vec::with_capacity(data_length);
            for cluster_offset in 0..cluster_count {
                let cluster = self
                    .first_cluster
                    .checked_add(
                        u32::try_from(cluster_offset)
                            .map_err(|_| MountVolumeStateError::InvalidOnDiskLayout)?,
                    )
                    .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
                if !boot_region.is_valid_cluster(cluster) {
                    return Err(MountVolumeStateError::InvalidOnDiskLayout);
                }
                let cluster_start = boot_region.cluster_offset(cluster)?;
                let mut cluster_bytes = vec![0; boot_region.cluster_size];
                block_device
                    .read_bytes(cluster_start, &mut cluster_bytes)
                    .map_err(|_| MountVolumeStateError::DeviceIo)?;
                let bytes_to_copy = cluster_bytes
                    .len()
                    .min(data_length.saturating_sub(directory_bytes.len()));
                directory_bytes.extend_from_slice(&cluster_bytes[..bytes_to_copy]);
            }
            return Ok(directory_bytes);
        }

        let mut remaining = data_length;
        let mut directory_bytes = Vec::with_capacity(data_length);
        let mut fat_reader = FatReader::new(block_device.as_ref(), boot_region);
        fat_reader.walk_cluster_chain(self.first_cluster, |_, cluster_bytes| {
            let bytes_to_copy = remaining.min(cluster_bytes.len());
            directory_bytes.extend_from_slice(&cluster_bytes[..bytes_to_copy]);
            remaining -= bytes_to_copy;
            if remaining == 0 {
                return Ok(ChainVisitControl::Stop);
            }
            Ok(ChainVisitControl::Continue)
        })?;
        if remaining != 0 {
            return Err(MountVolumeStateError::InvalidOnDiskLayout);
        }
        Ok(directory_bytes)
    }

    fn lookup_child_by_name(
        &self,
        fs: &Arc<ExfatFs>,
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        upcase_table: &UpcaseTable,
        lookup_name: &[u16],
        lookup_name_hash: u16,
    ) -> core::result::Result<Option<Arc<dyn Inode>>, MountVolumeStateError> {
        let directory_bytes = self.read_directory_bytes(block_device, boot_region)?;
        let mut entry_index = 0usize;
        while let Some(entry_offset) = entry_index.checked_mul(DIRECTORY_ENTRY_SIZE) {
            let entry_end = entry_offset
                .checked_add(DIRECTORY_ENTRY_SIZE)
                .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
            let Some(entry) = directory_bytes.get(entry_offset..entry_end) else {
                return Ok(None);
            };

            match entry[0] {
                END_OF_DIRECTORY_ENTRY_TYPE => return Ok(None),
                0x01..=0x7F => {
                    entry_index = entry_index
                        .checked_add(1)
                        .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
                }
                FILE_DIRECTORY_ENTRY_TYPE => {
                    let secondary_count = usize::from(entry[1]);
                    let expected_checksum = u16::from_le_bytes([entry[2], entry[3]]);
                    let entry_set = Self::validated_file_entry_set(
                        &directory_bytes,
                        entry_offset,
                        secondary_count,
                        expected_checksum,
                    )?;
                    let stream_entry = Self::file_stream_entry(entry_set)?;
                    let stored_name_hash = u16::from_le_bytes([stream_entry[4], stream_entry[5]]);
                    let candidate_name =
                        Self::file_name(entry_set, secondary_count, stream_entry)?;

                    if stored_name_hash == lookup_name_hash
                        && upcase_table.names_equal(lookup_name, &candidate_name)
                    {
                        let (inode_type, first_cluster, data_length, no_fat_chain) =
                            Self::file_entry_child_metadata(entry, stream_entry, boot_region)?;
                        let ino = self.entry_location_ino(entry_index)?;
                        let child_inode: Arc<dyn Inode> = Self::new_child(
                            fs,
                            ino,
                            inode_type,
                            boot_region.cluster_size,
                            data_length,
                            first_cluster,
                            data_length,
                            no_fat_chain,
                        );
                        return Ok(Some(child_inode));
                    }

                    entry_index = entry_index
                        .checked_add(secondary_count + 1)
                        .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
                }
                entry_type => {
                    if entry_type & ENTRY_TYPE_IN_USE_BIT == 0 {
                        entry_index = entry_index
                            .checked_add(1)
                            .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
                        continue;
                    }
                    let is_root_metadata = matches!(
                        entry_type,
                        ALLOCATION_BITMAP_ENTRY_TYPE
                            | UPCASE_TABLE_ENTRY_TYPE
                            | VOLUME_LABEL_ENTRY_TYPE
                    );
                    if self.data_length.is_none() && is_root_metadata {
                        entry_index = entry_index
                            .checked_add(1)
                            .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
                        continue;
                    }
                    if entry_type & ENTRY_TYPE_CATEGORY_BIT != 0 {
                        return Err(MountVolumeStateError::InvalidOnDiskLayout);
                    }

                    let secondary_count = usize::from(entry[1]);
                    let expected_checksum = u16::from_le_bytes([entry[2], entry[3]]);
                    Self::validated_file_entry_set(
                        &directory_bytes,
                        entry_offset,
                        secondary_count,
                        expected_checksum,
                    )?;
                    if entry_type & ENTRY_TYPE_IMPORTANCE_BIT == 0 {
                        return Err(MountVolumeStateError::InvalidOnDiskLayout);
                    }
                    entry_index = entry_index
                        .checked_add(secondary_count + 1)
                        .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
                }
            }
        }

        Ok(None)
    }

    fn validated_file_entry_set(
        directory_bytes: &[u8],
        entry_offset: usize,
        secondary_count: usize,
        expected_checksum: u16,
    ) -> core::result::Result<&[u8], MountVolumeStateError> {
        let entry_set_len = secondary_count
            .checked_add(1)
            .and_then(|entries| entries.checked_mul(DIRECTORY_ENTRY_SIZE))
            .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
        let entry_set_end = entry_offset
            .checked_add(entry_set_len)
            .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
        let entry_set = directory_bytes
            .get(entry_offset..entry_set_end)
            .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
        if Self::entry_set_checksum(entry_set, secondary_count) != expected_checksum {
            return Err(MountVolumeStateError::InvalidOnDiskLayout);
        }
        Ok(entry_set)
    }

    fn file_stream_entry(
        entry_set: &[u8],
    ) -> core::result::Result<&[u8], MountVolumeStateError> {
        let stream_entry = entry_set
            .get(DIRECTORY_ENTRY_SIZE..DIRECTORY_ENTRY_SIZE * 2)
            .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
        if stream_entry[0] != STREAM_EXTENSION_ENTRY_TYPE {
            return Err(MountVolumeStateError::InvalidOnDiskLayout);
        }
        if stream_entry[1] & 0x01 == 0 {
            return Err(MountVolumeStateError::InvalidOnDiskLayout);
        }
        Ok(stream_entry)
    }

    fn file_name(
        entry_set: &[u8],
        secondary_count: usize,
        stream_entry: &[u8],
    ) -> core::result::Result<Vec<u16>, MountVolumeStateError> {
        let name_length = usize::from(stream_entry[3]);
        if name_length == 0 || name_length > UpcaseTable::NAME_MAX {
            return Err(MountVolumeStateError::InvalidOnDiskLayout);
        }

        let name_entry_count = name_length.div_ceil(15);
        let required_secondary_count = name_entry_count
            .checked_add(1)
            .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
        if secondary_count < required_secondary_count {
            return Err(MountVolumeStateError::InvalidOnDiskLayout);
        }

        let mut candidate_name = Vec::with_capacity(name_length);
        for name_entry_index in 0..name_entry_count {
            let name_entry_offset = (name_entry_index + 2)
                .checked_mul(DIRECTORY_ENTRY_SIZE)
                .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
            let name_entry_end = name_entry_offset
                .checked_add(DIRECTORY_ENTRY_SIZE)
                .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
            let name_entry = entry_set
                .get(name_entry_offset..name_entry_end)
                .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
            if name_entry[0] != FILE_NAME_ENTRY_TYPE {
                return Err(MountVolumeStateError::InvalidOnDiskLayout);
            }
            for code_unit_bytes in name_entry[2..].chunks_exact(2) {
                if candidate_name.len() == name_length {
                    break;
                }
                candidate_name.push(u16::from_le_bytes([
                    code_unit_bytes[0],
                    code_unit_bytes[1],
                ]));
            }
        }
        if candidate_name.len() != name_length {
            return Err(MountVolumeStateError::InvalidOnDiskLayout);
        }

        Self::validate_trailing_secondaries(
            entry_set,
            required_secondary_count,
            secondary_count,
        )?;
        Ok(candidate_name)
    }

    fn validate_trailing_secondaries(
        entry_set: &[u8],
        required_secondary_count: usize,
        secondary_count: usize,
    ) -> core::result::Result<(), MountVolumeStateError> {
        for trailing_secondary_index in required_secondary_count..secondary_count {
            let trailing_secondary_offset = (trailing_secondary_index + 1)
                .checked_mul(DIRECTORY_ENTRY_SIZE)
                .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
            let trailing_secondary_end = trailing_secondary_offset
                .checked_add(DIRECTORY_ENTRY_SIZE)
                .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
            let trailing_secondary = entry_set
                .get(trailing_secondary_offset..trailing_secondary_end)
                .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
            if trailing_secondary[0] & ENTRY_TYPE_IN_USE_BIT == 0
                || trailing_secondary[0] & ENTRY_TYPE_CATEGORY_BIT == 0
                || trailing_secondary[0] & ENTRY_TYPE_IMPORTANCE_BIT == 0
            {
                return Err(MountVolumeStateError::InvalidOnDiskLayout);
            }
        }
        Ok(())
    }

    fn file_entry_child_metadata(
        entry: &[u8],
        stream_entry: &[u8],
        boot_region: &BootRegion,
    ) -> core::result::Result<(InodeType, u32, usize, bool), MountVolumeStateError> {
        let file_attributes = u16::from_le_bytes([entry[4], entry[5]]);
        let inode_type = if file_attributes & FILE_ATTRIBUTE_DIRECTORY != 0 {
            InodeType::Dir
        } else {
            InodeType::File
        };
        let first_cluster = u32::from_le_bytes([
            stream_entry[20],
            stream_entry[21],
            stream_entry[22],
            stream_entry[23],
        ]);
        let data_length = usize::try_from(u64::from_le_bytes([
            stream_entry[24],
            stream_entry[25],
            stream_entry[26],
            stream_entry[27],
            stream_entry[28],
            stream_entry[29],
            stream_entry[30],
            stream_entry[31],
        ]))
        .map_err(|_| MountVolumeStateError::InvalidOnDiskLayout)?;
        let no_fat_chain = stream_entry[1] & 0x02 != 0;
        if data_length != 0 {
            boot_region.validate_stream_data(
                first_cluster,
                u64::try_from(data_length)
                    .map_err(|_| MountVolumeStateError::InvalidOnDiskLayout)?,
            )?;
        } else if first_cluster != 0 {
            return Err(MountVolumeStateError::InvalidOnDiskLayout);
        }
        Ok((inode_type, first_cluster, data_length, no_fat_chain))
    }

    fn entry_location_ino(
        &self,
        entry_index: usize,
    ) -> core::result::Result<u64, MountVolumeStateError> {
        Ok((u64::from(self.first_cluster) << 32)
            | u64::from(
                u32::try_from(entry_index)
                    .map_err(|_| MountVolumeStateError::InvalidOnDiskLayout)?,
            ))
    }

    fn entry_set_checksum(entry_set: &[u8], secondary_count: usize) -> u16 {
        let mut checksum = 0u16;
        let number_of_bytes = (secondary_count + 1) * DIRECTORY_ENTRY_SIZE;
        for (index, byte) in entry_set.iter().take(number_of_bytes).enumerate() {
            if index == 2 || index == 3 {
                continue;
            }
            checksum = ((checksum & 1) << 15) + (checksum >> 1) + u16::from(*byte);
        }
        checksum
    }
}

impl crate::fs::vfs::inode::InodeIo for ExfatInode {
    fn read_at(
        &self,
        _offset: usize,
        _writer: &mut VmWriter,
        _status_flags: StatusFlags,
    ) -> Result<usize> {
        if self.type_() == InodeType::Dir {
            return_errno!(Errno::EISDIR);
        }
        return_errno!(Errno::EOPNOTSUPP);
    }

    fn write_at(
        &self,
        _offset: usize,
        _reader: &mut VmReader,
        _status_flags: StatusFlags,
    ) -> Result<usize> {
        if self.type_() == InodeType::Dir {
            return_errno!(Errno::EISDIR);
        }
        return_errno!(Errno::EOPNOTSUPP);
    }
}

impl Inode for ExfatInode {
    fn size(&self) -> usize {
        self.metadata.read().size
    }

    fn resize(&self, _new_size: usize) -> Result<()> {
        if self.type_() == InodeType::Dir {
            return_errno!(Errno::EISDIR);
        }
        return_errno!(Errno::EOPNOTSUPP);
    }

    fn metadata(&self) -> Metadata {
        *self.metadata.read()
    }

    fn ino(&self) -> u64 {
        self.metadata.read().ino
    }

    fn type_(&self) -> InodeType {
        self.metadata.read().type_
    }

    fn mode(&self) -> Result<InodeMode> {
        Ok(self.metadata.read().mode)
    }

    fn set_mode(&self, mode: InodeMode) -> Result<()> {
        self.metadata.write().mode = mode;
        Ok(())
    }

    fn owner(&self) -> Result<Uid> {
        Ok(self.metadata.read().uid)
    }

    fn set_owner(&self, uid: Uid) -> Result<()> {
        self.metadata.write().uid = uid;
        Ok(())
    }

    fn group(&self) -> Result<Gid> {
        Ok(self.metadata.read().gid)
    }

    fn set_group(&self, gid: Gid) -> Result<()> {
        self.metadata.write().gid = gid;
        Ok(())
    }

    fn atime(&self) -> Duration {
        self.metadata.read().last_access_at
    }

    fn set_atime(&self, time: Duration) {
        self.metadata.write().last_access_at = time;
    }

    fn mtime(&self) -> Duration {
        self.metadata.read().last_modify_at
    }

    fn set_mtime(&self, time: Duration) {
        self.metadata.write().last_modify_at = time;
    }

    fn ctime(&self) -> Duration {
        self.metadata.read().last_meta_change_at
    }

    fn set_ctime(&self, time: Duration) {
        self.metadata.write().last_meta_change_at = time;
    }

    fn page_cache(&self) -> Option<Arc<Vmo>> {
        None
    }

    fn create(&self, _name: &str, _type_: InodeType, _mode: InodeMode) -> Result<Arc<dyn Inode>> {
        return_errno!(Errno::EOPNOTSUPP);
    }

    fn mknod(&self, _name: &str, _mode: InodeMode, _type_: MknodType) -> Result<Arc<dyn Inode>> {
        return_errno!(Errno::EOPNOTSUPP);
    }

    fn open(
        &self,
        _access_mode: AccessMode,
        _status_flags: StatusFlags,
    ) -> Option<Result<Box<dyn FileIo>>> {
        None
    }

    fn readdir_at(&self, offset: usize, visitor: &mut dyn DirentVisitor) -> Result<usize> {
        if self.type_() != InodeType::Dir {
            return_errno!(Errno::ENOTDIR);
        }

        let fs = self
            .fs
            .upgrade()
            .ok_or_else(|| Error::with_message(Errno::EIO, "exFAT filesystem is not mounted"))?;
        let (block_device, boot_region, _, _) =
            fs.published_lookup_state().map_err(Error::from)?;
        let directory_bytes = self
            .read_directory_bytes(&block_device, &boot_region)
            .map_err(Error::from)?;

        let mut next_offset = offset;
        if next_offset == 0 {
            visitor.visit(".", self.ino(), self.type_(), next_offset)?;
            next_offset += 1;
        }
        if next_offset == 1 {
            visitor.visit("..", self.ino(), self.type_(), next_offset)?;
            next_offset += 1;
        }

        let mut visible_offset = 2usize;
        let mut entry_index = 0usize;
        while let Some(entry_offset) = entry_index.checked_mul(DIRECTORY_ENTRY_SIZE) {
            let entry_end = entry_offset
                .checked_add(DIRECTORY_ENTRY_SIZE)
                .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
            let Some(entry) = directory_bytes.get(entry_offset..entry_end) else {
                break;
            };

            match entry[0] {
                END_OF_DIRECTORY_ENTRY_TYPE => break,
                0x01..=0x7F => {
                    entry_index = entry_index
                        .checked_add(1)
                        .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
                }
                FILE_DIRECTORY_ENTRY_TYPE => {
                    let secondary_count = usize::from(entry[1]);
                    let expected_checksum = u16::from_le_bytes([entry[2], entry[3]]);
                    let entry_set = Self::validated_file_entry_set(
                        &directory_bytes,
                        entry_offset,
                        secondary_count,
                        expected_checksum,
                    )?;
                    let stream_entry = Self::file_stream_entry(entry_set)?;
                    let candidate_name =
                        Self::file_name(entry_set, secondary_count, stream_entry)?;
                    let (inode_type, _, _, _) =
                        Self::file_entry_child_metadata(entry, stream_entry, &boot_region)?;

                    if visible_offset >= offset {
                        let entry_name = String::from_utf16(&candidate_name)
                            .map_err(|_| MountVolumeStateError::InvalidOnDiskLayout)?;
                        visitor.visit(
                            &entry_name,
                            self.entry_location_ino(entry_index)?,
                            inode_type,
                            visible_offset,
                        )?;
                        next_offset = visible_offset
                            .checked_add(1)
                            .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
                    }
                    visible_offset = visible_offset
                        .checked_add(1)
                        .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
                    entry_index = entry_index
                        .checked_add(secondary_count + 1)
                        .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
                }
                entry_type => {
                    if entry_type & ENTRY_TYPE_IN_USE_BIT == 0 {
                        entry_index = entry_index
                            .checked_add(1)
                            .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
                        continue;
                    }
                    let is_root_metadata = matches!(
                        entry_type,
                        ALLOCATION_BITMAP_ENTRY_TYPE
                            | UPCASE_TABLE_ENTRY_TYPE
                            | VOLUME_LABEL_ENTRY_TYPE
                    );
                    if self.data_length.is_none() && is_root_metadata {
                        entry_index = entry_index
                            .checked_add(1)
                            .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
                        continue;
                    }
                    if entry_type & ENTRY_TYPE_CATEGORY_BIT != 0 {
                        return Err(MountVolumeStateError::InvalidOnDiskLayout.into());
                    }

                    let secondary_count = usize::from(entry[1]);
                    let expected_checksum = u16::from_le_bytes([entry[2], entry[3]]);
                    Self::validated_file_entry_set(
                        &directory_bytes,
                        entry_offset,
                        secondary_count,
                        expected_checksum,
                    )?;
                    if entry_type & ENTRY_TYPE_IMPORTANCE_BIT == 0 {
                        return Err(MountVolumeStateError::InvalidOnDiskLayout.into());
                    }
                    entry_index = entry_index
                        .checked_add(secondary_count + 1)
                        .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
                }
            }
        }
        Ok(next_offset.saturating_sub(offset))
    }

    fn link(&self, _old: &Arc<dyn Inode>, _name: &str) -> Result<()> {
        return_errno!(Errno::EOPNOTSUPP);
    }

    fn unlink(&self, _name: &str) -> Result<()> {
        return_errno!(Errno::EOPNOTSUPP);
    }

    fn rmdir(&self, _name: &str) -> Result<()> {
        return_errno!(Errno::EOPNOTSUPP);
    }

    fn lookup(&self, name: &str) -> Result<Arc<dyn Inode>> {
        if self.type_() != InodeType::Dir {
            return_errno!(Errno::ENOTDIR);
        }
        if name == "." || name == ".." {
            let inode: Arc<dyn Inode> = self
                .this
                .upgrade()
                .ok_or_else(|| Error::with_message(Errno::EIO, "exFAT inode is not published"))?;
            return Ok(inode);
        }

        let fs = self
            .fs
            .upgrade()
            .ok_or_else(|| Error::with_message(Errno::EIO, "exFAT filesystem is not mounted"))?;
        let (block_device, boot_region, upcase_table, options) =
            fs.published_lookup_state().map_err(Error::from)?;

        let normalized_name = if options.keep_last_dots {
            name
        } else {
            name.trim_end_matches('.')
        };
        if normalized_name.is_empty() {
            return_errno!(Errno::EINVAL);
        }

        let mut lookup_name = Vec::new();
        for character in normalized_name.chars() {
            if character <= '\u{001F}'
                || matches!(
                    character,
                    '"' | '*' | '/' | ':' | '<' | '>' | '?' | '\\' | '|'
                )
            {
                return_errno!(Errno::EINVAL);
            }
            let mut encoded = [0u16; 2];
            lookup_name.extend(character.encode_utf16(&mut encoded).iter().copied());
        }
        if lookup_name.len() > UpcaseTable::NAME_MAX {
            return_errno!(Errno::ENAMETOOLONG);
        }

        let lookup_name_hash = upcase_table.name_hash(&lookup_name);
        if let Some(child_inode) = self
            .lookup_child_by_name(
                &fs,
                &block_device,
                &boot_region,
                &upcase_table,
                &lookup_name,
                lookup_name_hash,
            )
            .map_err(Error::from)?
        {
            return Ok(child_inode);
        }

        return_errno!(Errno::ENOENT);
    }

    fn rename(&self, _old_name: &str, _target: &Arc<dyn Inode>, _new_name: &str) -> Result<()> {
        return_errno!(Errno::EOPNOTSUPP);
    }

    fn read_link(&self) -> Result<SymbolicLink> {
        if self.type_() == InodeType::Dir {
            return_errno!(Errno::EISDIR);
        }
        return_errno!(Errno::EOPNOTSUPP);
    }

    fn write_link(&self, _target: &str) -> Result<()> {
        if self.type_() == InodeType::Dir {
            return_errno!(Errno::EISDIR);
        }
        return_errno!(Errno::EOPNOTSUPP);
    }

    fn sync_all(&self) -> Result<()> {
        Ok(())
    }

    fn sync_data(&self) -> Result<()> {
        Ok(())
    }

    fn fallocate(&self, _mode: FallocMode, _offset: usize, _len: usize) -> Result<()> {
        return_errno!(Errno::EOPNOTSUPP);
    }

    fn fs(&self) -> Arc<dyn FileSystem> {
        match Weak::upgrade(&self.fs) {
            Some(fs) => fs,
            None => unreachable!("published exFAT inode must keep its filesystem alive"),
        }
    }

    fn extension(&self) -> &Extension {
        &self.extension
    }

    fn is_dentry_cacheable(&self) -> bool {
        false
    }
}

#[cfg(ktest)]
#[path = "test_support/lookup_resolution.rs"]
mod tests;
