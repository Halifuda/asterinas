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

/// Produced by: InodeRecord::read. Users: SquashfsInode, fs::Squashfs.
pub(in squashfs) enum InodeBody {
    /// Owner: InodeBody. Users: InodeRecord::file.
    File {
        /// Owner: InodeBody. Users: InodeRecord::file.
        inode: RegInode,
        /// Owner: InodeBody. Users: InodeRecord::file.
        block_list: MetadataPosition,
    },
    /// Owner: InodeBody. Users: InodeRecord::body.
    Dir {
        /// Owner: InodeBody. Users: InodeRecord::body.
        inode: DirInode,
    },
}

/// Produced by: InodeRecord::read. Users: SquashfsInode, fs::Squashfs.
pub(in squashfs) struct InodeRecord {
    base: BaseInode,
    body: InodeBody,
}

impl InodeRecord {
    /// Caller: fs::Squashfs::open, SquashfsInode::lookup_impl. Callee: BaseInode::decode,
    /// RegInode::decode, DirInode::decode, InodeKind::from_inode_type.
    pub(in squashfs) fn read(
        cursor: &mut MetadataCursor<'_>,
        super_block: &SuperBlock,
    ) -> Result<Self> {
        todo!()
    }

    /// Caller: SquashfsInode::metadata_impl, mode_impl, atime_impl, mtime_impl,
    /// ctime_impl. Callee: none.
    pub(in squashfs) fn base(&self) -> &BaseInode {
        todo!()
    }

    /// Caller: SquashfsInode::type_impl. Callee: none.
    pub(in squashfs) fn body(&self) -> &InodeBody {
        todo!()
    }

    /// Caller: SquashfsInode::read_page_async, size_impl. Callee: none.
    pub(in squashfs) fn file(&self) -> (&RegInode, MetadataPosition) {
        todo!()
    }

    /// Caller: SquashfsInode::type_impl. Callee: BaseInode::inode_type.
    pub(in squashfs) fn kind(&self) -> InodeKind {
        todo!()
    }

    /// Caller: SquashfsInode::ino_impl, fs::Squashfs::root_inode_impl.
    /// Callee: BaseInode::inode_number.
    pub(in squashfs) fn inode_number(&self) -> u64 {
        todo!()
    }
}
