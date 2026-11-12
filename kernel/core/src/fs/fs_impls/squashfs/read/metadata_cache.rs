// SPDX-License-Identifier: MPL-2.0

//! The mount's cache of decompressed metadata blocks.
//!
//! An inode record, a directory entry, and every index table are stored in metadata blocks:
//! a 2-byte length word, then a payload that decompresses to at most 8192 bytes.
//! Blocks are addressed by the image position they start at, and their stored lengths differ,
//! so a position cannot be computed from a block number; it has to be read.
//!
//! The image is read-only, so a block's decompressed payload never changes once it is in hand.
//! That is what makes a cache sound here: the cached bytes are the block,
//! not a snapshot that a writer could invalidate.
//!
//! The cache holds no image of its own.
//! A fetch is lent the image of the mount it is asked for and borrows it for that
//! call, so the opened image exists once per mount and lives with the filesystem
//! rather than with the cache.

#![expect(unused_variables, reason = "every body here is still `todo!()`")]
#![short_vis_path::add(squashfs)]

use alloc::{collections::BTreeMap, sync::Arc, vec::Vec};

use super::{super::format::superblock::ImageOffset, decompress::Decompressor, image::Image};
use crate::prelude::*;

/// Produced by: MetadataCache::fetch. Users: MetadataCursor::read_exact.
pub(super) struct MetadataBlock {
    position: ImageOffset,
    bytes: Vec<u8>,
}

impl MetadataBlock {
    /// Caller: MetadataCursor::read_exact. Callee: none.
    pub(super) fn position(&self) -> ImageOffset {
        todo!()
    }

    /// Caller: MetadataCursor::read_exact. Callee: none.
    pub(super) fn bytes(&self) -> &[u8] {
        todo!()
    }

    /// Caller: MetadataCursor::read_exact. Callee: none.
    pub(super) fn contains_offset(&self, offset: usize) -> bool {
        todo!()
    }
}

/// Owner: fs::Squashfs. Users: MetadataCursor, DirCursor, BlockListCursor.
pub(in squashfs) struct MetadataCache {
    blocks: Mutex<BTreeMap<ImageOffset, Arc<MetadataBlock>>>,
}

impl MetadataCache {
    /// Caller: fs::Squashfs::open. Callee: none.
    pub(in squashfs) fn new() -> Self {
        todo!()
    }

    /// Caller: MetadataCursor::read_exact, IdTable::value. Callee: Decompressor::decompress,
    /// MetadataBlockHeader::decode.
    pub(super) fn fetch(
        &self,
        image: &Image,
        position: ImageOffset,
        decompressor: &Decompressor,
    ) -> Result<Arc<MetadataBlock>> {
        todo!()
    }
}
