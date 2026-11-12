// SPDX-License-Identifier: MPL-2.0

//! The inode cache of one mount: the inode instances it has handed out.
//!
//! One image record can be reached through several directory entries, and two
//! concurrent first lookups can both read it before either has finished, so
//! construction alone cannot keep one inode per record.
//! The VFS attaches state to the inode object rather than to the record: the flock
//! list, the range locks, and the fsnotify publisher all hang off the object.
//! Two objects for one record would split that state, so this cache keeps at most
//! one live instance per inode number.
//!
//! The protocol is the one a kernel inode cache uses: a lookup asks the cache
//! first, and a construction registers its result afterwards and then keeps
//! whatever instance the cache holds. The first instance to be registered wins;
//! the loser is dropped, and the caller must hand the VFS the winner it was given
//! back.
//!
//! Entries are weak, so the cache never keeps an inode, its page cache, or the
//! mount alive, and an entry whose inode has been dropped is replaced the next
//! time its number is looked up.
//! The cache belongs to one mount and dies with it.

#![expect(unused_variables, reason = "every body here is still `todo!()`")]
#![short_vis_path::add(squashfs)]

use alloc::{
    collections::BTreeMap,
    sync::{Arc, Weak},
};

use crate::{fs::vfs::inode::Inode, prelude::*};

/// Owner: fs::Squashfs. Users: SquashfsInode::lookup_impl, Squashfs::root_inode_impl.
pub(in squashfs) struct InodeCache {
    entries: Mutex<BTreeMap<u64, Weak<dyn Inode>>>,
}

impl InodeCache {
    /// Caller: fs::Squashfs::open. Callee: none.
    pub(in squashfs) fn new() -> Self {
        todo!()
    }

    /// Caller: SquashfsInode::lookup_impl, fs::Squashfs::root_inode_impl. Callee: none.
    pub(in squashfs) fn lookup(&self, inode_number: u64) -> Option<Arc<dyn Inode>> {
        todo!()
    }

    /// Caller: SquashfsInode::lookup_impl, fs::Squashfs::root_inode_impl. Callee: none.
    pub(in squashfs) fn insert(&self, inode_number: u64, inode: Arc<dyn Inode>) -> Arc<dyn Inode> {
        todo!()
    }
}
