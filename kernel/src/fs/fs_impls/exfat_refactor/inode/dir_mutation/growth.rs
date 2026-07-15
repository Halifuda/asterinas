// SPDX-License-Identifier: MPL-2.0

//! Owns directory cluster-map growth and publication helpers.
//!
//! Method groups: directory cluster growth, attachment, and publication.

use aster_block::BlockDevice;

use super::super::{ClusterMap, ExfatInode, StreamExtensionDirEntry, state::InodeStateWriteGuard};
use crate::{
    fs::{
        file::InodeType,
        fs_impls::exfat_refactor::{
            bitmap::ClusterRange,
            boot::BootRegion,
            dir_entry_format::DirEntrySlotRange,
            fat::FatReader,
            fs::{AllocGuard, ExfatFs, FsState},
            invalid_on_disk_layout,
        },
    },
    prelude::*,
};

impl ExfatInode {
    pub(super) fn grow_directory_cluster_map(
        &self,
        cluster_map: StreamExtensionDirEntry,
        allocation_guard: &mut AllocGuard<'_>,
        fs_state: &mut FsState,
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        parent_inode_state_guard: Option<&InodeStateWriteGuard<'_>>,
        self_inode_state_guard: &InodeStateWriteGuard<'_>,
    ) -> Result<StreamExtensionDirEntry> {
        allocation_guard.allocate(1, None)?;
        let allocated_cluster = allocation_guard.single_cluster()?;
        let mut publication_complete = false;
        let update_result = (|| {
            Self::initialize_directory_cluster(block_device, boot_region, allocated_cluster)?;
            let (
                updated_cluster_map,
                updated_cluster_map_generation,
                updated_allocated_size,
                exposed_old_topology,
                exposure_error,
                prepared_parent_entry_set_write,
            ) = self.attach_directory_cluster(
                cluster_map,
                allocation_guard,
                self_inode_state_guard,
                block_device,
                boot_region,
                allocated_cluster,
                |updated_cluster_map| {
                    if updated_cluster_map.data_length.is_none() {
                        return Ok(None);
                    }
                    let parent_inode_state_guard = parent_inode_state_guard.ok_or_else(|| {
                        Error::with_message(
                            Errno::EINVAL,
                            "ordinary exFAT directory growth requires parent write-guard proof",
                        )
                    })?;
                    self.prepare_rewritten_entry_set_write_with_guard(
                        self_inode_state_guard,
                        parent_inode_state_guard,
                        boot_region,
                        |entry_view| {
                            let (inode_type, _first_cluster, _data_length, _no_fat_chain) =
                                entry_view.child_metadata(boot_region)?;
                            if inode_type != InodeType::Dir || !entry_view.is_directory() {
                                return Err(invalid_on_disk_layout());
                            }

                            let mut updated_entry_set = entry_view.to_mutable();
                            updated_entry_set.set_cluster_map(&updated_cluster_map)?;
                            Ok(Some(updated_entry_set.into_bytes()))
                        },
                    )
                },
            )?;
            if exposed_old_topology {
                self.commit_directory_cluster_map(
                    self_inode_state_guard,
                    updated_cluster_map_generation.clone(),
                    updated_allocated_size,
                )?;
                allocation_guard.commit_allocation();
                publication_complete = true;
                if let Some(error) = exposure_error {
                    return Err(error);
                }
            }
            let parent_entry_set_write_result =
                if let Some(prepared_parent_entry_set_write) = prepared_parent_entry_set_write {
                    let parent_inode_state_guard = parent_inode_state_guard.ok_or_else(|| {
                        Error::with_message(
                            Errno::EINVAL,
                            "ordinary exFAT directory growth requires parent write-guard proof",
                        )
                    })?;
                    let parent_inode = self_inode_state_guard.parent().ok_or_else(|| {
                        Error::with_message(
                            Errno::EIO,
                            "ordinary exFAT directory parent is not mounted",
                        )
                    })?;
                    if !parent_inode_state_guard.guards_inode(parent_inode.as_ref()) {
                        return Err(Error::new(Errno::EINVAL));
                    }
                    let entry_set_write_result = self.persist_prepared_entry_set_write_classified(
                        fs_state,
                        prepared_parent_entry_set_write,
                        parent_inode.as_ref(),
                        parent_inode_state_guard.metadata(),
                        true,
                    )?;
                    Some(entry_set_write_result)
                } else {
                    None
                };
            if !exposed_old_topology
                && matches!(parent_entry_set_write_result.as_ref(), Some(Ok(false)))
            {
                return Err(invalid_on_disk_layout());
            }
            if !exposed_old_topology {
                self.commit_directory_cluster_map(
                    self_inode_state_guard,
                    updated_cluster_map_generation,
                    updated_allocated_size,
                )?;
                allocation_guard.commit_allocation();
                publication_complete = true;
            }
            if let Some(entry_set_write_result) = parent_entry_set_write_result
                && !entry_set_write_result?
            {
                return Err(invalid_on_disk_layout());
            }
            Ok(updated_cluster_map)
        })();
        match update_result {
            Ok(updated_cluster_map) => {
                allocation_guard.commit_allocation();
                Ok(updated_cluster_map)
            }
            Err(error) => {
                if !publication_complete && allocation_guard.rollback_allocation()? {
                    ExfatFs::disable_unsupported_discard_after_release(fs_state);
                }
                Err(error)
            }
        }
    }

