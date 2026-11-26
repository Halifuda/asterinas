// SPDX-License-Identifier: MPL-2.0

//! The xattr id table: where each inode's group of extended attributes begins.
//!
//! An inode names its extended attributes by an index into this table,
//! not by a position in the image.
//! The table is two levels deep.
//! A 16-byte header at the table's image position states the xattr data area's
//! position and the number of id entries;
//! then an array of 8-byte absolute image positions names the metadata blocks that
//! hold the id entries back to back, 512 to a block and never across a block boundary.
//! The array occupies exactly the image bytes between the header and the image's
//! stored length, and the blocks it names end before the table's own position.
//! A superblock position of `NO_TABLE_START` means the image has no such table,
//! so a mount keeps none and every inode's xattr index is ignored.
//!
//! One id entry is 16 bytes: an xattr reference, the number of attribute entries in
//! its group, and the group's stored byte count.
//! The reference counts from the xattr data area's position,
//! and this module never adds that position to it:
//! the reader turns it into a `MetadataRef` through `MetadataRef::from_xattr_ref`.
//!
//! The index array is read once and kept, because every later lookup depends on it.
//! The blocks it points at are read through the metadata cache on demand,
//! and the id entries inside them name attribute records that belong to the xattr reader.
//! A mount will build the table, and an inode's xattr list and get paths will ask
//! it for that inode's group; neither path exists yet.
//!
//! The table belongs to the mount (`fs::Squashfs`) and lives as long as it does;
//! this module holds the index and the two header facts, and no image of its own.

#![expect(unused_variables, reason = "every body here is still `todo!()`")]
#![short_vis_path::add(squashfs)]

use alloc::vec::Vec;

use super::{
    super::format::{block::METADATA_BLOCK_SIZE, superblock::ImageOffset, xattr::XattrIdEntry},
    decompress::Decompressor,
    image::Image,
    metadata::MetadataRef,
    metadata_cache::MetadataCache,
};
use crate::prelude::*;

/// Users: XattrIdTable::load.
const XATTR_INDEX_ENTRY_SIZE: usize = 8;

/// Users: XattrIdTable::load, lookup.
const XATTR_ID_ENTRIES_PER_BLOCK: usize = METADATA_BLOCK_SIZE / XattrIdEntry::SIZE;

/// Owner: fs::Squashfs. Users: none.
pub(in squashfs) struct XattrIdTable {
    /// Owner: XattrIdTable. Users: lookup.
    index: Vec<ImageOffset>,
    /// Owner: XattrIdTable. Users: lookup.
    xattr_table_start: ImageOffset,
    /// Owner: XattrIdTable. Users: lookup.
    xattr_ids: u32,
}

impl XattrIdTable {
    /// Caller: none. Callee: Image::read_exact_at, XattrIdTableHeader::decode.
    pub(in squashfs) fn load(image: &Image) -> Result<Option<Self>> {
        todo!()
    }

    /// Caller: none. Callee: MetadataRef::new, MetadataReader::new, read_exact,
    /// XattrIdEntry::decode, MetadataRef::from_xattr_ref.
    pub(in squashfs) fn lookup(
        &self,
        xattr_index: u32,
        image: &Image,
        cache: &MetadataCache,
        decompressor: &Decompressor,
    ) -> Result<XattrGroup> {
        todo!()
    }
}

/// Produced by: XattrIdTable::lookup. Users: none.
#[derive(Clone, Copy, Debug)]
pub(in squashfs) struct XattrGroup {
    /// Owner: XattrGroup. Users: entry_list_position.
    entry_list_position: MetadataRef,
    /// Owner: XattrGroup. Users: entry_count.
    entry_count: u32,
    /// Owner: XattrGroup. Users: stored_xattr_len.
    stored_xattr_len: u32,
}

impl XattrGroup {
    /// Caller: none. Callee: none.
    pub(in squashfs) fn entry_list_position(&self) -> MetadataRef {
        todo!()
    }

    /// Caller: none. Callee: none.
    pub(in squashfs) fn entry_count(&self) -> u32 {
        todo!()
    }

    /// Caller: none. Callee: none.
    pub(in squashfs) fn stored_xattr_len(&self) -> u32 {
        todo!()
    }
}
