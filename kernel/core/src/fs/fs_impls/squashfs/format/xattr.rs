// SPDX-License-Identifier: MPL-2.0

//! The xattr records: the id table, its id entries, and the attribute entries and values.
//!
//! An inode does not carry its extended attributes; it carries an index into the image's
//! xattr id table, and one id entry names the group of attributes that index selects.
//! The id table is two levels deep.
//! A 16-byte header states the xattr data area's image position and the number of id
//! entries; an array of 8-byte image positions, held by the reader, names the metadata
//! blocks that store the id entries back to back.
//!
//! A group is a sequence of records in the xattr data area.
//! An entry opens with a 4-byte header whose type word carries the prefix and the
//! out-of-line bit, and the bare name follows with no padding and no terminator.
//! A value opens with a 4-byte stored length, and that many stored bytes follow;
//! in the out-of-line form those bytes are an [`XattrRef`] instead of the value.
//! The id entry's reference and the out-of-line reference are the same 48-bit-plus-16-bit
//! shape, both counted from the xattr data area's image position, not from zero.
//!
//! [`XattrRef`] is that shape in the image's own terms, and its `decode` is the only
//! place the 48-bit offset is separated from the 16-bit in-block offset.
//! An out-of-line value's reader will decode this type and resolve it against the
//! data area's position the same way; no such reader exists yet.
//! Resolving a reference against the data area's position belongs to the reader, so
//! this layer never depends on the read layer.
//! The id entry holds a reference, so it decodes field by field and carries no
//! `repr(C)` or `Pod`; the fixed-width headers that hold no reference do.
//!
//! Every `decode` here checks the record's width and the field range the record states
//! about itself: the id table's entry count is at least one.
//! Everything else belongs to the reader: a value length that overflows an `int` and a
//! name or value that runs past the metadata chain become I/O errors there, while an
//! unknown prefix or a reserved type bit makes the entry ignored rather than invalid.

#![expect(unused_variables, reason = "every body here is still `todo!()`")]
#![short_vis_path::add(squashfs)]

use super::superblock::ImageOffset;

/// Users: XattrEntryHeader::prefix.
const XATTR_PREFIX_MASK: u16 = 0x00ff;

/// Users: XattrEntryHeader::is_out_of_line, has_reserved_bits.
const XATTR_OUT_OF_LINE_BIT: u16 = 1 << 8;

/// Users: XattrEntryHeader::prefix_name.
const XATTR_USER_PREFIX: &str = "user.";

/// Users: XattrEntryHeader::prefix_name.
const XATTR_TRUSTED_PREFIX: &str = "trusted.";

/// Users: XattrEntryHeader::prefix_name.
const XATTR_SECURITY_PREFIX: &str = "security.";

/// Produced by: XattrRef::decode. Users: XattrIdEntry::decode, MetadataRef::from_xattr_ref.
#[derive(Clone, Copy, Debug)]
pub(in squashfs) struct XattrRef {
    /// Owner: XattrRef. Users: xattr_table_offset.
    xattr_table_offset: u64,
    /// Owner: XattrRef. Users: in_block_offset.
    in_block_offset: u16,
}

impl XattrRef {
    /// Users: XattrIdEntry::decode.
    pub(in squashfs) const SIZE: usize = 8;

    /// Caller: XattrIdEntry::decode. Callee: none.
    pub(in squashfs) fn decode(bytes: &[u8]) -> Option<Self> {
        todo!()
    }

    /// Caller: MetadataRef::from_xattr_ref. Callee: none.
    pub(in squashfs) fn xattr_table_offset(&self) -> u64 {
        todo!()
    }

    /// Caller: MetadataRef::from_xattr_ref. Callee: none.
    pub(in squashfs) fn in_block_offset(&self) -> u16 {
        todo!()
    }
}

/// Produced by: XattrIdTableHeader::decode. Users: read::xattr_id::XattrIdTable::load.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod)]
pub(in squashfs) struct XattrIdTableHeader {
    /// Owner: XattrIdTableHeader. Users: xattr_table_start.
    xattr_table_start: ImageOffset,
    /// Owner: XattrIdTableHeader. Users: xattr_ids.
    xattr_ids: u32,
    /// Owner: XattrIdTableHeader. Users: none.
    unused: u32,
}

impl XattrIdTableHeader {
    /// Users: read::xattr_id::XattrIdTable::load.
    pub(in squashfs) const SIZE: usize = 16;

