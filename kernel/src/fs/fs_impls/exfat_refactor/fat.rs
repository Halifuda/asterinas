// SPDX-License-Identifier: MPL-2.0

use alloc::{collections::BTreeSet, vec, vec::Vec};
use core::mem;

use aster_block::BlockDevice;
use ostd::mm::VmIo;

use super::{
    boot::BootRegion, device_io, invalid_on_disk_layout, invalid_operation_input,
};
use crate::prelude::*;

const FAT_END_OF_CHAIN: u32 = 0xFFFF_FFFF;

#[derive(Clone, Copy)]
pub(super) enum ChainVisitControl {
    Continue,
    Stop,
}

#[derive(Clone, Copy)]
pub(super) enum FatChainStep {
    Continue(u32),
    End,
}

pub(super) struct FatReader<'a> {
    block_device: &'a dyn BlockDevice,
    boot_region: &'a BootRegion,
    cached_sector_index: Option<u64>,
    cached_sector: Vec<u8>,
}

impl<'a> FatReader<'a> {
    pub(super) fn new(block_device: &'a dyn BlockDevice, boot_region: &'a BootRegion) -> Self {
        Self {
            block_device,
            boot_region,
            cached_sector_index: None,
            cached_sector: vec![0; boot_region.sector_size],
        }
    }

    pub(super) fn walk_cluster_chain<F>(
        &mut self,
        start_cluster: u32,
        mut visit_cluster_fn: F,
    ) -> Result<()>
    where
        F: FnMut(u32, &[u8]) -> Result<ChainVisitControl>,
    {
        if !self.boot_region.is_valid_cluster(start_cluster) {
            return Err(invalid_on_disk_layout());
        }
        let mut cluster_buffer = vec![0; self.boot_region.cluster_size];
        let mut current_cluster = start_cluster;
        let mut visited_clusters = BTreeSet::new();
        loop {
            if !visited_clusters.insert(current_cluster) {
                return Err(invalid_on_disk_layout());
            }
            let cluster_offset = self.boot_region.cluster_offset(current_cluster)?;
            Self::read_device_bytes(self.block_device, cluster_offset, &mut cluster_buffer)?;
            if matches!(
                visit_cluster_fn(current_cluster, &cluster_buffer)?,
                ChainVisitControl::Stop
            ) {
                return Ok(());
            }
            current_cluster = match self.next_cluster(current_cluster)? {
                FatChainStep::Continue(next_cluster) => next_cluster,
                FatChainStep::End => return Ok(()),
            };
        }
    }

    pub(super) fn next_cluster(
        &mut self,
        current_cluster: u32,
    ) -> Result<FatChainStep> {
        let entry_offset = u64::from(self.boot_region.fat_offset_sectors)
            .checked_mul(
                u64::try_from(self.boot_region.sector_size)
                    .map_err(|_| invalid_on_disk_layout())?,
            )
            .and_then(|offset| offset.checked_add(u64::from(current_cluster) * 4))
            .ok_or_else(invalid_on_disk_layout)?;
        let sector_size = u64::try_from(self.boot_region.sector_size)
            .map_err(|_| invalid_on_disk_layout())?;
        let sector_index = entry_offset / sector_size;
        if self.cached_sector_index != Some(sector_index) {
            let sector_offset = sector_index
                .checked_mul(sector_size)
                .ok_or_else(invalid_on_disk_layout)?;
            Self::read_device_bytes(
                self.block_device,
                usize::try_from(sector_offset).map_err(|_| invalid_on_disk_layout())?,
                &mut self.cached_sector,
            )?;
            self.cached_sector_index = Some(sector_index);
        }
        let entry_within_sector = usize::try_from(entry_offset % sector_size)
            .map_err(|_| invalid_on_disk_layout())?;
        let entry_end = entry_within_sector
            .checked_add(mem::size_of::<u32>())
            .ok_or_else(invalid_on_disk_layout)?;
        let next_cluster = {
            let entry = self
                .cached_sector
                .get(entry_within_sector..entry_end)
                .ok_or_else(invalid_on_disk_layout)?;
            u32::from_le_bytes([entry[0], entry[1], entry[2], entry[3]])
        };
        if next_cluster == 0xFFFF_FFF7 {
            return Err(invalid_on_disk_layout());
        }
        if next_cluster >= 0xFFFF_FFF8 {
            return Ok(FatChainStep::End);
        }
        if !self.boot_region.is_valid_cluster(next_cluster) {
            return Err(invalid_on_disk_layout());
        }
        Ok(FatChainStep::Continue(next_cluster))
    }

