// SPDX-License-Identifier: MPL-2.0

//! Reading an image: its bytes, its metadata stream, and the records built on them.
//!
//! # Reading a page
//!
//! ```text
//! page fault
//!   |
//!   v
//! PageCacheBackend::read_page_async(idx, page)
//!   |
//!   +- ... side: page already up to date -> the page cache answers, this code is not reached
//!   +- idx >= ceil(size / PAGE_SIZE) -> EINVAL
//!   |
//!   v
//! record().file() -> (&RegInode, block list start)
//!   |  assert fragment == NO_FRAGMENT_MARKER (this build's boundary)
//!   v
//! block = idx * PAGE_SIZE / block_size ; in_page = idx * PAGE_SIZE % block_size
//!   |  (mount guarantees PAGE_SIZE <= block_size, so a page never spans two blocks)
//!   v
//! MetaIndex::position(inode_number, block_list, target_block, ...)
//!   |
//!   +- ... side: scaled target is zero -> None (target lies in the first step)
//!   +- ... side: every slot is held -> None (walk the list in order)
//!   +- ... side: a recorded point matches -> resume from it
//!   +- rounds: acquire (mount lock) -> fill (slot lock, reads 4-byte entries) -> release
//!   |
//!   v
//! BlockListCursor::new(start = point, or the list's first entry)
//!   |
//!   v
//! next_block()  entry: low 24 bits length | bit 24 uncompressed | bits 25-31 must be zero
//!   |
//!   +- ... side: stored_len == 0 (sparse) -> zero the page, no device read, no cache entry
//!   |
//!   v
//! DataCache::fetch(key = (image position, expected length), stored_len, is_uncompressed)
//!   |
//!   +- ... side: hit -> BlockHold (pinned, never evicted while held)
//!   |
//!   +- miss: publish Filling -- release the lock -- read stored bytes from the image
//!            |                                      |
//!            |                                      +- ... side: bit 24 set -> copy as-is
//!            |                                      +- otherwise Decompressor::decompress
//!            v
//!        length <= expected -> publish Ready, wake waiters
//!   |
//!   +- failure -> FillGuard removes the entry and wakes waiters -> EIO (nothing published)
//!   |
//!   v
//! copy [in_page .. in_page + PAGE_SIZE) into the page; zero the page tail
//!   |  ... side: EOF page is zeroed in full before the valid bytes are copied
//!   v
//! all bytes present -> mark the page up to date
//!   |  ... side: on failure the page stays out of date and a later attempt retries
//!   v
//! return
//!   ... side: readahead (page cache batch entry) has no caller today
//!   ... side: decompressing straight into the page is not done
//!   ... side: fragment tails use the same DataCache key space (a later increment)
//! ```
//!
//! # Where this layer sits
//!
//! ```text
//!   image bytes          bounded by bytes_used
//!        |  Image::read_exact_at
//!        v
//!   metadata block       decompressed once, then cached
//!        |  MetadataCursor
//!        v
//!   metadata stream      a position plus bytes crossing block boundaries
//!        |  InodeRecord, DirCursor, BlockListCursor, IdTable
//!        v
//!   validated value      record, entry, data block, or id
//!        |  super::impl_for_vfs
//!        v
//!   VFS
//! ```
//!
//! Everything that reads the image lives here.
//! The layer holds no VFS state: the image is opened here and owned by the filesystem,
//! and the metadata cache holds no image of its own, so each fetch is lent the image it reads.
//! A reader is given the opened image, one decompression handle,
//! and the shared metadata cache, and asks for records by position.
//!
//! # Module map
//!
//! | Submodule | Responsibility |
//! |---|---|
//! | `block` | [`Image`](block::Image), the opened image and the bound every read obeys |
//! | `cache` | The cache of decompressed metadata blocks |
//! | `data` | A regular file's block list and the data blocks it names |
//! | `data_cache` | The bounded cache of decompressed data and fragment blocks |
//! | `decompress` | The mount's handle on decompression |
//! | `dir` | Walking a directory's content into entries |
//! | `id` | The id table that names owners and groups |
//! | `inode_record` | Reading an inode record out of the metadata stream |
//! | `meta_index` | The index of a large file's block list, which shortens the walk to one block |
//! | `metadata` | Positions in a metadata stream and reads that cross blocks |
//!
//! The dependency direction inside the layer is one way.
//! `inode_record`, `dir`, and `id` are built on `metadata`,
//! `metadata` is built on `cache`, `decompress`, and the opened image in `block`,
//! and `data` is built on `metadata`, `data_cache`, and the resolved points of
//! `meta_index`.
//! Both caches are built on `decompress` and the opened image in `block`,
//! and `meta_index` is built on `metadata`, the block list entries in `format`,
//! and the opened image in `block`.
//! Nothing here refers to a VFS type,
//! so the VFS layer above can be replaced without touching this one.

#![short_vis_path::add(squashfs)]

pub(in squashfs) mod block;
pub(in squashfs) mod cache;
pub(in squashfs) mod data;
pub(in squashfs) mod data_cache;
pub(in squashfs) mod decompress;
pub(in squashfs) mod dir;
pub(in squashfs) mod id;
pub(in squashfs) mod inode_record;
pub(in squashfs) mod meta_index;
pub(in squashfs) mod metadata;
