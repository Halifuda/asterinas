// SPDX-License-Identifier: MPL-2.0

//! The inode the VFS sees, and the page cache behind a regular file.
//!
//! # Opening an inode
//!
//! ```text
//! open(2)
//!   |
//!   v
//! path resolution: lookup_child(name)
//!   |
//!   +- ... side: dentry cache hit plus revalidation -> reuse the dentry (no Squashfs work)
//!   |
//!   v
//! SquashfsInode::lookup_impl(name)
//!   |
//!   v
//! DirCursor::new(dir_inode, start = 3)   // skips the synthesized . and ..
//!   |
//!   v
//! next_entry()
//!   |
//!   +- ... side: directory index i_count (a later increment) -> jump to the target block
//!   +- ... side: sorted-name early exit -> stop on the first greater name (not implemented)
//!   +- ... side: bad entry -> lookup fails with EIO, a listing truncates silently
//!   |
//!   v
//! entry matched: InodeRef = <start_block, offset>, ino = base + i16 delta, kind
//!   |
//!   v
//! InodeCache::lookup(ino)
//!   |
//!   +- ... side: hit -> reuse the same inode object (flock and fsnotify state stay whole)
//!   |
//!   v
//! InodeRecord::read(MetadataCursor, &super_block)
//!   |      +- inode table blocks come from MetadataCache (decompressed once)
//!   v
//! Arc::new_cyclic(SquashfsInode)
//!   |
//!   +- ... side: REG builds a PageCache(size, Weak<Self>); DIR builds none
//!   |
//!   v
//! InodeCache::insert(ino) -> winning instance
//!   |
//!   v
//! VFS builds the dentry and the file object; FileOps = SquashfsInode -> fd
//!   ... side: readdir reuses DirCursor and resumes from the caller's offset
//! ```
//!
//! A regular file's page cache is backed by the inode itself.
//! Because the cache holds its inode weakly,
//! a page fault that outlives the inode fails to upgrade,
//! instead of keeping a dead inode alive.
//! The rest of a page fault's path is drawn in [`super::read`].
//!
//! The `*_impl` methods here are what [`super::impl_for_vfs`] forwards to:
//! each answer is a fact about this inode rather than about the image or the mount.

#![expect(unused_variables, reason = "every body here is still `todo!()`")]
#![short_vis_path::add(squashfs)]

use core::time::Duration;

use io_util::batch::IoBatch;

use super::{format::inode::RegInode, fs::Squashfs, read::inode_record::InodeRecord};
use crate::{
    fs::{
        file::{InodeMode, InodeType, StatusFlags},
        utils::DirentVisitor,
        vfs::{
            file_system::FileSystem,
            inode::{Extension, Inode, Metadata},
        },
    },
    prelude::*,
    process::{Gid, Uid},
    vm::page_cache::{LockedCachePage, PageCache, PageCacheBackend, Vmo},
};

/// Produced by: SquashfsInode::new. Users: InodeCache, impl_for_vfs.
pub(in squashfs) struct SquashfsInode {
    record: InodeRecord,
    page_cache: Option<RwMutex<PageCache>>,
    fs: Weak<Squashfs>,
    extension: Extension,
}

impl SquashfsInode {
    /// Caller: fs::Squashfs::root_inode_impl, lookup_impl. Callee: PageCache::new_with_backend.
    pub(in squashfs) fn new(
        record: InodeRecord,
        fs: Weak<Squashfs>,
        self_weak: &Weak<SquashfsInode>,
    ) -> Result<Self> {
        todo!()
    }

    /// Caller: SquashfsInode::read_page_async, size_impl. Callee: none.
    fn record(&self) -> &InodeRecord {
        todo!()
    }

    /// Caller: SquashfsInode::fs_impl, owner_impl, group_impl, metadata_impl.
    /// Callee: Weak::upgrade.
    fn mount(&self) -> Arc<Squashfs> {
        self.fs
            .upgrade()
            .expect("the mount outlives every inode of it")
    }

    /// Caller: Inode::size via impl_for_vfs. Callee: InodeRecord::file.
    pub(in squashfs) fn size_impl(&self) -> usize {
        todo!()
    }

    /// Caller: Inode::resize via impl_for_vfs. Callee: none.
    pub(in squashfs) fn resize_impl(&self, _new_size: usize) -> Result<()> {
        todo!()
    }

    /// Caller: Inode::metadata via impl_for_vfs. Callee: InodeRecord::base, IdTable::value.
    pub(in squashfs) fn metadata_impl(&self) -> Result<Metadata> {
        todo!()
    }

    /// Caller: Inode::ino via impl_for_vfs. Callee: InodeRecord::inode_number.
    pub(in squashfs) fn ino_impl(&self) -> u64 {
        todo!()
    }

    /// Caller: Inode::type_ via impl_for_vfs. Callee: InodeRecord::kind.
    pub(in squashfs) fn type_impl(&self) -> InodeType {
        todo!()
    }

