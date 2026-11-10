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

/// The most entries one directory header can cover.
pub(in squashfs) const MAX_ENTRIES_PER_HEADER: u32 = 256;

/// The longest name a directory entry can state, in bytes.
pub(in squashfs) const MAX_NAME_LEN: usize = 256;

/// The 12-byte header that opens a run of directory entries.
///
/// `start_block` is a byte offset relative to the directory table's first
/// metadata block, not a block number, and `inode_number` is the base the run's
/// entries count from; neither can be resolved without the directory's own
/// content, so `decode` checks only the width.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod)]
pub(in squashfs) struct DirHeader {
    /// One less than the number of entries in this run.
    count: u32,
    /// The byte offset of the run's first inode relative to the inode table.
    start_block: u32,
    /// The inode number the run's first entry answers to.
    inode_number: u32,
}

impl DirHeader {
    /// The width of this header, in bytes.
    pub(in squashfs) const SIZE: usize = 12;

    /// Decodes one run header of a directory's content.
    pub(in squashfs) fn decode(bytes: &[u8]) -> Option<Self> {
        todo!()
    }

    /// Returns how many entries this run covers, as the header states it.
    ///
    /// The record stores one less than the count, so every caller that sizes a
    /// run reads the count through here rather than adding one itself.
    pub(in squashfs) fn entry_count(&self) -> u32 {
        todo!()
    }

    /// Returns the byte offset of the run's first inode relative to the inode
    /// table.
    pub(in squashfs) fn start_block(&self) -> u32 {
        todo!()
    }

    /// Returns the inode number the run's first entry answers to.
    pub(in squashfs) fn inode_number(&self) -> u32 {
        todo!()
    }
}

/// The 8-byte fixed prefix of a directory entry, before the name.
///
/// The name's bytes follow the prefix without a gap and are not terminated, so
/// the prefix alone is not the entry. `inode_number` is a signed increment from
/// the run header's base, which lets a run name inodes out of ascending order.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod)]
pub(in squashfs) struct DirEntryPrefix {
    /// The byte offset of the entry's inode inside the header's metadata block.
    offset: u16,
    /// The increment to add to the run header's inode number base.
    inode_number: i16,
    /// The entry's kind, which never names an extended kind.
    entry_type: u16,
    /// One less than the name's length in bytes.
    name_len_minus_one: u16,
}

impl DirEntryPrefix {
    /// The width of this prefix, in bytes.
    pub(in squashfs) const SIZE: usize = 8;

    /// Decodes the fixed prefix of one directory entry.
    pub(in squashfs) fn decode(bytes: &[u8]) -> Option<Self> {
        todo!()
    }

    /// Returns the name's length in bytes, as the prefix states it.
    pub(in squashfs) fn name_len(&self) -> usize {
        todo!()
    }

    /// Returns the byte offset of the entry's inode inside the header's metadata
    /// block.
    pub(in squashfs) fn offset(&self) -> u16 {
        todo!()
    }

    /// Returns the entry's signed increment from the run header's inode number.
    pub(in squashfs) fn inode_number(&self) -> i16 {
        todo!()
    }

    /// Returns the entry's type word.
    ///
    /// The word names one of the seven basic kinds; the caller maps it through
    /// [`InodeKind::from_dir_entry_type`](super::inode::InodeKind::from_dir_entry_type),
    /// which refuses a word outside them.
    pub(in squashfs) fn entry_type(&self) -> u16 {
        todo!()
    }
}
