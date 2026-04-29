// SPDX-License-Identifier: MPL-2.0

//! Stores inode cluster-map, dirty-state, admission, timestamp, and guard-order helpers.
//!
//! Method groups: dirty-state transitions, directory admission, regular-file snapshots,
//! directory byte I/O, timestamp conversion, child construction, and ordered write guards.

use alloc::vec::Vec;
use core::time::Duration;

use aster_block::BlockDevice;
use ostd::{
    mm::VmIo,
    sync::{RwMutexReadGuard, RwMutexWriteGuard},
};
use time::{Date, Month, OffsetDateTime, PrimitiveDateTime, Time, UtcOffset};

use super::{
    super::{
        boot::BootRegion,
        direntry::{DIRECTORY_ENTRY_SIZE, DirectoryEntrySlotRange},
        fat::{ChainVisitControl, FatChainStep, FatReader},
        fs::{
            AdmittedLookupState, AdmittedMutationState, ExfatFs, ExfatMountOptions,
            MountedVolumeState,
        },
        device_io, invalid_on_disk_layout, invalid_operation_input, unpublished_state,
        upcase::UpcaseTable,
    },
    ExfatInode,
};
use crate::{fs::file::InodeType, prelude::*};

#[derive(Clone, Copy, Eq, PartialEq)]
pub(super) struct ExfatInodeClusterMap {
    // `None` is reserved for the unbounded root directory; ordinary files and
    // directories always publish `Some(data_length)`.
    pub(super) data_length: Option<usize>,
    pub(super) first_cluster: u32,
    // `None` is reserved for the unbounded root directory.
    pub(super) valid_data_length: Option<usize>,
    pub(super) no_fat_chain: bool,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(super) enum DirtyLevel {
    Clean,
    Metadata,
    Data,
    DataAndMetadata,
}

#[derive(Clone, Copy, Default)]
pub(super) struct ExfatInodeDirtyState {
    next_generation: u64,
    content_generation: Option<u64>,
    metadata_generation: Option<u64>,
}

impl ExfatInodeDirtyState {
    fn next_dirty_generation(&mut self) -> u64 {
        self.next_generation = self.next_generation.saturating_add(1);
        self.next_generation
    }

    pub(super) fn dirty_level(self) -> DirtyLevel {
        match (self.content_generation, self.metadata_generation) {
            (None, None) => DirtyLevel::Clean,
            (None, Some(_)) => DirtyLevel::Metadata,
            (Some(_), None) => DirtyLevel::Data,
            (Some(_), Some(_)) => DirtyLevel::DataAndMetadata,
        }
    }

    pub(super) fn mark_content_publication(&mut self) {
        let generation = self.next_dirty_generation();
        self.content_generation = Some(generation);
        self.metadata_generation = None;
    }

    pub(super) fn mark_metadata_publication(&mut self) {
        self.metadata_generation = Some(self.next_dirty_generation());
    }

    pub(super) fn needs_sync_data(self) -> bool {
        matches!(
            self.dirty_level(),
            DirtyLevel::Data | DirtyLevel::DataAndMetadata
        )
    }

    pub(super) fn needs_sync_all(self) -> bool {
        self.dirty_level() != DirtyLevel::Clean
    }

    fn clear_published_content(&mut self, admitted: Self) {
        if admitted
            .content_generation
            .zip(self.content_generation)
            .is_some_and(|(admitted_generation, current_generation)| {
                current_generation <= admitted_generation
            })
        {
            self.content_generation = None;
        }
    }

    fn clear_published_metadata(&mut self, admitted: Self) {
        if admitted
            .metadata_generation
            .zip(self.metadata_generation)
            .is_some_and(|(admitted_generation, current_generation)| {
                current_generation <= admitted_generation
            })
        {
            self.metadata_generation = None;
        }
    }

    pub(super) fn publish_data(&mut self, admitted: Self) {
        self.clear_published_content(admitted);
    }

