// SPDX-License-Identifier: MPL-2.0

use alloc::{string::String, vec, vec::Vec};
use core::time::Duration;

use aster_block::BlockDevice;
use ostd::mm::VmIo;

use super::{
    boot::BootRegion,
    direntry::{
        self, DIRECTORY_ENTRY_SIZE, DirectoryEntryAnomalyKind, DirectoryEntrySlotRange,
        ScannedDirectoryEntry,
    },
    fat::{ChainVisitControl, FatReader},
    fs::{ExfatFs, ExfatMountOptions, MountVolumeStateError},
    upcase::UpcaseTable,
};
use crate::{
    fs::{
        file::{AccessMode, FileIo, InodeMode, InodeType, StatusFlags, mkmod},
        utils::DirentVisitor,
        vfs::{
            file_system::{FileSystem, FsFlags},
            inode::{Extension, FallocMode, Inode, Metadata, MknodType, SymbolicLink},
        },
    },
    prelude::*,
    process::{Gid, Uid},
    vm::vmo::Vmo,
};

#[derive(Clone, Copy, Eq, PartialEq)]
struct ExfatInodeStream {
    data_length: Option<usize>,
    first_cluster: u32,
    no_fat_chain: bool,
}

pub(super) struct ExfatInode {
    extension: Extension,
    fs: Weak<ExfatFs>,
    metadata: RwLock<Metadata>,
    stream: RwLock<ExfatInodeStream>,
    this: Weak<Self>,
}

