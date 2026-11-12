// SPDX-License-Identifier: MPL-2.0

//! The 96-byte superblock every reader of a mount takes its numbers from.
//!
//! The superblock is the only record a mount reads before it trusts anything else.
//! It states the magic word, the version, the block size and its logarithm,
//! the position and length of every table, and the reference to the root inode.
//! Those fields constrain each other:
//! the block size is a power of two that its own logarithm states,
//! no block may be smaller than a page, the tables must not overlap,
//! and the image may not claim more bytes than the device holds.
//!
//! [`SuperBlock`] is both that record and the geometry that results from reading it.
//! Decoding is the only place that validates the record,
//! so a value of the type is a record every later reader may rely on without
//! re-checking it.
//! The one relation the record cannot state about itself,
//! that the image claims no more bytes than the device holds,
//! is checked when the image is opened.

#![expect(unused_variables, reason = "every body here is still `todo!()`")]
#![short_vis_path::add(squashfs)]

use super::inode::InodeRef;

/// Users: Image, MetadataPosition, StoredBlock, MetaEntry, IdTable.
pub(in squashfs) type ImageOffset = u64;

/// Produced by: SuperBlock::decode. Users: Image, IdTable::load, InodeRecord::read.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod)]
pub(in squashfs) struct SuperBlock {
    /// Owner: SuperBlock. Users: decode.
    magic: u32,
    /// Owner: SuperBlock. Users: inodes.
    inodes: u32,
    /// Owner: SuperBlock. Users: none.
    mkfs_time: u32,
    /// Owner: SuperBlock. Users: block_size.
    block_size: u32,
    /// Owner: SuperBlock. Users: fragments.
    fragments: u32,
    /// Owner: SuperBlock. Users: compression_id.
    compression: u16,
    /// Owner: SuperBlock. Users: SuperBlock::decode.
    block_log: u16,
    /// Owner: SuperBlock. Users: none.
    flags: u16,
    /// Owner: SuperBlock. Users: no_ids.
    no_ids: u16,
    /// Owner: SuperBlock. Users: decode.
    major: u16,
    /// Owner: SuperBlock. Users: decode.
    minor: u16,
    /// Owner: SuperBlock. Users: root_inode.
    root_inode: InodeRef,
    /// Owner: SuperBlock. Users: bytes_used.
    bytes_used: u64,
    /// Owner: SuperBlock. Users: id_table_start.
    id_table_start: ImageOffset,
    /// Owner: SuperBlock. Users: xattr_id_table_start.
    xattr_id_table_start: ImageOffset,
    /// Owner: SuperBlock. Users: inode_table_start.
    inode_table_start: ImageOffset,
    /// Owner: SuperBlock. Users: directory_table_start.
    directory_table_start: ImageOffset,
    /// Owner: SuperBlock. Users: fragment_table_start.
    fragment_table_start: ImageOffset,
    /// Owner: SuperBlock. Users: lookup_table_start.
    lookup_table_start: ImageOffset,
}

impl SuperBlock {
    /// Users: read::image::Image::open.
    pub(in squashfs) const SIZE: usize = 96;

    /// Users: SuperBlock::decode.
    const MAGIC: u32 = 0x7371_7368;

    /// Caller: read::image::Image::open. Callee: InodeRef::decode.
    pub(in squashfs) fn decode(bytes: &[u8]) -> Option<Self> {
        todo!()
    }

    /// Caller: read::dir::DirCursor::next_entry. Callee: none.
    pub(in squashfs) fn inodes(&self) -> u32 {
        todo!()
    }

    /// Caller: fs::Squashfs::open, BlockListCursor::new. Callee: none.
    pub(in squashfs) fn block_size(&self) -> usize {
        todo!()
    }

    /// Caller: read::image::Image::read_exact_at. Callee: none.
    pub(in squashfs) fn bytes_used(&self) -> u64 {
        todo!()
    }

    /// Caller: fs::Squashfs::open. Callee: none.
    pub(in squashfs) fn inode_table_start(&self) -> ImageOffset {
        todo!()
    }

    /// Caller: read::dir::DirCursor::new. Callee: none.
    pub(in squashfs) fn directory_table_start(&self) -> ImageOffset {
        todo!()
    }

    /// Caller: none. Callee: none.
    pub(in squashfs) fn fragment_table_start(&self) -> ImageOffset {
        todo!()
    }

    /// Caller: read::id::IdTable::load. Callee: none.
    pub(in squashfs) fn id_table_start(&self) -> ImageOffset {
        todo!()
    }

    /// Caller: none. Callee: none.
    pub(in squashfs) fn xattr_id_table_start(&self) -> ImageOffset {
        todo!()
    }

    /// Caller: none. Callee: none.
    pub(in squashfs) fn lookup_table_start(&self) -> ImageOffset {
        todo!()
    }

    /// Caller: none. Callee: none.
    pub(in squashfs) fn fragments(&self) -> u32 {
        todo!()
    }

    /// Caller: read::id::IdTable::load, InodeRecord::read. Callee: none.
    pub(in squashfs) fn no_ids(&self) -> u16 {
        todo!()
    }

    /// Caller: fs::Squashfs::open. Callee: none.
    pub(in squashfs) fn compression_id(&self) -> u16 {
        todo!()
    }

    /// Caller: fs::Squashfs::open. Callee: none.
    pub(in squashfs) fn root_inode(&self) -> &InodeRef {
        todo!()
    }
}
