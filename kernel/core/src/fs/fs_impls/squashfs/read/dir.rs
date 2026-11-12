// SPDX-License-Identifier: MPL-2.0

//! Walking a directory's content into entries.
//!
//! A directory's content is a byte range of the directory table,
//! opened with a run header and holding entries back to back.
//! The range's length is the inode's record of it,
//! and the walk ends there rather than at a terminator,
//! so a truncated image shows up as a walk that cannot finish a record.
//!
//! Two numbering schemes meet here.
//! A directory's content counts the three bytes that `.` and `..` would occupy,
//! so the position a listing reports for an entry is the bytes consumed from the image plus three,
//! and an offset of zero or one is not a byte of any entry.
//! This module keeps the walk in the reported numbering,
//! because that is what a caller hands back to resume it.

#![expect(unused_variables, reason = "every body here is still `todo!()`")]
#![short_vis_path::add(squashfs)]

use alloc::string::String;

use super::{
    super::format::inode::{DirInode, InodeKind, InodeRef},
    decompress::Decompressor,
    image::Image,
    metadata::MetadataCursor,
    metadata_cache::MetadataCache,
};
use crate::prelude::*;

/// Produced by: DirCursor::next_entry. Users: SquashfsInode::lookup_impl, readdir_impl.
pub(in squashfs) struct DirEntry {
    inode_ref: InodeRef,
    inode_number: u64,
    kind: InodeKind,
    name: String,
}

impl DirEntry {
    /// Caller: SquashfsInode::lookup_impl. Callee: none.
    pub(in squashfs) fn inode_ref(&self) -> InodeRef {
        todo!()
    }

    /// Caller: SquashfsInode::lookup_impl, readdir_impl. Callee: none.
    pub(in squashfs) fn inode_number(&self) -> u64 {
        todo!()
    }

    /// Caller: SquashfsInode::lookup_impl, readdir_impl. Callee: none.
    pub(in squashfs) fn kind(&self) -> InodeKind {
        todo!()
    }

    /// Caller: SquashfsInode::lookup_impl, readdir_impl. Callee: none.
    pub(in squashfs) fn name(&self) -> &str {
        todo!()
    }
}

/// Produced by: DirCursor::new. Users: SquashfsInode::lookup_impl, readdir_impl.
pub(in squashfs) struct DirCursor<'a> {
    cursor: MetadataCursor<'a>,
    remaining: usize,
    consumed: usize,
}

impl<'a> DirCursor<'a> {
    /// Caller: SquashfsInode::lookup_impl, readdir_impl. Callee: DirHeader::decode.
    pub(in squashfs) fn new(
        image: &'a Image,
        cache: &'a MetadataCache,
        decompressor: &'a Decompressor,
        inode: &DirInode,
        start: usize,
    ) -> Self {
        todo!()
    }

    /// Caller: SquashfsInode::readdir_impl. Callee: MetadataCursor::position.
    pub(in squashfs) fn position(&self) -> usize {
        todo!()
    }

    /// Caller: SquashfsInode::lookup_impl, readdir_impl. Callee: DirHeader::decode,
    /// DirEntryPrefix::decode, MetadataCursor::read_exact, InodeKind::from_dir_entry_type.
    pub(in squashfs) fn next_entry(&mut self) -> Result<Option<DirEntry>> {
        todo!()
    }
}
