// SPDX-License-Identifier: MPL-2.0

//! The inode reference and the inode record bodies as the image states them.
//!
//! An inode is named by a 48-bit byte offset relative to the inode table,
//! plus a 16-bit offset inside the decompressed metadata block that holds it.
//! [`InodeRef`] is that pair in the image's own shape: table-relative and unresolved.
//! A reader adds the table's image position before it addresses a block.
//! The record starts with a 16-byte header common to every inode,
//! followed by a body whose layout the header's type word selects.
//!
//! Every type here decodes the bytes of one record and no more:
//! assembling a record out of the metadata stream,
//! and checking it against the mount,
//! happens in the readers built on these bytes.
//! This layer decodes the two bodies that a mount, a directory listing,
//! and a small file need:
//! the basic directory and the basic regular file.
//! The extended and the special bodies arrive with the readers that need them;
//! their type words are named here,
//! so an unknown word can be told apart from a known one.

#![expect(unused_variables, reason = "every body here is still `todo!()`")]
#![short_vis_path::add(squashfs)]

use super::superblock::ImageOffset;

/// Produced by: InodeKind::from_inode_type, from_dir_entry_type. Users: read::inode_record.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in squashfs) enum InodeKind {
    /// Owner: InodeKind. Users: from_inode_type, from_dir_entry_type.
    Dir,
    /// Owner: InodeKind. Users: from_inode_type, from_dir_entry_type.
    File,
    /// Owner: InodeKind. Users: from_inode_type, from_dir_entry_type.
    Symlink,
    /// Owner: InodeKind. Users: from_inode_type, from_dir_entry_type.
    BlockDevice,
    /// Owner: InodeKind. Users: from_inode_type, from_dir_entry_type.
    CharDevice,
    /// Owner: InodeKind. Users: from_inode_type, from_dir_entry_type.
    Fifo,
    /// Owner: InodeKind. Users: from_inode_type, from_dir_entry_type.
    Socket,
    /// Owner: InodeKind. Users: from_inode_type.
    ExtendedDir,
    /// Owner: InodeKind. Users: from_inode_type.
    ExtendedFile,
    /// Owner: InodeKind. Users: from_inode_type.
    ExtendedSymlink,
    /// Owner: InodeKind. Users: from_inode_type.
    ExtendedBlockDevice,
    /// Owner: InodeKind. Users: from_inode_type.
    ExtendedCharDevice,
    /// Owner: InodeKind. Users: from_inode_type.
    ExtendedFifo,
    /// Owner: InodeKind. Users: from_inode_type.
    ExtendedSocket,
}

impl InodeKind {
    /// Caller: read::inode_record::InodeRecord::read. Callee: none.
    pub(in squashfs) fn from_inode_type(word: u16) -> Option<Self> {
        todo!()
    }

    /// Caller: read::dir::DirCursor::next_entry. Callee: none.
    pub(in squashfs) fn from_dir_entry_type(word: u16) -> Option<Self> {
        todo!()
    }
}

/// Produced by: InodeRef::decode, from_parts. Users: SuperBlock, DirEntry.
#[derive(Clone, Copy, Debug)]
pub(in squashfs) struct InodeRef {
    /// Owner: InodeRef. Users: inode_table_offset.
    inode_table_offset: u64,
    /// Owner: InodeRef. Users: in_block_offset.
    in_block_offset: u16,
}

impl InodeRef {
    /// Users: read::inode_record::InodeRecord::read.
    pub(in squashfs) const SIZE: usize = 8;

    /// Caller: SuperBlock::decode. Callee: none.
    pub(in squashfs) fn decode(bytes: &[u8]) -> Option<Self> {
        todo!()
    }

    /// Caller: read::dir::DirCursor::next_entry. Callee: none.
    pub(in squashfs) fn from_parts(inode_table_offset: u64, in_block_offset: u16) -> Option<Self> {
        todo!()
    }

    /// Caller: MetadataRef::from_inode_ref. Callee: none.
    pub(in squashfs) fn inode_table_offset(&self) -> u64 {
        todo!()
    }

    /// Caller: MetadataRef::from_inode_ref. Callee: none.
    pub(in squashfs) fn in_block_offset(&self) -> u16 {
        todo!()
    }
}

/// Produced by: InodeHeader::decode. Users: read::inode_record::InodeRecord.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod)]
pub(in squashfs) struct InodeHeader {
    /// Owner: InodeHeader. Users: inode_type.
    inode_type: u16,
    /// Owner: InodeHeader. Users: mode.
    mode: u16,
    /// Owner: InodeHeader. Users: uid_index.
    uid_index: u16,
    /// Owner: InodeHeader. Users: gid_index.
    gid_index: u16,
    /// Owner: InodeHeader. Users: mtime.
    mtime: u32,
    /// Owner: InodeHeader. Users: inode_number.
    inode_number: u32,
}

impl InodeHeader {
    /// Users: read::inode_record::InodeRecord::read.
    pub(in squashfs) const SIZE: usize = 16;

    /// Caller: read::inode_record::InodeRecord::read. Callee: none.
    pub(in squashfs) fn decode(bytes: &[u8]) -> Option<Self> {
        todo!()
    }

