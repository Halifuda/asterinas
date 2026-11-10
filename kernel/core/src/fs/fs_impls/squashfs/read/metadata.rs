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
    super::format::superblock::ImageOffset, block::Image, cache::MetadataCache,
    decompress::Decompressor,
};
use crate::prelude::*;

/// A position in one metadata stream.
///
/// The in-block offset addresses bytes of a decompressed block, so a value that
/// reaches the block's size names nothing: no metadata block is that long. That
/// is the one thing a position can be wrong about on its own, and it is what
/// [`MetadataPosition::new`] refuses.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in squashfs) struct MetadataPosition {
    block_start: ImageOffset,
    offset: usize,
}

impl MetadataPosition {
    /// Builds a position, or `None` when the in-block offset leaves any block.
    pub(in squashfs) fn new(block_start: ImageOffset, offset: usize) -> Option<Self> {
        todo!()
    }

    /// Returns the image position of the block this position is inside.
    pub(in squashfs) fn block_start(&self) -> ImageOffset {
        todo!()
    }

    /// Returns the byte offset inside the block's decompressed payload.
    pub(in squashfs) fn offset(&self) -> usize {
        todo!()
    }
}

/// A cursor that reads bytes from a metadata stream, crossing block boundaries
/// as needed.
///
/// A cursor always holds a position the format can resolve, so a read either
/// returns the bytes asked for or fails because the stream ended early or a block
/// on the way could not be read. Both of those mean the image is malformed, which
/// is why they surface as I/O errors rather than as a short read.
///
/// The cursor borrows the image it reads: the metadata cache keeps no image, so
/// every block a cursor needs is fetched from the image the cursor was built on.
pub(in squashfs) struct MetadataCursor<'a> {
    image: &'a Image,
    cache: &'a MetadataCache,
    decompressor: &'a Decompressor,
    position: MetadataPosition,
}

impl<'a> MetadataCursor<'a> {
    /// Starts a cursor at a position of a stream read from `image`.
    pub(in squashfs) fn new(
        image: &'a Image,
        cache: &'a MetadataCache,
        decompressor: &'a Decompressor,
        position: MetadataPosition,
    ) -> Self {
        todo!()
    }

    /// Returns the position the next read starts at.
    pub(super) fn position(&self) -> MetadataPosition {
        todo!()
    }

    /// Fills `out` and advances past its bytes.
    ///
    /// A record is a fixed number of bytes for all but its variable tail, so the
    /// caller sizes `out` and this needs no growth.
    pub(super) fn read_exact(&mut self, out: &mut [u8]) -> Result<()> {
        // A block that runs out continues at the next stored block; there is no padding.
        todo!()
    }
}
