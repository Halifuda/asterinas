// SPDX-License-Identifier: MPL-2.0

//! The length-and-compression word in front of every stored block.
//!
//! Squashfs frames both metadata blocks and file data blocks the same way:
//! a length word whose top meaningful bit says whether the payload is stored compressed.
//! Metadata blocks use a 16-bit word and a fixed decompressed size;
//! a file's block list uses a 32-bit word and a size the reader derives per block.
//! This module decodes those words and nothing else:
//! the payload belongs to the layer that fetches it.
//!
//! A decoded value holds the two facts the word names, already separated,
//! so no reader masks a length or tests a flag for itself.
//! The `SIZE` of each type is still the width the record occupies on the image.

#![expect(unused_variables, reason = "every body here is still `todo!()`")]
#![short_vis_path::add(squashfs)]

/// The decompressed size of a metadata block, in bytes.
///
/// The value is a property of the format rather than of an image: a metadata
/// block's stored length varies with compression, but its decompressed payload
/// is always this long, except possibly for the last block of a chain.
pub(in squashfs) const METADATA_BLOCK_SIZE: usize = 8192;

/// The bit of a 16-bit length word that marks the payload as stored as-is.
const METADATA_UNCOMPRESSED_BIT: u16 = 1 << 15;

/// The bits of a 16-bit length word that hold the stored length.
const METADATA_STORED_LEN_MASK: u16 = !METADATA_UNCOMPRESSED_BIT;

/// The 2-byte header in front of a metadata block, as the two facts it names.
///
/// The header states the payload's stored size and whether it is compressed, and
/// that is all the framing an inode table, a directory table, or an id table
/// block carries. `decode` accepts any bit pattern of the right width: the
/// length bound belongs to the reader, which is the only place that knows how
/// many bytes of the image are left.
///
/// The value is not a byte image of the two-byte word: `decode` has already
/// separated the length from the flag, so the type carries neither `#[repr(C)]`
/// nor `Pod`. `SIZE` alone states the width the word occupies on the image.
#[derive(Clone, Copy, Debug)]
pub(in squashfs) struct MetadataBlockHeader {
    /// How many bytes the payload occupies on the image.
    stored_len: usize,
    /// Whether the payload is stored as-is instead of compressed.
    is_uncompressed: bool,
}

impl MetadataBlockHeader {
    /// The width this header occupies on the image, in bytes.
    pub(in squashfs) const SIZE: usize = 2;

    /// Decodes the header in front of a metadata block.
    ///
    /// A length field of zero is defined by the format as 32768, one past the
    /// largest length the 15-bit field can state, so the decoded length is that
    /// over-long value rather than zero; the reader's upper bound then rejects
    /// the block instead of reading it as empty.
    pub(in squashfs) fn decode(bytes: &[u8]) -> Option<Self> {
        todo!()
    }

    /// Returns how many bytes the block's payload occupies on the image.
    ///
    /// The length field's zero case is already resolved by `decode`, so the value
    /// is the format's own answer for the block and never a stand-in for
    /// "empty".
    pub(in squashfs) fn stored_len(&self) -> usize {
        todo!()
    }

    /// Reports whether the payload is stored as-is instead of compressed.
    pub(in squashfs) fn is_uncompressed(&self) -> bool {
        todo!()
    }
}

/// The 4-byte block list entry that names one file data block, as the two facts
/// it names.
///
/// The entry holds the block's stored length in its low 24 bits and the
/// uncompressed flag in bit 24. Bits 25 to 31 are reserved and must be zero; a
/// set reserved bit cannot describe any block, so `decode` refuses the entry
/// instead of letting a reader interpret a reserved bit as part of a length.
///
/// A stored length of zero marks a sparse block: no bytes of the image belong to
/// it, and it reads as zeros.
///
/// The value, like [`MetadataBlockHeader`], holds the two facts `decode` resolved
/// rather than a copy of the four-byte word, so the type carries neither
/// `#[repr(C)]` nor `Pod`. `SIZE` alone states the width the entry occupies on the
/// image.
#[derive(Clone, Copy, Debug)]
pub(in squashfs) struct BlockListEntry {
    /// How many bytes the data block occupies on the image.
    stored_len: usize,
    /// Whether the data block is stored as-is instead of compressed.
    is_uncompressed: bool,
}

impl BlockListEntry {
    /// The width this entry occupies on the image, in bytes.
    pub(in squashfs) const SIZE: usize = 4;

    /// The number of low bits that hold the stored length.
    const STORED_LEN_BITS: u32 = 24;

    /// The bit that marks the block as stored as-is instead of compressed.
    const UNCOMPRESSED_BIT: u32 = 1 << Self::STORED_LEN_BITS;

    /// Decodes one entry of a block list.
    ///
    /// Returns `None` when a reserved bit is set, because such an entry names no
    /// block: the reader that walks the list reports it as a malformed image
    /// rather than guessing what the bits meant.
    pub(in squashfs) fn decode(bytes: &[u8]) -> Option<Self> {
        todo!()
    }

    /// Returns how many bytes the data block occupies on the image.
    ///
    /// Zero marks a sparse block.
    pub(in squashfs) fn stored_len(&self) -> usize {
        todo!()
    }

    /// Reports whether the data block is stored as-is instead of compressed.
    pub(in squashfs) fn is_uncompressed(&self) -> bool {
        todo!()
    }
}
