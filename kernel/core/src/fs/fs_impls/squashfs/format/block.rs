// SPDX-License-Identifier: MPL-2.0

//! The length-and-compression word in front of every stored block.
//!
//! Squashfs frames both metadata blocks and file data blocks the same way:
//! a length word whose top meaningful bit says whether the payload is stored compressed.
//! Metadata blocks use a 16-bit word and a fixed decompressed size;
//! a file's block list uses a 32-bit word and a size the reader derives per block.
//! This module decodes those words and nothing else:
//! the payload belongs to the layer that fetches it.
//!
//! A decoded value holds the two facts the word names, already separated,
//! so no reader masks a length or tests a flag for itself.
//! The `SIZE` of each type is still the width the record occupies on the image.

#![expect(unused_variables, reason = "every body here is still `todo!()`")]
#![short_vis_path::add(squashfs)]

/// Users: MetadataCache::fetch, MetaIndex::fill, MetadataPosition::new.
pub(in squashfs) const METADATA_BLOCK_SIZE: usize = 8192;

/// Users: MetadataBlockHeader::decode.
const METADATA_UNCOMPRESSED_BIT: u16 = 1 << 15;

/// Users: MetadataBlockHeader::decode.
const METADATA_STORED_LEN_MASK: u16 = !METADATA_UNCOMPRESSED_BIT;

/// Produced by: MetadataBlockHeader::decode. Users: MetadataCache::fetch, MetaIndex::fill.
#[derive(Clone, Copy, Debug)]
pub(in squashfs) struct MetadataBlockHeader {
    /// Owner: MetadataBlockHeader. Users: stored_len.
    stored_len: usize,
    /// Owner: MetadataBlockHeader. Users: is_uncompressed.
    is_uncompressed: bool,
}

impl MetadataBlockHeader {
    /// Users: MetadataCache::fetch, MetaIndex::fill.
    pub(in squashfs) const SIZE: usize = 2;

    /// Caller: MetadataCache::fetch, MetaIndex::fill. Callee: none.
    pub(in squashfs) fn decode(bytes: &[u8]) -> Option<Self> {
        todo!()
    }

    /// Caller: MetadataCache::fetch. Callee: none.
    pub(in squashfs) fn stored_len(&self) -> usize {
        todo!()
    }

    /// Caller: MetadataCache::fetch. Callee: none.
    pub(in squashfs) fn is_uncompressed(&self) -> bool {
        todo!()
    }
}

/// Produced by: BlockListEntry::decode. Users: BlockListCursor::next_block,
/// MetaIndex::fill.
#[derive(Clone, Copy, Debug)]
pub(in squashfs) struct BlockListEntry {
    /// Owner: BlockListEntry. Users: stored_len.
    stored_len: usize,
    /// Owner: BlockListEntry. Users: is_uncompressed.
    is_uncompressed: bool,
}

impl BlockListEntry {
    /// Users: MetaIndex::fill.
    pub(in squashfs) const SIZE: usize = 4;

    /// Users: BlockListEntry::decode.
    const STORED_LEN_BITS: u32 = 24;

    /// Users: BlockListEntry::decode.
    const UNCOMPRESSED_BIT: u32 = 1 << Self::STORED_LEN_BITS;

    /// Caller: BlockListCursor::next_block, MetaIndex::fill. Callee: none.
    pub(in squashfs) fn decode(bytes: &[u8]) -> Option<Self> {
        todo!()
    }

    /// Caller: BlockListCursor::next_block. Callee: none.
    pub(in squashfs) fn stored_len(&self) -> usize {
        todo!()
    }

    /// Caller: BlockListCursor::next_block. Callee: none.
    pub(in squashfs) fn is_uncompressed(&self) -> bool {
        todo!()
    }
}
