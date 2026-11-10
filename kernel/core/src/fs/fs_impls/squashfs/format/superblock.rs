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

/// A byte position in the image, counted from the image's first byte, where a
/// reader seeks: a table's start or the start of a stored block.
///
/// The superblock states the image positions of its tables, and every reader that
/// seeks the image uses them. A position is not a length: `bytes_used` bounds how
/// many bytes exist, while a position may be anywhere below that bound, and both
/// are a `u64` on the image. Naming the position keeps the two apart where a bare
/// width would not.
///
/// A position a record states in a 32-bit field keeps the record's own width and
/// is not this type; its reader widens it when it starts seeking.
pub(in squashfs) type ImageOffset = u64;

/// The 96-byte record at the start of every Squashfs image, holding the geometry
/// it states.
///
/// `decode` verifies the length, the magic word, the version, and every relation
/// among the geometry fields that the format fixes: the block size is
/// `1 << block_log`, it is no larger than 1 MiB and no smaller than a page, the
/// id count is at least one, the image claims at least the superblock's own
/// bytes, the root inode's offset can address a metadata block, and the tables
/// appear in the order the format requires. A record that fails any of them
/// describes no mountable image and is rejected as a whole.
///
/// The value is kept for the life of the mount, so every number a reader needs is
/// still here; the fields no reader asks for, the creation time and the writer's
/// flags, are decoded but not exposed. The one relation the record cannot state
/// about itself is how many bytes the device holds; the reader that opens the
/// image checks it and refuses an image that reaches past the device.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod)]
pub(in squashfs) struct SuperBlock {
    /// The magic word that marks the image as Squashfs.
    magic: u32,
    /// The number of inodes in the image.
    inodes: u32,
    /// The creation time, in seconds since the epoch.
    mkfs_time: u32,
    /// The uncompressed size of a data block, in bytes.
    block_size: u32,
    /// The number of fragment entries, or zero when the image has no fragment table.
    fragments: u32,
    /// The compression id, which names the codec the image was written with.
    compression: u16,
    /// The base-2 logarithm of `block_size`.
    block_log: u16,
    /// The image's flags, which only describe how the writer chose to compress.
    flags: u16,
    /// The number of entries in the id table.
    no_ids: u16,
    /// The major format version.
    major: u16,
    /// The minor format version.
    minor: u16,
    /// The reference to the root directory's inode.
    root_inode: InodeRef,
    /// The number of bytes of the device the image occupies.
    bytes_used: u64,
    /// The image position of the id index table.
    id_table_start: ImageOffset,
    /// The image position of the xattr id index table, or an all-ones marker.
    xattr_id_table_start: ImageOffset,
    /// The image position of the inode table's first metadata block.
    inode_table_start: ImageOffset,
    /// The image position of the directory table's first metadata block.
    directory_table_start: ImageOffset,
    /// The image position of the fragment index table.
    fragment_table_start: ImageOffset,
    /// The image position of the export table, or an all-ones marker.
    lookup_table_start: ImageOffset,
}

impl SuperBlock {
    /// The width of this record, in bytes.
    pub(in squashfs) const SIZE: usize = 96;

    /// The word that marks a Squashfs image.
    const MAGIC: u32 = 0x7371_7368;

    /// Decodes a superblock record.
    pub(in squashfs) fn decode(bytes: &[u8]) -> Option<Self> {
        todo!()
    }

    /// Returns how many inodes the image states.
    ///
    /// A directory walk bounds every inode number it reports by this count, so an
    /// entry whose number falls outside it is a malformed directory.
    pub(in squashfs) fn inodes(&self) -> u32 {
        todo!()
    }

    /// Returns the uncompressed size of a data block, in bytes.
    ///
    /// The record states the value as a `u32`; decoding has already proved that it
    /// equals `1 << block_log` and does not exceed 1 MiB, so widening it to
    /// `usize` cannot truncate on any supported target.
    pub(in squashfs) fn block_size(&self) -> usize {
        todo!()
    }

    /// Returns the base-2 logarithm of the data block size, as the record states
    /// it.
    ///
    /// The field keeps the record's own `u16`; [`SuperBlock::block_size`] is the
    /// shifted value a reader uses, so no reader needs the logarithm.
    pub(in squashfs) fn block_log(&self) -> u16 {
        todo!()
    }

    /// Returns how many bytes of the device the image occupies.
    pub(in squashfs) fn bytes_used(&self) -> u64 {
        todo!()
    }

    /// Returns the image position of the inode table.
    pub(in squashfs) fn inode_table_start(&self) -> ImageOffset {
        todo!()
    }

    /// Returns the image position of the directory table.
    pub(in squashfs) fn directory_table_start(&self) -> ImageOffset {
        todo!()
    }

    /// Returns the image position of the fragment index table.
    ///
    /// The fragment code reads it, to locate the table the mount then keeps; no
    /// reader in this build opens fragments, so the accessor has no caller yet.
    pub(in squashfs) fn fragment_table_start(&self) -> ImageOffset {
        todo!()
    }

    /// Returns the image position of the id index table.
    pub(in squashfs) fn id_table_start(&self) -> ImageOffset {
        todo!()
    }

    /// Returns the image position of the xattr id index table, or the all-ones
    /// marker when the image has none.
    ///
    /// The xattr code reads it, to find the table whose entries it resolves
    /// attribute names through; no reader in this build opens xattrs.
    pub(in squashfs) fn xattr_id_table_start(&self) -> ImageOffset {
        todo!()
    }

    /// Returns the image position of the export table, or the all-ones marker
    /// when the image has none.
    ///
    /// The export code reads it, to resolve an inode number to an inode; no
    /// reader in this build exports anything.
    pub(in squashfs) fn lookup_table_start(&self) -> ImageOffset {
        todo!()
    }

    /// Returns how many fragment entries the image states.
    ///
    /// The fragment code reads it, to size the table it loads; no reader in this
    /// build reads a fragment, because the files this build reads are stored in
    /// whole blocks.
    pub(in squashfs) fn fragments(&self) -> u32 {
        todo!()
    }

    /// Returns how many id table entries the image states.
    pub(in squashfs) fn no_ids(&self) -> u16 {
        todo!()
    }

    /// Returns the compression id the image was written with.
    pub(in squashfs) fn compression_id(&self) -> u16 {
        todo!()
    }

    /// Returns the reference to the root directory's inode.
    pub(in squashfs) fn root_inode(&self) -> &InodeRef {
        todo!()
    }
}