    /// Caller: Inode::mode via impl_for_vfs. Callee: InodeRecord::base.
    pub(in squashfs) fn mode_impl(&self) -> Result<InodeMode> {
        todo!()
    }

    /// Caller: Inode::set_mode via impl_for_vfs. Callee: none.
    pub(in squashfs) fn set_mode_impl(&self, _mode: InodeMode) -> Result<()> {
        todo!()
    }

    /// Caller: Inode::owner via impl_for_vfs. Callee: IdTable::value.
    pub(in squashfs) fn owner_impl(&self) -> Result<Uid> {
        todo!()
    }

    /// Caller: Inode::set_owner via impl_for_vfs. Callee: none.
    pub(in squashfs) fn set_owner_impl(&self, _uid: Uid) -> Result<()> {
        todo!()
    }

    /// Caller: Inode::group via impl_for_vfs. Callee: IdTable::value.
    pub(in squashfs) fn group_impl(&self) -> Result<Gid> {
        todo!()
    }

    /// Caller: Inode::set_group via impl_for_vfs. Callee: none.
    pub(in squashfs) fn set_group_impl(&self, _gid: Gid) -> Result<()> {
        todo!()
    }

    /// Caller: Inode::atime via impl_for_vfs. Callee: InodeRecord::base.
    pub(in squashfs) fn atime_impl(&self) -> Duration {
        todo!()
    }

    /// Caller: Inode::set_atime via impl_for_vfs. Callee: none.
    pub(in squashfs) fn set_atime_impl(&self, _time: Duration) {
        todo!()
    }

    /// Caller: Inode::mtime via impl_for_vfs. Callee: InodeRecord::base.
    pub(in squashfs) fn mtime_impl(&self) -> Duration {
        todo!()
    }

    /// Caller: Inode::set_mtime via impl_for_vfs. Callee: none.
    pub(in squashfs) fn set_mtime_impl(&self, _time: Duration) {
        todo!()
    }

    /// Caller: Inode::ctime via impl_for_vfs. Callee: InodeRecord::base.
    pub(in squashfs) fn ctime_impl(&self) -> Duration {
        todo!()
    }

    /// Caller: Inode::set_ctime via impl_for_vfs. Callee: none.
    pub(in squashfs) fn set_ctime_impl(&self, _time: Duration) {
        todo!()
    }

    /// Caller: Inode::page_cache via impl_for_vfs. Callee: PageCache::as_vmo.
    pub(in squashfs) fn page_cache_impl(&self) -> Option<Arc<Vmo>> {
        todo!()
    }

    /// Caller: Inode::lookup via impl_for_vfs. Callee: DirCursor::new, next_entry,
    /// InodeCache::lookup, insert, InodeRecord::read.
    pub(in squashfs) fn lookup_impl(&self, name: &str) -> Result<Arc<dyn Inode>> {
        todo!()
    }

    /// Caller: FileOps::readdir_at via impl_for_vfs. Callee: DirCursor::new, next_entry.
    pub(in squashfs) fn readdir_impl(
        &self,
        offset: usize,
        visitor: &mut dyn DirentVisitor,
    ) -> Result<usize> {
        todo!()
    }

    /// Caller: FileOps::read_at via impl_for_vfs. Callee: none.
    pub(in squashfs) fn read_at_impl(
        &self,
        offset: usize,
        writer: &mut VmWriter,
        status_flags: StatusFlags,
    ) -> Result<usize> {
        todo!()
    }

    /// Caller: FileOps::write_at via impl_for_vfs. Callee: none.
    pub(in squashfs) fn write_at_impl(
        &self,
        _offset: usize,
        _reader: &mut VmReader,
        _status_flags: StatusFlags,
    ) -> Result<usize> {
        todo!()
    }

    /// Caller: Inode::fs via impl_for_vfs. Callee: SquashfsInode::mount.
    pub(in squashfs) fn fs_impl(&self) -> Arc<dyn FileSystem> {
        todo!()
    }

    /// Caller: Inode::extension via impl_for_vfs. Callee: none.
    pub(in squashfs) fn extension_impl(&self) -> &Extension {
        todo!()
    }
}

impl PageCacheBackend for SquashfsInode {
    /// Caller: the page cache. Callee: SquashfsInode::record, InodeRecord::file,
    /// RegInode::fragment, MetaIndex::position, BlockListCursor::new, StoredBlock::load.
    fn read_page_async(
        &self,
        idx: usize,
        locked_page: LockedCachePage,
        io_batch: &mut IoBatch,
    ) -> Result<()> {
        let (file, _block_list) = self.record().file();
        debug_assert_eq!(
            file.fragment(),
            RegInode::NO_FRAGMENT_MARKER,
            "a page is filled only for a file whose tail is in the data area"
        );
        // Block = idx * PAGE_SIZE / block_size; the page starts at the remainder.
        todo!()
    }

    /// Caller: the page cache. Callee: none.
    fn write_page_async(
        &self,
        _idx: usize,
        _locked_page: LockedCachePage,
        _io_batch: &mut IoBatch,
    ) -> Result<()> {
        todo!()
    }
}
