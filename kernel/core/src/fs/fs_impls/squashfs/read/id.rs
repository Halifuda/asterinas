// SPDX-License-Identifier: MPL-2.0

//! The id table: the owners and groups an image names.
//!
//! An inode record does not state a numerical owner; it states an index into the image's id table.
//! The table is two levels deep.
//! An index table of 8-byte entries names, in increasing order,
//! the image positions of the metadata blocks that hold the values;
//! each block then holds 4-byte values back to back.
//! A uid and a gid share one table and one index space,
//! so the same number is stored once however many records cite it.
//!
//! The index table is read once at mount and kept,
//! because every inode record read afterwards depends on it;
//! the blocks it points at are read through the metadata cache on demand.

#![expect(unused_variables, reason = "every body here is still `todo!()`")]
#![short_vis_path::add(squashfs)]

use alloc::vec::Vec;

use super::{
    super::format::superblock::ImageOffset, block::Image, cache::MetadataCache,
    decompress::Decompressor,
};
use crate::prelude::*;

/// The width of one id value, in bytes.
const ID_VALUE_SIZE: usize = 4;

/// The width of one entry of the id index table, in bytes.
const ID_INDEX_ENTRY_SIZE: usize = 8;

/// The mount's id table: the index of the value blocks plus what it was loaded for.
///
/// The index is the table's only mutable-looking part, and it is fixed at mount:
/// the entries strictly increase, each names a metadata block that starts before
/// the id index table, and consecutive entries are close enough that no block
/// between them could hold a full payload. Those are the facts that make an index
/// safe to resolve; a table that breaks any of them describes no readable table
/// and fails the mount.
pub(in squashfs) struct IdTable {
    index: Vec<ImageOffset>,
    count: u16,
}

impl IdTable {
    /// Loads the id index table of an image.
    ///
    /// Fails with an invalid argument when the index table's length disagrees
    /// with the id count, when its entries do not increase, or when they are
    /// spaced further apart than one metadata block; fails with an I/O error when
    /// the table's bytes cannot be read.
    ///
    /// The count and the table's position come from the image's own superblock, so a
    /// caller cannot hand this table a count that belongs to another image.
    pub(in squashfs) fn load(image: &Image) -> Result<Self> {
        todo!()
    }

    /// Returns the id value at an index of the table.
    ///
    /// Fails with an invalid argument for an index the image does not have and
    /// with an I/O error when the value's block cannot be read. The image is
    /// borrowed for this call rather than kept: the cache that resolves the
    /// block holds no image of its own.
    pub(in squashfs) fn value(
        &self,
        index: u16,
        image: &Image,
        cache: &MetadataCache,
        decompressor: &Decompressor,
    ) -> Result<u32> {
        todo!()
    }
}