    pub(super) fn publish_all(&mut self, admitted: Self) {
        self.clear_published_content(admitted);
        self.clear_published_metadata(admitted);
    }
}

#[derive(Clone, Copy)]
pub(super) enum InodeRewriteTarget {
    Directory,
    RegularFile,
}

#[derive(Clone, Copy)]
pub(super) enum InodeTimestampField {
    Accessed,
    Modified,
}

#[derive(Clone, Copy)]
pub(super) enum DirectoryContextMode {
    Lookup,
    Mutation,
}

pub(super) struct AdmittedDirectoryContext<'a> {
    access: DirectoryContextAccess<'a>,
}

enum DirectoryContextAccess<'a> {
    Lookup(AdmittedLookupState<'a>),
    Mutation(AdmittedMutationState<'a>),
}

impl AdmittedDirectoryContext<'_> {
    pub(super) fn block_device(&self) -> Arc<dyn BlockDevice> {
        match &self.access {
            DirectoryContextAccess::Lookup(admission) => admission.block_device.clone(),
            DirectoryContextAccess::Mutation(admission) => admission.block_device.clone(),
        }
    }

    pub(super) fn boot_region(&self) -> BootRegion {
        match &self.access {
            DirectoryContextAccess::Lookup(admission) => admission.boot_region,
            DirectoryContextAccess::Mutation(admission) => admission.boot_region,
        }
    }

    pub(super) fn forced_shutdown(&self) -> bool {
        match &self.access {
            DirectoryContextAccess::Lookup(admission) => admission.forced_shutdown,
            DirectoryContextAccess::Mutation(admission) => admission.forced_shutdown,
        }
    }

    pub(super) fn options(&self) -> ExfatMountOptions {
        match &self.access {
            DirectoryContextAccess::Lookup(admission) => admission.options.clone(),
            DirectoryContextAccess::Mutation(admission) => admission.options.clone(),
        }
    }

    pub(super) fn publication(&mut self) -> Result<&mut MountedVolumeState> {
        let DirectoryContextAccess::Mutation(admission) = &mut self.access else {
            return_errno_with_message!(
                Errno::EINVAL,
                "lookup admission has no mutation publication"
            );
        };
        admission
            .state_guard
            .as_mut()
            .ok_or(unpublished_state())
            .map_err(Error::from)
    }

    pub(super) fn upcase_table(&self) -> Arc<UpcaseTable> {
        match &self.access {
            DirectoryContextAccess::Lookup(admission) => admission.upcase_table.clone(),
            DirectoryContextAccess::Mutation(admission) => admission.upcase_table.clone(),
        }
    }
}

impl ExfatInode {
    // Admission