impl ExfatInode {
    fn scan_directory_entry_at<'a>(
        is_root_directory: bool,
        directory_bytes: &'a [u8],
        entry_index: usize,
    ) -> core::result::Result<ScannedDirectoryEntry<'a>, MountVolumeStateError> {
        direntry::scan_directory_entry(is_root_directory, directory_bytes, entry_index)
    }

    fn read_directory_bytes_for_stream(
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        stream: ExfatInodeStream,
    ) -> core::result::Result<Vec<u8>, MountVolumeStateError> {
        let Some(data_length) = stream.data_length else {
            let mut directory_bytes = Vec::new();
            let mut fat_reader = FatReader::new(block_device.as_ref(), boot_region);
            fat_reader.walk_cluster_chain(stream.first_cluster, |_, cluster_bytes| {
                directory_bytes.extend_from_slice(cluster_bytes);
                Ok(ChainVisitControl::Continue)
            })?;
            return Ok(directory_bytes);
        };

        if data_length == 0 {
            if stream.first_cluster != 0 {
                return Err(MountVolumeStateError::InvalidOnDiskLayout);
            }
            return Ok(Vec::new());
        }
        if data_length % DIRECTORY_ENTRY_SIZE != 0 {
            return Err(MountVolumeStateError::InvalidOnDiskLayout);
        }

        let data_length_u64 =
            u64::try_from(data_length).map_err(|_| MountVolumeStateError::InvalidOnDiskLayout)?;
        boot_region.validate_stream_data(stream.first_cluster, data_length_u64)?;

        if stream.no_fat_chain {
            let cluster_count = data_length.div_ceil(boot_region.cluster_size);
            let mut directory_bytes = Vec::with_capacity(data_length);
            for cluster_offset in 0..cluster_count {
                let cluster = stream
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
        fat_reader.walk_cluster_chain(stream.first_cluster, |_, cluster_bytes| {
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

    fn new(
        fs: &Arc<ExfatFs>,
        metadata: Metadata,
        first_cluster: u32,
        data_length: Option<usize>,
        no_fat_chain: bool,
    ) -> Arc<Self> {
        Arc::new_cyclic(|weak_self| Self {
            extension: Extension::new(),
            fs: Arc::downgrade(fs),
            metadata: RwLock::new(metadata),
            stream: RwLock::new(ExfatInodeStream {
                data_length,
                first_cluster,
                no_fat_chain,
            }),
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
        Self::read_directory_bytes_for_stream(block_device, boot_region, *self.stream.read())
    }

    fn next_directory_entry_scan<'a>(
        &self,
        directory_bytes: &'a [u8],
        entry_index: usize,
    ) -> core::result::Result<ScannedDirectoryEntry<'a>, MountVolumeStateError> {
        let stream = self.stream.read();
        Self::scan_directory_entry_at(stream.data_length.is_none(), directory_bytes, entry_index)
    }

    #[expect(dead_code)]
    // Exit plan: `pass_03`/`pass_04` should either consume this helper for the
    // emptiness gate or remove it once the final delete/rename admission seam is settled.
    fn first_directory_child_scan<'a>(
        &self,
        directory_bytes: &'a [u8],
    ) -> core::result::Result<Option<ScannedDirectoryEntry<'a>>, MountVolumeStateError> {
        let is_root_directory = self.stream.read().data_length.is_none();
        let mut entry_index = 0usize;
        loop {
            let entry_scan =
                Self::scan_directory_entry_at(is_root_directory, directory_bytes, entry_index)?;
            match entry_scan {
                ScannedDirectoryEntry::EndOfDirectory { .. } => return Ok(None),
                ScannedDirectoryEntry::Vacant(slot_range) => {
                    entry_index = slot_range.next_entry_index()?;
                }
                ScannedDirectoryEntry::Anomaly { .. } | ScannedDirectoryEntry::File(_) => {
                    return Ok(Some(entry_scan));
                }
            }
        }
    }

    fn find_vacant_entry_slots(
        is_root_directory: bool,
        directory_bytes: &[u8],
        required_entry_count: usize,
    ) -> core::result::Result<Option<DirectoryEntrySlotRange>, MountVolumeStateError> {
        if required_entry_count == 0 {
            return Err(MountVolumeStateError::InvalidOperationInput);
        }
        if directory_bytes.len() % DIRECTORY_ENTRY_SIZE != 0 {
            return Err(MountVolumeStateError::InvalidOnDiskLayout);
        }

        let total_entries = directory_bytes.len() / DIRECTORY_ENTRY_SIZE;
        let mut run_length = 0usize;
        let mut run_start_index = 0usize;
        let mut entry_index = 0usize;
        loop {
            match Self::scan_directory_entry_at(is_root_directory, directory_bytes, entry_index)? {
                ScannedDirectoryEntry::EndOfDirectory { entry_index } => {
                    let available_entries = total_entries
                        .checked_sub(entry_index)
                        .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
                    if run_length == 0 {
                        run_start_index = entry_index;
                    }
                    run_length = run_length
                        .checked_add(available_entries)
                        .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
                    if run_length >= required_entry_count {
                        return Ok(Some(DirectoryEntrySlotRange::new(
                            run_start_index,
                            required_entry_count,
                        )?));
                    }
                    return Ok(None);
                }
                ScannedDirectoryEntry::Vacant(slot_range) => {
                    if run_length == 0 {
                        run_start_index = slot_range.first_entry_index();
                    }
                    run_length = run_length
                        .checked_add(slot_range.entry_count())
                        .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
                    if run_length >= required_entry_count {
                        return Ok(Some(DirectoryEntrySlotRange::new(
                            run_start_index,
                            required_entry_count,
                        )?));
                    }
                    entry_index = slot_range.next_entry_index()?;
                }
                ScannedDirectoryEntry::File(entry_view) => {
                    run_length = 0;
                    entry_index = entry_view.slot_range().next_entry_index()?;
                }
                ScannedDirectoryEntry::Anomaly { .. } => {
                    return Err(MountVolumeStateError::InvalidOnDiskLayout);
                }
            }
        }
    }

    fn reserve_directory_entry_slots(
        &self,
        stream: &mut ExfatInodeStream,
        fs: &Arc<ExfatFs>,
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        required_entry_count: usize,
    ) -> core::result::Result<(Vec<u8>, DirectoryEntrySlotRange), MountVolumeStateError> {
        loop {
            let directory_bytes =
                Self::read_directory_bytes_for_stream(block_device, boot_region, *stream)?;
            if let Some(slot_range) =
                Self::find_vacant_entry_slots(
                    stream.data_length.is_none(),
                    &directory_bytes,
                    required_entry_count,
                )?
            {
                return Ok((directory_bytes, slot_range));
            }
            self.grow_directory_stream(stream, fs, block_device, boot_region)?;
        }
    }

    fn grow_directory_stream(
        &self,
        stream: &mut ExfatInodeStream,
        fs: &Arc<ExfatFs>,
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
    ) -> core::result::Result<(), MountVolumeStateError> {
        let (allocated_ranges, _) = fs.allocate_free_space(1)?;
        let allocated_cluster = match allocated_ranges.as_slice() {
            [allocated_range] if allocated_range.cluster_count == 1 => {
                allocated_range.start_cluster
            }
            _ => {
                let _ = fs.free_allocated_space(&allocated_ranges);
                return Err(MountVolumeStateError::InconsistentAccounting);
            }
        };

        if let Err(error) =
            Self::initialize_directory_cluster(block_device, boot_region, allocated_cluster)
        {
            let _ = fs.free_allocated_space(&allocated_ranges);
            return Err(error);
        }

        if let Err(error) =
            self.attach_directory_cluster(stream, block_device, boot_region, allocated_cluster)
        {
            let _ = fs.free_allocated_space(&allocated_ranges);
            return Err(error);
        }
        Ok(())
    }

    fn attach_directory_cluster(
        &self,
        stream: &mut ExfatInodeStream,
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        allocated_cluster: u32,
    ) -> core::result::Result<(), MountVolumeStateError> {
        let next_data_length = match stream.data_length {
            Some(data_length) => data_length
                .checked_add(boot_region.cluster_size)
                .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?,
            None => boot_region.cluster_size,
        };

        let mut fat_reader = FatReader::new(block_device.as_ref(), boot_region);
        match stream.data_length {
            Some(0) => {
                fat_reader.terminate_cluster_chain(allocated_cluster)?;
            }
            Some(data_length) if stream.no_fat_chain => {
                let cluster_count = data_length.div_ceil(boot_region.cluster_size);
                fat_reader.link_contiguous_chain_to_cluster(
                    stream.first_cluster,
                    cluster_count,
                    allocated_cluster,
                )?;
            }
            Some(_) => {
                fat_reader.append_cluster_to_chain(stream.first_cluster, allocated_cluster)?;
            }
            None => {
                fat_reader.append_cluster_to_chain(stream.first_cluster, allocated_cluster)?;
            }
        }

        match stream.data_length {
            Some(0) => {
                stream.first_cluster = allocated_cluster;
                stream.data_length = Some(next_data_length);
                stream.no_fat_chain = false;
            }
            Some(_) if stream.no_fat_chain => {
                stream.data_length = Some(next_data_length);
                stream.no_fat_chain = false;
            }
            Some(_) => stream.data_length = Some(next_data_length),
            None => stream.data_length = None,
        }
        let mut metadata = self.metadata.write();
        metadata.size = metadata
            .size
            .checked_add(boot_region.cluster_size)
            .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
        Ok(())
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
        loop {
            match self.next_directory_entry_scan(&directory_bytes, entry_index)? {
                ScannedDirectoryEntry::EndOfDirectory { .. } => return Ok(None),
                ScannedDirectoryEntry::Vacant(slot_range) => {
                    entry_index = slot_range.next_entry_index()?;
                }
                ScannedDirectoryEntry::File(entry_view) => {
                    let candidate_name = entry_view.name()?;
                    let stored_name_hash = entry_view.stored_name_hash();
                    if stored_name_hash == lookup_name_hash
                        && upcase_table.names_equal(lookup_name, &candidate_name)
                    {
                        let (inode_type, first_cluster, data_length, no_fat_chain) =
                            entry_view.child_metadata(boot_region)?;
                        let ino =
                            self.entry_location_ino(entry_view.slot_range().first_entry_index())?;
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
                    entry_index = entry_view.slot_range().next_entry_index()?;
                }
                ScannedDirectoryEntry::Anomaly {
                    kind,
                    slot_range,
                } => {
                    if kind == DirectoryEntryAnomalyKind::BenignUnrecognizedEntrySet {
                        entry_index = slot_range.next_entry_index()?;
                        continue;
                    }
                    return Err(MountVolumeStateError::InvalidOnDiskLayout);
                }
            }
        }
    }

    fn entry_location_ino(
        &self,
        entry_index: usize,
    ) -> core::result::Result<u64, MountVolumeStateError> {
        let stream = self.stream.read();
        Ok((u64::from(stream.first_cluster) << 32)
            | u64::from(
                u32::try_from(entry_index)
                    .map_err(|_| MountVolumeStateError::InvalidOnDiskLayout)?,
            ))
    }

    fn admitted_name(
        name: &str,
        options: &ExfatMountOptions,
    ) -> core::result::Result<Vec<u16>, Error> {
        let normalized_name = if options.keep_last_dots {
            name
        } else {
            name.trim_end_matches('.')
        };
        if normalized_name.is_empty() || normalized_name == "." || normalized_name == ".." {
            return_errno_with_message!(Errno::EINVAL, "invalid exFAT name");
        }

        let mut admitted_name = Vec::new();
        for character in normalized_name.chars() {
            if character <= '\u{001F}'
                || matches!(
                    character,
                    '"' | '*' | '/' | ':' | '<' | '>' | '?' | '\\' | '|'
                )
            {
                return_errno_with_message!(Errno::EINVAL, "invalid exFAT name");
            }
            let mut encoded = [0u16; 2];
            admitted_name.extend(character.encode_utf16(&mut encoded).iter().copied());
        }
        if admitted_name.len() > UpcaseTable::NAME_MAX {
            return_errno!(Errno::ENAMETOOLONG);
        }
        Ok(admitted_name)
    }

    fn slot_range_bytes(
        slot_range: DirectoryEntrySlotRange,
    ) -> core::result::Result<core::ops::Range<usize>, MountVolumeStateError> {
        let byte_start = slot_range
            .first_entry_index()
            .checked_mul(DIRECTORY_ENTRY_SIZE)
            .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
        let byte_len = slot_range
            .entry_count()
            .checked_mul(DIRECTORY_ENTRY_SIZE)
            .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
        let byte_end = byte_start
            .checked_add(byte_len)
            .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
        Ok(byte_start..byte_end)
    }

    fn slot_range_is_vacant(
        directory_bytes: &[u8],
        slot_range: DirectoryEntrySlotRange,
    ) -> core::result::Result<bool, MountVolumeStateError> {
        let slot_range_bytes = Self::slot_range_bytes(slot_range)?;
        let Some(slot_bytes) = directory_bytes.get(slot_range_bytes) else {
            return Ok(false);
        };
        Ok(slot_bytes
            .chunks_exact(DIRECTORY_ENTRY_SIZE)
            .all(|entry| entry[0] == 0 || entry[0] & 0x80 == 0))
    }

    fn write_directory_bytes_for_stream(
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        directory_bytes: &[u8],
        stream: ExfatInodeStream,
    ) -> core::result::Result<(), MountVolumeStateError> {
        let expected_length = match stream.data_length {
            Some(data_length) => data_length,
            None => directory_bytes.len(),
        };
        if directory_bytes.len() != expected_length {
            return Err(MountVolumeStateError::InvalidOperationInput);
        }
        if directory_bytes.is_empty() {
            return Ok(());
        }

        if stream.no_fat_chain {
            let cluster_count = directory_bytes.len().div_ceil(boot_region.cluster_size);
            for cluster_offset in 0..cluster_count {
                let cluster = stream
                    .first_cluster
                    .checked_add(
                        u32::try_from(cluster_offset)
                            .map_err(|_| MountVolumeStateError::InvalidOnDiskLayout)?,
                    )
                    .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
                if !boot_region.is_valid_cluster(cluster) {
                    return Err(MountVolumeStateError::InvalidOnDiskLayout);
                }
                let byte_offset = cluster_offset
                    .checked_mul(boot_region.cluster_size)
                    .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
                let byte_end = directory_bytes
                    .len()
                    .min(byte_offset.saturating_add(boot_region.cluster_size));
                block_device
                    .write_bytes(
                        boot_region.cluster_offset(cluster)?,
                        &directory_bytes[byte_offset..byte_end],
                    )
                    .map_err(|_| MountVolumeStateError::DeviceIo)?;
            }
            return Ok(());
        }

        let mut remaining = directory_bytes;
        let mut fat_reader = FatReader::new(block_device.as_ref(), boot_region);
        fat_reader.walk_cluster_chain(stream.first_cluster, |cluster, _| {
            let bytes_to_write = remaining.len().min(boot_region.cluster_size);
            block_device
                .write_bytes(
                    boot_region.cluster_offset(cluster)?,
                    &remaining[..bytes_to_write],
                )
                .map_err(|_| MountVolumeStateError::DeviceIo)?;
            remaining = &remaining[bytes_to_write..];
            if remaining.is_empty() {
                return Ok(ChainVisitControl::Stop);
            }
            Ok(ChainVisitControl::Continue)
        })?;
        if !remaining.is_empty() {
            return Err(MountVolumeStateError::InvalidOnDiskLayout);
        }
        Ok(())
    }

    fn initialize_directory_cluster(
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        first_cluster: u32,
    ) -> core::result::Result<(), MountVolumeStateError> {
        let cluster_offset = boot_region.cluster_offset(first_cluster)?;
        let cluster_bytes = vec![0; boot_region.cluster_size];
        block_device
            .write_bytes(cluster_offset, &cluster_bytes)
            .map_err(|_| MountVolumeStateError::DeviceIo)
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

    fn create(&self, name: &str, type_: InodeType, mode: InodeMode) -> Result<Arc<dyn Inode>> {
        if self.type_() != InodeType::Dir {
            return_errno!(Errno::ENOTDIR);
        }
        if !matches!(type_, InodeType::File | InodeType::Dir) {
            return_errno!(Errno::EOPNOTSUPP);
        }

        let fs = self
            .fs
            .upgrade()
            .ok_or_else(|| Error::with_message(Errno::EIO, "exFAT filesystem is not mounted"))?;
        let (block_device, boot_region, upcase_table, options) =
            fs.published_lookup_state().map_err(Error::from)?;
        if options.fs_flags.contains(FsFlags::RDONLY) {
            return_errno!(Errno::EROFS);
        }

        let admitted_name = Self::admitted_name(name, &options)?;
        let name_hash = upcase_table.name_hash(&admitted_name);
        if self
            .lookup_child_by_name(
                &fs,
                &block_device,
                &boot_region,
                &upcase_table,
                &admitted_name,
                name_hash,
            )
            .map_err(Error::from)?
            .is_some()
        {
            return_errno!(Errno::EEXIST);
        }

        let required_entry_count = direntry::file_entry_set_entry_count(admitted_name.len())
            .map_err(Error::from)?;
        let mut stream = self.stream.write();
        let (_, slot_range) = self
            .reserve_directory_entry_slots(
                &mut stream,
                &fs,
                &block_device,
                &boot_region,
                required_entry_count,
            )
            .map_err(Error::from)?;

        let mut allocated_directory_ranges = None;
        let (first_cluster, data_length, no_fat_chain) =
            if type_ == InodeType::Dir && !options.zero_size_dir {
                let (allocated_ranges, _) = fs.allocate_free_space(1).map_err(Error::from)?;
                let Some(allocated_range) = allocated_ranges.first() else {
                    return Err(Error::from(MountVolumeStateError::InconsistentAccounting));
                };
                if allocated_range.cluster_count != 1 {
                    let _ = fs.free_allocated_space(&allocated_ranges);
                    return Err(Error::from(MountVolumeStateError::InconsistentAccounting));
                }
                let allocated_cluster = allocated_range.start_cluster;
                if let Err(error) = Self::initialize_directory_cluster(
                    &block_device,
                    &boot_region,
                    allocated_cluster,
                ) {
                    let _ = fs.free_allocated_space(&allocated_ranges);
                    return Err(Error::from(error));
                }
                allocated_directory_ranges = Some(allocated_ranges);
                (allocated_cluster, boot_region.cluster_size, true)
            } else {
                (0, 0, false)
            };

        let latest_directory_bytes =
            Self::read_directory_bytes_for_stream(&block_device, &boot_region, *stream).map_err(
                |error| {
                    if let Some(allocated_ranges) = allocated_directory_ranges.as_ref() {
                        let _ = fs.free_allocated_space(allocated_ranges);
                    }
                    Error::from(error)
                },
            )?;
        let mut entry_index = 0usize;
        loop {
            match Self::scan_directory_entry_at(
                stream.data_length.is_none(),
                &latest_directory_bytes,
                entry_index,
            )
            .map_err(|error| {
                if let Some(allocated_ranges) = allocated_directory_ranges.as_ref() {
                    let _ = fs.free_allocated_space(allocated_ranges);
                }
                Error::from(error)
            })? {
                ScannedDirectoryEntry::EndOfDirectory { .. } => break,
                ScannedDirectoryEntry::Vacant(slot_range) => {
                    entry_index = slot_range.next_entry_index().map_err(|error| {
                        if let Some(allocated_ranges) = allocated_directory_ranges.as_ref() {
                            let _ = fs.free_allocated_space(allocated_ranges);
                        }
                        Error::from(error)
                    })?;
                }
                ScannedDirectoryEntry::File(entry_view) => {
                    let candidate_name = entry_view.name().map_err(|error| {
                        if let Some(allocated_ranges) = allocated_directory_ranges.as_ref() {
                            let _ = fs.free_allocated_space(allocated_ranges);
                        }
                        Error::from(error)
                    })?;
                    if entry_view.stored_name_hash() == name_hash
                        && upcase_table.names_equal(&admitted_name, &candidate_name)
                    {
                        if let Some(allocated_ranges) = allocated_directory_ranges.as_ref() {
                            let _ = fs.free_allocated_space(allocated_ranges);
                        }
                        return_errno!(Errno::EEXIST);
                    }
                    entry_index = entry_view.slot_range().next_entry_index().map_err(|error| {
                        if let Some(allocated_ranges) = allocated_directory_ranges.as_ref() {
                            let _ = fs.free_allocated_space(allocated_ranges);
                        }
                        Error::from(error)
                    })?;
                }
                ScannedDirectoryEntry::Anomaly {
                    kind,
                    slot_range,
                } => {
                    if kind == DirectoryEntryAnomalyKind::BenignUnrecognizedEntrySet {
                        entry_index = slot_range.next_entry_index().map_err(|error| {
                            if let Some(allocated_ranges) = allocated_directory_ranges.as_ref() {
                                let _ = fs.free_allocated_space(allocated_ranges);
                            }
                            Error::from(error)
                        })?;
                        continue;
                    }
                    if let Some(allocated_ranges) = allocated_directory_ranges.as_ref() {
                        let _ = fs.free_allocated_space(allocated_ranges);
                    }
                    return Err(Error::from(MountVolumeStateError::InvalidOnDiskLayout));
                }
            }
        }
        if !Self::slot_range_is_vacant(&latest_directory_bytes, slot_range).map_err(|error| {
            if let Some(allocated_ranges) = allocated_directory_ranges.as_ref() {
                let _ = fs.free_allocated_space(allocated_ranges);
            }
            Error::from(error)
        })? {
            if let Some(allocated_ranges) = allocated_directory_ranges.as_ref() {
                let _ = fs.free_allocated_space(allocated_ranges);
            }
            return_errno!(Errno::ENOSPC);
        }

        let entry_set = direntry::encode_file_entry_set(
            &admitted_name,
            name_hash,
            type_,
            first_cluster,
            data_length,
            no_fat_chain,
        )
        .map_err(|error| {
            if let Some(allocated_ranges) = allocated_directory_ranges.as_ref() {
                let _ = fs.free_allocated_space(allocated_ranges);
            }
            Error::from(error)
        })?;
        let mut published_directory_bytes = latest_directory_bytes;
        let slot_range_bytes = Self::slot_range_bytes(slot_range).map_err(|error| {
            if let Some(allocated_ranges) = allocated_directory_ranges.as_ref() {
                let _ = fs.free_allocated_space(allocated_ranges);
            }
            Error::from(error)
        })?;
        published_directory_bytes[slot_range_bytes.clone()].copy_from_slice(&entry_set);
        Self::write_directory_bytes_for_stream(
            &block_device,
            &boot_region,
            &published_directory_bytes,
            *stream,
        )
        .map_err(|error| {
            if let Some(allocated_ranges) = allocated_directory_ranges.as_ref() {
                let _ = fs.free_allocated_space(allocated_ranges);
            }
            Error::from(error)
        })?;
        drop(stream);

        let child_size = if type_ == InodeType::Dir {
            data_length
        } else {
            0
        };
        let child_inode = Self::new_child(
            &fs,
            self.entry_location_ino(slot_range.first_entry_index())
                .map_err(Error::from)?,
            type_,
            boot_region.cluster_size,
            child_size,
            first_cluster,
            data_length,
            no_fat_chain,
        );
        child_inode.metadata.write().mode = mode;
        let child_inode: Arc<dyn Inode> = child_inode;
        Ok(child_inode)
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
        loop {
            match self.next_directory_entry_scan(&directory_bytes, entry_index)? {
                ScannedDirectoryEntry::EndOfDirectory { .. } => break,
                ScannedDirectoryEntry::Vacant(slot_range) => {
                    entry_index = slot_range.next_entry_index()?;
                }
                ScannedDirectoryEntry::File(entry_view) => {
                    let candidate_name = entry_view.name()?;
                    let (inode_type, _, _, _) = entry_view.child_metadata(&boot_region)?;

                    if visible_offset >= offset {
                        let entry_name = String::from_utf16(&candidate_name)
                            .map_err(|_| MountVolumeStateError::InvalidOnDiskLayout)?;
                        visitor.visit(
                            &entry_name,
                            self.entry_location_ino(entry_view.slot_range().first_entry_index())?,
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
                    entry_index = entry_view.slot_range().next_entry_index()?;
                }
                ScannedDirectoryEntry::Anomaly {
                    kind,
                    slot_range,
                } => {
                    if kind == DirectoryEntryAnomalyKind::BenignUnrecognizedEntrySet {
                        entry_index = slot_range.next_entry_index()?;
                        continue;
                    }
                    return Err(MountVolumeStateError::InvalidOnDiskLayout.into());
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

        let lookup_name = Self::admitted_name(name, &options)?;

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
