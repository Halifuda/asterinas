// SPDX-License-Identifier: MPL-2.0

//! Maps regular-file clusters into page-cache I/O ranges and serves cached reads.
//!
//! Method groups: cluster-map validation, cluster lookup, and read dispatch.

use ostd::mm::VmIo;

use super::{
    super::boot::BootRegion,
    ClusterMap, ExfatInode, StreamExtensionDirEntry,
};
use crate::{
    fs::file::{InodeType, StatusFlags},
    prelude::*,
};

impl ExfatInode {
    pub(super) fn validate_regular_file_mapping_shape(
        boot_region: &BootRegion,
        cluster_map: &StreamExtensionDirEntry,
        data_length: usize,
    ) -> Result<()> {
        let data_length_u64 = u64::try_from(data_length).map_err(|_| Error::new(Errno::EINVAL))?;
        match boot_region.validate_stream_data(cluster_map.first_cluster, data_length_u64) {
            Ok(()) => Ok(()),
            Err(_) => return_errno!(Errno::EINVAL),
        }
    }

    pub(super) fn mapped_regular_file_cluster(
        boot_region: &BootRegion,
        cluster_map: &ClusterMap,
        cluster_index: usize,
    ) -> Result<u32> {
        let (data_length, _) = cluster_map.validated_lengths()?;
        let stream_extension = cluster_map.stream_extension();
        if stream_extension.no_fat_chain {
            let cluster_count = data_length.div_ceil(boot_region.cluster_size);
            if cluster_index >= cluster_count {
                return_errno!(Errno::EINVAL);
            }
            let last_cluster = stream_extension
                .first_cluster
                .checked_add(
                    u32::try_from(cluster_count.saturating_sub(1))
                        .map_err(|_| Error::new(Errno::EINVAL))?,
                )
                .ok_or_else(|| Error::new(Errno::EINVAL))?;
            if !boot_region.is_valid_cluster(last_cluster) {
                return_errno!(Errno::EINVAL);
            }
            return stream_extension
                .first_cluster
                .checked_add(u32::try_from(cluster_index).map_err(|_| Error::new(Errno::EINVAL))?)
                .ok_or_else(|| Error::new(Errno::EINVAL));
        }

        cluster_map.mapped_cluster(boot_region, cluster_index)
    }

    pub(super) fn read_at_impl(
        &self,
        offset: usize,
        writer: &mut VmWriter,
        _status_flags: StatusFlags,
    ) -> Result<usize> {
        let fs = self
            .fs
            .upgrade()
            .ok_or_else(|| Error::with_message(Errno::EIO, "exFAT filesystem is not mounted"))?;
        let fs_state = fs.fs_state.read();
        let mount_state = fs_state.mount_state.as_ref().ok_or_else(super::super::not_mounted)?;
        if mount_state.forced_shutdown
            || mount_state.volume_flags.clear_to_zero
            || mount_state.volume_flags.media_failure
        {
            return_errno!(Errno::EIO);
        }
        let inode_state_guard = self.inode_state_read_guard();
        match inode_state_guard.metadata().type_ {
            InodeType::Dir => return_errno!(Errno::EISDIR),
            InodeType::File => {}
            _ => return_errno!(Errno::EOPNOTSUPP),
        }
        let allocation_guard = fs.allocation_read_guard()?;

        let (_cluster_map, data_length, _valid_data_length) =
            self.cluster_map_for_admitted_read(&inode_state_guard, &allocation_guard)?;
        if !writer.has_avail() || data_length == 0 {
            return Ok(0);
        }

        let page_cache = self
            .page_cache_handle(inode_state_guard.metadata())
            .ok_or_else(|| {
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
        let read_len = read_end - read_start;

        {
            let mut limited_writer = writer.clone_exclusive();
            limited_writer.limit(read_len);
            page_cache
                .read(read_start, &mut limited_writer)
                .map_err(Error::from)?;
        }
        writer.skip(read_len);
        Ok(read_len)
    }
}
