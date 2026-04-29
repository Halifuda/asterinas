// SPDX-License-Identifier: MPL-2.0

use aster_block::{
    BlockDevice,
    bio::{Bio, BioDirection, BioSegment, BioType, BioWaiter},
    id::Sid,
};
use ostd::mm::{Segment, VmIo};

use super::{
    super::{
        boot::BootRegion,
        fat::{FatChainStep, FatReader},
        invalid_on_disk_layout,
    },
    ExfatInode, ExfatInodeClusterMap,
};
use crate::{
    fs::{
        file::{InodeType, StatusFlags},
        vfs::{inode::Inode, page_cache::CachePage},
    },
    prelude::*,
};

impl ExfatInode {
    pub(super) fn validate_regular_file_mapping_shape(
        boot_region: &BootRegion,
        cluster_map: &ExfatInodeClusterMap,
        data_length: usize,
    ) -> Result<()> {
        let data_length_u64 = u64::try_from(data_length).map_err(|_| Error::new(Errno::EINVAL))?;
        match boot_region.validate_stream_data(cluster_map.first_cluster, data_length_u64) {
            Ok(()) => Ok(()),
            Err(_) => return_errno!(Errno::EINVAL),
        }
    }

    pub(super) fn mapped_regular_file_cluster(
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        cluster_map: &ExfatInodeClusterMap,
        data_length: usize,
        cluster_index: usize,
    ) -> Result<u32> {
        if cluster_map.no_fat_chain {
            let cluster_count = data_length.div_ceil(boot_region.cluster_size);
            if cluster_index >= cluster_count {
                return_errno!(Errno::EINVAL);
            }
            let last_cluster = cluster_map
                .first_cluster
                .checked_add(
                    u32::try_from(cluster_count.saturating_sub(1))
                        .map_err(|_| Error::new(Errno::EINVAL))?,
                )
                .ok_or_else(|| Error::new(Errno::EINVAL))?;
            if !boot_region.is_valid_cluster(last_cluster) {
                return_errno!(Errno::EINVAL);
            }
            return cluster_map
                .first_cluster
                .checked_add(u32::try_from(cluster_index).map_err(|_| Error::new(Errno::EINVAL))?)
                .ok_or_else(|| Error::new(Errno::EINVAL));
        }

        let mut fat_reader = FatReader::new(block_device.as_ref(), boot_region);
        let mut current_cluster = cluster_map.first_cluster;
        for _ in 0..cluster_index {
            current_cluster = match fat_reader.next_cluster(current_cluster) {
                Ok(FatChainStep::Continue(next_cluster)) => next_cluster,
                Ok(FatChainStep::End) | Err(_) => return_errno!(Errno::EIO),
            };
        }
        Ok(current_cluster)
    }

