// SPDX-License-Identifier: MPL-2.0

use alloc::{string::String, vec, vec::Vec};
use core::time::Duration;

use aster_block::{
    BlockDevice,
    bio::{Bio, BioDirection, BioSegment, BioStatus, BioType, BioWaiter},
    id::Sid,
};
use ostd::{
    mm::{FallibleVmWrite, Segment, VmIo, VmReader, io::util::HasVmReaderWriter},
    sync::{RwMutex, RwMutexReadGuard, RwMutexWriteGuard},
};
use spin::Once;
use time::{Date, Month, OffsetDateTime, PrimitiveDateTime, Time, UtcOffset};

use super::{
    bitmap::ClusterRange,
    boot::BootRegion,
    direntry::{
        self, DIRECTORY_ENTRY_SIZE, DirectoryEntryAnomalyKind, DirectoryEntrySlotRange,
        FileEntrySetView, ScannedDirectoryEntry, WritableDirectoryEntrySlotSpan,
    },
    fat::{ChainVisitControl, FatChainStep, FatReader},
    fs::{ExfatFs, ExfatMountOptions, MountVolumeStateError, MountedVolumeState},
    upcase::UpcaseTable,
};
use crate::{
    fs::{
        file::{AccessMode, FileIo, InodeMode, InodeType, StatusFlags, chmod, mkmod},
        utils::DirentVisitor,
        vfs::{
            file_system::{FileSystem, FsFlags},
            inode::{Extension, FallocMode, Inode, Metadata, MknodType, SymbolicLink},
            page_cache::{CachePage, PageCache, PageCacheBackend},
        },
    },
    prelude::*,
    process::{Gid, Uid},
    time::clocks::RealTimeCoarseClock,
    vm::vmo::Vmo,
};

#[derive(Clone, Copy, Eq, PartialEq)]
struct ExfatInodeStream {
    data_length: Option<usize>,
    first_cluster: u32,
    valid_data_length: Option<usize>,
    no_fat_chain: bool,
}

#[derive(Clone, Copy, Default)]
struct ExfatInodeDirtyState {
    data_generation: u64,
    required_metadata_generation: u64,
    metadata_generation: u64,
    persisted_data_generation: u64,
    persisted_required_metadata_generation: u64,
    persisted_metadata_generation: u64,
}

impl ExfatInodeDirtyState {
    fn mark_content_publication(&mut self) {
        self.data_generation = self.data_generation.saturating_add(1);
        self.required_metadata_generation = self.required_metadata_generation.saturating_add(1);
        self.metadata_generation = self.metadata_generation.saturating_add(1);
    }

    fn mark_metadata_publication(&mut self) {
        self.metadata_generation = self.metadata_generation.saturating_add(1);
    }

    fn needs_sync_data(self) -> bool {
        self.data_generation > self.persisted_data_generation
            || self.required_metadata_generation > self.persisted_required_metadata_generation
    }

    fn needs_sync_all(self) -> bool {
        self.needs_sync_data() || self.metadata_generation > self.persisted_metadata_generation
    }

    fn publish_data(&mut self, admitted: Self) {
        self.persisted_data_generation =
            self.persisted_data_generation.max(admitted.data_generation);
        self.persisted_required_metadata_generation = self
            .persisted_required_metadata_generation
            .max(admitted.required_metadata_generation);
    }

    fn publish_all(&mut self, admitted: Self) {
        self.publish_data(admitted);
        self.persisted_metadata_generation = self
            .persisted_metadata_generation
            .max(admitted.metadata_generation);
    }
}

#[derive(Clone, Copy)]
enum FileSyncScope {
    Data,
    All,
}

impl FileSyncScope {
    fn needs_device_sync(self, dirty_state: ExfatInodeDirtyState) -> bool {
        match self {
            Self::Data => dirty_state.needs_sync_data(),
            Self::All => dirty_state.needs_sync_all(),
        }
    }
}

#[derive(Clone, Copy)]
enum RewriteTarget {
    Directory,
    RegularFile,
}

#[derive(Clone, Copy)]
enum TimestampFieldKind {
    Accessed,
    Modified,
}

pub(super) struct ExfatInode {
    admission: RwMutex<()>,
    dirty_state: RwLock<ExfatInodeDirtyState>,
    extension: Extension,
    fs: Weak<ExfatFs>,
    metadata: RwLock<Metadata>,
    parent: Weak<Self>,
    page_cache: Once<Option<PageCache>>,
    stream: RwLock<ExfatInodeStream>,
    this: Weak<Self>,
}

impl ExfatInode {
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

    pub(super) fn read_root_directory<T>(
        &self,
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        read_root_directory_fn: impl FnOnce(&[u8]) -> core::result::Result<T, MountVolumeStateError>,
    ) -> core::result::Result<T, MountVolumeStateError> {
        let _directory_guard = self.admission.read();
        let stream = *self.stream.read();
        if stream.data_length.is_some() {
            return Err(MountVolumeStateError::InvalidOperationInput);
        }

        let directory_bytes =
            Self::read_directory_bytes_for_stream(block_device, boot_region, stream)?;
        read_root_directory_fn(&directory_bytes)
    }

    pub(super) fn rewrite_root_directory<T>(
        &self,
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        rewrite_root_directory_fn: impl FnOnce(
            &mut Vec<u8>,
        )
            -> core::result::Result<T, MountVolumeStateError>,
    ) -> core::result::Result<T, MountVolumeStateError> {
        let _directory_guards = Self::ordered_directory_write_guards(vec![self]);
        let stream = *self.stream.read();
        if stream.data_length.is_some() {
            return Err(MountVolumeStateError::InvalidOperationInput);
        }

        let mut directory_bytes =
            Self::read_directory_bytes_for_stream(block_device, boot_region, stream)?;
        let rewrite_result = rewrite_root_directory_fn(&mut directory_bytes)?;
        Self::write_directory_bytes_for_stream(
            block_device,
            boot_region,
            &directory_bytes,
            stream,
        )?;
        Ok(rewrite_result)
    }

