// SPDX-License-Identifier: MPL-2.0

use alloc::{string::String, vec, vec::Vec};
use core::time::Duration;

use aster_block::{
    BlockDevice,
    bio::{Bio, BioSegment, BioType, BioWaiter, BioDirection},
    id::Sid,
};
use ostd::{
    mm::{FallibleVmWrite, Segment, VmIo, VmReader, io::util::HasVmReaderWriter},
    sync::{PreemptDisabled, RwLockReadGuard},
};
use spin::Once;

use super::{
    bitmap::ClusterRange,
    boot::BootRegion,
    direntry::{
        self, DIRECTORY_ENTRY_SIZE, DirectoryEntryAnomalyKind, DirectoryEntrySlotRange,
        FileEntrySetView, ScannedDirectoryEntry, WritableDirectoryEntrySlotSpan,
    },
    fat::{ChainVisitControl, FatChainStep, FatReader},
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
            page_cache::{CachePage, PageCache, PageCacheBackend},
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
    valid_data_length: Option<usize>,
    no_fat_chain: bool,
}

pub(super) struct ExfatInode {
    extension: Extension,
    fs: Weak<ExfatFs>,
    metadata: RwLock<Metadata>,
    page_cache: Once<Option<PageCache>>,
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
        valid_data_length: Option<usize>,
        no_fat_chain: bool,
    ) -> Arc<Self> {
        Arc::new_cyclic(|weak_self| Self {
            extension: Extension::new(),
            fs: Arc::downgrade(fs),
            metadata: RwLock::new(metadata),
            page_cache: Once::new(),
            stream: RwLock::new(ExfatInodeStream {
                data_length,
                first_cluster,
                valid_data_length,
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
        Self::new(fs, metadata, root_cluster, None, None, false)
    }

    fn new_child(
        fs: &Arc<ExfatFs>,
        ino: u64,
        inode_type: InodeType,
        cluster_size: usize,
        size: usize,
        first_cluster: u32,
        data_length: usize,
        valid_data_length: usize,
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
        Self::new(
            fs,
            metadata,
            first_cluster,
            Some(data_length),
            Some(valid_data_length),
            no_fat_chain,
        )
    }

    fn read_directory_bytes(
        &self,
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
    ) -> core::result::Result<Vec<u8>, MountVolumeStateError> {
        Self::read_directory_bytes_for_stream(block_device, boot_region, *self.stream.read())
    }

    fn read_revalidated_directory_bytes(
        &self,
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
    ) -> core::result::Result<(ExfatInodeStream, Vec<u8>), MountVolumeStateError> {
        loop {
            let first_stream = *self.stream.read();
            let first_directory_bytes =
                Self::read_directory_bytes_for_stream(block_device, boot_region, first_stream)?;
            let second_stream = *self.stream.read();
            let second_directory_bytes =
                Self::read_directory_bytes_for_stream(block_device, boot_region, second_stream)?;
            if first_stream == second_stream && first_directory_bytes == second_directory_bytes {
                return Ok((second_stream, second_directory_bytes));
            }
        }
    }

    fn validated_regular_file_stream(
        &self,
    ) -> Result<RwLockReadGuard<'_, ExfatInodeStream, PreemptDisabled>> {
        match self.type_() {
            InodeType::Dir => return_errno!(Errno::EISDIR),
            InodeType::File => {}
            _ => return_errno!(Errno::EOPNOTSUPP),
        }

        let stream = self.stream.read();
        let Some(data_length) = stream.data_length else {
            return_errno!(Errno::EINVAL);
        };
        let Some(valid_data_length) = stream.valid_data_length else {
            return_errno!(Errno::EINVAL);
        };
        if valid_data_length > data_length {
            return_errno!(Errno::EINVAL);
        }
        if data_length == 0 && (stream.first_cluster != 0 || valid_data_length != 0) {
            return_errno!(Errno::EINVAL);
        }
        Ok(stream)
    }

    fn validate_regular_file_mapping_shape(
        boot_region: &BootRegion,
        stream: &ExfatInodeStream,
        data_length: usize,
    ) -> Result<()> {
        let data_length_u64 =
            u64::try_from(data_length).map_err(|_| Error::new(Errno::EINVAL))?;
        match boot_region.validate_stream_data(stream.first_cluster, data_length_u64) {
            Ok(()) => Ok(()),
            Err(_) => return_errno!(Errno::EINVAL),
        }
    }

    fn mapped_regular_file_cluster(
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        stream: &ExfatInodeStream,
        data_length: usize,
        cluster_index: usize,
    ) -> Result<u32> {
        if stream.no_fat_chain {
            let cluster_count = data_length.div_ceil(boot_region.cluster_size);
            if cluster_index >= cluster_count {
                return_errno!(Errno::EINVAL);
            }
            let last_cluster = stream
                .first_cluster
                .checked_add(
                    u32::try_from(cluster_count.saturating_sub(1))
                        .map_err(|_| Error::new(Errno::EINVAL))?,
                )
                .ok_or_else(|| Error::new(Errno::EINVAL))?;
            if !boot_region.is_valid_cluster(last_cluster) {
                return_errno!(Errno::EINVAL);
            }
            return stream
                .first_cluster
                .checked_add(u32::try_from(cluster_index).map_err(|_| Error::new(Errno::EINVAL))?)
                .ok_or_else(|| Error::new(Errno::EINVAL));
        }

        let mut fat_reader = FatReader::new(block_device.as_ref(), boot_region);
        let mut current_cluster = stream.first_cluster;
        for _ in 0..cluster_index {
            current_cluster = match fat_reader.next_cluster(current_cluster) {
                Ok(FatChainStep::Continue(next_cluster)) => next_cluster,
                Ok(FatChainStep::End) | Err(_) => return_errno!(Errno::EIO),
            };
        }
        Ok(current_cluster)
    }

    fn map_regular_file_logical_offset(
        &self,
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        offset: usize,
    ) -> Result<Option<usize>> {
        let stream = self.validated_regular_file_stream()?;
        let data_length = stream
            .data_length
            .ok_or_else(|| Error::new(Errno::EINVAL))?;
        let valid_data_length = stream
            .valid_data_length
            .ok_or_else(|| Error::new(Errno::EINVAL))?;
        if data_length == 0 || offset >= data_length || offset >= valid_data_length {
            return Ok(None);
        }

        Self::validate_regular_file_mapping_shape(boot_region, &stream, data_length)?;
        let cluster_size = boot_region.cluster_size;
        let cluster_index = offset / cluster_size;
        let cluster = Self::mapped_regular_file_cluster(
            block_device,
            boot_region,
            &stream,
            data_length,
            cluster_index,
        )?;
        let cluster_start = boot_region
            .cluster_offset(cluster)
            .map_err(|_| Error::new(Errno::EINVAL))?;
        cluster_start
            .checked_add(offset % cluster_size)
            .map(Some)
            .ok_or_else(|| Error::new(Errno::EINVAL))
    }

    fn regular_file_npages(&self) -> Result<usize> {
        let stream = self.validated_regular_file_stream()?;
        let data_length = stream
            .data_length
            .ok_or_else(|| Error::new(Errno::EINVAL))?;
        Ok(data_length.div_ceil(PAGE_SIZE))
    }

    fn regular_file_page_bio_ranges(
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        stream: &ExfatInodeStream,
        data_length: usize,
        file_offset: usize,
        len: usize,
    ) -> Result<Vec<(usize, usize, usize)>> {
        if len == 0 {
            return Ok(Vec::new());
        }

        Self::validate_regular_file_mapping_shape(boot_region, stream, data_length)?;

        let cluster_size = boot_region.cluster_size;
        let cluster_index = file_offset / cluster_size;
        let mut cluster_offset = file_offset % cluster_size;
        let mut current_cluster = Self::mapped_regular_file_cluster(
            block_device,
            boot_region,
            stream,
            data_length,
            cluster_index,
        )?;
        let mut page_offset = 0usize;
        let mut remaining = len;
        let mut ranges: Vec<(usize, usize, usize)> = Vec::new();
        let mut fat_reader = (!stream.no_fat_chain).then(|| FatReader::new(block_device.as_ref(), boot_region));

        while remaining != 0 {
            let chunk_len = remaining.min(cluster_size - cluster_offset);
            let chunk_offset = boot_region
                .cluster_offset(current_cluster)
                .map_err(Error::from)?
                .checked_add(cluster_offset)
                .ok_or_else(|| Error::new(Errno::EINVAL))?;

            if let Some((last_page_offset, last_disk_offset, last_len)) = ranges.last_mut()
                && last_page_offset
                    .checked_add(*last_len)
                    .zip(last_disk_offset.checked_add(*last_len))
                    == Some((page_offset, chunk_offset))
            {
                *last_len = last_len
                    .checked_add(chunk_len)
                    .ok_or_else(|| Error::new(Errno::EINVAL))?;
            } else {
                ranges.push((page_offset, chunk_offset, chunk_len));
            }

            page_offset = page_offset
                .checked_add(chunk_len)
                .ok_or_else(|| Error::new(Errno::EINVAL))?;
            remaining -= chunk_len;
            cluster_offset = 0;
            if remaining == 0 {
                break;
            }

            current_cluster = if let Some(fat_reader) = fat_reader.as_mut() {
                match fat_reader.next_cluster(current_cluster) {
                    Ok(FatChainStep::Continue(next_cluster)) => next_cluster,
                    Ok(FatChainStep::End) | Err(_) => return_errno!(Errno::EIO),
                }
            } else {
                current_cluster
                    .checked_add(1)
                    .ok_or_else(|| Error::new(Errno::EINVAL))?
            };
        }

        Ok(ranges)
    }

    fn regular_file_page_range(
        &self,
        idx: usize,
    ) -> Result<(RwLockReadGuard<'_, ExfatInodeStream, PreemptDisabled>, usize, usize, usize)> {
        let stream = self.validated_regular_file_stream()?;
        let data_length = stream
            .data_length
            .ok_or_else(|| Error::new(Errno::EINVAL))?;
        let valid_data_length = stream
            .valid_data_length
            .ok_or_else(|| Error::new(Errno::EINVAL))?;
        let file_offset = idx
            .checked_mul(PAGE_SIZE)
            .ok_or_else(|| Error::new(Errno::EINVAL))?;
        if file_offset >= data_length {
            return_errno!(Errno::EINVAL);
        }

        let page_end = file_offset
            .checked_add(PAGE_SIZE)
            .ok_or_else(|| Error::new(Errno::EINVAL))?
            .min(data_length);
        let initialized_end = page_end.min(valid_data_length);
        let initialized_len = initialized_end.saturating_sub(file_offset);

        Ok((stream, data_length, file_offset, initialized_len))
    }

    fn regular_file_page_waiter(
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        frame: &CachePage,
        stream: &ExfatInodeStream,
        data_length: usize,
        file_offset: usize,
        initialized_len: usize,
        bio_type: BioType,
    ) -> Result<BioWaiter> {
        let page_ranges = Self::regular_file_page_bio_ranges(
            block_device,
            boot_region,
            stream,
            data_length,
            file_offset,
            initialized_len,
        )?;
        let page_segment: ostd::mm::USegment = Segment::from(frame.clone()).into();
        let mut bio_waiter = BioWaiter::new();

        for (page_offset, disk_offset, len) in page_ranges {
            let page_end = page_offset
                .checked_add(len)
                .ok_or_else(|| Error::new(Errno::EINVAL))?;
            let bio_segment = BioSegment::new_from_segment_slice(
                page_segment.clone(),
                page_offset..page_end,
                match bio_type {
                    BioType::Read => BioDirection::FromDevice,
                    BioType::Write => BioDirection::ToDevice,
                    BioType::Flush => return_errno!(Errno::EINVAL),
                },
            );
            let bio = Bio::new(
                bio_type,
                Sid::from_offset(disk_offset),
                vec![bio_segment],
                None,
            );
            bio_waiter.concat(
                bio.submit(block_device.as_ref())
                    .map_err(Error::from)?,
            );
        }

        Ok(bio_waiter)
    }

    fn read_regular_file_at(
        &self,
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        offset: usize,
        writer: &mut VmWriter,
    ) -> Result<usize> {
        let stream = self.validated_regular_file_stream()?;
        let data_length = stream
            .data_length
            .ok_or_else(|| Error::new(Errno::EINVAL))?;
        let valid_data_length = stream
            .valid_data_length
            .ok_or_else(|| Error::new(Errno::EINVAL))?;
        if !writer.has_avail() {
            return Ok(0);
        }
        if data_length == 0 {
            return Ok(0);
        }

        Self::validate_regular_file_mapping_shape(boot_region, &stream, data_length)?;
        if offset >= data_length {
            return Ok(0);
        }

        let read_end = offset
            .checked_add(writer.avail())
            .ok_or_else(|| Error::new(Errno::EINVAL))?
            .min(data_length);
        let initialized_end = read_end.min(valid_data_length);
        let mut initialized_remaining = if offset >= initialized_end {
            0
        } else {
            initialized_end
                .checked_sub(offset)
                .ok_or_else(|| Error::new(Errno::EINVAL))?
        };
        let mut copied_len = 0usize;
        if initialized_remaining != 0 {
            let cluster_size = boot_region.cluster_size;
            let cluster_index = offset / cluster_size;
            let mut cluster_offset = offset % cluster_size;

            if stream.no_fat_chain {
                let mut current_cluster = Self::mapped_regular_file_cluster(
                    block_device,
                    boot_region,
                    &stream,
                    data_length,
                    cluster_index,
                )?;
                let mut cluster_buffer = vec![0; cluster_size];
                while initialized_remaining != 0 {
                    let chunk_len = initialized_remaining.min(cluster_size - cluster_offset);
                    let cluster_start = boot_region
                        .cluster_offset(current_cluster)
                        .map_err(Error::from)?;
                    block_device
                        .read_bytes(cluster_start, &mut cluster_buffer)
                        .map_err(|_| Error::new(Errno::EIO))?;
                    let chunk_end = cluster_offset
                        .checked_add(chunk_len)
                        .ok_or_else(|| Error::new(Errno::EINVAL))?;
                    let mut reader = VmReader::from(&cluster_buffer[cluster_offset..chunk_end]);
                    copied_len = copied_len
                        .checked_add(writer.write_fallible(&mut reader)?)
                        .ok_or_else(|| Error::new(Errno::EINVAL))?;
                    initialized_remaining -= chunk_len;
                    cluster_offset = 0;
                    if initialized_remaining == 0 {
                        break;
                    }
                    current_cluster = current_cluster
                        .checked_add(1)
                        .ok_or_else(|| Error::new(Errno::EINVAL))?;
                }
            } else {
                let mut fat_reader = FatReader::new(block_device.as_ref(), boot_region);
                let mut cluster_buffer = vec![0; cluster_size];
                let mut current_cluster = Self::mapped_regular_file_cluster(
                    block_device,
                    boot_region,
                    &stream,
                    data_length,
                    cluster_index,
                )?;
                while initialized_remaining != 0 {
                    let chunk_len = initialized_remaining.min(cluster_size - cluster_offset);
                    let cluster_start = boot_region
                        .cluster_offset(current_cluster)
                        .map_err(|_| Error::new(Errno::EIO))?;
                    block_device
                        .read_bytes(cluster_start, &mut cluster_buffer)
                        .map_err(|_| Error::new(Errno::EIO))?;
                    let chunk_end = cluster_offset
                        .checked_add(chunk_len)
                        .ok_or_else(|| Error::new(Errno::EINVAL))?;
                    let mut reader = VmReader::from(&cluster_buffer[cluster_offset..chunk_end]);
                    copied_len = copied_len
                        .checked_add(writer.write_fallible(&mut reader)?)
                        .ok_or_else(|| Error::new(Errno::EINVAL))?;
                    initialized_remaining -= chunk_len;
                    cluster_offset = 0;
                    if initialized_remaining == 0 {
                        break;
                    }
                    current_cluster = match fat_reader.next_cluster(current_cluster) {
                        Ok(FatChainStep::Continue(next_cluster)) => next_cluster,
                        Ok(FatChainStep::End) | Err(_) => return_errno!(Errno::EIO),
                    };
                }
            }
        }

        let zeroed_len = read_end
            .checked_sub(initialized_end)
            .ok_or_else(|| Error::new(Errno::EINVAL))?;
        copied_len = copied_len
            .checked_add(writer.fill_zeros(zeroed_len)?)
            .ok_or_else(|| Error::new(Errno::EINVAL))?;

        Ok(copied_len)
    }

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
        let (stream, directory_bytes) =
            self.read_revalidated_directory_bytes(block_device, boot_region)?;
        let Some((
            slot_range,
            inode_type,
            first_cluster,
            data_length,
            valid_data_length,
            no_fat_chain,
        )) =
            Self::locate_named_child(
                &directory_bytes,
                stream.data_length.is_none(),
                boot_region,
                upcase_table,
                lookup_name,
                lookup_name_hash,
            )?
        else {
            return Ok(None);
        };
        let ino = (u64::from(stream.first_cluster) << 32)
            | u64::from(
                u32::try_from(slot_range.first_entry_index())
                    .map_err(|_| MountVolumeStateError::InvalidOnDiskLayout)?,
            );
        let child_inode: Arc<dyn Inode> = Self::new_child(
            fs,
            ino,
            inode_type,
            boot_region.cluster_size,
            data_length,
            first_cluster,
            data_length,
            valid_data_length,
            no_fat_chain,
        );
        Ok(Some(child_inode))
    }

    fn locate_named_child(
        directory_bytes: &[u8],
        is_root_directory: bool,
        boot_region: &BootRegion,
        upcase_table: &UpcaseTable,
        lookup_name: &[u16],
        lookup_name_hash: u16,
    ) -> core::result::Result<
        Option<(DirectoryEntrySlotRange, InodeType, u32, usize, usize, bool)>,
        MountVolumeStateError,
    > {
        let Some(entry_view) = Self::locate_named_child_view(
            directory_bytes,
            is_root_directory,
            upcase_table,
            lookup_name,
            lookup_name_hash,
        )?
        else {
            return Ok(None);
        };
        let slot_range = entry_view.slot_range();
        let (inode_type, first_cluster, data_length, no_fat_chain) =
            entry_view.child_metadata(boot_region)?;
        let entry_set = directory_bytes
            .get(Self::slot_range_bytes(slot_range)?)
            .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
        let stream_entry = entry_set
            .get(DIRECTORY_ENTRY_SIZE..DIRECTORY_ENTRY_SIZE * 2)
            .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
        let valid_data_length = usize::try_from(u64::from_le_bytes([
            stream_entry[8],
            stream_entry[9],
            stream_entry[10],
            stream_entry[11],
            stream_entry[12],
            stream_entry[13],
            stream_entry[14],
            stream_entry[15],
        ]))
        .map_err(|_| MountVolumeStateError::InvalidOnDiskLayout)?;
        if valid_data_length > data_length {
            return Err(MountVolumeStateError::InvalidOnDiskLayout);
        }
        Ok(Some((
            slot_range,
            inode_type,
            first_cluster,
            data_length,
            valid_data_length,
            no_fat_chain,
        )))
    }

    fn locate_named_child_view<'a>(
        directory_bytes: &'a [u8],
        is_root_directory: bool,
        upcase_table: &UpcaseTable,
        lookup_name: &[u16],
        lookup_name_hash: u16,
    ) -> core::result::Result<Option<FileEntrySetView<'a>>, MountVolumeStateError> {
        let mut entry_index = 0usize;
        loop {
            match Self::scan_directory_entry_at(is_root_directory, directory_bytes, entry_index)? {
                ScannedDirectoryEntry::EndOfDirectory { .. } => return Ok(None),
                ScannedDirectoryEntry::Vacant(slot_range) => {
                    entry_index = slot_range.next_entry_index()?;
                }
                ScannedDirectoryEntry::File(entry_view) => {
                    let candidate_name = entry_view.name()?;
                    if entry_view.stored_name_hash() == lookup_name_hash
                        && upcase_table.names_equal(lookup_name, &candidate_name)
                    {
                        return Ok(Some(entry_view));
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

    fn child_inode_from_directory_entry(
        fs: &Arc<ExfatFs>,
        boot_region: &BootRegion,
        parent_first_cluster: u32,
        slot_range: DirectoryEntrySlotRange,
        inode_type: InodeType,
        first_cluster: u32,
        data_length: usize,
        valid_data_length: usize,
        no_fat_chain: bool,
    ) -> core::result::Result<Arc<Self>, MountVolumeStateError> {
        let child_ino = (u64::from(parent_first_cluster) << 32)
            | u64::from(
                u32::try_from(slot_range.first_entry_index())
                    .map_err(|_| MountVolumeStateError::InvalidOnDiskLayout)?,
            );
        Ok(Self::new_child(
            fs,
            child_ino,
            inode_type,
            boot_region.cluster_size,
            data_length,
            first_cluster,
            data_length,
            valid_data_length,
            no_fat_chain,
        ))
    }

    fn ensure_directory_entry_is_empty(
        child_inode: &Arc<Self>,
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
    ) -> Result<()> {
        let child_directory_bytes = child_inode
            .read_directory_bytes(block_device, boot_region)
            .map_err(Error::from)?;
        if let Some(first_child_scan) = child_inode
            .first_directory_child_scan(&child_directory_bytes)
            .map_err(Error::from)?
        {
            match first_child_scan {
                ScannedDirectoryEntry::Anomaly { .. } => {
                    return Err(Error::from(MountVolumeStateError::InvalidOnDiskLayout));
                }
                ScannedDirectoryEntry::File(_) => return_errno!(Errno::ENOTEMPTY),
                ScannedDirectoryEntry::EndOfDirectory { .. }
                | ScannedDirectoryEntry::Vacant(_) => unreachable!(),
            }
        }
        Ok(())
    }

    fn rename_within_directory(
        &self,
        stream: &mut ExfatInodeStream,
        fs: &Arc<ExfatFs>,
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        upcase_table: &UpcaseTable,
        old_name: &[u16],
        old_name_hash: u16,
        new_name: &[u16],
        new_name_hash: u16,
    ) -> Result<()> {
        let current_directory_bytes =
            Self::read_directory_bytes_for_stream(block_device, boot_region, *stream)
                .map_err(Error::from)?;
        let Some(current_source_view) = Self::locate_named_child_view(
            &current_directory_bytes,
            stream.data_length.is_none(),
            upcase_table,
            old_name,
            old_name_hash,
        )
        .map_err(Error::from)?
        else {
            return_errno!(Errno::ENOENT);
        };
        let source_name = current_source_view.name().map_err(Error::from)?;
        let current_source_slot_range = current_source_view.slot_range();
        let current_target_view = Self::locate_named_child_view(
            &current_directory_bytes,
            stream.data_length.is_none(),
            upcase_table,
            new_name,
            new_name_hash,
        )
        .map_err(Error::from)?;
        if current_target_view
            .map(FileEntrySetView::slot_range)
            .is_some_and(|slot_range| slot_range == current_source_slot_range)
            && source_name == new_name
        {
            return Ok(());
        }
        let current_renamed_entry_set =
            direntry::renamed_entry_set(current_source_view, new_name, new_name_hash)
                .map_err(Error::from)?;
        let required_entry_count = current_renamed_entry_set.len() / DIRECTORY_ENTRY_SIZE;

        let (source_inode_type, _, _, _) =
            current_source_view.child_metadata(boot_region).map_err(Error::from)?;
        let mut replaced_target_ranges = Vec::new();
        let mut final_slot_range = current_source_slot_range;
        if let Some(target_view) = current_target_view.filter(|entry_view| {
            entry_view.slot_range() != current_source_slot_range
        }) {
            let (target_inode_type, first_cluster, data_length, no_fat_chain) =
                target_view.child_metadata(boot_region).map_err(Error::from)?;
            if source_inode_type == InodeType::Dir && target_inode_type != InodeType::Dir {
                return_errno!(Errno::ENOTDIR);
            }
            if source_inode_type != InodeType::Dir && target_inode_type == InodeType::Dir {
                return_errno!(Errno::EISDIR);
            }
            if target_inode_type == InodeType::Dir {
                let child_inode = Self::child_inode_from_directory_entry(
                    fs,
                    boot_region,
                    stream.first_cluster,
                    target_view.slot_range(),
                    target_inode_type,
                    first_cluster,
                    data_length,
                    data_length,
                    no_fat_chain,
                )
                .map_err(Error::from)?;
                Self::ensure_directory_entry_is_empty(&child_inode, block_device, boot_region)?;
            }
            replaced_target_ranges = Self::allocated_cluster_ranges(
                block_device,
                boot_region,
                first_cluster,
                data_length,
                no_fat_chain,
            )
            .map_err(Error::from)?;
            if current_source_slot_range.entry_count() < required_entry_count {
                final_slot_range = target_view.slot_range();
            }
        }

        let (mut renamed_directory_bytes, source_slot_range, renamed_entry_set) =
            if final_slot_range == current_source_slot_range
                && current_source_slot_range.entry_count() < required_entry_count
            {
                let (latest_directory_bytes, reserved_slot_range) = self
                    .reserve_directory_entry_slots(
                        stream,
                        fs,
                        block_device,
                        boot_region,
                        required_entry_count,
                    )
                    .map_err(Error::from)?;
                final_slot_range = reserved_slot_range;
                let Some(latest_source_view) = Self::locate_named_child_view(
                    &latest_directory_bytes,
                    stream.data_length.is_none(),
                    upcase_table,
                    old_name,
                    old_name_hash,
                )
                .map_err(Error::from)?
                else {
                    return_errno!(Errno::ENOENT);
                };
                let source_slot_range = latest_source_view.slot_range();
                let renamed_entry_set =
                    direntry::renamed_entry_set(latest_source_view, new_name, new_name_hash)
                        .map_err(Error::from)?;
                (
                    latest_directory_bytes,
                    source_slot_range,
                    renamed_entry_set,
                )
            } else {
                (
                    current_directory_bytes,
                    current_source_slot_range,
                    current_renamed_entry_set,
                )
            };

        let target_slot_range = Self::locate_named_child_view(
            &renamed_directory_bytes,
            stream.data_length.is_none(),
            upcase_table,
            new_name,
            new_name_hash,
        )
        .map_err(Error::from)?
        .filter(|entry_view| {
            entry_view.slot_range() != source_slot_range && entry_view.slot_range() != final_slot_range
        })
        .map(FileEntrySetView::slot_range);
        if let Some(target_slot_range) = target_slot_range {
            let slot_range_bytes = Self::slot_range_bytes(target_slot_range).map_err(Error::from)?;
            let overwritten_entry_set = renamed_directory_bytes
                .get_mut(slot_range_bytes)
                .ok_or(MountVolumeStateError::InvalidOnDiskLayout)
                .map_err(Error::from)?;
            let mut overwritten_entry_set =
                WritableDirectoryEntrySlotSpan::new(target_slot_range, overwritten_entry_set)
                    .map_err(Error::from)?;
            direntry::invalidate_entry_set(&mut overwritten_entry_set).map_err(Error::from)?;
        }
        if final_slot_range != source_slot_range {
            let slot_range_bytes = Self::slot_range_bytes(source_slot_range).map_err(Error::from)?;
            let removed_entry_set = renamed_directory_bytes
                .get_mut(slot_range_bytes)
                .ok_or(MountVolumeStateError::InvalidOnDiskLayout)
                .map_err(Error::from)?;
            let mut removed_entry_set =
                WritableDirectoryEntrySlotSpan::new(source_slot_range, removed_entry_set)
                    .map_err(Error::from)?;
            direntry::invalidate_entry_set(&mut removed_entry_set).map_err(Error::from)?;
        }

        let final_slot_bytes = Self::slot_range_bytes(final_slot_range).map_err(Error::from)?;
        let destination_entry_set = renamed_directory_bytes
            .get_mut(final_slot_bytes)
            .ok_or(MountVolumeStateError::InvalidOnDiskLayout)
            .map_err(Error::from)?;
        let mut destination_entry_set =
            WritableDirectoryEntrySlotSpan::new(final_slot_range, destination_entry_set)
                .map_err(Error::from)?;
        direntry::invalidate_entry_set(&mut destination_entry_set).map_err(Error::from)?;
        destination_entry_set
            .bytes_mut()
            .get_mut(..renamed_entry_set.len())
            .ok_or(MountVolumeStateError::InvalidOnDiskLayout)
            .map_err(Error::from)?
            .copy_from_slice(&renamed_entry_set);
        Self::write_directory_bytes_for_stream(
            block_device,
            boot_region,
            &renamed_directory_bytes,
            *stream,
        )
        .map_err(Error::from)?;

        if !replaced_target_ranges.is_empty() {
            let _ = fs.free_allocated_space(&replaced_target_ranges);
        }
        Ok(())
    }

    fn rename_across_directories(
        &self,
        source_stream: &mut ExfatInodeStream,
        target_directory: &ExfatInode,
        target_stream: &mut ExfatInodeStream,
        fs: &Arc<ExfatFs>,
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        upcase_table: &UpcaseTable,
        old_name: &[u16],
        old_name_hash: u16,
        new_name: &[u16],
        new_name_hash: u16,
    ) -> Result<()> {
        let source_directory_bytes =
            Self::read_directory_bytes_for_stream(block_device, boot_region, *source_stream)
                .map_err(Error::from)?;
        let Some(source_view) = Self::locate_named_child_view(
            &source_directory_bytes,
            source_stream.data_length.is_none(),
            upcase_table,
            old_name,
            old_name_hash,
        )
        .map_err(Error::from)?
        else {
            return_errno!(Errno::ENOENT);
        };
        let source_slot_range = source_view.slot_range();
        let (source_inode_type, _, _, _) =
            source_view.child_metadata(boot_region).map_err(Error::from)?;
        let renamed_entry_set =
            direntry::renamed_entry_set(source_view, new_name, new_name_hash)
                .map_err(Error::from)?;
        let required_entry_count = renamed_entry_set.len() / DIRECTORY_ENTRY_SIZE;

        let target_directory_bytes =
            Self::read_directory_bytes_for_stream(block_device, boot_region, *target_stream)
                .map_err(Error::from)?;
        let target_view = Self::locate_named_child_view(
            &target_directory_bytes,
            target_stream.data_length.is_none(),
            upcase_table,
            new_name,
            new_name_hash,
        )
        .map_err(Error::from)?;
        let (mut published_target_directory_bytes, target_slot_range, replaced_target_ranges) =
            if let Some(target_view) = target_view {
                let target_slot_range = target_view.slot_range();
                let (target_inode_type, first_cluster, data_length, no_fat_chain) =
                    target_view.child_metadata(boot_region).map_err(Error::from)?;
                if source_inode_type == InodeType::Dir && target_inode_type != InodeType::Dir {
                    return_errno!(Errno::ENOTDIR);
                }
                if source_inode_type != InodeType::Dir && target_inode_type == InodeType::Dir {
                    return_errno!(Errno::EISDIR);
                }
                if target_inode_type == InodeType::Dir {
                    let child_inode = Self::child_inode_from_directory_entry(
                        fs,
                        boot_region,
                        target_stream.first_cluster,
                        target_slot_range,
                        target_inode_type,
                        first_cluster,
                        data_length,
                        data_length,
                        no_fat_chain,
                    )
                    .map_err(Error::from)?;
                    Self::ensure_directory_entry_is_empty(&child_inode, block_device, boot_region)?;
                }
                let target_ranges = Self::allocated_cluster_ranges(
                    block_device,
                    boot_region,
                    first_cluster,
                    data_length,
                    no_fat_chain,
                )
                .map_err(Error::from)?;
                (target_directory_bytes, target_slot_range, target_ranges)
            } else {
                let (latest_target_directory_bytes, reserved_slot_range) = target_directory
                    .reserve_directory_entry_slots(
                        target_stream,
                        fs,
                        block_device,
                        boot_region,
                        required_entry_count,
                    )
                    .map_err(Error::from)?;
                (latest_target_directory_bytes, reserved_slot_range, Vec::new())
            };

        let target_slot_bytes = Self::slot_range_bytes(target_slot_range).map_err(Error::from)?;
        let destination_entry_set = published_target_directory_bytes
            .get_mut(target_slot_bytes)
            .ok_or(MountVolumeStateError::InvalidOnDiskLayout)
            .map_err(Error::from)?;
        let mut destination_entry_set =
            WritableDirectoryEntrySlotSpan::new(target_slot_range, destination_entry_set)
                .map_err(Error::from)?;
        direntry::invalidate_entry_set(&mut destination_entry_set).map_err(Error::from)?;
        destination_entry_set
            .bytes_mut()
            .get_mut(..renamed_entry_set.len())
            .ok_or(MountVolumeStateError::InvalidOnDiskLayout)
            .map_err(Error::from)?
            .copy_from_slice(&renamed_entry_set);
        Self::write_directory_bytes_for_stream(
            block_device,
            boot_region,
            &published_target_directory_bytes,
            *target_stream,
        )
        .map_err(Error::from)?;

        let mut invalidated_source_directory_bytes = source_directory_bytes;
        let source_slot_bytes = Self::slot_range_bytes(source_slot_range).map_err(Error::from)?;
        let removed_entry_set = invalidated_source_directory_bytes
            .get_mut(source_slot_bytes)
            .ok_or(MountVolumeStateError::InvalidOnDiskLayout)
            .map_err(Error::from)?;
        let mut removed_entry_set =
            WritableDirectoryEntrySlotSpan::new(source_slot_range, removed_entry_set)
                .map_err(Error::from)?;
        direntry::invalidate_entry_set(&mut removed_entry_set).map_err(Error::from)?;
        Self::write_directory_bytes_for_stream(
            block_device,
            boot_region,
            &invalidated_source_directory_bytes,
            *source_stream,
        )
        .map_err(Error::from)?;

        if !replaced_target_ranges.is_empty() {
            let _ = fs.free_allocated_space(&replaced_target_ranges);
        }
        Ok(())
    }

    fn allocated_cluster_ranges(
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        first_cluster: u32,
        data_length: usize,
        no_fat_chain: bool,
    ) -> core::result::Result<Vec<ClusterRange>, MountVolumeStateError> {
        if data_length == 0 {
            if first_cluster != 0 {
                return Err(MountVolumeStateError::InvalidOnDiskLayout);
            }
            return Ok(Vec::new());
        }

        boot_region.validate_stream_data(
            first_cluster,
            u64::try_from(data_length).map_err(|_| MountVolumeStateError::InvalidOnDiskLayout)?,
        )?;
        let expected_cluster_count = data_length.div_ceil(boot_region.cluster_size);
        if no_fat_chain {
            return Ok(vec![ClusterRange {
                start_cluster: first_cluster,
                cluster_count: expected_cluster_count,
            }]);
        }

        let mut cluster_ranges = Vec::new();
        let mut current_range_start = 0u32;
        let mut current_range_count = 0usize;
        let mut previous_cluster: Option<u32> = None;
        let mut total_cluster_count = 0usize;
        let mut fat_reader = FatReader::new(block_device.as_ref(), boot_region);
        fat_reader.walk_cluster_chain(first_cluster, |cluster, _| {
            total_cluster_count = total_cluster_count
                .checked_add(1)
                .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
            match previous_cluster {
                Some(previous_cluster) if previous_cluster.checked_add(1) == Some(cluster) => {
                    current_range_count = current_range_count
                        .checked_add(1)
                        .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
                }
                Some(_) => {
                    cluster_ranges.push(ClusterRange {
                        start_cluster: current_range_start,
                        cluster_count: current_range_count,
                    });
                    current_range_start = cluster;
                    current_range_count = 1;
                }
                None => {
                    current_range_start = cluster;
                    current_range_count = 1;
                }
            }
            previous_cluster = Some(cluster);
            Ok(ChainVisitControl::Continue)
        })?;
        if current_range_count == 0 || total_cluster_count != expected_cluster_count {
            return Err(MountVolumeStateError::InvalidOnDiskLayout);
        }
        cluster_ranges.push(ClusterRange {
            start_cluster: current_range_start,
            cluster_count: current_range_count,
        });
        Ok(cluster_ranges)
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

impl PageCacheBackend for ExfatInode {
    fn read_page_async(&self, idx: usize, frame: &CachePage) -> Result<BioWaiter> {
        let fs = self
            .fs
            .upgrade()
            .ok_or_else(|| Error::with_message(Errno::EIO, "exFAT filesystem is not mounted"))?;
        let (block_device, boot_region, anomaly, _, _) =
            fs.published_lookup_state().map_err(Error::from)?;
        if anomaly.clear_to_zero || anomaly.media_failure {
            return_errno!(Errno::EIO);
        }

        let (stream, data_length, file_offset, initialized_len) =
            self.regular_file_page_range(idx)?;
        let initialized_sector_len =
            initialized_len - (initialized_len % boot_region.sector_size);
        if initialized_sector_len < PAGE_SIZE {
            frame
                .writer()
                .skip(initialized_sector_len)
                .fill_zeros(PAGE_SIZE - initialized_sector_len);
        }
        if initialized_sector_len == 0 {
            return Ok(BioWaiter::new());
        }

        Self::regular_file_page_waiter(
            &block_device,
            &boot_region,
            frame,
            &stream,
            data_length,
            file_offset,
            initialized_sector_len,
            BioType::Read,
        )
    }

    fn write_page_async(&self, idx: usize, frame: &CachePage) -> Result<BioWaiter> {
        let fs = self
            .fs
            .upgrade()
            .ok_or_else(|| Error::with_message(Errno::EIO, "exFAT filesystem is not mounted"))?;
        let (block_device, boot_region, anomaly, _, options) =
            fs.published_lookup_state().map_err(Error::from)?;
        if anomaly.clear_to_zero || anomaly.media_failure {
            return_errno!(Errno::EIO);
        }
        if options.fs_flags.contains(FsFlags::RDONLY) {
            return_errno!(Errno::EROFS);
        }

        let (stream, data_length, file_offset, initialized_len) =
            self.regular_file_page_range(idx)?;
        let initialized_sector_len =
            initialized_len - (initialized_len % boot_region.sector_size);
        if initialized_sector_len == 0 {
            return Ok(BioWaiter::new());
        }

        Self::regular_file_page_waiter(
            &block_device,
            &boot_region,
            frame,
            &stream,
            data_length,
            file_offset,
            initialized_sector_len,
            BioType::Write,
        )
    }

    fn npages(&self) -> usize {
        self.regular_file_npages().unwrap_or(0)
    }
}

impl crate::fs::vfs::inode::InodeIo for ExfatInode {
    fn read_at(
        &self,
        offset: usize,
        writer: &mut VmWriter,
        _status_flags: StatusFlags,
    ) -> Result<usize> {
        match self.type_() {
            InodeType::Dir => return_errno!(Errno::EISDIR),
            InodeType::File => {}
            _ => return_errno!(Errno::EOPNOTSUPP),
        }

        let fs = self
            .fs
            .upgrade()
            .ok_or_else(|| Error::with_message(Errno::EIO, "exFAT filesystem is not mounted"))?;
        let (block_device, boot_region, anomaly, _, _) =
            fs.published_lookup_state().map_err(Error::from)?;
        if anomaly.clear_to_zero || anomaly.media_failure {
            return_errno!(Errno::EIO);
        }

        self.read_regular_file_at(&block_device, &boot_region, offset, writer)
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
        if self.type_() != InodeType::File {
            return None;
        }

        self.page_cache
            .call_once(|| {
                let stream = self.validated_regular_file_stream().ok()?;
                let data_length = stream.data_length?;
                let this = self.this.upgrade()?;
                let backend: Arc<dyn PageCacheBackend> = this;
                PageCache::with_capacity(data_length, Arc::downgrade(&backend)).ok()
            })
            .as_ref()
            .map(|page_cache| page_cache.pages().clone())
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
        let _directory_mutation = fs.begin_directory_mutation();
        let (block_device, boot_region, _, upcase_table, options) =
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
        let (block_device, boot_region, _, _, _) =
            fs.published_lookup_state().map_err(Error::from)?;
        let (stream, directory_bytes) = self
            .read_revalidated_directory_bytes(&block_device, &boot_region)
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
            match Self::scan_directory_entry_at(
                stream.data_length.is_none(),
                &directory_bytes,
                entry_index,
            )? {
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
                        let entry_ino = (u64::from(stream.first_cluster) << 32)
                            | u64::from(
                                u32::try_from(entry_view.slot_range().first_entry_index())
                                    .map_err(|_| MountVolumeStateError::InvalidOnDiskLayout)?,
                            );
                        visitor.visit(
                            &entry_name,
                            entry_ino,
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

    fn unlink(&self, name: &str) -> Result<()> {
        if self.type_() != InodeType::Dir {
            return_errno!(Errno::ENOTDIR);
        }

        let fs = self
            .fs
            .upgrade()
            .ok_or_else(|| Error::with_message(Errno::EIO, "exFAT filesystem is not mounted"))?;
        let _directory_mutation = fs.begin_directory_mutation();
        let (block_device, boot_region, _, upcase_table, options) =
            fs.published_lookup_state().map_err(Error::from)?;
        if options.fs_flags.contains(FsFlags::RDONLY) {
            return_errno!(Errno::EROFS);
        }

        let admitted_name = Self::admitted_name(name, &options)?;
        let lookup_name_hash = upcase_table.name_hash(&admitted_name);

        let stream = self.stream.write();
        let is_root_directory = stream.data_length.is_none();
        let directory_bytes =
            Self::read_directory_bytes_for_stream(&block_device, &boot_region, *stream)
                .map_err(Error::from)?;
        let Some((slot_range, inode_type, first_cluster, data_length, _, no_fat_chain)) =
            Self::locate_named_child(
                &directory_bytes,
                is_root_directory,
                &boot_region,
                &upcase_table,
                &admitted_name,
                lookup_name_hash,
            )
            .map_err(Error::from)?
        else {
            return_errno!(Errno::ENOENT);
        };
        if inode_type == InodeType::Dir {
            return_errno!(Errno::EISDIR);
        }

        let allocated_cluster_ranges = Self::allocated_cluster_ranges(
            &block_device,
            &boot_region,
            first_cluster,
            data_length,
            no_fat_chain,
        )
        .map_err(Error::from)?;

        let mut invalidated_directory_bytes = directory_bytes;
        let slot_range_bytes = Self::slot_range_bytes(slot_range).map_err(Error::from)?;
        let removed_entry_set = invalidated_directory_bytes
            .get_mut(slot_range_bytes)
            .ok_or(MountVolumeStateError::InvalidOnDiskLayout)
            .map_err(Error::from)?;
        let mut removed_entry_set = WritableDirectoryEntrySlotSpan::new(slot_range, removed_entry_set)
            .map_err(Error::from)?;
        direntry::invalidate_entry_set(&mut removed_entry_set).map_err(Error::from)?;
        Self::write_directory_bytes_for_stream(
            &block_device,
            &boot_region,
            &invalidated_directory_bytes,
            *stream,
        )
        .map_err(Error::from)?;
        drop(stream);

        if !allocated_cluster_ranges.is_empty() {
            let _ = fs.free_allocated_space(&allocated_cluster_ranges);
        }
        Ok(())
    }

    fn rmdir(&self, name: &str) -> Result<()> {
        if self.type_() != InodeType::Dir {
            return_errno!(Errno::ENOTDIR);
        }

        let fs = self
            .fs
            .upgrade()
            .ok_or_else(|| Error::with_message(Errno::EIO, "exFAT filesystem is not mounted"))?;
        let _directory_mutation = fs.begin_directory_mutation();
        let (block_device, boot_region, _, upcase_table, options) =
            fs.published_lookup_state().map_err(Error::from)?;
        if options.fs_flags.contains(FsFlags::RDONLY) {
            return_errno!(Errno::EROFS);
        }

        let admitted_name = Self::admitted_name(name, &options)?;
        let lookup_name_hash = upcase_table.name_hash(&admitted_name);

        let stream = self.stream.write();
        let is_root_directory = stream.data_length.is_none();
        let directory_bytes =
            Self::read_directory_bytes_for_stream(&block_device, &boot_region, *stream)
                .map_err(Error::from)?;
        let Some((slot_range, inode_type, first_cluster, data_length, _, no_fat_chain)) =
            Self::locate_named_child(
                &directory_bytes,
                is_root_directory,
                &boot_region,
                &upcase_table,
                &admitted_name,
                lookup_name_hash,
            )
            .map_err(Error::from)?
        else {
            return_errno!(Errno::ENOENT);
        };
        if inode_type != InodeType::Dir {
            return_errno!(Errno::ENOTDIR);
        }

        let child_inode = Self::child_inode_from_directory_entry(
            &fs,
            &boot_region,
            stream.first_cluster,
            slot_range,
            inode_type,
            first_cluster,
            data_length,
            data_length,
            no_fat_chain,
        )
        .map_err(Error::from)?;
        Self::ensure_directory_entry_is_empty(&child_inode, &block_device, &boot_region)?;

        let allocated_cluster_ranges = Self::allocated_cluster_ranges(
            &block_device,
            &boot_region,
            first_cluster,
            data_length,
            no_fat_chain,
        )
        .map_err(Error::from)?;

        let mut invalidated_directory_bytes = directory_bytes;
        let slot_range_bytes = Self::slot_range_bytes(slot_range).map_err(Error::from)?;
        let removed_entry_set = invalidated_directory_bytes
            .get_mut(slot_range_bytes)
            .ok_or(MountVolumeStateError::InvalidOnDiskLayout)
            .map_err(Error::from)?;
        let mut removed_entry_set = WritableDirectoryEntrySlotSpan::new(slot_range, removed_entry_set)
            .map_err(Error::from)?;
        direntry::invalidate_entry_set(&mut removed_entry_set).map_err(Error::from)?;
        Self::write_directory_bytes_for_stream(
            &block_device,
            &boot_region,
            &invalidated_directory_bytes,
            *stream,
        )
        .map_err(Error::from)?;
        drop(stream);

        if !allocated_cluster_ranges.is_empty() {
            let _ = fs.free_allocated_space(&allocated_cluster_ranges);
        }
        Ok(())
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
        let (block_device, boot_region, _, upcase_table, options) =
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

    fn rename(&self, old_name: &str, target: &Arc<dyn Inode>, new_name: &str) -> Result<()> {
        if self.type_() != InodeType::Dir {
            return_errno!(Errno::ENOTDIR);
        }

        let Some(target_directory) = target.downcast_ref::<Self>() else {
            return_errno!(Errno::EXDEV);
        };
        if target_directory.type_() != InodeType::Dir {
            return_errno!(Errno::ENOTDIR);
        }

        let fs = self
            .fs
            .upgrade()
            .ok_or_else(|| Error::with_message(Errno::EIO, "exFAT filesystem is not mounted"))?;
        let target_fs = target_directory
            .fs
            .upgrade()
            .ok_or_else(|| Error::with_message(Errno::EIO, "exFAT filesystem is not mounted"))?;
        if !Arc::ptr_eq(&fs, &target_fs) {
            return_errno!(Errno::EXDEV);
        }

        let _directory_mutation = fs.begin_directory_mutation();
        let (block_device, boot_region, _, upcase_table, options) =
            fs.published_lookup_state().map_err(Error::from)?;
        if options.fs_flags.contains(FsFlags::RDONLY) {
            return_errno!(Errno::EROFS);
        }

        let admitted_old_name = Self::admitted_name(old_name, &options)?;
        let admitted_new_name = Self::admitted_name(new_name, &options)?;
        let old_name_hash = upcase_table.name_hash(&admitted_old_name);
        let new_name_hash = upcase_table.name_hash(&admitted_new_name);

        if self.ino() == target_directory.ino() {
            let mut stream = self.stream.write();
            return self.rename_within_directory(
                &mut stream,
                &fs,
                &block_device,
                &boot_region,
                &upcase_table,
                &admitted_old_name,
                old_name_hash,
                &admitted_new_name,
                new_name_hash,
            );
        }

        if self.ino() < target_directory.ino() {
            let mut source_stream = self.stream.write();
            let mut target_stream = target_directory.stream.write();
            return self.rename_across_directories(
                &mut source_stream,
                target_directory,
                &mut target_stream,
                &fs,
                &block_device,
                &boot_region,
                &upcase_table,
                &admitted_old_name,
                old_name_hash,
                &admitted_new_name,
                new_name_hash,
            );
        }

        let mut target_stream = target_directory.stream.write();
        let mut source_stream = self.stream.write();
        self.rename_across_directories(
            &mut source_stream,
            target_directory,
            &mut target_stream,
            &fs,
            &block_device,
            &boot_region,
            &upcase_table,
            &admitted_old_name,
            old_name_hash,
            &admitted_new_name,
            new_name_hash,
        )
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
#[path = "test_support/inode_ktests.rs"]
mod tests;
