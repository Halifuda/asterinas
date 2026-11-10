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
    block::Image,
    cache::MetadataCache,
    decompress::Decompressor,
    metadata::MetadataCursor,
};
use crate::prelude::*;

/// One directory entry that a walk has finished reading.
///
/// The name is held as a string because that is what a listing hands on; an
/// image whose names are not valid UTF-8 cannot be listed, and the walk reports
/// that as an I/O error rather than inventing a replacement.
///
/// The inode number is the run header's base plus the entry's own signed
/// increment, and it must land in `1..=inodes`: a sum of zero or one past the
/// image's inode count names no inode, so [`DirCursor::next_entry`] reports it as
/// an I/O error instead of handing it on.
pub(in squashfs) struct DirEntry {
    inode_ref: InodeRef,
    inode_number: u64,
    kind: InodeKind,
    name: String,
}

impl DirEntry {
    /// Returns the reference to the entry's inode, relative to the inode table.
    pub(in squashfs) fn inode_ref(&self) -> InodeRef {
        todo!()
    }

    /// Returns the inode number the entry names, already known to be in range.
    pub(in squashfs) fn inode_number(&self) -> u64 {
        todo!()
    }

    /// Returns the entry's kind.
    pub(in squashfs) fn kind(&self) -> InodeKind {
        todo!()
    }

    /// Returns the entry's name.
    pub(in squashfs) fn name(&self) -> &str {
        todo!()
    }
}

/// A walk over one directory's entries in the order the image stores them.
///
/// A walk always stands on an entry boundary it reached itself, so resuming one
/// at a position a listing reported finds the same entry again; a position inside
/// an entry is skipped over rather than half-read.
///
/// A record that does not fit the remaining content ends the walk as an I/O
/// error. That is the failure a malformed directory shows, and it is the caller's
/// decision, not the walk's, that a listing treats it as the end of the directory
/// while a lookup treats it as an error.
///
/// The inode count every entry's number is checked against is the one the image
/// states, taken from the image the walk reads.
pub(in squashfs) struct DirCursor<'a> {
    cursor: MetadataCursor<'a>,
    remaining: usize,
    consumed: usize,
}

impl<'a> DirCursor<'a> {
    /// Starts a walk over a directory's content at a reported position.
    ///
    /// `start` is in the reported numbering, so three or more bytes of content
    /// have been consumed when it is three or more; the walk skips to the entry
    /// that starts there.
    pub(in squashfs) fn new(
        image: &'a Image,
        cache: &'a MetadataCache,
        decompressor: &'a Decompressor,
        inode: &DirInode,
        start: usize,
    ) -> Self {
        todo!()
    }

    /// Returns the reported position the next entry will be read at.
    pub(in squashfs) fn position(&self) -> usize {
        todo!()
    }

    /// Reads the next entry, or `None` when the content is exhausted.
    ///
    /// Fails with an I/O error when a record does not fit the remaining content, or
    /// when an entry's inode number leaves the image's inode count.
    pub(in squashfs) fn next_entry(&mut self) -> Result<Option<DirEntry>> {
        todo!()
    }
}
