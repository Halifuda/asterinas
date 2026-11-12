// SPDX-License-Identifier: MPL-2.0

//! Positions in, and reads across, a metadata stream.
//!
//! The inode table, the directory table, and the index tables are separate streams,
//! each a chain of metadata blocks laid end to end.
//! A position in a stream is the pair the format uses everywhere:
//! the image position of a block plus a byte offset inside its decompressed payload.
//! Because blocks differ in stored length,
//! a byte offset in the stream cannot be turned into a position by arithmetic;
//! a reader walks block by block, and this module owns that walk.

#![expect(unused_variables, reason = "every body here is still `todo!()`")]
#![short_vis_path::add(squashfs)]

use super::{
    super::format::superblock::ImageOffset, decompress::Decompressor, image::Image,
    metadata_cache::MetadataCache,
};
use crate::prelude::*;

/// Produced by: MetadataPosition::new. Users: MetadataCursor, BlockListStart.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in squashfs) struct MetadataPosition {
    block_start: ImageOffset,
    offset: usize,
}

impl MetadataPosition {
    /// Caller: InodeRecord::read, DirCursor::new, MetaEntry::position. Callee: none.
    pub(in squashfs) fn new(block_start: ImageOffset, offset: usize) -> Option<Self> {
        todo!()
    }

    /// Caller: MetadataCursor::read_exact, MetaEntry::record. Callee: none.
    pub(in squashfs) fn block_start(&self) -> ImageOffset {
        todo!()
    }

    /// Caller: MetadataCursor::read_exact, MetaEntry::record. Callee: none.
    pub(in squashfs) fn offset(&self) -> usize {
        todo!()
    }
}

/// Produced by: MetadataCursor::new. Users: InodeRecord::read, DirCursor, BlockListCursor.
pub(in squashfs) struct MetadataCursor<'a> {
    image: &'a Image,
    cache: &'a MetadataCache,
    decompressor: &'a Decompressor,
    position: MetadataPosition,
}

impl<'a> MetadataCursor<'a> {
    /// Caller: InodeRecord::read, DirCursor::new, BlockListCursor::new. Callee: none.
    pub(in squashfs) fn new(
        image: &'a Image,
        cache: &'a MetadataCache,
        decompressor: &'a Decompressor,
        position: MetadataPosition,
    ) -> Self {
        todo!()
    }

    /// Caller: DirCursor::position, BlockListCursor::new. Callee: none.
    pub(super) fn position(&self) -> MetadataPosition {
        todo!()
    }

    /// Caller: InodeRecord::read, DirCursor::next_entry, MetaIndexClaim::fill.
    /// Callee: MetadataCache::fetch.
    pub(super) fn read_exact(&mut self, out: &mut [u8]) -> Result<()> {
        // A block that runs out continues at the next stored block; there is no padding.
        todo!()
    }
}
