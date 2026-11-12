// SPDX-License-Identifier: MPL-2.0

//! An opened image and the one bound every read of it obeys.
//!
//! A Squashfs image is a byte range of a block device,
//! and its superblock names that range's length.
//! Every later read of the image is bounded by that number rather than by the device's size,
//! because a device may hold more than one image,
//! and bytes past the image belong to whatever follows it.
//!
//! This module is also where a mount starts.

#![expect(unused_variables, reason = "every body here is still `todo!()`")]
#![short_vis_path::add(squashfs)]

use alloc::sync::Arc;

use aster_block::BlockDevice;

use super::super::format::superblock::{ImageOffset, SuperBlock};
use crate::prelude::*;

/// Produced by: Image::open. Users: fs::Squashfs.
pub(in squashfs) struct Image {
    device: Arc<dyn BlockDevice>,
    super_block: SuperBlock,
}

impl Image {
    /// Caller: fs::Squashfs::open. Callee: SuperBlock::decode, Image::read_exact_at.
    pub(in squashfs) fn open(device: Arc<dyn BlockDevice>) -> Result<Self> {
        // Read the record at position zero first; it states the bound every read obeys.
        todo!()
    }

    /// Caller: DirCursor::new, BlockListCursor::new. Callee: none.
    pub(in squashfs) fn super_block(&self) -> &SuperBlock {
        todo!()
    }

    /// Caller: Image::open, MetadataCache::fetch, DataCache::fetch, IdTable::load. Callee: none.
    pub(super) fn read_exact_at(&self, position: ImageOffset, out: &mut [u8]) -> Result<()> {
        todo!()
    }
}
