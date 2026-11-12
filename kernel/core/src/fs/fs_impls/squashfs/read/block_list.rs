// SPDX-License-Identifier: MPL-2.0

//! A regular file's block list and the data blocks it names.
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
    data_cache::{BlockHold, DataCache},
    decompress::Decompressor,
    image::Image,
    meta_index::{BlockListStart, MetaIndexPosition},
    metadata::MetadataCursor,
    metadata_cache::MetadataCache,
};
use crate::prelude::*;

/// Produced by: BlockListCursor::next_block. Users: inode::read_page_async.
pub(in squashfs) struct StoredBlock {
    image_position: ImageOffset,
    stored_len: usize,
    expected_len: usize,
    is_uncompressed: bool,
}

impl StoredBlock {
    /// Caller: StoredBlock::load. Callee: none.
    pub(in squashfs) fn image_position(&self) -> ImageOffset {
        todo!()
    }

    /// Caller: StoredBlock::load. Callee: none.
    pub(in squashfs) fn stored_len(&self) -> usize {
        todo!()
    }

    /// Caller: StoredBlock::load. Callee: none.
    pub(in squashfs) fn expected_len(&self) -> usize {
        todo!()
    }

    /// Caller: StoredBlock::load. Callee: none.
    pub(in squashfs) fn is_uncompressed(&self) -> bool {
        todo!()
    }

    /// Caller: inode::read_page_async. Callee: DataCache::fetch.
    pub(in squashfs) fn load<'c>(
        &self,
        cache: &'c DataCache,
        image: &Image,
        decompressor: &Decompressor,
    ) -> Result<BlockHold<'c>> {
        todo!()
    }
}

/// Produced by: BlockListCursor::new. Users: inode::read_page_async.
pub(in squashfs) struct BlockListCursor<'a> {
    cursor: MetadataCursor<'a>,
    image_position: ImageOffset,
    blocks_left: usize,
    block_size: usize,
    last_expected_len: usize,
}

impl<'a> BlockListCursor<'a> {
    /// Caller: inode::read_page_async. Callee: RegInode::data_block_count,
    /// last_block_expected_len.
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

    /// Caller: inode::read_page_async. Callee: BlockListEntry::decode.
    pub(in squashfs) fn next_block(&mut self) -> Result<Option<StoredBlock>> {
        // Advance the running position by the stored length; the last entry uses
        // the record's derived length, every earlier one the block size.
        todo!()
    }
}
