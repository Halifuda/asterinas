// SPDX-License-Identifier: MPL-2.0

//! The inode reference and the inode record bodies as the image states them.
//!
//! An inode is named by a 48-bit byte offset relative to the inode table,
//! plus a 16-bit offset inside the decompressed metadata block that holds it.
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

/// The inode type word of an inode record.
///
/// The word is one of fourteen values; anything else leaves an inode record
/// without a type, which no reader can interpret.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in squashfs) enum InodeKind {
    /// A directory whose content fits the basic record.
    Dir,
    /// A regular file whose content fits the basic record.
    File,
    /// A symbolic link whose target fits the basic record.
    Symlink,
    /// A block device node whose numbers fit the basic record.
    BlockDevice,
    /// A character device node whose numbers fit the basic record.
    CharDevice,
    /// A named pipe.
    Fifo,
    /// A socket.
    Socket,
    /// A directory, with a 64-bit size and an optional directory index.
    ExtendedDir,
    /// A regular file, with a 64-bit size and a sparse-byte count.
    ExtendedFile,
    /// A symbolic link, with a 64-bit target length.
    ExtendedSymlink,
    /// A block device node that needs the extended number layout.
    ExtendedBlockDevice,
    /// A character device node that needs the extended number layout.
    ExtendedCharDevice,
    /// A named pipe, in the extended layout.
    ExtendedFifo,
    /// A socket, in the extended layout.
    ExtendedSocket,
}

impl InodeKind {
    /// Returns the kind named by an inode record's type word.
    ///
    /// A word outside the fourteen defined values names no kind, which the
    /// caller reports as a malformed inode.
    pub(in squashfs) fn from_inode_type(word: u16) -> Option<Self> {
        todo!()
    }

    /// Returns the kind named by a directory entry's type word.
    ///
    /// Directory entries carry only the seven basic kinds, so a word outside
    /// them names no kind even when it would be valid in an inode record. The
    /// two callers answer such a word differently, which is why this rule is
    /// not folded into [`InodeKind::from_inode_type`].
    pub(in squashfs) fn from_dir_entry_type(word: u16) -> Option<Self> {
        todo!()
    }
}

/// The 8-byte reference that names one inode.
///
/// The reference is a 48-bit byte offset relative to the table the inode lives
/// in, followed by a 16-bit offset inside the decompressed metadata block at
/// that position. The block offset is a byte offset, not a block number, because
/// metadata blocks vary in stored length. `decode` accepts a reference only when
/// the in-block offset can address a metadata block at all; a larger offset
/// cannot be resolved by any reader and is reported as a malformed image.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod)]
pub(in squashfs) struct InodeRef {
    /// The low six bytes of the table-relative block offset.
    block_start: [u8; 6],
    /// The byte offset inside the decompressed metadata block.
    offset: u16,
}

impl InodeRef {
    /// The width this reference occupies on the image, in bytes.
    pub(in squashfs) const SIZE: usize = 8;

    /// Decodes the reference a superblock or an index table states.
    pub(in squashfs) fn decode(bytes: &[u8]) -> Option<Self> {
        todo!()
    }

    /// Builds a reference from the two parts a directory walk already holds.
    ///
    /// Returns `None` when the block start does not fit the six bytes the
    /// reference holds, or when the in-block offset reaches
    /// [`METADATA_BLOCK_SIZE`](super::block::METADATA_BLOCK_SIZE), because no
    /// metadata block is that long and no reader could resolve it.
    pub(in squashfs) fn from_parts(block_start: u64, offset: u16) -> Option<Self> {
        todo!()
    }

    /// Returns the byte offset relative to the table's first metadata block.
    pub(in squashfs) fn block_start(&self) -> u64 {
        todo!()
    }

    /// Returns the byte offset inside the decompressed metadata block.
    pub(in squashfs) fn offset(&self) -> u16 {
        todo!()
    }
}

/// The 16-byte header shared by every inode record.
///
/// The header fixes what every inode has: a type, permission bits, the two id
/// table indexes that name its owner and group, a modification time, and the
/// inode number. Its `uid` and `gid` are table indexes, not values, so the
/// header alone cannot say who owns the inode; only a reader holding the id table
/// can.
///
/// `decode` rejects the two field values that leave no inode to build: a type
/// word outside the fourteen defined kinds and an inode number of zero. The
/// permission bits' type nibble and the id indexes are checked against the mount's
/// own facts by the reader, because those failures are reported differently.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod)]
pub(in squashfs) struct BaseInode {
    /// The inode type word.
    inode_type: u16,
    /// The permission bits, without the type nibble.
    mode: u16,
    /// The index of the owner's id in the id table.
    uid: u16,
    /// The index of the group's id in the id table.
    gid: u16,
    /// The modification time, in seconds since the epoch.
    mtime: u32,
    /// The inode number this record answers to.
    inode_number: u32,
}

impl BaseInode {
    /// The width of this header, in bytes.
    pub(in squashfs) const SIZE: usize = 16;

    /// Decodes the header in front of an inode body.
    pub(in squashfs) fn decode(bytes: &[u8]) -> Option<Self> {
        todo!()
    }

    /// Returns the record's type word, already known to name a defined kind.
    pub(in squashfs) fn inode_type(&self) -> u16 {
        todo!()
    }