    pub(super) fn map_regular_file_logical_offset(
        &self,
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        offset: usize,
    ) -> Result<Option<usize>> {
        let fs = self
            .fs
            .upgrade()
            .ok_or_else(|| Error::with_message(Errno::EIO, "exFAT filesystem is not mounted"))?;
        let admission = fs.published_lookup_state().map_err(Error::from)?;
        if admission.anomaly.clear_to_zero || admission.anomaly.media_failure {
            return_errno!(Errno::EIO);
        }

        let (_owner_guard, cluster_map, data_length, valid_data_length) =
            self.admitted_regular_file_cluster_map_snapshot()?;
        if data_length == 0 || offset >= data_length || offset >= valid_data_length {
            return Ok(None);
        }

        Self::validate_regular_file_mapping_shape(boot_region, &cluster_map, data_length)?;
        let cluster_size = boot_region.cluster_size;
        let cluster_index = offset / cluster_size;
        let cluster = Self::mapped_regular_file_cluster(
            block_device,
            boot_region,
            &cluster_map,
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

    pub(super) fn regular_file_page_bio_ranges(
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        cluster_map: &ExfatInodeClusterMap,
        data_length: usize,
        file_offset: usize,
        len: usize,
    ) -> Result<Vec<(usize, usize, usize)>> {
        if len == 0 {
            return Ok(Vec::new());
        }

        Self::validate_regular_file_mapping_shape(boot_region, cluster_map, data_length)?;

        let cluster_size = boot_region.cluster_size;
        let cluster_index = file_offset / cluster_size;
        let mut cluster_offset = file_offset % cluster_size;
        let mut current_cluster = Self::mapped_regular_file_cluster(
            block_device,
            boot_region,
            cluster_map,
            data_length,
            cluster_index,
        )?;
        let mut page_offset = 0usize;
        let mut remaining = len;
        let mut ranges: Vec<(usize, usize, usize)> = Vec::new();
        let mut fat_reader =
            (!cluster_map.no_fat_chain).then(|| FatReader::new(block_device.as_ref(), boot_region));

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

            let using_fat_chain = fat_reader.is_some();
            current_cluster = match Self::advance_cluster(current_cluster, fat_reader.as_mut()) {
                Ok(Some(next_cluster)) => next_cluster,
                Ok(None) | Err(_) if using_fat_chain => return_errno!(Errno::EIO),
                Ok(None) | Err(_) => return_errno!(Errno::EINVAL),
            };
        }

        Ok(ranges)
    }

    pub(super) fn regular_file_page_range(
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

    pub(super) fn regular_file_page_waiter(
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        frame: &CachePage,
        cluster_map: &ExfatInodeClusterMap,
        data_length: usize,
        file_offset: usize,
        initialized_len: usize,
        bio_type: BioType,
    ) -> Result<BioWaiter> {
        let page_ranges = Self::regular_file_page_bio_ranges(
            block_device,
            boot_region,
            cluster_map,
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

    pub(super) fn read_regular_file_at(
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        cluster_map: ExfatInodeClusterMap,
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

        Self::validate_regular_file_mapping_shape(boot_region, &cluster_map, data_length)?;
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
            let mut fat_reader =
                (!cluster_map.no_fat_chain).then(|| FatReader::new(block_device.as_ref(), boot_region));
            let mut cluster_buffer = vec![0; cluster_size];
            let mut current_cluster = Self::mapped_regular_file_cluster(
                block_device,
                boot_region,
                &cluster_map,
                data_length,
                cluster_index,
            )?;
            while initialized_remaining != 0 {
                let chunk_len = initialized_remaining.min(cluster_size - cluster_offset);
                let cluster_start = boot_region.cluster_offset(current_cluster).map_err(|_| {
                    if cluster_map.no_fat_chain {
                        invalid_on_disk_layout()
                    } else {
                        Error::new(Errno::EIO)
                    }
                })?;
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
                let using_fat_chain = fat_reader.is_some();
                current_cluster = match Self::advance_cluster(current_cluster, fat_reader.as_mut())
                {
                    Ok(Some(next_cluster)) => next_cluster,
                    Ok(None) | Err(_) if using_fat_chain => return_errno!(Errno::EIO),
                    Ok(None) | Err(_) => return_errno!(Errno::EINVAL),
                };
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

    pub(super) fn read_at_impl(
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
        let admission = fs.published_lookup_state().map_err(Error::from)?;
        if admission.forced_shutdown
            || admission.anomaly.clear_to_zero
            || admission.anomaly.media_failure
        {
            return_errno!(Errno::EIO);
        }

        let (_owner_guard, _cluster_map, data_length, _valid_data_length) =
            self.admitted_regular_file_cluster_map_snapshot()?;
        if !writer.has_avail() || data_length == 0 {
            return Ok(0);
        }

        let page_cache = self.page_cache_handle().ok_or_else(|| {
            Error::with_message(Errno::EIO, "regular exFAT file has no page cache")
        })?;
        let read_start = offset.min(data_length);
        let read_end = offset
            .checked_add(writer.avail())
            .ok_or_else(|| Error::new(Errno::EINVAL))?
            .min(data_length);
        if read_start == read_end {
            return Ok(0);
        }

        page_cache.pages().read(read_start, writer)?;
        Ok(read_end - read_start)
    }
}