    /// Caller: read::inode_record::InodeRecord::kind. Callee: none.
    pub(in squashfs) fn inode_type(&self) -> u16 {
        todo!()
    }

    /// Caller: inode::SquashfsInode::mode_impl. Callee: none.
    pub(in squashfs) fn mode(&self) -> u16 {
        todo!()
    }

    /// Caller: inode::SquashfsInode::owner_impl. Callee: none.
    pub(in squashfs) fn uid_index(&self) -> u16 {
        todo!()
    }

    /// Caller: inode::SquashfsInode::group_impl. Callee: none.
    pub(in squashfs) fn gid_index(&self) -> u16 {
        todo!()
    }

    /// Caller: inode::SquashfsInode::metadata_impl. Callee: none.
    pub(in squashfs) fn mtime(&self) -> u32 {
        todo!()
    }

    /// Caller: read::inode_record::InodeRecord::inode_number. Callee: none.
    pub(in squashfs) fn inode_number(&self) -> u32 {
        todo!()
    }
}

const _: () = assert!(size_of::<InodeHeader>() == InodeHeader::SIZE);

/// Produced by: RegInode::decode. Users: read::inode_record, read::block_list.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod)]
pub(in squashfs) struct RegInode {
    /// Owner: RegInode. Users: data_block_start.
    data_block_start: u32,
    /// Owner: RegInode. Users: fragment_index.
    fragment_index: u32,
    /// Owner: RegInode. Users: fragment_in_block_offset.
    fragment_in_block_offset: u32,
    /// Owner: RegInode. Users: file_size.
    file_size: u32,
}

impl RegInode {
    /// Users: read::inode_record::InodeRecord::read.
    pub(in squashfs) const SIZE: usize = 16;

    /// Users: RegInode::data_block_count, inode::read_page_async.
    pub(in squashfs) const NO_FRAGMENT_MARKER: u32 = 0xffff_ffff;

    /// Caller: read::inode_record::InodeRecord::read. Callee: none.
    pub(in squashfs) fn decode(bytes: &[u8]) -> Option<Self> {
        todo!()
    }

    /// Caller: read::block_list::BlockListCursor::new. Callee: none.
    pub(in squashfs) fn data_block_start(&self) -> ImageOffset {
        todo!()
    }

    /// Caller: RegInode::data_block_count, last_block_expected_len,
    /// inode::read_page_async. Callee: none.
    pub(in squashfs) fn fragment_index(&self) -> u32 {
        todo!()
    }

    /// Caller: read::block_list::BlockListCursor::new. Callee: none.
    pub(in squashfs) fn fragment_in_block_offset(&self) -> u32 {
        todo!()
    }

    /// Caller: RegInode::data_block_count, last_block_expected_len. Callee: none.
    pub(in squashfs) fn file_size(&self) -> u32 {
        todo!()
    }

    /// Caller: read::block_list::BlockListCursor::new. Callee: file_size, fragment_index.
    pub(in squashfs) fn data_block_count(&self, block_size: usize) -> usize {
        todo!()
    }

    /// Caller: read::block_list::BlockListCursor::new. Callee: file_size, fragment_index.
    pub(in squashfs) fn last_block_expected_len(&self, block_size: usize) -> usize {
        todo!()
    }
}

const _: () = assert!(size_of::<RegInode>() == RegInode::SIZE);

/// Produced by: DirInode::decode. Users: read::dir::DirCursor::new, inode::metadata_impl.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod)]
pub(in squashfs) struct DirInode {
    /// Owner: DirInode. Users: directory_table_offset.
    directory_table_offset: u32,
    /// Owner: DirInode. Users: nlink.
    nlink: u32,
    /// Owner: DirInode. Users: stored_size_plus_dot_entries.
    stored_size_plus_dot_entries: u16,
    /// Owner: DirInode. Users: in_block_offset.
    in_block_offset: u16,
    /// Owner: DirInode. Users: parent_inode.
    parent_inode: u32,
}

impl DirInode {
    /// Users: read::inode_record::InodeRecord::read.
    pub(in squashfs) const SIZE: usize = 16;

    /// Caller: read::inode_record::InodeRecord::read. Callee: none.
    pub(in squashfs) fn decode(bytes: &[u8]) -> Option<Self> {
        todo!()
    }

    /// Caller: read::dir::DirCursor::new. Callee: none.
    pub(in squashfs) fn directory_table_offset(&self) -> u32 {
        todo!()
    }

    /// Caller: inode::SquashfsInode::metadata_impl. Callee: none.
    pub(in squashfs) fn nlink(&self) -> u32 {
        todo!()
    }

    /// Caller: read::dir::DirCursor::new. Callee: none.
    pub(in squashfs) fn stored_size_plus_dot_entries(&self) -> u16 {
        todo!()
    }

    /// Caller: read::dir::DirCursor::new. Callee: none.
    pub(in squashfs) fn in_block_offset(&self) -> u16 {
        todo!()
    }

    /// Caller: inode::SquashfsInode::lookup_impl. Callee: none.
    pub(in squashfs) fn parent_inode(&self) -> u32 {
        todo!()
    }
}

const _: () = assert!(size_of::<DirInode>() == DirInode::SIZE);