    /// Returns the permission bits, without the type nibble.
    pub(in squashfs) fn mode(&self) -> u16 {
        todo!()
    }

    /// Returns the index of the owner's id in the id table.
    pub(in squashfs) fn uid(&self) -> u16 {
        todo!()
    }

    /// Returns the index of the group's id in the id table.
    pub(in squashfs) fn gid(&self) -> u16 {
        todo!()
    }

    /// Returns the modification time, in seconds since the epoch.
    pub(in squashfs) fn mtime(&self) -> u32 {
        todo!()
    }

    /// Returns the inode number this record answers to.
    pub(in squashfs) fn inode_number(&self) -> u32 {
        todo!()
    }
}

/// The 16 bytes that follow the header of a basic regular file.
///
/// The record's block list follows these bytes without a gap, so a reader that
/// stops here knows where the list starts. Every field is a plain number; the
/// constraints that involve other records, such as the fragment index bound or
/// the file size the mount's block size allows, are the reader's to enforce.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod)]
pub(in squashfs) struct RegInode {
    /// The image position of the file's first data block.
    start_block: u32,
    /// The fragment holding the file's tail, or the no-fragment marker.
    fragment: u32,
    /// The tail's byte offset inside that fragment block.
    offset: u32,
    /// The file's length in bytes.
    file_size: u32,
}

impl RegInode {
    /// The width of this body, in bytes.
    pub(in squashfs) const SIZE: usize = 16;

    /// The value of the fragment field that states the file keeps its whole content
    /// in the data area.
    pub(in squashfs) const NO_FRAGMENT_MARKER: u32 = 0xffff_ffff;

    /// Decodes the body of a basic regular file.
    pub(in squashfs) fn decode(bytes: &[u8]) -> Option<Self> {
        todo!()
    }

    /// Returns the image position of the file's first data block.
    pub(in squashfs) fn start_block(&self) -> u32 {
        todo!()
    }

    /// Returns the index of the fragment holding the file's tail, or the
    /// no-fragment marker.
    pub(in squashfs) fn fragment(&self) -> u32 {
        todo!()
    }

    /// Returns the tail's byte offset inside the fragment block.
    pub(in squashfs) fn offset(&self) -> u32 {
        todo!()
    }

    /// Returns the file's length in bytes.
    pub(in squashfs) fn file_size(&self) -> u32 {
        todo!()
    }

    /// Returns how many four-byte entries the file's block list holds.
    ///
    /// The count is one entry per listed data block. A file whose tail lives in a
    /// fragment block lists only the full blocks before that tail, so its count is
    /// the size divided by the block size and rounded down; a file whose tail is in
    /// the data area lists every block it has, so its count is the same division
    /// rounded up.
    /// This is the only place the count is derived, so no caller rounds a size its
    /// own way: the block list the walk reads and the tail a page fill may have to
    /// reach are counted alike.
    pub(in squashfs) fn data_block_count(&self, block_size: usize) -> usize {
        todo!()
    }

    /// Returns the length the file's last listed data block must decode to.
    ///
    /// Only the last listed block of a file whose tail lives in the data area can
    /// be short, and it holds the file's remainder; a file whose size is a whole
    /// number of blocks has no remainder, so even its last listed block is full. A
    /// file whose tail lives in a fragment block lists only full blocks, since the
    /// partial tail is not a list entry.
    /// This is the only place the length is derived, so a last block is never
    /// expected to decode to nothing.
    pub(in squashfs) fn last_block_expected_len(&self, block_size: usize) -> usize {
        todo!()
    }
}

/// The 16 bytes that follow the header of a basic directory.
///
/// The directory's content is a byte range of the directory table, not a block
/// list: `start_block` is relative to the directory table's first metadata block
/// and `offset` locates the first byte inside that block. The record's length
/// counts three bytes that are not stored, because the entries `.` and `..` are
/// synthesized by the reader instead of encoded.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod)]
pub(in squashfs) struct DirInode {
    /// The byte offset of the content relative to the directory table.
    start_block: u32,
    /// The number of links to this directory, including the parent's entry.
    nlink: u32,
    /// The content's length, counted with the three synthesized bytes.
    file_size: u16,
    /// The byte offset of the content inside its first metadata block.
    offset: u16,
    /// The inode number the synthesized `..` entry resolves to.
    parent_inode: u32,
}

impl DirInode {
    /// The width of this body, in bytes.
    pub(in squashfs) const SIZE: usize = 16;

    /// Decodes the body of a basic directory.
    pub(in squashfs) fn decode(bytes: &[u8]) -> Option<Self> {
        todo!()
    }

    /// Returns the byte offset of the content relative to the directory table.
    pub(in squashfs) fn start_block(&self) -> u32 {
        todo!()
    }

    /// Returns how many links the directory has, including the parent's entry.
    pub(in squashfs) fn nlink(&self) -> u32 {
        todo!()
    }

    /// Returns the content's length, counted with the three synthesized bytes.
    pub(in squashfs) fn file_size(&self) -> u16 {
        todo!()
    }

    /// Returns the byte offset of the content inside its first metadata block.
    pub(in squashfs) fn offset(&self) -> u16 {
        todo!()
    }

    /// Returns the inode number the synthesized `..` entry resolves to.
    pub(in squashfs) fn parent_inode(&self) -> u32 {
        todo!()
    }
}
