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

/// The inode cache: the instances one mount has handed out, keyed by inode number.
///
/// The key is the number a directory entry names, which is the run header's base
/// plus the entry's signed increment; the root inode is registered under the
/// number its own record states.
/// A number may appear once at most while its inode lives, and that is what makes
/// every later lookup of the same record return the same object instead of a
/// second one holding its own copy of the VFS state.
pub(in squashfs) struct InodeCache {
    entries: Mutex<BTreeMap<u64, Weak<dyn Inode>>>,
}

impl InodeCache {
    /// Creates an empty cache.
    ///
    /// A mount starts its cache empty and registers the root inode as it builds
    /// it.
    pub(in squashfs) fn new() -> Self {
        todo!()
    }

    /// Returns the live inode registered under an inode number, if there is one.
    ///
    /// An entry whose inode has been dropped is removed and reported as absent, so
    /// a stale entry never hides the construction that replaces it.
    pub(in squashfs) fn lookup(&self, inode_number: u64) -> Option<Arc<dyn Inode>> {
        todo!()
    }

    /// Registers an inode under the number it answers to and returns the instance
    /// the cache keeps.
    ///
    /// An expired entry is replaced by `inode`. When a live instance is already
    /// registered, the cache keeps that instance and drops `inode`, so the caller
    /// must return the value handed back here rather than the inode it built:
    /// both describe the same record, but only one of them may carry the record's
    /// VFS state.
    pub(in squashfs) fn insert(&self, inode_number: u64, inode: Arc<dyn Inode>) -> Arc<dyn Inode> {
        todo!()
    }
}
