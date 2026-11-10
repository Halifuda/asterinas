// SPDX-License-Identifier: MPL-2.0

//! Reading an inode record out of the inode table.
//!
//! [`format::inode`](super::super::format::inode) decodes the fixed-width records the image states,
//! and nothing more:
//! it turns the bytes of a header or a body into a value,
//! and knows nothing of the stream around them.
//! This module assembles those records into one [`InodeRecord`]:
//! it follows the metadata stream from an inode reference,
//! reads the 16-byte header every inode has,
//! and then reads the body the header's type word selects.
//! The body's width is what decides where the record's variable-length tail,
//! the block list of a regular file, begins; nothing separates them on the image.
//!
//! Reading a record is also the first place an inode is checked against the mount:
//! the id indexes it carries have to fit the id table the mount loaded,
//! and the permission bits must not restate the type the type word already names.
//! The resulting value is the `InodeRecord` the rest of the mount carries,
//! already validated against the superblock the mount accepted.

#![expect(unused_variables, reason = "every body here is still `todo!()`")]
#![short_vis_path::add(squashfs)]

use super::{
    super::format::{
        inode::{BaseInode, DirInode, InodeKind, RegInode},
        superblock::SuperBlock,
    },
    metadata::{MetadataCursor, MetadataPosition},
};
use crate::prelude::*;

/// The body of an inode record, as the record's type word selects it.
///
/// This module reads the two bodies a mount, a directory listing, and a small
/// file need. The bodies of the remaining kinds have widths of their own, so a
/// record of one of those kinds cannot be finished here yet; its type word is
/// still named, which is what keeps a known-but-unread kind apart from an
/// unknown one.
pub(in squashfs) enum InodeBody {
    /// A basic regular file, whose block list starts right after the body.
    File {
        /// The fixed body of the record.
        inode: RegInode,
        /// The position the record's block list starts at.
        block_list: MetadataPosition,
    },
    /// A basic directory, whose content starts at its own stated position.
    Dir {
        /// The fixed body of the record.
        inode: DirInode,
    },
}

/// One inode record, header and body, as the inode table states it.
///
/// The value is already checked against the mount, so a reader may take the
/// owner's index and the record's size without re-validating either against the
/// image. It is not yet the owner's number: the id index still has to be resolved
/// through the id table, which only a holder of that table can do.
pub(in squashfs) struct InodeRecord {
    base: BaseInode,
    body: InodeBody,
}

impl InodeRecord {
    /// Reads the record a cursor is positioned at and advances past it.
    ///
    /// Fails with an I/O error when the record's bytes cannot be read or its
    /// in-block offset leaves the block, when its permission bits carry a type
    /// nibble, and with an invalid argument when the type word names no kind, the
    /// inode number is zero, or an id index reaches past the id count the
    /// superblock states.
    pub(in squashfs) fn read(
        cursor: &mut MetadataCursor<'_>,
        super_block: &SuperBlock,
    ) -> Result<Self> {
        todo!()
    }

    /// Returns the header every inode record starts with.
    pub(in squashfs) fn base(&self) -> &BaseInode {
        todo!()
    }

    /// Returns the body the record's type word selected.
    pub(in squashfs) fn body(&self) -> &InodeBody {
        todo!()
    }

    /// Returns the regular file's body and the position its block list starts at.
    ///
    /// Only a regular file record states a block list, and only a regular file gets a
    /// page cache, so a caller reached through a page fill holds such a record. The
    /// accessor states that once instead of a match in every reader of the block list,
    /// and the body asserts the kind rather than answering for one the record does not
    /// carry.
    pub(in squashfs) fn file(&self) -> (&RegInode, MetadataPosition) {
        todo!()
    }

    /// Returns the kind the record's type word names.
    pub(in squashfs) fn kind(&self) -> InodeKind {
        todo!()
    }

    /// Returns the inode number this record answers to.
    pub(in squashfs) fn inode_number(&self) -> u64 {
        todo!()
    }
}
