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
    super::format::superblock::ImageOffset, decompress::Decompressor, image::Image,
    metadata_cache::MetadataCache,
};
use crate::prelude::*;

/// Users: IdTable::value.
const ID_VALUE_SIZE: usize = 4;

/// Users: IdTable::load.
const ID_INDEX_ENTRY_SIZE: usize = 8;

/// Owner: fs::Squashfs. Users: SquashfsInode::owner_impl, group_impl, metadata_impl.
pub(in squashfs) struct IdTable {
    index: Vec<ImageOffset>,
    count: u16,
}

impl IdTable {
    /// Caller: fs::Squashfs::open. Callee: Image::read_exact_at.
    pub(in squashfs) fn load(image: &Image) -> Result<Self> {
        todo!()
    }

    /// Caller: SquashfsInode::owner_impl, group_impl, metadata_impl.
    /// Callee: MetadataCache::fetch.
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