    fn new(
        fs: &Arc<ExfatFs>,
        metadata: Metadata,
        first_cluster: u32,
        data_length: Option<usize>,
        valid_data_length: Option<usize>,
        no_fat_chain: bool,
        parent: Weak<Self>,
    ) -> Arc<Self> {
        Arc::new_cyclic(|weak_self| Self {
            admission: RwMutex::new(()),
            dirty_state: RwLock::new(ExfatInodeDirtyState::default()),
            extension: Extension::new(),
            fs: Arc::downgrade(fs),
            metadata: RwLock::new(metadata),
            parent,
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

    pub(super) fn new_root(fs: &Arc<ExfatFs>, root_cluster: u32, cluster_size: usize) -> Arc<Self> {
        let mut metadata = Metadata::new_dir(
            u64::from(root_cluster),
            mkmod!(u+rwx, g+rx, o+rx),
            cluster_size,
            fs.container_device_id(),
        );
        metadata.size = cluster_size;
        Self::new(fs, metadata, root_cluster, None, None, false, Weak::new())
    }

    fn new_child(
        fs: &Arc<ExfatFs>,
        parent: Weak<Self>,
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
            parent,
        )
    }

    fn regular_file_allocated_sectors(
        boot_region: &BootRegion,
        data_length: usize,
    ) -> core::result::Result<usize, MountVolumeStateError> {
        let allocated_clusters = if data_length == 0 {
            0
        } else {
            data_length.div_ceil(boot_region.cluster_size)
        };
        allocated_clusters
            .checked_mul(boot_region.sectors_per_cluster)
            .ok_or(MountVolumeStateError::InvalidOnDiskLayout)
    }

    fn decoded_exfat_timestamp(
        timestamp_bytes: [u8; 4],
        ten_ms_increment: Option<u8>,
        utc_offset_byte: u8,
    ) -> core::result::Result<Duration, MountVolumeStateError> {
        if timestamp_bytes == [0; 4] && ten_ms_increment.unwrap_or(0) == 0 {
            return Ok(Duration::ZERO);
        }

        let encoded_date = u16::from_le_bytes([timestamp_bytes[2], timestamp_bytes[3]]);
        let encoded_year = 1980i32 + i32::from(encoded_date >> 9);
        let encoded_month = u8::try_from((encoded_date >> 5) & 0x0f)
            .map_err(|_| MountVolumeStateError::InvalidOnDiskLayout)?;
        let encoded_day = u8::try_from(encoded_date & 0x1f)
            .map_err(|_| MountVolumeStateError::InvalidOnDiskLayout)?;
        let month = Month::try_from(encoded_month)
            .map_err(|_| MountVolumeStateError::InvalidOnDiskLayout)?;
        let date = Date::from_calendar_date(encoded_year, month, encoded_day)
            .map_err(|_| MountVolumeStateError::InvalidOnDiskLayout)?;

        let time = if let Some(ten_ms_increment) = ten_ms_increment {
            if ten_ms_increment >= 200 {
                return Err(MountVolumeStateError::InvalidOnDiskLayout);
            }

            let encoded_time = u16::from_le_bytes([timestamp_bytes[0], timestamp_bytes[1]]);
            let seconds = u8::try_from(encoded_time & 0x1f)
                .map_err(|_| MountVolumeStateError::InvalidOnDiskLayout)?
                .checked_mul(2)
                .and_then(|seconds| seconds.checked_add(ten_ms_increment / 100))
                .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
            let milliseconds = u16::from(ten_ms_increment % 100) * 10;
            let hour = u8::try_from((encoded_time >> 11) & 0x1f)
                .map_err(|_| MountVolumeStateError::InvalidOnDiskLayout)?;
            let minute = u8::try_from((encoded_time >> 5) & 0x3f)
                .map_err(|_| MountVolumeStateError::InvalidOnDiskLayout)?;
            Time::from_hms_milli(hour, minute, seconds, milliseconds)
                .map_err(|_| MountVolumeStateError::InvalidOnDiskLayout)?
        } else {
            Time::MIDNIGHT
        };

        let utc_offset = Self::exfat_utc_offset(utc_offset_byte)?;
        let date_time = PrimitiveDateTime::new(date, time).assume_offset(utc_offset);
        let unix_timestamp_nanos = u64::try_from(date_time.unix_timestamp_nanos())
            .map_err(|_| MountVolumeStateError::InvalidOnDiskLayout)?;
        Ok(Duration::from_nanos(unix_timestamp_nanos))
    }

    fn exfat_utc_offset(
        utc_offset_byte: u8,
    ) -> core::result::Result<UtcOffset, MountVolumeStateError> {
        if utc_offset_byte & 0x80 == 0 {
            return Ok(UtcOffset::UTC);
        }

        let quarter_hours = (((utc_offset_byte & 0x7f) as i8) << 1) >> 1;
        UtcOffset::from_whole_seconds(i32::from(quarter_hours) * 15 * 60)
            .map_err(|_| MountVolumeStateError::InvalidOnDiskLayout)
    }

    fn encoded_exfat_timestamp_fields(
        timestamp: Duration,
        utc_offset_byte: u8,
    ) -> Result<([u8; 4], u8, u8)> {
        let unix_nanos =
            i128::try_from(timestamp.as_nanos()).map_err(|_| Error::new(Errno::EINVAL))?;
        let utc_offset = Self::exfat_utc_offset(utc_offset_byte).map_err(Error::from)?;
        let date_time = OffsetDateTime::from_unix_timestamp_nanos(unix_nanos)
            .map_err(|_| Error::new(Errno::EINVAL))?
            .to_offset(utc_offset);
        let encoded_utc_offset = if utc_offset_byte & 0x80 == 0 {
            0
        } else {
            utc_offset_byte
        };
        let (
            encoded_year,
            encoded_month,
            encoded_day,
            encoded_hour,
            encoded_minute,
            encoded_second,
            encoded_millisecond,
        ) = match date_time.year() {
            ..1980 => (1980, 1u8, 1u8, 0u8, 0u8, 0u8, 0u16),
            2108.. => (2107, 12u8, 31u8, 23u8, 59u8, 59u8, 990u16),
            year => (
                year,
                date_time.month() as u8,
                date_time.day(),
                date_time.hour(),
                date_time.minute(),
                date_time.second(),
                date_time.millisecond(),
            ),
        };
        let date = ((u16::try_from(encoded_year - 1980).map_err(|_| Error::new(Errno::EINVAL))?)
            << 9)
            | (u16::from(encoded_month) << 5)
            | u16::from(encoded_day);
        let time = (u16::from(encoded_hour) << 11)
            | (u16::from(encoded_minute) << 5)
            | u16::from(encoded_second / 2);
        let date_bytes = date.to_le_bytes();
        let time_bytes = time.to_le_bytes();
        let hundredths_increment = u16::from(encoded_second % 2) * 100 + (encoded_millisecond / 10);
        Ok((
            [time_bytes[0], time_bytes[1], date_bytes[0], date_bytes[1]],
            u8::try_from(hundredths_increment).map_err(|_| Error::new(Errno::EINVAL))?,
            encoded_utc_offset,
        ))
    }

    fn directory_metadata_projection(&self) -> Result<Metadata> {
        let mut metadata = *self.metadata.read();
        if metadata.type_ != InodeType::Dir {
            return Ok(metadata);
        }

        let fs = self
            .fs
            .upgrade()
            .ok_or_else(|| Error::with_message(Errno::EIO, "exFAT filesystem is not mounted"))?;
        let (_state_guard, block_device, boot_region, anomaly, _upcase_table, _options) =
            fs.admitted_lookup_state().map_err(Error::from)?;
        if anomaly.clear_to_zero || anomaly.media_failure {
            return_errno!(Errno::EIO);
        }

        let Some(parent) = self.parent.upgrade() else {
            if self.stream.read().data_length.is_none() {
                return Ok(metadata);
            }
            return Err(Error::with_message(
                Errno::EIO,
                "ordinary exFAT directory parent is not published",
            ));
        };
        let _parent_guard = parent.admission.read();
        let parent_stream = *parent.stream.read();
        let directory_bytes =
            Self::read_directory_bytes_for_stream(&block_device, &boot_region, parent_stream)
                .map_err(Error::from)?;
        let entry_index =
            usize::try_from(metadata.ino as u32).map_err(|_| Error::new(Errno::EIO))?;
        let entry_view = match direntry::scan_directory_entry(
            parent_stream.data_length.is_none(),
            &directory_bytes,
            entry_index,
        )
        .map_err(Error::from)?
        {
            ScannedDirectoryEntry::File(entry_view) => entry_view,
            _ => return Err(Error::from(MountVolumeStateError::InvalidOnDiskLayout)),
        };
        if entry_view.file_attributes() & direntry::FILE_ATTRIBUTE_DIRECTORY == 0 {
            return Err(Error::from(MountVolumeStateError::InvalidOnDiskLayout));
        }

        let (inode_type, _first_cluster, data_length, _no_fat_chain) = entry_view
            .child_metadata(&boot_region)
            .map_err(Error::from)?;
        if inode_type != InodeType::Dir {
            return Err(Error::from(MountVolumeStateError::InvalidOnDiskLayout));
        }

        let entry_set = directory_bytes
            .get(direntry::slot_range_bytes(entry_view.slot_range()).map_err(Error::from)?)
            .ok_or(MountVolumeStateError::InvalidOnDiskLayout)
            .map_err(Error::from)?;
        let create_timestamp = entry_set
            .get(direntry::CREATE_TIMESTAMP_OFFSET..direntry::CREATE_TIMESTAMP_OFFSET + 4)
            .ok_or(MountVolumeStateError::InvalidOnDiskLayout)
            .map_err(Error::from)?
            .try_into()
            .map_err(|_| Error::from(MountVolumeStateError::InvalidOnDiskLayout))?;
        let create_ten_ms_increment = *entry_set
            .get(direntry::CREATE_10MS_INCREMENT_OFFSET)
            .ok_or(MountVolumeStateError::InvalidOnDiskLayout)
            .map_err(Error::from)?;
        let create_utc_offset = *entry_set
            .get(direntry::CREATE_UTC_OFFSET_OFFSET)
            .ok_or(MountVolumeStateError::InvalidOnDiskLayout)
            .map_err(Error::from)?;
        let _create_at = Self::decoded_exfat_timestamp(
            create_timestamp,
            Some(create_ten_ms_increment),
            create_utc_offset,
        )
        .map_err(Error::from)?;
        let last_accessed_timestamp = entry_set
            .get(
                direntry::LAST_ACCESSED_TIMESTAMP_OFFSET
                    ..direntry::LAST_ACCESSED_TIMESTAMP_OFFSET + 4,
            )
            .ok_or(MountVolumeStateError::InvalidOnDiskLayout)
            .map_err(Error::from)?
            .try_into()
            .map_err(|_| Error::from(MountVolumeStateError::InvalidOnDiskLayout))?;
        let last_accessed_utc_offset = *entry_set
            .get(direntry::LAST_ACCESSED_UTC_OFFSET_OFFSET)
            .ok_or(MountVolumeStateError::InvalidOnDiskLayout)
            .map_err(Error::from)?;
        let last_access_at =
            Self::decoded_exfat_timestamp(last_accessed_timestamp, None, last_accessed_utc_offset)
                .map_err(Error::from)?;
        let last_modified_timestamp = entry_set
            .get(
                direntry::LAST_MODIFIED_TIMESTAMP_OFFSET
                    ..direntry::LAST_MODIFIED_TIMESTAMP_OFFSET + 4,
            )
            .ok_or(MountVolumeStateError::InvalidOnDiskLayout)
            .map_err(Error::from)?
            .try_into()
            .map_err(|_| Error::from(MountVolumeStateError::InvalidOnDiskLayout))?;
        let last_modified_ten_ms_increment = *entry_set
            .get(direntry::LAST_MODIFIED_10MS_INCREMENT_OFFSET)
            .ok_or(MountVolumeStateError::InvalidOnDiskLayout)
            .map_err(Error::from)?;
        let last_modified_utc_offset = *entry_set
            .get(direntry::LAST_MODIFIED_UTC_OFFSET_OFFSET)
            .ok_or(MountVolumeStateError::InvalidOnDiskLayout)
            .map_err(Error::from)?;
        let last_modify_at = Self::decoded_exfat_timestamp(
            last_modified_timestamp,
            Some(last_modified_ten_ms_increment),
            last_modified_utc_offset,
        )
        .map_err(Error::from)?;
        let writable_bits = metadata.mode & mkmod!(a+w);
        metadata.mode = chmod!(metadata.mode, a-w);
        if entry_view.file_attributes() & direntry::FILE_ATTRIBUTE_READ_ONLY == 0 {
            metadata.mode |= writable_bits;
        }
        metadata.last_access_at = last_access_at;
        metadata.last_meta_change_at = last_modify_at;
        metadata.last_modify_at = last_modify_at;
        metadata.nr_sectors_allocated =
            Self::regular_file_allocated_sectors(&boot_region, data_length).map_err(Error::from)?;
        metadata.size = data_length;
        Ok(metadata)
    }

    fn rewrite_directory_self_entry_set(
        &self,
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        rewrite_entry_set_fn: impl FnOnce(FileEntrySetView<'_>, &[u8]) -> Result<Option<Vec<u8>>>,
        update_metadata_fn: impl FnOnce(&mut Metadata),
    ) -> Result<bool> {
        let parent = self.parent.upgrade().ok_or_else(|| {
            Error::with_message(
                Errno::EIO,
                "ordinary exFAT directory parent is not published",
            )
        })?;
        let _directory_guards = Self::ordered_directory_write_guards(vec![self, parent.as_ref()]);
        let parent_stream = *parent.stream.read();
        let mut directory_bytes =
            Self::read_directory_bytes_for_stream(block_device, boot_region, parent_stream)
                .map_err(Error::from)?;
        let entry_index =
            usize::try_from(self.metadata.read().ino as u32).map_err(|_| Error::new(Errno::EIO))?;
        let entry_view = match direntry::scan_directory_entry(
            parent_stream.data_length.is_none(),
            &directory_bytes,
            entry_index,
        )
        .map_err(Error::from)?
        {
            ScannedDirectoryEntry::File(entry_view) => entry_view,
            _ => return Err(Error::from(MountVolumeStateError::InvalidOnDiskLayout)),
        };
        if entry_view.file_attributes() & direntry::FILE_ATTRIBUTE_DIRECTORY == 0 {
            return Err(Error::from(MountVolumeStateError::InvalidOnDiskLayout));
        }
        let (inode_type, _first_cluster, _data_length, _no_fat_chain) = entry_view
            .child_metadata(boot_region)
            .map_err(Error::from)?;
        if inode_type != InodeType::Dir {
            return Err(Error::from(MountVolumeStateError::InvalidOnDiskLayout));
        }

        let slot_range_bytes = direntry::slot_range_bytes(entry_view.slot_range())
            .map_err(Error::from)?;
        let source_entry_set = directory_bytes
            .get(slot_range_bytes.clone())
            .ok_or(MountVolumeStateError::InvalidOnDiskLayout)
            .map_err(Error::from)?;
        let Some(republished_entry_set) = rewrite_entry_set_fn(entry_view, source_entry_set)?
        else {
            return Ok(false);
        };
        let destination_entry_set = directory_bytes
            .get_mut(slot_range_bytes)
            .ok_or(MountVolumeStateError::InvalidOnDiskLayout)
            .map_err(Error::from)?;
        destination_entry_set.copy_from_slice(&republished_entry_set);
        Self::write_directory_bytes_for_stream(
            block_device,
            boot_region,
            &directory_bytes,
            parent_stream,
        )
        .map_err(Error::from)?;
        let mut metadata = self.metadata.write();
        update_metadata_fn(&mut metadata);
        Ok(true)
    }

    fn refresh_directory_metadata_after_namespace_mutation(
        &self,
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        timestamp: Duration,
    ) -> Result<()> {
        if self.metadata.read().type_ != InodeType::Dir {
            return_errno!(Errno::ENOTDIR);
        }

        if self.stream.read().data_length.is_none() {
            let mut metadata = self.metadata.write();
            metadata.last_meta_change_at = timestamp;
            metadata.last_modify_at = timestamp;
            drop(metadata);
            self.mark_metadata_publication_dirty();
            return Ok(());
        }

        let durable_updated = self.rewrite_directory_self_entry_set(
            block_device,
            boot_region,
            |entry_view, source_entry_set| {
                let utc_offset_byte = *source_entry_set
                    .get(direntry::LAST_MODIFIED_UTC_OFFSET_OFFSET)
                    .ok_or(MountVolumeStateError::InvalidOnDiskLayout)
                    .map_err(Error::from)?;
                let (timestamp_bytes, ten_ms_increment, encoded_utc_offset_byte) =
                    Self::encoded_exfat_timestamp_fields(timestamp, utc_offset_byte)?;
                direntry::republished_entry_set(
                    entry_view,
                    &direntry::FileEntrySetFieldUpdates {
                        last_modified_fields: Some((
                            timestamp_bytes,
                            ten_ms_increment,
                            encoded_utc_offset_byte,
                        )),
                        ..Default::default()
                    },
                )
                .map(Some)
                .map_err(Error::from)
            },
            |metadata| {
                metadata.last_meta_change_at = timestamp;
                metadata.last_modify_at = timestamp;
            },
        )?;
        if durable_updated {
            self.mark_metadata_publication_dirty();
        }
        Ok(())
    }

    fn metadata_projection(&self) -> Metadata {
        let mut metadata = *self.metadata.read();
        if metadata.type_ == InodeType::Dir {
            return self.directory_metadata_projection().unwrap_or(metadata);
        }
        if metadata.type_ != InodeType::File {
            return metadata;
        }

        let Some(fs) = self.fs.upgrade() else {
            return metadata;
        };
        let Ok((state_guard, _block_device, boot_region, anomaly, _upcase_table, _options)) =
            fs.admitted_lookup_state()
        else {
            return metadata;
        };
        if anomaly.clear_to_zero || anomaly.media_failure {
            drop(state_guard);
            return metadata;
        }

        let Ok((owner_guard, _stream, data_length, _valid_data_length)) =
            self.admitted_regular_file_stream_snapshot()
        else {
            drop(state_guard);
            return metadata;
        };
        let Ok(allocated_sectors) = Self::regular_file_allocated_sectors(&boot_region, data_length)
        else {
            drop(owner_guard);
            drop(state_guard);
            return metadata;
        };
        metadata.size = data_length;
        metadata.nr_sectors_allocated = allocated_sectors;
        drop(owner_guard);
        drop(state_guard);
        metadata
    }

    fn admitted_directory_snapshot(
        &self,
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
    ) -> core::result::Result<
        (RwMutexReadGuard<'_, ()>, ExfatInodeStream, Vec<u8>),
        MountVolumeStateError,
    > {
        let owner = self.admission.read();
        let stream = *self.stream.read();
        let directory_bytes =
            Self::read_directory_bytes_for_stream(block_device, boot_region, stream)?;
        Ok((owner, stream, directory_bytes))
    }

    fn admitted_regular_file_stream_snapshot(
        &self,
    ) -> Result<(RwMutexReadGuard<'_, ()>, ExfatInodeStream, usize, usize)> {
        match self.metadata.read().type_ {
            InodeType::Dir => return_errno!(Errno::EISDIR),
            InodeType::File => {}
            _ => return_errno!(Errno::EOPNOTSUPP),
        }

        let owner = self.admission.read();
        let stream = *self.stream.read();
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
        Ok((owner, stream, data_length, valid_data_length))
    }

    fn ordered_directory_write_guards<'a>(
        mut directories: Vec<&'a ExfatInode>,
    ) -> Vec<RwMutexWriteGuard<'a, ()>> {
        directories.sort_by_key(|directory| directory.metadata.read().ino);
        directories.dedup_by_key(|directory| directory.metadata.read().ino);
        directories
            .into_iter()
            .map(|directory| directory.admission.write())
            .collect()
    }

    fn validate_regular_file_mapping_shape(
        boot_region: &BootRegion,
        stream: &ExfatInodeStream,
        data_length: usize,
    ) -> Result<()> {
        let data_length_u64 = u64::try_from(data_length).map_err(|_| Error::new(Errno::EINVAL))?;
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
        let fs = self
            .fs
            .upgrade()
            .ok_or_else(|| Error::with_message(Errno::EIO, "exFAT filesystem is not mounted"))?;
        let (_, _, anomaly, _, _) = fs.published_lookup_state().map_err(Error::from)?;
        if anomaly.clear_to_zero || anomaly.media_failure {
            return_errno!(Errno::EIO);
        }

        let (_owner_guard, stream, data_length, valid_data_length) =
            self.admitted_regular_file_stream_snapshot()?;
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
        let mut fat_reader =
            (!stream.no_fat_chain).then(|| FatReader::new(block_device.as_ref(), boot_region));

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
        idx: usize,
        data_length: usize,
        valid_data_length: usize,
    ) -> Result<(usize, usize)> {
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

        Ok((file_offset, initialized_len))
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
            bio_waiter.concat(bio.submit(block_device.as_ref()).map_err(Error::from)?);
        }

        Ok(bio_waiter)
    }

    fn read_regular_file_at(
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        stream: ExfatInodeStream,
        data_length: usize,
        valid_data_length: usize,
        offset: usize,
        writer: &mut VmWriter,
    ) -> Result<usize> {
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

    fn mutate_regular_file_range(
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        stream: &ExfatInodeStream,
        data_length: usize,
        offset: usize,
        len: usize,
        mut fill_chunk_fn: impl FnMut(&mut [u8]) -> Result<()>,
    ) -> Result<()> {
        if len == 0 {
            return Ok(());
        }

        Self::validate_regular_file_mapping_shape(boot_region, stream, data_length)?;
        let write_end = offset
            .checked_add(len)
            .ok_or_else(|| Error::new(Errno::EINVAL))?;
        if write_end > data_length {
            return_errno!(Errno::EOPNOTSUPP);
        }

        let cluster_size = boot_region.cluster_size;
        let cluster_index = offset / cluster_size;
        let mut cluster_offset = offset % cluster_size;
        let mut remaining = len;
        let mut current_cluster = Self::mapped_regular_file_cluster(
            block_device,
            boot_region,
            stream,
            data_length,
            cluster_index,
        )?;
        let mut fat_reader =
            (!stream.no_fat_chain).then(|| FatReader::new(block_device.as_ref(), boot_region));
        let mut cluster_buffer = vec![0; cluster_size];
        while remaining != 0 {
            let chunk_len = remaining.min(cluster_size - cluster_offset);
            let chunk_end = cluster_offset
                .checked_add(chunk_len)
                .ok_or_else(|| Error::new(Errno::EINVAL))?;
            let cluster_start = boot_region
                .cluster_offset(current_cluster)
                .map_err(Error::from)?;
            block_device
                .read_bytes(cluster_start, &mut cluster_buffer)
                .map_err(|_| Error::new(Errno::EIO))?;
            fill_chunk_fn(&mut cluster_buffer[cluster_offset..chunk_end])?;
            block_device
                .write_bytes(cluster_start, &cluster_buffer)
                .map_err(|_| Error::new(Errno::EIO))?;
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
        Ok(())
    }

    fn grow_regular_file_stream(
        fs: &Arc<ExfatFs>,
        publication: &MountedVolumeState,
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        stream: ExfatInodeStream,
        new_data_length: usize,
    ) -> Result<ExfatInodeStream> {
        let Some(current_data_length) = stream.data_length else {
            return_errno!(Errno::EINVAL);
        };
        let Some(current_valid_data_length) = stream.valid_data_length else {
            return_errno!(Errno::EINVAL);
        };
        if current_valid_data_length > current_data_length || new_data_length < current_data_length
        {
            return_errno!(Errno::EINVAL);
        }
        if new_data_length == current_data_length {
            return Ok(stream);
        }

        let current_allocated_clusters = if current_data_length == 0 {
            0
        } else {
            current_data_length.div_ceil(boot_region.cluster_size)
        };
        let target_allocated_clusters = new_data_length.div_ceil(boot_region.cluster_size);
        if target_allocated_clusters == current_allocated_clusters {
            return Ok(ExfatInodeStream {
                data_length: Some(new_data_length),
                ..stream
            });
        }

        let additional_clusters = target_allocated_clusters
            .checked_sub(current_allocated_clusters)
            .ok_or_else(|| Error::new(Errno::EINVAL))?;
        let (allocated_ranges, _) = fs
            .allocate_free_space_with_publication(publication, additional_clusters)
            .map_err(Error::from)?;
        let allocated_cluster_count =
            allocated_ranges
                .iter()
                .try_fold(0usize, |total_clusters, range| {
                    total_clusters
                        .checked_add(range.cluster_count)
                        .ok_or_else(|| Error::from(MountVolumeStateError::InconsistentAccounting))
                })?;
        if allocated_cluster_count != additional_clusters {
            return Err(Error::from(MountVolumeStateError::InconsistentAccounting));
        }
        let first_new_cluster = allocated_ranges
            .first()
            .ok_or_else(|| Error::from(MountVolumeStateError::InconsistentAccounting))?
            .start_cluster;
        let stays_contiguous = if current_allocated_clusters == 0 {
            allocated_ranges.len() == 1
        } else if stream.no_fat_chain {
            stream.first_cluster.checked_add(
                u32::try_from(current_allocated_clusters)
                    .map_err(|_| Error::from(MountVolumeStateError::InvalidOnDiskLayout))?,
            ) == Some(first_new_cluster)
                && allocated_ranges.len() == 1
        } else {
            false
        };
        if !stays_contiguous {
            let mut fat_reader = FatReader::new(block_device.as_ref(), boot_region);
            let link_allocated_ranges_fn =
                |fat_reader: &mut FatReader<'_>| -> core::result::Result<(), MountVolumeStateError> {
                    for (range_index, range) in allocated_ranges.iter().enumerate() {
                        let next_range_start = allocated_ranges
                            .get(range_index + 1)
                            .map(|next_range| next_range.start_cluster);
                        match (range.cluster_count, next_range_start) {
                            (0, _) => return Err(MountVolumeStateError::InvalidOperationInput),
                            (1, None) => fat_reader.terminate_cluster_chain(range.start_cluster)?,
                            (cluster_count, None) => {
                                let last_cluster = range
                                    .start_cluster
                                    .checked_add(
                                        u32::try_from(cluster_count - 1)
                                            .map_err(|_| MountVolumeStateError::InvalidOperationInput)?,
                                    )
                                    .ok_or(MountVolumeStateError::InvalidOperationInput)?;
                                fat_reader.link_contiguous_chain_to_cluster(
                                    range.start_cluster,
                                    cluster_count - 1,
                                    last_cluster,
                                )?;
                            }
                            (cluster_count, Some(next_cluster)) => {
                                fat_reader.link_contiguous_chain_to_cluster(
                                    range.start_cluster,
                                    cluster_count,
                                    next_cluster,
                                )?;
                            }
                        }
                    }
                    Ok(())
                };
            if current_allocated_clusters == 0 {
                link_allocated_ranges_fn(&mut fat_reader).map_err(Error::from)?;
            } else {
                if stream.no_fat_chain {
                    fat_reader
                        .link_contiguous_chain_to_cluster(
                            stream.first_cluster,
                            current_allocated_clusters,
                            first_new_cluster,
                        )
                        .map_err(Error::from)?;
                } else {
                    fat_reader
                        .append_cluster_to_chain(stream.first_cluster, first_new_cluster)
                        .map_err(Error::from)?;
                }
                link_allocated_ranges_fn(&mut fat_reader).map_err(Error::from)?;
            }
        }

        Ok(ExfatInodeStream {
            data_length: Some(new_data_length),
            first_cluster: if current_allocated_clusters == 0 {
                first_new_cluster
            } else {
                stream.first_cluster
            },
            no_fat_chain: stays_contiguous,
            ..stream
        })
    }

    fn republish_regular_file_entry_set(
        &self,
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        published_stream: ExfatInodeStream,
        timestamp: Duration,
    ) -> Result<()> {
        let Some(data_length) = published_stream.data_length else {
            return_errno!(Errno::EINVAL);
        };
        let Some(valid_data_length) = published_stream.valid_data_length else {
            return_errno!(Errno::EINVAL);
        };
        if valid_data_length > data_length {
            return_errno!(Errno::EINVAL);
        }
        if data_length == 0 {
            if published_stream.first_cluster != 0 || valid_data_length != 0 {
                return_errno!(Errno::EINVAL);
            }
        } else {
            boot_region
                .validate_stream_data(
                    published_stream.first_cluster,
                    u64::try_from(data_length).map_err(|_| Error::new(Errno::EINVAL))?,
                )
                .map_err(Error::from)?;
        }

        let (timestamp_bytes, hundredths_increment, utc_offset_byte) =
            Self::encoded_exfat_timestamp_fields(timestamp, 0)?;
        let last_modified_fields = (timestamp_bytes, hundredths_increment, utc_offset_byte);
        self.rewrite_regular_file_entry_set(
            block_device,
            boot_region,
            |entry_view, _source_entry_set| {
                let stream_flags = if published_stream.no_fat_chain {
                    0x03
                } else {
                    0x01
                };
                let valid_data_length =
                    u64::try_from(valid_data_length).map_err(|_| Error::new(Errno::EINVAL))?;
                let data_length =
                    u64::try_from(data_length).map_err(|_| Error::new(Errno::EINVAL))?;
                direntry::republished_entry_set(
                    entry_view,
                    &direntry::FileEntrySetFieldUpdates {
                        data_length: Some(data_length),
                        first_cluster: Some(published_stream.first_cluster),
                        last_modified_fields: Some(last_modified_fields),
                        stream_flags: Some(stream_flags),
                        valid_data_length: Some(valid_data_length),
                        ..Default::default()
                    },
                )
                .map(Some)
                .map_err(Error::from)
            },
        )?;
        Ok(())
    }

    fn rewrite_regular_file_entry_set(
        &self,
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        rewrite_entry_set_fn: impl FnOnce(FileEntrySetView<'_>, &[u8]) -> Result<Option<Vec<u8>>>,
    ) -> Result<bool> {
        let parent = self
            .parent
            .upgrade()
            .ok_or_else(|| Error::new(Errno::EIO))?;
        let _parent_guard = parent.admission.write();
        let parent_stream = *parent.stream.read();
        let mut directory_bytes =
            Self::read_directory_bytes_for_stream(block_device, boot_region, parent_stream)
                .map_err(Error::from)?;
        let entry_index =
            usize::try_from(self.metadata.read().ino as u32).map_err(|_| Error::new(Errno::EIO))?;
        let entry_view = match direntry::scan_directory_entry(
            parent_stream.data_length.is_none(),
            &directory_bytes,
            entry_index,
        )
        .map_err(Error::from)?
        {
            ScannedDirectoryEntry::File(entry_view) => entry_view,
            _ => return Err(Error::from(MountVolumeStateError::InvalidOnDiskLayout)),
        };
        let slot_range = entry_view.slot_range();
        let slot_range_bytes = direntry::slot_range_bytes(slot_range).map_err(Error::from)?;
        let source_entry_set = directory_bytes
            .get(slot_range_bytes.clone())
            .ok_or(MountVolumeStateError::InvalidOnDiskLayout)
            .map_err(Error::from)?;
        let Some(republished_entry_set) = rewrite_entry_set_fn(entry_view, source_entry_set)?
        else {
            return Ok(false);
        };
        let destination_entry_set = directory_bytes
            .get_mut(slot_range_bytes)
            .ok_or(MountVolumeStateError::InvalidOnDiskLayout)
            .map_err(Error::from)?;
        destination_entry_set.copy_from_slice(&republished_entry_set);
        Self::write_directory_bytes_for_stream(
            block_device,
            boot_region,
            &directory_bytes,
            parent_stream,
        )
        .map_err(Error::from)?;
        Ok(true)
    }

    fn mark_content_publication_dirty(&self) {
        self.dirty_state.write().mark_content_publication();
    }

    fn mark_metadata_publication_dirty(&self) {
        self.dirty_state.write().mark_metadata_publication();
    }

    fn sync_regular_file(&self, scope: FileSyncScope) -> Result<()> {
        let fs = self
            .fs
            .upgrade()
            .ok_or_else(|| Error::with_message(Errno::EIO, "exFAT filesystem is not mounted"))?;
        let (state_guard, block_device, _boot_region, anomaly, _upcase_table, _options) =
            fs.admitted_lookup_state().map_err(Error::from)?;
        if anomaly.clear_to_zero || anomaly.media_failure {
            return_errno!(Errno::EIO);
        }

        let (owner_guard, _stream, data_length, _valid_data_length) =
            self.admitted_regular_file_stream_snapshot()?;
        let admitted_dirty_state = *self.dirty_state.read();
        let needs_device_sync = scope.needs_device_sync(admitted_dirty_state);
        let page_cache = self
            .page_cache
            .get()
            .and_then(|maybe_page_cache| maybe_page_cache.as_ref());
        drop(owner_guard);
        drop(state_guard);

        let needs_page_writeback = page_cache.is_some_and(|page_cache| {
            data_length != 0 && page_cache.has_dirty_pages(0..data_length)
        });

        if needs_page_writeback {
            if let Some(page_cache) = page_cache {
                page_cache.evict_range(0..data_length)?;
                let (_state_guard, _block_device, _boot_region, anomaly, _upcase_table, _options) =
                    fs.admitted_lookup_state().map_err(Error::from)?;
                if anomaly.clear_to_zero || anomaly.media_failure {
                    return_errno!(Errno::EIO);
                }

                let (_owner_guard, _stream, _data_length, _valid_data_length) =
                    self.admitted_regular_file_stream_snapshot()?;
            }
        }

        if needs_page_writeback || needs_device_sync {
            match block_device.sync()? {
                BioStatus::Complete => {
                    let (_state_guard, _block_device, _boot_region, anomaly, _upcase_table, _options) =
                        fs.admitted_lookup_state().map_err(Error::from)?;
                    if anomaly.clear_to_zero || anomaly.media_failure {
                        return_errno!(Errno::EIO);
                    }

                    let (_owner_guard, _stream, _data_length, _valid_data_length) =
                        self.admitted_regular_file_stream_snapshot()?;
                    let mut dirty_state = self.dirty_state.write();
                    match scope {
                        FileSyncScope::Data => dirty_state.publish_data(admitted_dirty_state),
                        FileSyncScope::All => dirty_state.publish_all(admitted_dirty_state),
                    }
                    Ok(())
                }
                _ => return_errno!(Errno::EIO),
            }
        } else {
            Ok(())
        }
    }

    fn first_directory_child_scan<'a>(
        &self,
        stream: ExfatInodeStream,
        directory_bytes: &'a [u8],
    ) -> core::result::Result<Option<ScannedDirectoryEntry<'a>>, MountVolumeStateError> {
        let is_root_directory = stream.data_length.is_none();
        let mut entry_index = 0usize;
        loop {
            let entry_scan =
                direntry::scan_directory_entry(is_root_directory, directory_bytes, entry_index)?;
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
            match direntry::scan_directory_entry(is_root_directory, directory_bytes, entry_index)? {
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
        mut stream: ExfatInodeStream,
        publication: &mut MountedVolumeState,
        fs: &Arc<ExfatFs>,
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        required_entry_count: usize,
    ) -> core::result::Result<
        (ExfatInodeStream, Vec<u8>, DirectoryEntrySlotRange),
        MountVolumeStateError,
    > {
        loop {
            let directory_bytes =
                Self::read_directory_bytes_for_stream(block_device, boot_region, stream)?;
            if let Some(slot_range) = Self::find_vacant_entry_slots(
                stream.data_length.is_none(),
                &directory_bytes,
                required_entry_count,
            )? {
                return Ok((stream, directory_bytes, slot_range));
            }
            stream =
                self.grow_directory_stream(stream, publication, fs, block_device, boot_region)?;
        }
    }

    fn grow_directory_stream(
        &self,
        stream: ExfatInodeStream,
        publication: &mut MountedVolumeState,
        fs: &Arc<ExfatFs>,
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
    ) -> core::result::Result<ExfatInodeStream, MountVolumeStateError> {
        let (allocated_ranges, _) = fs.allocate_free_space_with_publication(publication, 1)?;
        let allocated_cluster = match allocated_ranges.as_slice() {
            [allocated_range] if allocated_range.cluster_count == 1 => {
                allocated_range.start_cluster
            }
            _ => {
                let _ = fs.free_allocated_space_with_publication(publication, &allocated_ranges);
                return Err(MountVolumeStateError::InconsistentAccounting);
            }
        };

        if let Err(error) =
            Self::initialize_directory_cluster(block_device, boot_region, allocated_cluster)
        {
            let _ = fs.free_allocated_space_with_publication(publication, &allocated_ranges);
            return Err(error);
        }

        let updated_stream = match self.attach_directory_cluster(
            stream,
            block_device,
            boot_region,
            allocated_cluster,
        ) {
            Ok(updated_stream) => updated_stream,
            Err(error) => {
                let _ = fs.free_allocated_space_with_publication(publication, &allocated_ranges);
                return Err(error);
            }
        };
        Ok(updated_stream)
    }

    fn attach_directory_cluster(
        &self,
        stream: ExfatInodeStream,
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        allocated_cluster: u32,
    ) -> core::result::Result<ExfatInodeStream, MountVolumeStateError> {
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

        let updated_stream = match stream.data_length {
            Some(0) => ExfatInodeStream {
                first_cluster: allocated_cluster,
                data_length: Some(next_data_length),
                no_fat_chain: false,
                ..stream
            },
            Some(_) if stream.no_fat_chain => ExfatInodeStream {
                data_length: Some(next_data_length),
                no_fat_chain: false,
                ..stream
            },
            Some(_) => ExfatInodeStream {
                data_length: Some(next_data_length),
                ..stream
            },
            None => ExfatInodeStream {
                data_length: None,
                ..stream
            },
        };
        {
            let mut published_stream = self.stream.write();
            if *published_stream != stream {
                return Err(MountVolumeStateError::InvalidOnDiskLayout);
            }
            *published_stream = updated_stream;
        }
        let mut metadata = self.metadata.write();
        metadata.size = metadata
            .size
            .checked_add(boot_region.cluster_size)
            .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
        Ok(updated_stream)
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
        let (_owner_guard, stream, directory_bytes) =
            self.admitted_directory_snapshot(block_device, boot_region)?;
        let Some(entry_view) = Self::locate_named_child_view(
            &directory_bytes,
            stream.data_length.is_none(),
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
        let ino = (u64::from(stream.first_cluster) << 32)
            | u64::from(
                u32::try_from(slot_range.first_entry_index())
                    .map_err(|_| MountVolumeStateError::InvalidOnDiskLayout)?,
            );
        let entry_set = directory_bytes
            .get(direntry::slot_range_bytes(slot_range)?)
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

        let child_inode = Self::new_child(
            fs,
            self.this.clone(),
            ino,
            inode_type,
            boot_region.cluster_size,
            data_length,
            first_cluster,
            data_length,
            valid_data_length,
            no_fat_chain,
        );
        if inode_type == InodeType::File {
            let last_accessed_timestamp = entry_set
                .get(
                    direntry::LAST_ACCESSED_TIMESTAMP_OFFSET
                        ..direntry::LAST_ACCESSED_TIMESTAMP_OFFSET + 4,
                )
                .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?
                .try_into()
                .map_err(|_| MountVolumeStateError::InvalidOnDiskLayout)?;
            let last_accessed_utc_offset = *entry_set
                .get(direntry::LAST_ACCESSED_UTC_OFFSET_OFFSET)
                .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
            let last_access_at = Self::decoded_exfat_timestamp(
                last_accessed_timestamp,
                None,
                last_accessed_utc_offset,
            )?;
            let last_modified_timestamp = entry_set
                .get(
                    direntry::LAST_MODIFIED_TIMESTAMP_OFFSET
                        ..direntry::LAST_MODIFIED_TIMESTAMP_OFFSET + 4,
                )
                .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?
                .try_into()
                .map_err(|_| MountVolumeStateError::InvalidOnDiskLayout)?;
            let last_modified_ten_ms_increment = *entry_set
                .get(direntry::LAST_MODIFIED_10MS_INCREMENT_OFFSET)
                .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
            let last_modified_utc_offset = *entry_set
                .get(direntry::LAST_MODIFIED_UTC_OFFSET_OFFSET)
                .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
            let last_modify_at = Self::decoded_exfat_timestamp(
                last_modified_timestamp,
                Some(last_modified_ten_ms_increment),
                last_modified_utc_offset,
            )?;
            let allocated_sectors = Self::regular_file_allocated_sectors(boot_region, data_length)?;
            let mut metadata = child_inode.metadata.write();
            if entry_view.file_attributes() & direntry::FILE_ATTRIBUTE_READ_ONLY != 0 {
                metadata.mode = chmod!(metadata.mode, a-w);
            }
            metadata.last_access_at = last_access_at;
            metadata.last_meta_change_at = last_modify_at;
            metadata.last_modify_at = last_modify_at;
            metadata.nr_sectors_allocated = allocated_sectors;
            metadata.size = data_length;
        }
        let child_inode: Arc<dyn Inode> = child_inode;
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
            .get(direntry::slot_range_bytes(slot_range)?)
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
            match direntry::scan_directory_entry(is_root_directory, directory_bytes, entry_index)? {
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
                ScannedDirectoryEntry::Anomaly { kind, slot_range } => {
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
        parent: &Self,
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
            parent.this.clone(),
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
        let (_owner_guard, stream, child_directory_bytes) = child_inode
            .admitted_directory_snapshot(block_device, boot_region)
            .map_err(Error::from)?;
        if let Some(first_child_scan) = child_inode
            .first_directory_child_scan(stream, &child_directory_bytes)
            .map_err(Error::from)?
        {
            match first_child_scan {
                ScannedDirectoryEntry::Anomaly { .. } => {
                    return Err(Error::from(MountVolumeStateError::InvalidOnDiskLayout));
                }
                ScannedDirectoryEntry::File(_) => return_errno!(Errno::ENOTEMPTY),
                ScannedDirectoryEntry::EndOfDirectory { .. } | ScannedDirectoryEntry::Vacant(_) => {
                    unreachable!()
                }
            }
        }
        Ok(())
    }

    fn rename_within_directory(
        &self,
        mut stream: ExfatInodeStream,
        target_child_inode: Option<&Arc<Self>>,
        publication: &mut MountedVolumeState,
        fs: &Arc<ExfatFs>,
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        upcase_table: &UpcaseTable,
        old_name: &[u16],
        old_name_hash: u16,
        new_name: &[u16],
        new_name_hash: u16,
    ) -> Result<bool> {
        let current_directory_bytes =
            Self::read_directory_bytes_for_stream(block_device, boot_region, stream)
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
            return Ok(false);
        }
        let current_renamed_entry_set =
            direntry::renamed_entry_set(current_source_view, new_name, new_name_hash)
                .map_err(Error::from)?;
        let required_entry_count = current_renamed_entry_set.len() / DIRECTORY_ENTRY_SIZE;

        let (source_inode_type, _, _, _) = current_source_view
            .child_metadata(boot_region)
            .map_err(Error::from)?;
        let mut replaced_target_ranges = Vec::new();
        let mut final_slot_range = current_source_slot_range;
        if let Some(target_view) = current_target_view
            .filter(|entry_view| entry_view.slot_range() != current_source_slot_range)
        {
            let (target_inode_type, first_cluster, data_length, no_fat_chain) = target_view
                .child_metadata(boot_region)
                .map_err(Error::from)?;
            if source_inode_type == InodeType::Dir && target_inode_type != InodeType::Dir {
                return_errno!(Errno::ENOTDIR);
            }
            if source_inode_type != InodeType::Dir && target_inode_type == InodeType::Dir {
                return_errno!(Errno::EISDIR);
            }
            if target_inode_type == InodeType::Dir {
                let Some(child_inode) = target_child_inode else {
                    return Err(Error::from(MountVolumeStateError::InvalidOnDiskLayout));
                };
                Self::ensure_directory_entry_is_empty(child_inode, block_device, boot_region)?;
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
                let (updated_stream, latest_directory_bytes, reserved_slot_range) = self
                    .reserve_directory_entry_slots(
                        stream,
                        publication,
                        fs,
                        block_device,
                        boot_region,
                        required_entry_count,
                    )
                    .map_err(Error::from)?;
                stream = updated_stream;
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
                (latest_directory_bytes, source_slot_range, renamed_entry_set)
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
            entry_view.slot_range() != source_slot_range
                && entry_view.slot_range() != final_slot_range
        })
        .map(FileEntrySetView::slot_range);
        if let Some(target_slot_range) = target_slot_range {
            let slot_range_bytes =
                direntry::slot_range_bytes(target_slot_range).map_err(Error::from)?;
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
            let slot_range_bytes =
                direntry::slot_range_bytes(source_slot_range).map_err(Error::from)?;
            let removed_entry_set = renamed_directory_bytes
                .get_mut(slot_range_bytes)
                .ok_or(MountVolumeStateError::InvalidOnDiskLayout)
                .map_err(Error::from)?;
            let mut removed_entry_set =
                WritableDirectoryEntrySlotSpan::new(source_slot_range, removed_entry_set)
                    .map_err(Error::from)?;
            direntry::invalidate_entry_set(&mut removed_entry_set).map_err(Error::from)?;
        }

        let final_slot_bytes = direntry::slot_range_bytes(final_slot_range).map_err(Error::from)?;
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
            stream,
        )
        .map_err(Error::from)?;

        if !replaced_target_ranges.is_empty() {
            let _ = fs.free_allocated_space_with_publication(publication, &replaced_target_ranges);
        }
        Ok(true)
    }

    fn rename_across_directories(
        &self,
        source_stream: ExfatInodeStream,
        target_directory: &ExfatInode,
        mut target_stream: ExfatInodeStream,
        target_child_inode: Option<&Arc<Self>>,
        publication: &mut MountedVolumeState,
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
            Self::read_directory_bytes_for_stream(block_device, boot_region, source_stream)
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
        let (source_inode_type, _, _, _) = source_view
            .child_metadata(boot_region)
            .map_err(Error::from)?;
        let renamed_entry_set = direntry::renamed_entry_set(source_view, new_name, new_name_hash)
            .map_err(Error::from)?;
        let required_entry_count = renamed_entry_set.len() / DIRECTORY_ENTRY_SIZE;

        let target_directory_bytes =
            Self::read_directory_bytes_for_stream(block_device, boot_region, target_stream)
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
                let (target_inode_type, first_cluster, data_length, no_fat_chain) = target_view
                    .child_metadata(boot_region)
                    .map_err(Error::from)?;
                if source_inode_type == InodeType::Dir && target_inode_type != InodeType::Dir {
                    return_errno!(Errno::ENOTDIR);
                }
                if source_inode_type != InodeType::Dir && target_inode_type == InodeType::Dir {
                    return_errno!(Errno::EISDIR);
                }
                if target_inode_type == InodeType::Dir {
                    let Some(child_inode) = target_child_inode else {
                        return Err(Error::from(MountVolumeStateError::InvalidOnDiskLayout));
                    };
                    Self::ensure_directory_entry_is_empty(child_inode, block_device, boot_region)?;
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
                let (updated_target_stream, latest_target_directory_bytes, reserved_slot_range) =
                    target_directory
                        .reserve_directory_entry_slots(
                            target_stream,
                            publication,
                            fs,
                            block_device,
                            boot_region,
                            required_entry_count,
                        )
                        .map_err(Error::from)?;
                target_stream = updated_target_stream;
                (
                    latest_target_directory_bytes,
                    reserved_slot_range,
                    Vec::new(),
                )
            };

        let target_slot_bytes = direntry::slot_range_bytes(target_slot_range).map_err(Error::from)?;
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
            target_stream,
        )
        .map_err(Error::from)?;

        let mut invalidated_source_directory_bytes = source_directory_bytes;
        let source_slot_bytes = direntry::slot_range_bytes(source_slot_range).map_err(Error::from)?;
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
            source_stream,
        )
        .map_err(Error::from)?;

        if !replaced_target_ranges.is_empty() {
            let _ = fs.free_allocated_space_with_publication(publication, &replaced_target_ranges);
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

    fn reject_published_identity_change(
        &self,
        matches_requested_fn: impl FnOnce(&Metadata) -> bool,
    ) -> Result<()> {
        let fs = self
            .fs
            .upgrade()
            .ok_or_else(|| Error::with_message(Errno::EIO, "exFAT filesystem is not mounted"))?;
        let (_state_guard, _block_device, _boot_region, anomaly, _upcase_table, _options) =
            fs.admitted_mutation_state().map_err(Error::from)?;
        if anomaly.clear_to_zero || anomaly.media_failure {
            return_errno!(Errno::EIO);
        }

        let _owner_guard = self.admission.write();
        let metadata = self.metadata.read();
        if matches_requested_fn(&metadata) {
            return Ok(());
        }
        return_errno!(Errno::EPERM);
    }

    fn rewrite_timestamp(
        &self,
        target: RewriteTarget,
        field_kind: TimestampFieldKind,
        time: Duration,
    ) {
        let Some(fs) = self.fs.upgrade() else {
            return;
        };
        // TODO: These timestamp setters still admit through the generic mounted-mutation gate and
        // reuse the currently stored exFAT UTC-offset byte because `ExfatMountOptions` does not
        // yet own explicit `allow_utime` / timezone policy. Once the later `meso_09`
        // mount-policy follow-up publishes that owner-local policy under `ExfatFs`, remove this
        // seam and route timestamp admission plus UTC-offset selection through that dedicated path.
        let Ok((_state_guard, block_device, boot_region, anomaly, _upcase_table, _options)) =
            fs.admitted_mutation_state().map_err(Error::from)
        else {
            return;
        };
        if anomaly.clear_to_zero || anomaly.media_failure {
            return;
        }

        match target {
            RewriteTarget::Directory => {
                if self.stream.read().data_length.is_none() {
                    return;
                }
                if self
                    .rewrite_directory_self_entry_set(
                        &block_device,
                        &boot_region,
                        |entry_view, source_entry_set| match field_kind {
                            TimestampFieldKind::Accessed => {
                                let utc_offset_byte = *source_entry_set
                                    .get(direntry::LAST_ACCESSED_UTC_OFFSET_OFFSET)
                                    .ok_or(MountVolumeStateError::InvalidOnDiskLayout)
                                    .map_err(Error::from)?;
                                let (timestamp_bytes, _ten_ms_increment, encoded_utc_offset_byte) =
                                    Self::encoded_exfat_timestamp_fields(time, utc_offset_byte)?;
                                direntry::republished_entry_set(
                                    entry_view,
                                    &direntry::FileEntrySetFieldUpdates {
                                        last_accessed_fields: Some((
                                            [0, 0, timestamp_bytes[2], timestamp_bytes[3]],
                                            encoded_utc_offset_byte,
                                        )),
                                        ..Default::default()
                                    },
                                )
                                .map(Some)
                                .map_err(Error::from)
                            }
                            TimestampFieldKind::Modified => {
                                let utc_offset_byte = *source_entry_set
                                    .get(direntry::LAST_MODIFIED_UTC_OFFSET_OFFSET)
                                    .ok_or(MountVolumeStateError::InvalidOnDiskLayout)
                                    .map_err(Error::from)?;
                                let (timestamp_bytes, ten_ms_increment, encoded_utc_offset_byte) =
                                    Self::encoded_exfat_timestamp_fields(time, utc_offset_byte)?;
                                direntry::republished_entry_set(
                                    entry_view,
                                    &direntry::FileEntrySetFieldUpdates {
                                        last_modified_fields: Some((
                                            timestamp_bytes,
                                            ten_ms_increment,
                                            encoded_utc_offset_byte,
                                        )),
                                        ..Default::default()
                                    },
                                )
                                .map(Some)
                                .map_err(Error::from)
                            }
                        },
                        |metadata| match field_kind {
                            TimestampFieldKind::Accessed => metadata.last_access_at = time,
                            TimestampFieldKind::Modified => {
                                metadata.last_meta_change_at = time;
                                metadata.last_modify_at = time;
                            }
                        },
                    )
                    .is_ok_and(|updated| updated)
                {
                    self.mark_metadata_publication_dirty();
                }
            }
            RewriteTarget::RegularFile => {
                let _owner_guard = self.admission.write();
                if self
                    .rewrite_regular_file_entry_set(
                        &block_device,
                        &boot_region,
                        |entry_view, source_entry_set| match field_kind {
                            TimestampFieldKind::Accessed => {
                                let utc_offset_byte = *source_entry_set
                                    .get(direntry::LAST_ACCESSED_UTC_OFFSET_OFFSET)
                                    .ok_or(MountVolumeStateError::InvalidOnDiskLayout)
                                    .map_err(Error::from)?;
                                let (timestamp_bytes, _ten_ms_increment, encoded_utc_offset_byte) =
                                    Self::encoded_exfat_timestamp_fields(time, utc_offset_byte)?;
                                direntry::republished_entry_set(
                                    entry_view,
                                    &direntry::FileEntrySetFieldUpdates {
                                        last_accessed_fields: Some((
                                            [0, 0, timestamp_bytes[2], timestamp_bytes[3]],
                                            encoded_utc_offset_byte,
                                        )),
                                        ..Default::default()
                                    },
                                )
                                .map(Some)
                                .map_err(Error::from)
                            }
                            TimestampFieldKind::Modified => {
                                let utc_offset_byte = *source_entry_set
                                    .get(direntry::LAST_MODIFIED_UTC_OFFSET_OFFSET)
                                    .ok_or(MountVolumeStateError::InvalidOnDiskLayout)
                                    .map_err(Error::from)?;
                                let (timestamp_bytes, ten_ms_increment, encoded_utc_offset_byte) =
                                    Self::encoded_exfat_timestamp_fields(time, utc_offset_byte)?;
                                direntry::republished_entry_set(
                                    entry_view,
                                    &direntry::FileEntrySetFieldUpdates {
                                        last_modified_fields: Some((
                                            timestamp_bytes,
                                            ten_ms_increment,
                                            encoded_utc_offset_byte,
                                        )),
                                        ..Default::default()
                                    },
                                )
                                .map(Some)
                                .map_err(Error::from)
                            }
                        },
                    )
                    .is_ok()
                {
                    let mut metadata = self.metadata.write();
                    match field_kind {
                        TimestampFieldKind::Accessed => metadata.last_access_at = time,
                        TimestampFieldKind::Modified => metadata.last_modify_at = time,
                    }
                    metadata.last_meta_change_at = RealTimeCoarseClock::get().read_time();
                    drop(metadata);
                    self.mark_metadata_publication_dirty();
                }
            }
        }
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

        let (_owner_guard, stream, data_length, valid_data_length) =
            self.admitted_regular_file_stream_snapshot()?;
        let (file_offset, initialized_len) =
            Self::regular_file_page_range(idx, data_length, valid_data_length)?;
        let initialized_sector_len = initialized_len - (initialized_len % boot_region.sector_size);
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

        let (_owner_guard, stream, data_length, valid_data_length) =
            self.admitted_regular_file_stream_snapshot()?;
        let (file_offset, initialized_len) =
            Self::regular_file_page_range(idx, data_length, valid_data_length)?;
        let initialized_sector_len = initialized_len
            .div_ceil(boot_region.sector_size)
            .checked_mul(boot_region.sector_size)
            .ok_or_else(|| Error::new(Errno::EINVAL))?;
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
        self.metadata.read().size.div_ceil(PAGE_SIZE)
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
        let (state_guard, block_device, boot_region, anomaly, _, _) =
            fs.admitted_lookup_state().map_err(Error::from)?;
        if anomaly.clear_to_zero || anomaly.media_failure {
            return_errno!(Errno::EIO);
        }

        {
            let _state_guard = state_guard;
            let (_owner_guard, stream, data_length, valid_data_length) =
                self.admitted_regular_file_stream_snapshot()?;
            Self::read_regular_file_at(
                &block_device,
                &boot_region,
                stream,
                data_length,
                valid_data_length,
                offset,
                writer,
            )
        }
    }

    fn write_at(
        &self,
        offset: usize,
        reader: &mut VmReader,
        status_flags: StatusFlags,
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
        let (mut state_guard, block_device, boot_region, anomaly, _upcase_table, options) =
            fs.admitted_mutation_state().map_err(Error::from)?;
        if anomaly.clear_to_zero || anomaly.media_failure {
            return_errno!(Errno::EIO);
        }
        if options.fs_flags.contains(FsFlags::RDONLY) {
            return_errno!(Errno::EROFS);
        }
        if !reader.has_remain() {
            return Ok(0);
        }

        let write_len = reader.remain();
        {
            let _owner_guard = self.admission.write();
            let mut stream = self.stream.write();
            let Some(data_length) = stream.data_length else {
                return_errno!(Errno::EINVAL);
            };
            let Some(valid_data_length) = stream.valid_data_length else {
                return_errno!(Errno::EINVAL);
            };
            if valid_data_length > data_length {
                return_errno!(Errno::EINVAL);
            }

            let effective_offset = if status_flags.contains(StatusFlags::O_APPEND) {
                data_length
            } else {
                offset
            };
            if status_flags.contains(StatusFlags::O_DIRECT)
                && (!effective_offset.is_multiple_of(boot_region.sector_size)
                    || !write_len.is_multiple_of(boot_region.sector_size))
            {
                return_errno!(Errno::EINVAL);
            }
            let write_end = effective_offset
                .checked_add(write_len)
                .ok_or_else(|| Error::new(Errno::EINVAL))?;
            let publication = state_guard
                .as_ref()
                .ok_or(MountVolumeStateError::UnpublishedState)
                .map_err(Error::from)?;
            let published_data_length = data_length.max(write_end);
            let mut published_stream = if write_end > data_length {
                Self::grow_regular_file_stream(
                    &fs,
                    publication,
                    &block_device,
                    &boot_region,
                    *stream,
                    published_data_length,
                )?
            } else {
                *stream
            };

            let zero_fill_len = if effective_offset > valid_data_length {
                effective_offset
                    .checked_sub(valid_data_length)
                    .ok_or_else(|| Error::new(Errno::EINVAL))?
            } else {
                0
            };
            if zero_fill_len != 0 {
                Self::mutate_regular_file_range(
                    &block_device,
                    &boot_region,
                    &published_stream,
                    published_data_length,
                    valid_data_length,
                    zero_fill_len,
                    |chunk| {
                        chunk.fill(0);
                        Ok(())
                    },
                )?;
            }
            Self::mutate_regular_file_range(
                &block_device,
                &boot_region,
                &published_stream,
                published_data_length,
                effective_offset,
                write_len,
                |chunk| {
                    let mut writer = VmWriter::from(chunk).to_fallible();
                    writer.write_fallible(reader)?;
                    Ok(())
                },
            )?;

            let published_valid_data_length = valid_data_length.max(write_end);
            let timestamp = RealTimeCoarseClock::get().read_time();
            published_stream.data_length = Some(published_data_length);
            published_stream.valid_data_length = Some(published_valid_data_length);
            let page_cache = self
                .page_cache
                .get()
                .and_then(|maybe_page_cache| maybe_page_cache.as_ref());
            let cache_invalidate_start = valid_data_length.min(effective_offset);
            if let Some(page_cache) = page_cache {
                if published_data_length > data_length {
                    page_cache.resize(published_data_length)?;
                }
                page_cache.discard_range(cache_invalidate_start..write_end);
            }
            if let Err(error) = self.republish_regular_file_entry_set(
                &block_device,
                &boot_region,
                published_stream,
                timestamp,
            ) {
                if published_data_length > data_length {
                    if let Some(page_cache) = page_cache {
                        let _ = page_cache.resize(data_length);
                    }
                }
                return Err(error);
            }

            let allocated_clusters = if published_data_length == 0 {
                0
            } else {
                published_data_length.div_ceil(boot_region.cluster_size)
            };
            let allocated_sectors = allocated_clusters
                .checked_mul(boot_region.sectors_per_cluster)
                .ok_or_else(|| Error::new(Errno::EINVAL))?;

            {
                let mut metadata = self.metadata.write();
                metadata.nr_sectors_allocated = allocated_sectors;
                metadata.last_meta_change_at = timestamp;
                metadata.last_modify_at = timestamp;
                metadata.size = published_data_length;
            }
            *stream = published_stream;
            self.mark_content_publication_dirty();
        }
        drop(state_guard);
        if status_flags.contains(StatusFlags::O_SYNC) {
            self.sync_all()?;
        } else if status_flags.contains(StatusFlags::O_DSYNC) {
            self.sync_data()?;
        }

        Ok(write_len)
    }
}

impl Inode for ExfatInode {
    fn size(&self) -> usize {
        self.metadata_projection().size
    }

    fn resize(&self, new_size: usize) -> Result<()> {
        match self.type_() {
            InodeType::Dir => return_errno!(Errno::EISDIR),
            InodeType::File => {}
            _ => return_errno!(Errno::EOPNOTSUPP),
        }

        let fs = self
            .fs
            .upgrade()
            .ok_or_else(|| Error::with_message(Errno::EIO, "exFAT filesystem is not mounted"))?;
        let (mut state_guard, block_device, boot_region, anomaly, _upcase_table, options) =
            fs.admitted_mutation_state().map_err(Error::from)?;
        if anomaly.clear_to_zero || anomaly.media_failure {
            return_errno!(Errno::EIO);
        }
        if options.fs_flags.contains(FsFlags::RDONLY) {
            return_errno!(Errno::EROFS);
        }

        let _owner_guard = self.admission.write();
        let mut stream = self.stream.write();
        let Some(data_length) = stream.data_length else {
            return_errno!(Errno::EINVAL);
        };
        let Some(valid_data_length) = stream.valid_data_length else {
            return_errno!(Errno::EINVAL);
        };
        if valid_data_length > data_length {
            return_errno!(Errno::EINVAL);
        }
        if new_size == data_length {
            return Ok(());
        }
        let page_cache = self
            .page_cache
            .get()
            .and_then(|maybe_page_cache| maybe_page_cache.as_ref());
        let timestamp = RealTimeCoarseClock::get().read_time();

        if new_size > data_length {
            let publication = state_guard
                .as_ref()
                .ok_or(MountVolumeStateError::UnpublishedState)
                .map_err(Error::from)?;
            let mut published_stream = Self::grow_regular_file_stream(
                &fs,
                publication,
                &block_device,
                &boot_region,
                *stream,
                new_size,
            )?;
            if valid_data_length < new_size {
                Self::mutate_regular_file_range(
                    &block_device,
                    &boot_region,
                    &published_stream,
                    new_size,
                    valid_data_length,
                    new_size
                        .checked_sub(valid_data_length)
                        .ok_or_else(|| Error::new(Errno::EINVAL))?,
                    |chunk| {
                        chunk.fill(0);
                        Ok(())
                    },
                )?;
            }
            published_stream.valid_data_length = Some(new_size);
            if let Some(page_cache) = page_cache {
                page_cache.resize(new_size)?;
                page_cache.discard_range(valid_data_length..new_size);
            }
            if let Err(error) = self.republish_regular_file_entry_set(
                &block_device,
                &boot_region,
                published_stream,
                timestamp,
            ) {
                if let Some(page_cache) = page_cache {
                    let _ = page_cache.resize(data_length);
                }
                return Err(error);
            }

            let allocated_clusters = new_size.div_ceil(boot_region.cluster_size);
            let allocated_sectors = allocated_clusters
                .checked_mul(boot_region.sectors_per_cluster)
                .ok_or_else(|| Error::new(Errno::EINVAL))?;
            {
                let mut metadata = self.metadata.write();
                metadata.last_meta_change_at = timestamp;
                metadata.last_modify_at = timestamp;
                metadata.nr_sectors_allocated = allocated_sectors;
                metadata.size = new_size;
            }
            *stream = published_stream;
            self.mark_content_publication_dirty();
            return Ok(());
        }

        let current_ranges = Self::allocated_cluster_ranges(
            &block_device,
            &boot_region,
            stream.first_cluster,
            data_length,
            stream.no_fat_chain,
        )
        .map_err(Error::from)?;
        let retained_clusters = if new_size == 0 {
            0
        } else {
            new_size.div_ceil(boot_region.cluster_size)
        };
        let mut retained_clusters_remaining = retained_clusters;
        let mut retained_is_contiguous = true;
        let mut previous_retained_cluster: Option<u32> = None;
        let mut first_retained_cluster = 0u32;
        let mut last_retained_cluster = 0u32;
        let mut released_ranges = Vec::new();
        for range in &current_ranges {
            if retained_clusters_remaining == 0 {
                released_ranges.push(*range);
                continue;
            }

            let retained_in_range = retained_clusters_remaining.min(range.cluster_count);
            if retained_in_range != 0 {
                let retained_last_cluster = range
                    .start_cluster
                    .checked_add(
                        u32::try_from(retained_in_range - 1)
                            .map_err(|_| Error::from(MountVolumeStateError::InvalidOnDiskLayout))?,
                    )
                    .ok_or_else(|| Error::from(MountVolumeStateError::InvalidOnDiskLayout))?;
                if let Some(previous_retained_cluster) = previous_retained_cluster {
                    if previous_retained_cluster.checked_add(1) != Some(range.start_cluster) {
                        retained_is_contiguous = false;
                    }
                } else {
                    first_retained_cluster = range.start_cluster;
                }
                previous_retained_cluster = Some(retained_last_cluster);
                last_retained_cluster = retained_last_cluster;
            }
            if retained_in_range < range.cluster_count {
                let released_start_cluster = range
                    .start_cluster
                    .checked_add(
                        u32::try_from(retained_in_range)
                            .map_err(|_| Error::from(MountVolumeStateError::InvalidOnDiskLayout))?,
                    )
                    .ok_or_else(|| Error::from(MountVolumeStateError::InvalidOnDiskLayout))?;
                released_ranges.push(ClusterRange {
                    start_cluster: released_start_cluster,
                    cluster_count: range.cluster_count - retained_in_range,
                });
            }
            retained_clusters_remaining -= retained_in_range;
        }
        if retained_clusters_remaining != 0 {
            return Err(Error::from(MountVolumeStateError::InvalidOnDiskLayout));
        }

        let published_stream = ExfatInodeStream {
            data_length: Some(new_size),
            first_cluster: if retained_clusters == 0 {
                0
            } else {
                first_retained_cluster
            },
            valid_data_length: Some(valid_data_length.min(new_size)),
            no_fat_chain: retained_clusters != 0 && retained_is_contiguous,
        };
        if let Some(page_cache) = page_cache {
            page_cache.resize(new_size)?;
        }
        if let Err(error) = self.republish_regular_file_entry_set(
            &block_device,
            &boot_region,
            published_stream,
            timestamp,
        ) {
            if let Some(page_cache) = page_cache {
                let _ = page_cache.resize(data_length);
            }
            return Err(error);
        }

        let allocated_sectors = retained_clusters
            .checked_mul(boot_region.sectors_per_cluster)
            .ok_or_else(|| Error::new(Errno::EINVAL))?;
        {
            let mut metadata = self.metadata.write();
            metadata.last_meta_change_at = timestamp;
            metadata.last_modify_at = timestamp;
            metadata.nr_sectors_allocated = allocated_sectors;
            metadata.size = new_size;
        }
        *stream = published_stream;
        self.mark_content_publication_dirty();

        if retained_clusters != 0 && !published_stream.no_fat_chain {
            FatReader::new(block_device.as_ref(), &boot_region)
                .terminate_cluster_chain(last_retained_cluster)
                .map_err(Error::from)?;
        }
        if !released_ranges.is_empty() {
            let publication = state_guard
                .as_mut()
                .ok_or(MountVolumeStateError::UnpublishedState)
                .map_err(Error::from)?;
            fs.free_allocated_space_with_publication(publication, &released_ranges)
                .map_err(Error::from)?;
        }
        Ok(())
    }

    fn metadata(&self) -> Metadata {
        self.metadata_projection()
    }

    fn ino(&self) -> u64 {
        self.metadata_projection().ino
    }

    fn type_(&self) -> InodeType {
        self.metadata_projection().type_
    }

    fn mode(&self) -> Result<InodeMode> {
        if self.metadata.read().type_ == InodeType::Dir {
            return self
                .directory_metadata_projection()
                .map(|metadata| metadata.mode);
        }
        Ok(self.metadata_projection().mode)
    }

    fn set_mode(&self, mode: InodeMode) -> Result<()> {
        if self.metadata.read().type_ == InodeType::Dir {
            let fs = self.fs.upgrade().ok_or_else(|| {
                Error::with_message(Errno::EIO, "exFAT filesystem is not mounted")
            })?;
            let (_state_guard, block_device, boot_region, anomaly, _upcase_table, _options) =
                fs.admitted_mutation_state().map_err(Error::from)?;
            if anomaly.clear_to_zero || anomaly.media_failure {
                return_errno!(Errno::EIO);
            }

            let requested_writable = mode.intersects(mkmod!(a+w));
            if self.stream.read().data_length.is_none() {
                let current_writable = self.metadata.read().mode.intersects(mkmod!(a+w));
                if requested_writable == current_writable {
                    return Ok(());
                }
                return_errno!(Errno::EOPNOTSUPP);
            }

            let durable_updated = self.rewrite_directory_self_entry_set(
                &block_device,
                &boot_region,
                |entry_view, _source_entry_set| {
                    let current_attributes = entry_view.file_attributes();
                    let current_writable =
                        current_attributes & direntry::FILE_ATTRIBUTE_READ_ONLY == 0;
                    if requested_writable == current_writable {
                        return Ok(None);
                    }

                    let mut file_attributes =
                        current_attributes | direntry::FILE_ATTRIBUTE_DIRECTORY;
                    if requested_writable {
                        file_attributes &= !direntry::FILE_ATTRIBUTE_READ_ONLY;
                    } else {
                        file_attributes |= direntry::FILE_ATTRIBUTE_READ_ONLY;
                    }
                    direntry::republished_entry_set(
                        entry_view,
                        &direntry::FileEntrySetFieldUpdates {
                            file_attributes: Some(file_attributes),
                            ..Default::default()
                        },
                    )
                    .map(Some)
                    .map_err(Error::from)
                },
                |metadata| {
                    let writable_bits = metadata.mode & mkmod!(a+w);
                    metadata.mode = chmod!(metadata.mode, a-w);
                    if requested_writable {
                        metadata.mode |= writable_bits;
                    }
                },
            )?;
            if durable_updated {
                self.mark_metadata_publication_dirty();
            }
            return Ok(());
        }

        if self.metadata.read().type_ != InodeType::File {
            self.metadata.write().mode = mode;
            return Ok(());
        }

        let fs = self
            .fs
            .upgrade()
            .ok_or_else(|| Error::with_message(Errno::EIO, "exFAT filesystem is not mounted"))?;
        let (_state_guard, block_device, boot_region, anomaly, _upcase_table, _options) =
            fs.admitted_mutation_state().map_err(Error::from)?;
        if anomaly.clear_to_zero || anomaly.media_failure {
            return_errno!(Errno::EIO);
        }

        let _owner_guard = self.admission.write();
        let requested_writable = mode.intersects(mkmod!(a+w));
        let durable_updated = self.rewrite_regular_file_entry_set(
            &block_device,
            &boot_region,
            |entry_view, _source_entry_set| {
                if requested_writable
                    == (entry_view.file_attributes() & direntry::FILE_ATTRIBUTE_READ_ONLY != 0)
                {
                    let mut file_attributes = entry_view.file_attributes();
                    if requested_writable {
                        file_attributes &= !direntry::FILE_ATTRIBUTE_READ_ONLY;
                    } else {
                        file_attributes |= direntry::FILE_ATTRIBUTE_READ_ONLY;
                    }
                    return direntry::republished_entry_set(
                        entry_view,
                        &direntry::FileEntrySetFieldUpdates {
                            file_attributes: Some(file_attributes),
                            ..Default::default()
                        },
                    )
                    .map(Some)
                    .map_err(Error::from);
                }
                Ok(None)
            },
        )?;

        let mut metadata = self.metadata.write();
        metadata.mode = chmod!(chmod!(metadata.mode, a-w), u+w);
        if !requested_writable {
            metadata.mode = chmod!(metadata.mode, a-w);
        }
        if durable_updated {
            metadata.last_meta_change_at = RealTimeCoarseClock::get().read_time();
        }
        drop(metadata);
        if durable_updated {
            self.mark_metadata_publication_dirty();
        }
        Ok(())
    }

    fn owner(&self) -> Result<Uid> {
        if self.metadata.read().type_ == InodeType::Dir {
            return self
                .directory_metadata_projection()
                .map(|metadata| metadata.uid);
        }
        Ok(self.metadata_projection().uid)
    }

    fn set_owner(&self, uid: Uid) -> Result<()> {
        let inode_type = self.metadata.read().type_;
        if !matches!(inode_type, InodeType::Dir | InodeType::File) {
            self.metadata.write().uid = uid;
            return Ok(());
        }
        self.reject_published_identity_change(|metadata| metadata.uid == uid)
    }

    fn group(&self) -> Result<Gid> {
        if self.metadata.read().type_ == InodeType::Dir {
            return self
                .directory_metadata_projection()
                .map(|metadata| metadata.gid);
        }
        Ok(self.metadata_projection().gid)
    }

    fn set_group(&self, gid: Gid) -> Result<()> {
        let inode_type = self.metadata.read().type_;
        if !matches!(inode_type, InodeType::Dir | InodeType::File) {
            self.metadata.write().gid = gid;
            return Ok(());
        }
        self.reject_published_identity_change(|metadata| metadata.gid == gid)
    }

    fn atime(&self) -> Duration {
        self.metadata_projection().last_access_at
    }

    fn set_atime(&self, time: Duration) {
        let inode_type = self.metadata.read().type_;
        match inode_type {
            InodeType::Dir => {
                self.rewrite_timestamp(RewriteTarget::Directory, TimestampFieldKind::Accessed, time)
            }
            InodeType::File => self.rewrite_timestamp(
                RewriteTarget::RegularFile,
                TimestampFieldKind::Accessed,
                time,
            ),
            _ => self.metadata.write().last_access_at = time,
        }
    }

    fn mtime(&self) -> Duration {
        self.metadata_projection().last_modify_at
    }

    fn set_mtime(&self, time: Duration) {
        let inode_type = self.metadata.read().type_;
        match inode_type {
            InodeType::Dir => self.rewrite_timestamp(
                RewriteTarget::Directory,
                TimestampFieldKind::Modified,
                time,
            ),
            InodeType::File => self.rewrite_timestamp(
                RewriteTarget::RegularFile,
                TimestampFieldKind::Modified,
                time,
            ),
            _ => self.metadata.write().last_modify_at = time,
        }
    }

    fn ctime(&self) -> Duration {
        self.metadata_projection().last_meta_change_at
    }

    fn set_ctime(&self, time: Duration) {
        if self.metadata.read().type_ == InodeType::Dir {
            let Some(fs) = self.fs.upgrade() else {
                return;
            };
            // TODO: Directory `set_ctime()` remains a bounded synthetic no-op until the later
            // `meso_09` mount-policy follow-up gives `ExfatFs` an explicit owner-local
            // `allow_utime` admission path and the broader metadata policy decides whether this
            // VFS-facing request should keep refusing or be absorbed into another durable family.
            let Ok((_state_guard, _block_device, _boot_region, anomaly, _upcase_table, _options)) =
                fs.admitted_mutation_state().map_err(Error::from)
            else {
                return;
            };
            if anomaly.clear_to_zero || anomaly.media_failure {
                return;
            }
            return;
        }

        if self.metadata.read().type_ != InodeType::File {
            self.metadata.write().last_meta_change_at = time;
            return;
        }

        let Some(fs) = self.fs.upgrade() else {
            return;
        };
        // TODO: `set_ctime()` still shares the generic mounted-mutation gate until the later
        // `meso_09` mount-policy follow-up gives `ExfatFs` an explicit owner-local `allow_utime`
        // admission path for timestamp setters. Remove this seam once that dedicated gate exists.
        let Ok((_state_guard, _block_device, _boot_region, anomaly, _upcase_table, _options)) =
            fs.admitted_mutation_state().map_err(Error::from)
        else {
            return;
        };
        if anomaly.clear_to_zero || anomaly.media_failure {
            return;
        }

        let _owner_guard = self.admission.write();
        self.metadata.write().last_meta_change_at = time;
    }

    fn page_cache(&self) -> Option<Arc<Vmo>> {
        if self.type_() != InodeType::File {
            return None;
        }

        self.page_cache
            .call_once(|| {
                let this = self.this.upgrade()?;
                let backend: Arc<dyn PageCacheBackend> = this;
                let capacity = self.metadata.read().size;
                PageCache::with_capacity(capacity, Arc::downgrade(&backend)).ok()
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
        let (mut state_guard, block_device, boot_region, _anomaly, upcase_table, options) =
            fs.admitted_mutation_state().map_err(Error::from)?;
        if options.fs_flags.contains(FsFlags::RDONLY) {
            return_errno!(Errno::EROFS);
        }

        let admitted_name = Self::admitted_name(name, &options)?;
        let name_hash = upcase_table.name_hash(&admitted_name);
        let required_entry_count =
            direntry::file_entry_set_entry_count(admitted_name.len()).map_err(Error::from)?;
        let child_inode = {
            let _directory_guards = Self::ordered_directory_write_guards(vec![self]);
            let stream = *self.stream.read();
            let current_directory_bytes =
                Self::read_directory_bytes_for_stream(&block_device, &boot_region, stream)
                    .map_err(Error::from)?;
            if Self::locate_named_child_view(
                &current_directory_bytes,
                stream.data_length.is_none(),
                &upcase_table,
                &admitted_name,
                name_hash,
            )
            .map_err(Error::from)?
            .is_some()
            {
                return_errno!(Errno::EEXIST);
            }

            let publication = state_guard
                .as_mut()
                .ok_or(MountVolumeStateError::UnpublishedState)
                .map_err(Error::from)?;
            let (stream, mut published_directory_bytes, slot_range) = self
                .reserve_directory_entry_slots(
                    stream,
                    publication,
                    &fs,
                    &block_device,
                    &boot_region,
                    required_entry_count,
                )
                .map_err(Error::from)?;

            let mut allocated_directory_ranges = None;
            let (first_cluster, data_length, no_fat_chain) = if type_ == InodeType::Dir
                && !options.zero_size_dir
            {
                let (allocated_ranges, _) = fs
                    .allocate_free_space_with_publication(publication, 1)
                    .map_err(Error::from)?;
                let allocated_cluster = match allocated_ranges.as_slice() {
                    [allocated_range] if allocated_range.cluster_count == 1 => {
                        allocated_range.start_cluster
                    }
                    _ => {
                        let _ = fs
                            .free_allocated_space_with_publication(publication, &allocated_ranges);
                        return Err(Error::from(MountVolumeStateError::InconsistentAccounting));
                    }
                };
                if let Err(error) = Self::initialize_directory_cluster(
                    &block_device,
                    &boot_region,
                    allocated_cluster,
                ) {
                    let _ =
                        fs.free_allocated_space_with_publication(publication, &allocated_ranges);
                    return Err(Error::from(error));
                }
                allocated_directory_ranges = Some(allocated_ranges);
                (allocated_cluster, boot_region.cluster_size, true)
            } else {
                (0, 0, false)
            };

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
                    let _ = fs.free_allocated_space_with_publication(publication, allocated_ranges);
                }
                Error::from(error)
            })?;
            let slot_range_bytes = direntry::slot_range_bytes(slot_range).map_err(|error| {
                if let Some(allocated_ranges) = allocated_directory_ranges.as_ref() {
                    let _ = fs.free_allocated_space_with_publication(publication, allocated_ranges);
                }
                Error::from(error)
            })?;
            published_directory_bytes[slot_range_bytes.clone()].copy_from_slice(&entry_set);
            Self::write_directory_bytes_for_stream(
                &block_device,
                &boot_region,
                &published_directory_bytes,
                stream,
            )
            .map_err(|error| {
                if let Some(allocated_ranges) = allocated_directory_ranges.as_ref() {
                    let _ = fs.free_allocated_space_with_publication(publication, allocated_ranges);
                }
                Error::from(error)
            })?;

            let child_size = if type_ == InodeType::Dir {
                data_length
            } else {
                0
            };
            let child_inode = Self::new_child(
                &fs,
                self.this.clone(),
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
            child_inode
        };
        self.refresh_directory_metadata_after_namespace_mutation(
            &block_device,
            &boot_region,
            RealTimeCoarseClock::get().read_time(),
        )?;
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
        let (state_guard, block_device, boot_region, _, _, _) =
            fs.admitted_lookup_state().map_err(Error::from)?;
        let (_owner_guard, stream, directory_bytes) = {
            let _state_guard = state_guard;
            self.admitted_directory_snapshot(&block_device, &boot_region)
                .map_err(Error::from)?
        };

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
            match direntry::scan_directory_entry(
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
                        visitor.visit(&entry_name, entry_ino, inode_type, visible_offset)?;
                        next_offset = visible_offset
                            .checked_add(1)
                            .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
                    }
                    visible_offset = visible_offset
                        .checked_add(1)
                        .ok_or(MountVolumeStateError::InvalidOnDiskLayout)?;
                    entry_index = entry_view.slot_range().next_entry_index()?;
                }
                ScannedDirectoryEntry::Anomaly { kind, slot_range } => {
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
        let (mut state_guard, block_device, boot_region, _anomaly, upcase_table, options) =
            fs.admitted_mutation_state().map_err(Error::from)?;
        if options.fs_flags.contains(FsFlags::RDONLY) {
            return_errno!(Errno::EROFS);
        }

        let admitted_name = Self::admitted_name(name, &options)?;
        let lookup_name_hash = upcase_table.name_hash(&admitted_name);

        let allocated_cluster_ranges = {
            let _directory_guards = Self::ordered_directory_write_guards(vec![self]);
            let stream = *self.stream.read();
            let is_root_directory = stream.data_length.is_none();
            let directory_bytes =
                Self::read_directory_bytes_for_stream(&block_device, &boot_region, stream)
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
            let slot_range_bytes = direntry::slot_range_bytes(slot_range).map_err(Error::from)?;
            let removed_entry_set = invalidated_directory_bytes
                .get_mut(slot_range_bytes)
                .ok_or(MountVolumeStateError::InvalidOnDiskLayout)
                .map_err(Error::from)?;
            let mut removed_entry_set =
                WritableDirectoryEntrySlotSpan::new(slot_range, removed_entry_set)
                    .map_err(Error::from)?;
            direntry::invalidate_entry_set(&mut removed_entry_set).map_err(Error::from)?;
            Self::write_directory_bytes_for_stream(
                &block_device,
                &boot_region,
                &invalidated_directory_bytes,
                stream,
            )
            .map_err(Error::from)?;
            allocated_cluster_ranges
        };

        if !allocated_cluster_ranges.is_empty() {
            let publication = state_guard
                .as_mut()
                .ok_or(MountVolumeStateError::UnpublishedState)
                .map_err(Error::from)?;
            let _ =
                fs.free_allocated_space_with_publication(publication, &allocated_cluster_ranges);
        }
        self.refresh_directory_metadata_after_namespace_mutation(
            &block_device,
            &boot_region,
            RealTimeCoarseClock::get().read_time(),
        )?;
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
        let (mut state_guard, block_device, boot_region, _anomaly, upcase_table, options) =
            fs.admitted_mutation_state().map_err(Error::from)?;
        if options.fs_flags.contains(FsFlags::RDONLY) {
            return_errno!(Errno::EROFS);
        }

        let admitted_name = Self::admitted_name(name, &options)?;
        let lookup_name_hash = upcase_table.name_hash(&admitted_name);

        let allocated_cluster_ranges = {
            let _directory_guards = Self::ordered_directory_write_guards(vec![self]);
            let stream = *self.stream.read();
            let directory_bytes =
                Self::read_directory_bytes_for_stream(&block_device, &boot_region, stream)
                    .map_err(Error::from)?;
            let Some((slot_range, inode_type, first_cluster, data_length, _, no_fat_chain)) =
                Self::locate_named_child(
                    &directory_bytes,
                    stream.data_length.is_none(),
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
                self,
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
            let slot_range_bytes = direntry::slot_range_bytes(slot_range).map_err(Error::from)?;
            let removed_entry_set = invalidated_directory_bytes
                .get_mut(slot_range_bytes)
                .ok_or(MountVolumeStateError::InvalidOnDiskLayout)
                .map_err(Error::from)?;
            let mut removed_entry_set =
                WritableDirectoryEntrySlotSpan::new(slot_range, removed_entry_set)
                    .map_err(Error::from)?;
            direntry::invalidate_entry_set(&mut removed_entry_set).map_err(Error::from)?;
            Self::write_directory_bytes_for_stream(
                &block_device,
                &boot_region,
                &invalidated_directory_bytes,
                stream,
            )
            .map_err(Error::from)?;
            allocated_cluster_ranges
        };

        if !allocated_cluster_ranges.is_empty() {
            let publication = state_guard
                .as_mut()
                .ok_or(MountVolumeStateError::UnpublishedState)
                .map_err(Error::from)?;
            let _ =
                fs.free_allocated_space_with_publication(publication, &allocated_cluster_ranges);
        }
        self.refresh_directory_metadata_after_namespace_mutation(
            &block_device,
            &boot_region,
            RealTimeCoarseClock::get().read_time(),
        )?;
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
        let (state_guard, block_device, boot_region, _, upcase_table, options) =
            fs.admitted_lookup_state().map_err(Error::from)?;

        let lookup_name = Self::admitted_name(name, &options)?;

        let lookup_name_hash = upcase_table.name_hash(&lookup_name);
        let child_inode = {
            let _state_guard = state_guard;
            self.lookup_child_by_name(
                &fs,
                &block_device,
                &boot_region,
                &upcase_table,
                &lookup_name,
                lookup_name_hash,
            )
            .map_err(Error::from)?
        };
        if let Some(child_inode) = child_inode {
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

        let (mut state_guard, block_device, boot_region, _anomaly, upcase_table, options) =
            fs.admitted_mutation_state().map_err(Error::from)?;
        if options.fs_flags.contains(FsFlags::RDONLY) {
            return_errno!(Errno::EROFS);
        }

        let admitted_old_name = Self::admitted_name(old_name, &options)?;
        let admitted_new_name = Self::admitted_name(new_name, &options)?;
        let old_name_hash = upcase_table.name_hash(&admitted_old_name);
        let new_name_hash = upcase_table.name_hash(&admitted_new_name);

        if self.metadata.read().ino == target_directory.metadata.read().ino {
            let renamed = {
                let _directory_guards = Self::ordered_directory_write_guards(vec![self]);
                let stream = *self.stream.read();
                let directory_bytes =
                    Self::read_directory_bytes_for_stream(&block_device, &boot_region, stream)
                        .map_err(Error::from)?;
                let Some(source_view) = Self::locate_named_child_view(
                    &directory_bytes,
                    stream.data_length.is_none(),
                    &upcase_table,
                    &admitted_old_name,
                    old_name_hash,
                )
                .map_err(Error::from)?
                else {
                    return_errno!(Errno::ENOENT);
                };
                let target_child_inode = Self::locate_named_child_view(
                    &directory_bytes,
                    stream.data_length.is_none(),
                    &upcase_table,
                    &admitted_new_name,
                    new_name_hash,
                )
                .map_err(Error::from)?
                .filter(|target_view| target_view.slot_range() != source_view.slot_range())
                .map(|target_view| {
                    let (target_inode_type, first_cluster, data_length, no_fat_chain) =
                        target_view.child_metadata(&boot_region)?;
                    if target_inode_type != InodeType::Dir {
                        return Ok(None);
                    }
                    Self::child_inode_from_directory_entry(
                        self,
                        &fs,
                        &boot_region,
                        stream.first_cluster,
                        target_view.slot_range(),
                        target_inode_type,
                        first_cluster,
                        data_length,
                        data_length,
                        no_fat_chain,
                    )
                    .map(Some)
                })
                .transpose()
                .map_err(Error::from)?
                .flatten();
                let publication = state_guard
                    .as_mut()
                    .ok_or(MountVolumeStateError::UnpublishedState)
                    .map_err(Error::from)?;
                let stream = *self.stream.read();
                self.rename_within_directory(
                    stream,
                    target_child_inode.as_ref(),
                    publication,
                    &fs,
                    &block_device,
                    &boot_region,
                    &upcase_table,
                    &admitted_old_name,
                    old_name_hash,
                    &admitted_new_name,
                    new_name_hash,
                )?
            };
            if renamed {
                self.refresh_directory_metadata_after_namespace_mutation(
                    &block_device,
                    &boot_region,
                    RealTimeCoarseClock::get().read_time(),
                )?;
            }
            return Ok(());
        }

        {
            let _directory_guards =
                Self::ordered_directory_write_guards(vec![self, target_directory]);
            let target_stream = *target_directory.stream.read();
            let target_directory_bytes =
                Self::read_directory_bytes_for_stream(&block_device, &boot_region, target_stream)
                    .map_err(Error::from)?;
            let target_child_inode = Self::locate_named_child_view(
                &target_directory_bytes,
                target_stream.data_length.is_none(),
                &upcase_table,
                &admitted_new_name,
                new_name_hash,
            )
            .map_err(Error::from)?
            .map(|target_view| {
                let (target_inode_type, first_cluster, data_length, no_fat_chain) =
                    target_view.child_metadata(&boot_region)?;
                if target_inode_type != InodeType::Dir {
                    return Ok(None);
                }
                Self::child_inode_from_directory_entry(
                    target_directory,
                    &fs,
                    &boot_region,
                    target_stream.first_cluster,
                    target_view.slot_range(),
                    target_inode_type,
                    first_cluster,
                    data_length,
                    data_length,
                    no_fat_chain,
                )
                .map(Some)
            })
            .transpose()
            .map_err(Error::from)?
            .flatten();
            let publication = state_guard
                .as_mut()
                .ok_or(MountVolumeStateError::UnpublishedState)
                .map_err(Error::from)?;
            let source_stream = *self.stream.read();
            let target_stream = *target_directory.stream.read();
            self.rename_across_directories(
                source_stream,
                target_directory,
                target_stream,
                target_child_inode.as_ref(),
                publication,
                &fs,
                &block_device,
                &boot_region,
                &upcase_table,
                &admitted_old_name,
                old_name_hash,
                &admitted_new_name,
                new_name_hash,
            )?;
        }
        let timestamp = RealTimeCoarseClock::get().read_time();
        self.refresh_directory_metadata_after_namespace_mutation(
            &block_device,
            &boot_region,
            timestamp,
        )?;
        target_directory.refresh_directory_metadata_after_namespace_mutation(
            &block_device,
            &boot_region,
            timestamp,
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
        if self.type_() != InodeType::File {
            return Ok(());
        }

        self.sync_regular_file(FileSyncScope::All)
    }

    fn sync_data(&self) -> Result<()> {
        if self.type_() != InodeType::File {
            return Ok(());
        }

        self.sync_regular_file(FileSyncScope::Data)
    }

    fn fallocate(&self, _mode: FallocMode, _offset: usize, _len: usize) -> Result<()> {
        if self.type_() == InodeType::Dir {
            return_errno!(Errno::EISDIR);
        }
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
