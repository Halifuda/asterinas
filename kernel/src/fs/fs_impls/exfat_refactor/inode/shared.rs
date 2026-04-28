// SPDX-License-Identifier: MPL-2.0

use super::*;

#[derive(Clone, Copy, Eq, PartialEq)]
pub(super) struct ExfatInodeStream {
    pub(super) data_length: Option<usize>,
    pub(super) first_cluster: u32,
    pub(super) valid_data_length: Option<usize>,
    pub(super) no_fat_chain: bool,
}

#[derive(Clone, Copy, Default)]
pub(super) struct ExfatInodeDirtyState {
    data_generation: u64,
    required_metadata_generation: u64,
    metadata_generation: u64,
    persisted_data_generation: u64,
    persisted_required_metadata_generation: u64,
    persisted_metadata_generation: u64,
}

impl ExfatInodeDirtyState {
    pub(super) fn mark_content_publication(&mut self) {
        self.data_generation = self.data_generation.saturating_add(1);
        self.required_metadata_generation = self.required_metadata_generation.saturating_add(1);
        self.metadata_generation = self.metadata_generation.saturating_add(1);
    }

    pub(super) fn mark_metadata_publication(&mut self) {
        self.metadata_generation = self.metadata_generation.saturating_add(1);
    }

    pub(super) fn needs_sync_data(self) -> bool {
        self.data_generation > self.persisted_data_generation
            || self.required_metadata_generation > self.persisted_required_metadata_generation
    }

    pub(super) fn needs_sync_all(self) -> bool {
        self.needs_sync_data() || self.metadata_generation > self.persisted_metadata_generation
    }

    pub(super) fn publish_data(&mut self, admitted: Self) {
        self.persisted_data_generation =
            self.persisted_data_generation.max(admitted.data_generation);
        self.persisted_required_metadata_generation = self
            .persisted_required_metadata_generation
            .max(admitted.required_metadata_generation);
    }

    pub(super) fn publish_all(&mut self, admitted: Self) {
        self.publish_data(admitted);
        self.persisted_metadata_generation = self
            .persisted_metadata_generation
            .max(admitted.metadata_generation);
    }
}

#[derive(Clone, Copy)]
pub(super) enum FileSyncScope {
    Data,
    All,
}

impl FileSyncScope {
    pub(super) fn needs_device_sync(self, dirty_state: ExfatInodeDirtyState) -> bool {
        match self {
            Self::Data => dirty_state.needs_sync_data(),
            Self::All => dirty_state.needs_sync_all(),
        }
    }
}

impl ExfatInode {
    pub(super) fn read_directory_bytes_for_stream(
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

        let mut remaining = data_length;
        let mut directory_bytes = Vec::with_capacity(data_length);
        let mut current_cluster = stream.first_cluster;
        let mut fat_reader =
            (!stream.no_fat_chain).then(|| FatReader::new(block_device.as_ref(), boot_region));
        while remaining != 0 {
            let cluster_start = boot_region.cluster_offset(current_cluster)?;
            let mut cluster_bytes = vec![0; boot_region.cluster_size];
            block_device
                .read_bytes(cluster_start, &mut cluster_bytes)
                .map_err(|_| MountVolumeStateError::DeviceIo)?;
            let bytes_to_copy = remaining.min(cluster_bytes.len());
            directory_bytes.extend_from_slice(&cluster_bytes[..bytes_to_copy]);
            remaining -= bytes_to_copy;
            if remaining == 0 {
                break;
            }
            current_cluster = match Self::advance_cluster(current_cluster, fat_reader.as_mut())? {
                Some(next_cluster) => next_cluster,
                None => return Err(MountVolumeStateError::InvalidOnDiskLayout),
            };
        }
        Ok(directory_bytes)
    }

    pub(super) fn regular_file_allocated_sectors(
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

    pub(super) fn decoded_exfat_timestamp(
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

    pub(super) fn exfat_utc_offset(
        utc_offset_byte: u8,
    ) -> core::result::Result<UtcOffset, MountVolumeStateError> {
        if utc_offset_byte & 0x80 == 0 {
            return Ok(UtcOffset::UTC);
        }

        let quarter_hours = (((utc_offset_byte & 0x7f) as i8) << 1) >> 1;
        UtcOffset::from_whole_seconds(i32::from(quarter_hours) * 15 * 60)
            .map_err(|_| MountVolumeStateError::InvalidOnDiskLayout)
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

    pub(super) fn admitted_directory_snapshot(
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

    pub(super) fn admitted_regular_file_stream_snapshot(
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

    pub(super) fn ordered_directory_write_guards<'a>(
        mut directories: Vec<&'a ExfatInode>,
    ) -> Vec<RwMutexWriteGuard<'a, ()>> {
        directories.sort_by_key(|directory| directory.metadata.read().ino);
        directories.dedup_by_key(|directory| directory.metadata.read().ino);
        directories
            .into_iter()
            .map(|directory| directory.admission.write())
            .collect()
    }

    pub(super) fn advance_cluster(
        current_cluster: u32,
        fat_reader: Option<&mut FatReader<'_>>,
    ) -> core::result::Result<Option<u32>, MountVolumeStateError> {
        match fat_reader {
            Some(fat_reader) => match fat_reader.next_cluster(current_cluster) {
                Ok(FatChainStep::Continue(next_cluster)) => Ok(Some(next_cluster)),
                Ok(FatChainStep::End) => Ok(None),
                Err(error) => Err(error),
            },
            None => current_cluster
                .checked_add(1)
                .map(Some)
                .ok_or(MountVolumeStateError::InvalidOnDiskLayout),
        }
    }

    pub(super) fn mark_content_publication_dirty(&self) {
        self.dirty_state.write().mark_content_publication();
    }

    pub(super) fn mark_metadata_publication_dirty(&self) {
        self.dirty_state.write().mark_metadata_publication();
    }

    pub(super) fn first_directory_child_scan<'a>(
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

    pub(super) fn entry_location_ino(
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

    pub(super) fn write_directory_bytes_for_stream(
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

        let mut remaining = directory_bytes;
        let mut current_cluster = stream.first_cluster;
        let mut fat_reader =
            (!stream.no_fat_chain).then(|| FatReader::new(block_device.as_ref(), boot_region));
        while !remaining.is_empty() {
            let bytes_to_write = remaining.len().min(boot_region.cluster_size);
            block_device
                .write_bytes(
                    boot_region.cluster_offset(current_cluster)?,
                    &remaining[..bytes_to_write],
                )
                .map_err(|_| MountVolumeStateError::DeviceIo)?;
            remaining = &remaining[bytes_to_write..];
            if remaining.is_empty() {
                break;
            }
            current_cluster = match Self::advance_cluster(current_cluster, fat_reader.as_mut())? {
                Some(next_cluster) => next_cluster,
                None => return Err(MountVolumeStateError::InvalidOnDiskLayout),
            };
        }
        Ok(())
    }

    pub(super) fn initialize_directory_cluster(
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