    pub(super) fn admitted_directory_context<'a>(
        &self,
        fs: &'a Arc<ExfatFs>,
        mode: DirectoryContextMode,
    ) -> Result<AdmittedDirectoryContext<'a>> {
        if self.metadata.read().type_ != InodeType::Dir {
            return_errno!(Errno::ENOTDIR);
        }
        let access = match mode {
            DirectoryContextMode::Lookup => {
                DirectoryContextAccess::Lookup(fs.admitted_lookup_state().map_err(Error::from)?)
            }
            DirectoryContextMode::Mutation => {
                DirectoryContextAccess::Mutation(fs.admitted_mutation_state().map_err(Error::from)?)
            }
        };
        Ok(AdmittedDirectoryContext { access })
    }

    pub(super) fn admitted_directory_snapshot(
        &self,
    ) -> Result<(RwMutexReadGuard<'_, ()>, ExfatInodeClusterMap)> {
        let owner = self.admission.read();
        let cluster_map = *self.cluster_map.read();
        Ok((owner, cluster_map))
    }

    pub(super) fn admitted_regular_file_cluster_map_snapshot(
        &self,
    ) -> Result<(RwMutexReadGuard<'_, ()>, ExfatInodeClusterMap, usize, usize)> {
        match self.metadata.read().type_ {
            InodeType::Dir => return_errno!(Errno::EISDIR),
            InodeType::File => {}
            _ => return_errno!(Errno::EOPNOTSUPP),
        }

        let owner = self.admission.read();
        let cluster_map = *self.cluster_map.read();
        let Some(data_length) = cluster_map.data_length else {
            return_errno!(Errno::EINVAL);
        };
        let Some(valid_data_length) = cluster_map.valid_data_length else {
            return_errno!(Errno::EINVAL);
        };
        if valid_data_length > data_length {
            return_errno!(Errno::EINVAL);
        }
        if data_length == 0 && (cluster_map.first_cluster != 0 || valid_data_length != 0) {
            return_errno!(Errno::EINVAL);
        }
        Ok((owner, cluster_map, data_length, valid_data_length))
    }

    // Directory I/O

    pub(super) fn read_directory_bytes_for_cluster_map(
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        cluster_map: ExfatInodeClusterMap,
    ) -> Result<Vec<u8>> {
        let Some(data_length) = cluster_map.data_length else {
            let mut directory_bytes = Vec::new();
            let mut fat_reader = FatReader::new(block_device.as_ref(), boot_region);
            fat_reader.walk_cluster_chain(cluster_map.first_cluster, |_, cluster_bytes| {
                directory_bytes.extend_from_slice(cluster_bytes);
                Ok(ChainVisitControl::Continue)
            })?;
            return Ok(directory_bytes);
        };

        if data_length == 0 {
            if cluster_map.first_cluster != 0 {
                return Err(invalid_on_disk_layout());
            }
            return Ok(Vec::new());
        }
        if data_length % DIRECTORY_ENTRY_SIZE != 0 {
            return Err(invalid_on_disk_layout());
        }

        let data_length_u64 =
            u64::try_from(data_length).map_err(|_| invalid_on_disk_layout())?;
        boot_region.validate_stream_data(cluster_map.first_cluster, data_length_u64)?;
        if cluster_map.no_fat_chain {
            let mut remaining = data_length;
            let mut directory_bytes = Vec::with_capacity(data_length);
            let mut current_cluster = cluster_map.first_cluster;
            while remaining != 0 {
                let cluster_start = boot_region.cluster_offset(current_cluster)?;
                let mut cluster_bytes = vec![0; boot_region.cluster_size];
                block_device
                    .read_bytes(cluster_start, &mut cluster_bytes)
                    .map_err(|_| device_io())?;
                let bytes_to_copy = remaining.min(cluster_bytes.len());
                directory_bytes.extend_from_slice(&cluster_bytes[..bytes_to_copy]);
                remaining -= bytes_to_copy;
                if remaining == 0 {
                    return Ok(directory_bytes);
                }
                current_cluster = Self::advance_cluster(current_cluster, None)?
                    .ok_or_else(invalid_on_disk_layout)?;
            }
            return Err(invalid_on_disk_layout());
        }

        let mut remaining = data_length;
        let mut directory_bytes = Vec::with_capacity(data_length);
        let mut fat_reader = FatReader::new(block_device.as_ref(), boot_region);
        fat_reader.walk_cluster_chain(cluster_map.first_cluster, |_, cluster_bytes| {
            let bytes_to_copy = remaining.min(cluster_bytes.len());
            directory_bytes.extend_from_slice(&cluster_bytes[..bytes_to_copy]);
            remaining -= bytes_to_copy;
            Ok(if remaining == 0 {
                ChainVisitControl::Stop
            } else {
                ChainVisitControl::Continue
            })
        })?;
        if remaining != 0 {
            return Err(invalid_on_disk_layout());
        }
        Ok(directory_bytes)
    }

    pub(super) fn write_directory_bytes_for_cluster_map(
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        directory_bytes: &[u8],
        cluster_map: ExfatInodeClusterMap,
    ) -> Result<()> {
        let expected_length = match cluster_map.data_length {
            Some(data_length) => data_length,
            None => directory_bytes.len(),
        };
        if directory_bytes.len() != expected_length {
            return Err(invalid_operation_input());
        }
        if directory_bytes.is_empty() {
            return Ok(());
        }
        if cluster_map.data_length.is_some() && cluster_map.no_fat_chain {
            let mut remaining = directory_bytes;
            let mut current_cluster = cluster_map.first_cluster;
            while !remaining.is_empty() {
                let bytes_to_write = remaining.len().min(boot_region.cluster_size);
                block_device
                    .write_bytes(
                        boot_region.cluster_offset(current_cluster)?,
                        &remaining[..bytes_to_write],
                    )
                    .map_err(|_| device_io())?;
                remaining = &remaining[bytes_to_write..];
                if remaining.is_empty() {
                    return Ok(());
                }
                current_cluster = Self::advance_cluster(current_cluster, None)?
                    .ok_or_else(invalid_on_disk_layout)?;
            }
            return Err(invalid_on_disk_layout());
        }

        let mut remaining = directory_bytes;
        let mut current_cluster = cluster_map.first_cluster;
        let mut fat_reader =
            (!cluster_map.no_fat_chain).then(|| FatReader::new(block_device.as_ref(), boot_region));
        while !remaining.is_empty() {
            let bytes_to_write = remaining.len().min(boot_region.cluster_size);
            block_device
                .write_bytes(
                    boot_region.cluster_offset(current_cluster)?,
                    &remaining[..bytes_to_write],
                )
                .map_err(|_| device_io())?;
            remaining = &remaining[bytes_to_write..];
            if remaining.is_empty() {
                break;
            }
            current_cluster = match Self::advance_cluster(current_cluster, fat_reader.as_mut())? {
                Some(next_cluster) => next_cluster,
                None => return Err(invalid_on_disk_layout()),
            };
        }
        Ok(())
    }

    pub(super) fn initialize_directory_cluster(
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        first_cluster: u32,
    ) -> Result<()> {
        let cluster_offset = boot_region.cluster_offset(first_cluster)?;
        let cluster_bytes = vec![0; boot_region.cluster_size];
        block_device
            .write_bytes(cluster_offset, &cluster_bytes)
            .map_err(|_| device_io())
    }

    pub(super) fn advance_cluster(
        current_cluster: u32,
        fat_reader: Option<&mut FatReader<'_>>,
    ) -> Result<Option<u32>> {
        match fat_reader {
            Some(fat_reader) => match fat_reader.next_cluster(current_cluster) {
                Ok(FatChainStep::Continue(next_cluster)) => Ok(Some(next_cluster)),
                Ok(FatChainStep::End) => Ok(None),
                Err(error) => Err(error),
            },
            None => current_cluster
                .checked_add(1)
                .map(Some)
                .ok_or(invalid_on_disk_layout()),
        }
    }

    // Dirty tracking

    pub(super) fn mark_content_publication_dirty(&self) {
        self.dirty_state.write().mark_content_publication();
    }

    pub(super) fn mark_metadata_publication_dirty(&self) {
        self.dirty_state.write().mark_metadata_publication();
    }

    // Identity

    pub(super) fn entry_location_ino(
        &self,
        entry_index: usize,
    ) -> Result<u64> {
        let cluster_map = self.cluster_map.read();
        Ok((u64::from(cluster_map.first_cluster) << 32)
            | u64::from(u32::try_from(entry_index).map_err(|_| invalid_on_disk_layout())?))
    }

    pub(super) fn child_inode_from_directory_entry(
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
    ) -> Result<Arc<Self>> {
        let child_ino = (u64::from(parent_first_cluster) << 32)
            | u64::from(
                u32::try_from(slot_range.first_entry_index())
                    .map_err(|_| invalid_on_disk_layout())?,
            );
        Ok(Self::new_child(
            fs,
            parent.weak_self(),
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

    // Input validation

    pub(super) fn admitted_name(
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

    // Timestamp codec

    pub(super) fn decoded_exfat_timestamp(
        timestamp_bytes: [u8; 4],
        ten_ms_increment: Option<u8>,
        utc_offset_byte: u8,
    ) -> Result<Duration> {
        if timestamp_bytes == [0; 4] && ten_ms_increment.unwrap_or(0) == 0 {
            return Ok(Duration::ZERO);
        }

        let encoded_date = u16::from_le_bytes([timestamp_bytes[2], timestamp_bytes[3]]);
        let encoded_year = 1980i32 + i32::from(encoded_date >> 9);
        let encoded_month = u8::try_from((encoded_date >> 5) & 0x0f)
            .map_err(|_| invalid_on_disk_layout())?;
        let encoded_day =
            u8::try_from(encoded_date & 0x1f).map_err(|_| invalid_on_disk_layout())?;
        let month =
            Month::try_from(encoded_month).map_err(|_| invalid_on_disk_layout())?;
        let date = Date::from_calendar_date(encoded_year, month, encoded_day)
            .map_err(|_| invalid_on_disk_layout())?;

        let time = if let Some(ten_ms_increment) = ten_ms_increment {
            if ten_ms_increment >= 200 {
                return Err(invalid_on_disk_layout());
            }

            let encoded_time = u16::from_le_bytes([timestamp_bytes[0], timestamp_bytes[1]]);
            let seconds = u8::try_from(encoded_time & 0x1f)
                .map_err(|_| invalid_on_disk_layout())?
                .checked_mul(2)
                .and_then(|seconds| seconds.checked_add(ten_ms_increment / 100))
                .ok_or(invalid_on_disk_layout())?;
            let milliseconds = u16::from(ten_ms_increment % 100) * 10;
            let hour = u8::try_from((encoded_time >> 11) & 0x1f)
                .map_err(|_| invalid_on_disk_layout())?;
            let minute = u8::try_from((encoded_time >> 5) & 0x3f)
                .map_err(|_| invalid_on_disk_layout())?;
            Time::from_hms_milli(hour, minute, seconds, milliseconds)
                .map_err(|_| invalid_on_disk_layout())?
        } else {
            Time::MIDNIGHT
        };

        let utc_offset = Self::exfat_utc_offset(utc_offset_byte)?;
        let date_time = PrimitiveDateTime::new(date, time).assume_offset(utc_offset);
        let unix_timestamp_nanos = u64::try_from(date_time.unix_timestamp_nanos())
            .map_err(|_| invalid_on_disk_layout())?;
        Ok(Duration::from_nanos(unix_timestamp_nanos))
    }

    pub(super) fn exfat_utc_offset(
        utc_offset_byte: u8,
    ) -> Result<UtcOffset> {
        if utc_offset_byte & 0x80 == 0 {
            return Ok(UtcOffset::UTC);
        }

        let quarter_hours = (((utc_offset_byte & 0x7f) as i8) << 1) >> 1;
        UtcOffset::from_whole_seconds(i32::from(quarter_hours) * 15 * 60)
            .map_err(|_| invalid_on_disk_layout())
    }

    pub(super) fn encoded_exfat_timestamp_fields(
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

    // Misc computation

    pub(super) fn regular_file_allocated_sectors(
        boot_region: &BootRegion,
        data_length: usize,
    ) -> Result<usize> {
        let allocated_clusters = if data_length == 0 {
            0
        } else {
            data_length.div_ceil(boot_region.cluster_size)
        };
        allocated_clusters
            .checked_mul(boot_region.sectors_per_cluster)
            .ok_or(invalid_on_disk_layout())
    }

    // Other helpers

    pub(super) fn directory_write_guards_by_ino<'a>(
        mut directories: Vec<&'a ExfatInode>,
    ) -> Vec<RwMutexWriteGuard<'a, ()>> {
        directories.sort_by_key(|directory| directory.metadata.read().ino);
        directories.dedup_by_key(|directory| directory.metadata.read().ino);
        directories
            .into_iter()
            .map(|directory| directory.admission.write())
            .collect()
    }
}