    fn attach_directory_cluster(
        &self,
        cluster_map: StreamExtensionDirEntry,
        allocation_guard: &AllocGuard<'_>,
        self_inode_state_guard: &InodeStateWriteGuard<'_>,
        block_device: &Arc<dyn BlockDevice>,
        boot_region: &BootRegion,
        allocated_cluster: u32,
        prepare_parent_entry_set_write_fn: impl FnOnce(
            StreamExtensionDirEntry,
        ) -> Result<
            Option<(DirEntrySlotRange, Vec<u8>, Vec<u8>, Vec<(usize, bool)>)>,
        >,
    ) -> Result<(
        StreamExtensionDirEntry,
        Arc<ClusterMap>,
        usize,
        bool,
        Option<Error>,
        Option<(DirEntrySlotRange, Vec<u8>, Vec<u8>, Vec<(usize, bool)>)>,
    )> {
        let next_data_length = match cluster_map.data_length {
            Some(data_length) => data_length
                .checked_add(boot_region.cluster_size)
                .ok_or(invalid_on_disk_layout())?,
            None => boot_region.cluster_size,
        };

        let admitted_cluster_map = match cluster_map.data_length {
            Some(_) => self.cluster_map_for_write_guard(
                self_inode_state_guard,
                allocation_guard,
                cluster_map,
            ),
            None => self_inode_state_guard
                .cached_cluster_map()
                .filter(|generation| generation.stream_extension() == cluster_map)
                .ok_or_else(invalid_on_disk_layout),
        }?;
        if self_inode_state_guard.dir_entry_stream() != cluster_map {
            return Err(invalid_on_disk_layout());
        }
        if cluster_map.data_length.is_none()
            && !self_inode_state_guard
                .cached_cluster_map()
                .as_ref()
                .is_some_and(|current| Arc::ptr_eq(current, &admitted_cluster_map))
        {
            return Err(invalid_on_disk_layout());
        }

        let updated_cluster_map = match cluster_map.data_length {
            Some(0) => StreamExtensionDirEntry {
                data_length: Some(next_data_length),
                first_cluster: allocated_cluster,
                valid_data_length: Some(next_data_length),
                no_fat_chain: false,
            },
            Some(_) if cluster_map.no_fat_chain => StreamExtensionDirEntry {
                data_length: Some(next_data_length),
                valid_data_length: Some(next_data_length),
                no_fat_chain: false,
                ..cluster_map
            },
            Some(_) => StreamExtensionDirEntry {
                data_length: Some(next_data_length),
                valid_data_length: Some(next_data_length),
                ..cluster_map
            },
            None => cluster_map,
        };
        let updated_generation = Arc::new(admitted_cluster_map.appended(
            boot_region,
            updated_cluster_map,
            &[ClusterRange {
                start_cluster: allocated_cluster,
                cluster_count: 1,
            }],
        )?);
        let updated_allocated_size = updated_generation.allocated_byte_length(boot_region)?;
        let exposed_old_topology = match cluster_map.data_length {
            None => true,
            Some(data_length) => data_length != 0 && !cluster_map.no_fat_chain,
        };
        let prepared_parent_entry_set_write =
            prepare_parent_entry_set_write_fn(updated_cluster_map)?;

        let mut fat_reader = FatReader::new(block_device.as_ref(), boot_region);
        let exposure_error = match cluster_map.data_length {
            Some(0) => {
                fat_reader.terminate_cluster_chain(allocated_cluster)?;
                None
            }
            Some(data_length) if cluster_map.no_fat_chain => {
                let cluster_count = data_length.div_ceil(boot_region.cluster_size);
                fat_reader.link_contiguous_chain_to_cluster(
                    cluster_map.first_cluster,
                    cluster_count,
                    allocated_cluster,
                )?;
                None
            }
            Some(_) | None => {
                fat_reader.terminate_cluster_chain(allocated_cluster)?;
                let tail_cluster = admitted_cluster_map
                    .terminal_cluster(boot_region)?
                    .ok_or_else(invalid_on_disk_layout)?;
                fat_reader
                    .link_prepared_chain_to_tail(tail_cluster, allocated_cluster)?
                    .err()
            }
        };
        Ok((
            updated_cluster_map,
            updated_generation,
            updated_allocated_size,
            exposed_old_topology,
            exposure_error,
            prepared_parent_entry_set_write,
        ))
    }

    fn commit_directory_cluster_map(
        &self,
        self_inode_state_guard: &InodeStateWriteGuard<'_>,
        updated_cluster_map_generation: Arc<ClusterMap>,
        updated_allocated_size: usize,
    ) -> Result<()> {
        let metadata = self_inode_state_guard.metadata();
        let previous_size = metadata.size;
        let page_cache_context = self.page_cache_context_for_mapping(
            metadata,
            updated_cluster_map_generation.clone(),
            updated_allocated_size,
            updated_allocated_size,
        )?;
        let _ = self_inode_state_guard
            .replace_dir_entry_stream(updated_cluster_map_generation.stream_extension());
        self_inode_state_guard.set_cached_cluster_map(updated_cluster_map_generation);
        let _ = self_inode_state_guard.replace_page_cache_context(page_cache_context);
        self_inode_state_guard.with_metadata_mut(|metadata| {
            metadata.size = updated_allocated_size;
        });
        if let Some(page_cache) = self
            .page_cache
            .get()
            .and_then(|page_cache| page_cache.as_ref())
        {
            page_cache.resize(updated_allocated_size, previous_size)?;
        }
        Ok(())
    }
}
