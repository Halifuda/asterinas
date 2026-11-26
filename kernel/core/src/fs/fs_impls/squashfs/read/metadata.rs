// SPDX-License-Identifier: MPL-2.0

//! The metadata ref and the reads that cross a metadata stream.
//!
//! The inode table, the directory table, and the index tables are separate streams,
//! each a chain of metadata blocks laid end to end.
//! A [`MetadataRef`] is the pair the format uses everywhere:
//! the image position of a block plus a byte offset inside its decompressed payload.
//! Because blocks differ in stored length,
//! a byte offset in the stream cannot be turned into a position by arithmetic;
//! a reader walks block by block, and this module owns that walk.
//!
//! Every offset the image states relative to a table is resolved here,
//! against that table's image position: the 32-bit ones through
//! [`MetadataRef::from_table_offset`], the 48-bit inode reference through
//! [`MetadataRef::from_inode_ref`].
//! The 48-bit xattr reference resolves the same way through
//! [`MetadataRef::from_xattr_ref`], against the xattr data area's position.
//! All three add the table's position once and leave the block-offset bound to
//! [`MetadataRef::new`].

#![expect(unused_variables, reason = "every body here is still `todo!()`")]
#![short_vis_path::add(squashfs)]

use super::{
    super::format::{inode::InodeRef, superblock::ImageOffset, xattr::XattrRef},
    decompress::Decompressor,
    image::Image,
    metadata_cache::MetadataCache,
};
use crate::prelude::*;

/// Produced by: MetadataRef::new, from_inode_ref, from_table_offset, from_xattr_ref.
/// Users: MetadataReader, BlockListStart.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in squashfs) struct MetadataRef {
    block_position: ImageOffset,
    in_block_offset: usize,
}

impl MetadataRef {
    /// Caller: MetadataRef::from_relative, XattrIdTable::lookup. Callee: none.
    pub(super) fn new(block_position: ImageOffset, in_block_offset: usize) -> Option<Self> {
        todo!()
    }

    /// Caller: MetadataRef::from_inode_ref, from_table_offset, from_xattr_ref.
    /// Callee: MetadataRef::new.
    fn from_relative(
        relative_offset: u64,
        in_block_offset: u16,
        table_start: ImageOffset,
    ) -> Option<Self> {
        todo!()
    }

    /// Caller: fs::Squashfs::open, SquashfsInode::lookup_impl.
    /// Callee: MetadataRef::from_relative.
    pub(in squashfs) fn from_inode_ref(
        reference: &InodeRef,
        inode_table_start: ImageOffset,
    ) -> Option<Self> {
        Self::from_relative(
            reference.inode_table_offset(),
            reference.in_block_offset(),
            inode_table_start,
        )
    }

    /// Caller: DirCursor::new, MetaEntry::position. Callee: MetadataRef::from_relative.
    pub(super) fn from_table_offset(
        table_relative_offset: u32,
        in_block_offset: u16,
        table_start: ImageOffset,
    ) -> Option<Self> {
        Self::from_relative(
            u64::from(table_relative_offset),
            in_block_offset,
            table_start,
        )
    }

    /// Caller: XattrIdTable::lookup. Callee: MetadataRef::from_relative.
    pub(super) fn from_xattr_ref(
        reference: &XattrRef,
        xattr_table_start: ImageOffset,
    ) -> Option<Self> {
        Self::from_relative(
            reference.xattr_table_offset(),
            reference.in_block_offset(),
            xattr_table_start,
        )
    }

    /// Caller: MetadataReader::read_exact, MetaEntry::record. Callee: none.
    pub(super) fn block_position(&self) -> ImageOffset {
        todo!()
    }

    /// Caller: MetadataReader::read_exact, MetaEntry::record. Callee: none.
    pub(super) fn in_block_offset(&self) -> usize {
        todo!()
    }
}

/// Produced by: MetadataReader::new. Users: InodeRecord::read, DirCursor, BlockListCursor.
pub(in squashfs) struct MetadataReader<'a> {
    image: &'a Image,
    cache: &'a MetadataCache,
    decompressor: &'a Decompressor,
    position: MetadataRef,
}

impl<'a> MetadataReader<'a> {
    /// Caller: InodeRecord::read, DirCursor::new, BlockListCursor::new. Callee: none.
    pub(in squashfs) fn new(
        image: &'a Image,
        cache: &'a MetadataCache,
        decompressor: &'a Decompressor,
        position: MetadataRef,
    ) -> Self {
        todo!()
    }

    /// Caller: DirCursor::position, BlockListCursor::new. Callee: none.
    pub(super) fn position(&self) -> MetadataRef {
        todo!()
    }

    /// Caller: InodeRecord::read, DirCursor::next_entry, MetaIndexClaim::fill.
    /// Callee: MetadataCache::fetch.
    pub(super) fn read_exact(&mut self, out: &mut [u8]) -> Result<()> {
        // A block that runs out continues at the next stored block; there is no padding.
        todo!()
    }
}
