// SPDX-License-Identifier: MPL-2.0

//! Directory headers and directory entries.
//!
//! A directory's content is a sequence of runs.
//! Each run opens with a header that names at most 256 entries,
//! the inode each entry's number is counted from,
//! and the metadata position where that first inode lives.
//! The entries follow the header back to back,
//! each a fixed prefix plus a name of its own length, with no padding and no terminator.
//!
//! The two bounds that a malformed image can break,
//! a header covering more than 256 entries and a name longer than 256 bytes,
//! are reported to the reader rather than rejected here:
//! readdir ends the listing quietly while lookup fails,
//! so the decision needs a caller that knows which of the two it is serving.

#![expect(unused_variables, reason = "every body here is still `todo!()`")]
#![short_vis_path::add(squashfs)]

/// Users: read::dir::DirCursor::next_entry.
pub(in squashfs) const MAX_ENTRIES_PER_HEADER: u32 = 256;

/// Users: read::dir::DirCursor::next_entry.
pub(in squashfs) const MAX_NAME_LEN: usize = 256;

/// Produced by: DirHeader::decode. Users: read::dir::DirCursor::new, next_entry.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod)]
pub(in squashfs) struct DirHeader {
    /// Owner: DirHeader. Users: entry_count.
    count: u32,
    /// Owner: DirHeader. Users: start_block.
    start_block: u32,
    /// Owner: DirHeader. Users: inode_number.
    inode_number: u32,
}

impl DirHeader {
    /// Users: read::dir::DirCursor::new.
    pub(in squashfs) const SIZE: usize = 12;

    /// Caller: read::dir::DirCursor::new, next_entry. Callee: none.
    pub(in squashfs) fn decode(bytes: &[u8]) -> Option<Self> {
        todo!()
    }

    /// Caller: read::dir::DirCursor::next_entry. Callee: none.
    pub(in squashfs) fn entry_count(&self) -> u32 {
        todo!()
    }

    /// Caller: read::dir::DirCursor::new. Callee: none.
    pub(in squashfs) fn start_block(&self) -> u32 {
        todo!()
    }

    /// Caller: read::dir::DirCursor::next_entry. Callee: none.
    pub(in squashfs) fn inode_number(&self) -> u32 {
        todo!()
    }
}

/// Produced by: DirEntryPrefix::decode. Users: read::dir::DirCursor::next_entry.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod)]
pub(in squashfs) struct DirEntryPrefix {
    /// Owner: DirEntryPrefix. Users: offset.
    offset: u16,
    /// Owner: DirEntryPrefix. Users: inode_number.
    inode_number: i16,
    /// Owner: DirEntryPrefix. Users: entry_type.
    entry_type: u16,
    /// Owner: DirEntryPrefix. Users: name_len.
    name_len_minus_one: u16,
}

impl DirEntryPrefix {
    /// Users: read::dir::DirCursor::next_entry.
    pub(in squashfs) const SIZE: usize = 8;

    /// Caller: read::dir::DirCursor::next_entry. Callee: none.
    pub(in squashfs) fn decode(bytes: &[u8]) -> Option<Self> {
        todo!()
    }

    /// Caller: read::dir::DirCursor::next_entry. Callee: none.
    pub(in squashfs) fn name_len(&self) -> usize {
        todo!()
    }

    /// Caller: read::dir::DirCursor::next_entry. Callee: none.
    pub(in squashfs) fn offset(&self) -> u16 {
        todo!()
    }

    /// Caller: read::dir::DirCursor::next_entry. Callee: none.
    pub(in squashfs) fn inode_number(&self) -> i16 {
        todo!()
    }

    /// Caller: read::dir::DirCursor::next_entry. Callee: none.
    pub(in squashfs) fn entry_type(&self) -> u16 {
        todo!()
    }
}