    /// Caller: read::xattr_id::XattrIdTable::load. Callee: none.
    pub(in squashfs) fn decode(bytes: &[u8]) -> Option<Self> {
        todo!()
    }

    /// Caller: read::xattr_id::XattrIdTable::load. Callee: none.
    pub(in squashfs) fn xattr_table_start(&self) -> ImageOffset {
        todo!()
    }

    /// Caller: read::xattr_id::XattrIdTable::load. Callee: none.
    pub(in squashfs) fn xattr_ids(&self) -> u32 {
        todo!()
    }
}

const _: () = assert!(size_of::<XattrIdTableHeader>() == XattrIdTableHeader::SIZE);

/// Produced by: XattrIdEntry::decode. Users: read::xattr_id::XattrIdTable::lookup.
#[derive(Clone, Copy, Debug)]
pub(in squashfs) struct XattrIdEntry {
    /// Owner: XattrIdEntry. Users: xattr.
    xattr: XattrRef,
    /// Owner: XattrIdEntry. Users: entry_count.
    entry_count: u32,
    /// Owner: XattrIdEntry. Users: stored_xattr_len.
    stored_xattr_len: u32,
}

impl XattrIdEntry {
    /// Users: read::xattr_id::XattrIdTable::load, lookup.
    pub(in squashfs) const SIZE: usize = 16;

    /// Caller: read::xattr_id::XattrIdTable::lookup. Callee: XattrRef::decode.
    pub(in squashfs) fn decode(bytes: &[u8]) -> Option<Self> {
        todo!()
    }

    /// Caller: read::xattr_id::XattrIdTable::lookup. Callee: none.
    pub(in squashfs) fn xattr(&self) -> &XattrRef {
        todo!()
    }

    /// Caller: read::xattr_id::XattrIdTable::lookup. Callee: none.
    pub(in squashfs) fn entry_count(&self) -> u32 {
        todo!()
    }

    /// Caller: read::xattr_id::XattrIdTable::lookup. Callee: none.
    pub(in squashfs) fn stored_xattr_len(&self) -> u32 {
        todo!()
    }
}

/// Produced by: XattrEntryHeader::decode. Users: none.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod)]
pub(in squashfs) struct XattrEntryHeader {
    /// Owner: XattrEntryHeader. Users: prefix, is_out_of_line, has_reserved_bits.
    xattr_type: u16,
    /// Owner: XattrEntryHeader. Users: stored_name_len.
    stored_name_len: u16,
}

impl XattrEntryHeader {
    /// Users: none.
    pub(in squashfs) const SIZE: usize = 4;

    /// Caller: none. Callee: none.
    pub(in squashfs) fn decode(bytes: &[u8]) -> Option<Self> {
        todo!()
    }

    /// Caller: XattrEntryHeader::prefix_name. Callee: none.
    fn prefix(&self) -> u8 {
        todo!()
    }

    /// Caller: none. Callee: XattrEntryHeader::prefix.
    pub(in squashfs) fn prefix_name(&self) -> Option<&'static str> {
        todo!()
    }

    /// Caller: none. Callee: none.
    pub(in squashfs) fn is_out_of_line(&self) -> bool {
        todo!()
    }

    /// Caller: none. Callee: none.
    pub(in squashfs) fn has_reserved_bits(&self) -> bool {
        todo!()
    }

    /// Caller: none. Callee: none.
    pub(in squashfs) fn stored_name_len(&self) -> usize {
        todo!()
    }
}

const _: () = assert!(size_of::<XattrEntryHeader>() == XattrEntryHeader::SIZE);

/// Produced by: XattrValueHeader::decode. Users: none.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod)]
pub(in squashfs) struct XattrValueHeader {
    /// Owner: XattrValueHeader. Users: stored_value_len.
    stored_value_len: u32,
}

impl XattrValueHeader {
    /// Users: none.
    pub(in squashfs) const SIZE: usize = 4;

    /// Caller: none. Callee: none.
    pub(in squashfs) fn decode(bytes: &[u8]) -> Option<Self> {
        todo!()
    }

    /// Caller: none. Callee: none.
    pub(in squashfs) fn stored_value_len(&self) -> usize {
        todo!()
    }
}

const _: () = assert!(size_of::<XattrValueHeader>() == XattrValueHeader::SIZE);
