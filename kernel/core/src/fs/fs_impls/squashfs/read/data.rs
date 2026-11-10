// SPDX-License-Identifier: MPL-2.0

//! The data blocks of a regular file.
//!
//! A regular file's data is not described by a table of extents;
//! it is described by a list of per-block lengths that follows the inode record.
//! The list holds one 4-byte entry per block,
//! its length follows from the file size and the mount's block size,
//! and it can span more than one metadata block,
//! so the list is read forward from a position rather than indexed.
//!
//! The first data block sits at the record's stated image position;
//! each later block sits right after the previous one.
//! A block of length zero occupies no image bytes and reads as zeros.
//!
//! Reading one block is also where the image's two halves meet:
//! the entry says how many bytes to fetch,
//! and the block size says how many bytes are expected back.
//! Only a file's last data block may be expected to decode to less than the block
//! size; every block must decode to exactly its expected length.
//!
//! A reader never takes a block's bytes from the image directly:
//! [`StoredBlock::load`] takes the mount's hold on the block's decoded bytes from
//! the data cache, which reads and decodes a key once while its entry stays
//! resident.
//! A sparse block has no stored bytes, so it never reaches the cache and its pages
//! are zeroed instead.

#![expect(unused_variables, reason = "every body here is still `todo!()`")]
#![short_vis_path::add(squashfs)]

use super::{
    super::format::{inode::RegInode, superblock::ImageOffset},
    block::Image,
    cache::MetadataCache,
    data_cache::{BlockHold, DataCache},
    decompress::Decompressor,
    meta_index::{BlockListStart, MetaIndexPosition},
    metadata::MetadataCursor,
};
use crate::prelude::*;

/// One data block of a file, located but not yet read.
///
/// The three lengths and the storage flag are the whole contract between the walk
/// and a reader: where the block's stored bytes start, how many of them there are,
/// how many bytes the block must decode to, and whether those bytes are stored
/// as-is. Keeping the expected length alongside the stored one is what lets a
/// reader notice a block that decoded short instead of handing a partially filled
/// page onwards; the flag is what tells the cache whether a block needs the codec
/// at all.
///
/// A block is never read from the image by a reader of its own: taking its bytes
/// goes through the mount's data cache, which reads and decodes the block at most
/// once for every page that asks for it.
/// A block whose stored length is zero occupies no image bytes, reads as zeros,
/// and never enters the cache.
/// A fragment block, which carries the tails of several files, comes through the
/// same cache under the same key space with the mount's block size as its expected
/// length; only a reader's own tail must fit inside its decode, so it may decode to
/// less than that length, while a data block must decode to exactly its expected
/// length.
pub(in squashfs) struct StoredBlock {
    image_position: ImageOffset,
    stored_len: usize,
    expected_len: usize,
    is_uncompressed: bool,
}

impl StoredBlock {
    /// Returns the image position the block's stored bytes start at.
    pub(in squashfs) fn image_position(&self) -> ImageOffset {
        todo!()
    }

    /// Returns how many bytes the block occupies on the image.
    pub(in squashfs) fn stored_len(&self) -> usize {
        todo!()
    }

    /// Returns how many bytes the block must decode to.
    pub(in squashfs) fn expected_len(&self) -> usize {
        todo!()
    }

    /// Reports whether the block is stored as-is instead of compressed.
    pub(in squashfs) fn is_uncompressed(&self) -> bool {
        todo!()
    }

    /// Takes the mount's hold on the block's decoded bytes.
    ///
    /// A hit returns the block already published; a miss reads the stored bytes
    /// from `image`, decodes them, and publishes the result, so every later caller
    /// of the same key takes a hold on the same bytes.
    /// A failed read or decode publishes nothing, so the next caller retries rather
    /// than inheriting an entry that names no usable block.
    ///
    /// The held bytes are at most `expected_len`, because the cache only bounds a
    /// block by the length its key states. A data block must decode to exactly that
    /// length, and the page fill checks it on the held bytes; a shorter block is a
    /// malformed image rather than a hit that returned fewer bytes.
    ///
    /// A sparse block has no stored bytes and never reaches the cache; the page fill
    /// zeroes the pages it covers instead, so the cache states the premise as an
    /// assertion rather than as an error a caller could reach.
    pub(in squashfs) fn load<'c>(
        &self,
        cache: &'c DataCache,
        image: &Image,
        decompressor: &Decompressor,
    ) -> Result<BlockHold<'c>> {
        todo!()
    }
}

/// A walk over one regular file's block list, tracking where each block sits.
///
/// A walk starts either at the point the mount's index recorded for the block a
/// caller wants, or at the list's first entry when the index gave no point.
/// The page fill resolves that point before the walk starts, so the walk performs
/// no lookup, records no point, and changes no slot; filling a slot belongs to the
/// index itself.
///
/// A point names a block at most one step before the wanted one, so the entries the
/// walk reads before it stay inside metadata blocks the cache already holds.
/// A walk without a point reads in order from the record's own end.
///
/// A file whose tail lives in a fragment block stores one block fewer than its
/// size suggests; the walk starts from the count the record's fields state, so a
/// file that disagrees with itself is a malformed record rather than a short read.
/// Handing the walk such a file is the page fill's to avoid: this build's page fill
/// serves only files whose tail is in the data area and asserts that premise, so the
/// walk never sees a list that stops before the block a page asks for.
///
/// The block size the walk measures every entry against is the one the image
/// states, taken from the image the walk reads, so no caller can pair the walk
/// with another image's block size.
pub(in squashfs) struct BlockListCursor<'a> {
    cursor: MetadataCursor<'a>,
    image_position: ImageOffset,
    blocks_left: usize,
    block_size: usize,
    last_expected_len: usize,
}

impl<'a> BlockListCursor<'a> {
    /// Starts a walk at the block list of a regular file of `image`.
    ///
    /// Uses `start` as the walk's first entry and first image position when the
    /// mount's index has a point for the wanted block, so the entries before it are
    /// never read.
    /// Without a point, the walk starts at the list's first entry, which is the
    /// position right after the inode record, and at the record's first data
    /// block; a file the index does not cover is therefore read in order from the
    /// record's end.
    ///
    /// `block_list` states how many entries the list holds, leaving out a tail kept
    /// in a fragment block; `inode` states the file's length and whether its tail is
    /// in a fragment, which decide the last entry's expected length.
    /// That length is derived here once, from the record, so no step of the walk
    /// works it out again from the file's size.
    pub(in squashfs) fn new(
        image: &'a Image,
        cache: &'a MetadataCache,
        decompressor: &'a Decompressor,
        block_list: &BlockListStart,
        inode: &RegInode,
        start: Option<MetaIndexPosition>,
    ) -> Self {
        todo!()
    }

    /// Returns the block at the walk's position and advances to the next one.
    ///
    /// Only the list's last entry can be expected to decode to less than the block
    /// size, and only when the file's tail stays in the data area; every earlier
    /// entry is expected to decode to a full block, and a file whose size is a whole
    /// number of blocks states a full block for its last entry as well.
    /// A sparse entry stores no bytes; the block returned has a stored length of
    /// zero, and its reader zeroes the pages it covers instead of loading it.
    ///
    /// Fails with an I/O error when an entry's reserved bits are set or an entry
    /// claims more bytes than the block's expected length.
    pub(in squashfs) fn next_block(&mut self) -> Result<Option<StoredBlock>> {
        // Advance the running position by the stored length; the last entry uses
        // the record's derived length, every earlier one the block size.
        todo!()
    }
}
