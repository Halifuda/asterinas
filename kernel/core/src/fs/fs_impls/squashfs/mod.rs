// SPDX-License-Identifier: MPL-2.0

//! A read-only Squashfs 4.0 implementation.
//!
//! A Squashfs image packs a whole directory tree into one compressed byte range,
//! and that range lives on a block device.
//! Metadata blocks hold the inode and directory tables,
//! and file data is stored in blocks whose lengths a per-file list names.
//! Nothing in the image is writable,
//! so every operation the VFS offers either reads the image or refuses.
//!
//! The dependency direction is one way:
//! the VFS layer calls down into the readers,
//! which call down into the format,
//! and no layer below refers to a type above it.
//!
//! # Module map
//!
//! | Submodule | Responsibility |
//! |---|---|
//! | [`format`] | The on-disk records, decoded from bytes without I/O or allocation |
//! | [`fs`] | The filesystem state one instance owns and the validation a mount performs |
//! | [`fs_type`] | The `squashfs` registration entry |
//! | [`impl_for_vfs`] | The `FileSystem`, `Inode`, and `FileOps` implementations |
//! | [`inode`] | One inode's state and the page cache behind a regular file |
//! | [`inode_cache`] | The per-mount inode cache that keeps one inode object per image record |
//! | [`read`] | Reading the image, its metadata stream, and the records built on them |

// Nothing below this module is reachable until `SquashfsType::create` opens an
// image, so every item of the subtree is dead code until then.
#![expect(dead_code, reason = "nothing constructs a Squashfs value yet")]

mod format;
mod fs;
mod fs_type;
mod impl_for_vfs;
mod inode;
mod inode_cache;
mod read;

pub(in crate::fs::fs_impls) use self::fs_type::init;