    pub(super) fn append_cluster_to_chain(
        &mut self,
        start_cluster: u32,
        appended_cluster: u32,
    ) -> Result<()> {
        if !self.boot_region.is_valid_cluster(appended_cluster) {
            return Err(invalid_operation_input());
        }

        let mut current_cluster = start_cluster;
        let mut visited_clusters = BTreeSet::new();
        loop {
            if !visited_clusters.insert(current_cluster) {
                return Err(invalid_on_disk_layout());
            }
            match self.next_cluster(current_cluster)? {
                FatChainStep::Continue(next_cluster) => current_cluster = next_cluster,
                FatChainStep::End => {
                    self.write_cluster_entry(appended_cluster, FAT_END_OF_CHAIN)?;
                    self.write_cluster_entry(current_cluster, appended_cluster)?;
                    return Ok(());
                }
            }
        }
    }

    pub(super) fn link_contiguous_chain_to_cluster(
        &mut self,
        start_cluster: u32,
        cluster_count: usize,
        appended_cluster: u32,
    ) -> Result<()> {
        if cluster_count == 0 || !self.boot_region.is_valid_cluster(appended_cluster) {
            return Err(invalid_operation_input());
        }

        self.write_cluster_entry(appended_cluster, FAT_END_OF_CHAIN)?;
        for cluster_offset in (0..cluster_count).rev() {
            let current_cluster = start_cluster
                .checked_add(
                    u32::try_from(cluster_offset)
                        .map_err(|_| invalid_operation_input())?,
                )
                .ok_or_else(invalid_operation_input)?;
            if !self.boot_region.is_valid_cluster(current_cluster) {
                return Err(invalid_operation_input());
            }

            let next_cluster = if cluster_offset + 1 == cluster_count {
                appended_cluster
            } else {
                current_cluster
                    .checked_add(1)
                    .ok_or_else(invalid_operation_input)?
            };
            if next_cluster != appended_cluster && !self.boot_region.is_valid_cluster(next_cluster)
            {
                return Err(invalid_operation_input());
            }
            self.write_cluster_entry(current_cluster, next_cluster)?;
        }
        Ok(())
    }

    pub(super) fn terminate_cluster_chain(
        &mut self,
        cluster: u32,
    ) -> Result<()> {
        self.write_cluster_entry(cluster, FAT_END_OF_CHAIN)
    }

    fn read_device_bytes(
        block_device: &dyn BlockDevice,
        offset: usize,
        buffer: &mut [u8],
    ) -> Result<()> {
        block_device
            .read_bytes(offset, buffer)
            .map_err(|_| device_io())
    }

    fn write_cluster_entry(
        &mut self,
        cluster: u32,
        next_cluster: u32,
    ) -> Result<()> {
        if !self.boot_region.is_valid_cluster(cluster) {
            return Err(invalid_operation_input());
        }

        let entry_offset = u64::from(self.boot_region.fat_offset_sectors)
            .checked_mul(
                u64::try_from(self.boot_region.sector_size)
                    .map_err(|_| invalid_on_disk_layout())?,
            )
            .and_then(|offset| offset.checked_add(u64::from(cluster) * 4))
            .ok_or_else(invalid_on_disk_layout)?;
        let sector_size = u64::try_from(self.boot_region.sector_size)
            .map_err(|_| invalid_on_disk_layout())?;
        let sector_index = entry_offset / sector_size;
        if self.cached_sector_index != Some(sector_index) {
            let sector_offset = sector_index
                .checked_mul(sector_size)
                .ok_or_else(invalid_on_disk_layout)?;
            Self::read_device_bytes(
                self.block_device,
                usize::try_from(sector_offset).map_err(|_| invalid_on_disk_layout())?,
                &mut self.cached_sector,
            )?;
            self.cached_sector_index = Some(sector_index);
        }

        let entry_within_sector = usize::try_from(entry_offset % sector_size)
            .map_err(|_| invalid_on_disk_layout())?;
        let entry_end = entry_within_sector
            .checked_add(mem::size_of::<u32>())
            .ok_or_else(invalid_on_disk_layout)?;
        self.cached_sector
            .get_mut(entry_within_sector..entry_end)
            .ok_or_else(invalid_on_disk_layout)?
            .copy_from_slice(&next_cluster.to_le_bytes());

        let sector_offset = sector_index
            .checked_mul(sector_size)
            .ok_or_else(invalid_on_disk_layout)?;
        self.block_device
            .write_bytes(
                usize::try_from(sector_offset).map_err(|_| invalid_on_disk_layout())?,
                &self.cached_sector,
            )
            .map_err(|_| device_io())
    }
}
