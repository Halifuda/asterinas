// SPDX-License-Identifier: MPL-2.0

//! The on-disk records of a Squashfs 4.0 image.
//!
//! ```text
//!   image bytes
//!        |  a reader hands over exactly SIZE bytes
//!        v
//!   on-disk record        this layer: decode, field by field
//!        |
//!        v
//!   checked value         no I/O, no allocation, no state
//!        |
//!        v
//!   the reader above (read/), then the VFS
//! ```
//!
//! Every type here decodes a fixed-width record from bytes the caller already holds.
//! Nothing here performs I/O, holds a reference, or allocates:
//! a decode either hands back a checked value or returns `None`,
//! so an image's bytes never reach a reader as a half-valid structure.
//!
//! Multi-byte fields are little endian on the image,
//! and are converted with `from_le_bytes` on the way in.
//! A record's `decode` compares the length first,
//! then the record's own magic word if it has one,
//! then the field ranges the format fixes.
//!
//! Every caller hands `decode` a byte range of exactly the record's width.
//! The readers guarantee that by fetching a record's bytes whole,
//! through the metadata cursor or from the image,
//! so a range too short to hold the record never reaches a `decode`,
//! and the `None` a `decode` returns is always the record's own invalidity.
//! That is what lets a short read be reported as an I/O error,
//! while a rejected record carries the error its caller's contract fixes:
//! an invalid argument for a record the mount validates,
//! an I/O error for a block list entry a reader is walking.
//!
//! Readdir and lookup answer some field ranges with different errors;
//! those ranges are left to those readers rather than folded into `decode`,
//! which can only say "rejected".
//!
//! The layouts here restate the format's own definitions,
//! including the ones no reader of this layer repeats.
//!
//! # Module map
//!
//! | Submodule | Responsibility |
//! |---|---|
//! | `block` | The length-and-compression word in front of every stored block |
//! | `dir` | Directory headers and directory entries |
//! | `inode` | The 48-bit inode reference and the inode record bodies |
//! | `superblock` | The 96-byte superblock and the geometry it states |

#![short_vis_path::add(squashfs)]

pub(in squashfs) mod block;
pub(in squashfs) mod dir;
pub(in squashfs) mod inode;
pub(in squashfs) mod superblock;
