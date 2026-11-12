// SPDX-License-Identifier: MPL-2.0

//! Where the mount's decompression lives.
//!
//! A Squashfs image stores its blocks compressed, and every block is independent of its neighbours:
//! a reader can ask for one block's decompressed bytes without having read the block before it.
//! The readers need one operation:
//! a mount-wide handle turns a stored block into the bytes it was written from.
//!
//! Which codec sits behind that operation,
//! and whether it keeps any state between calls, is not settled here.
//! The value keeps the image's own compression id,
//! so that the choice can be checked against it once the codec is fixed,
//! and offers the one operation above;
//! nothing else about the codec leaks into this module.

#![expect(unused_variables, reason = "every body here is still `todo!()`")]
#![short_vis_path::add(squashfs)]

use crate::prelude::*;

/// Owner: fs::Squashfs. Users: MetadataCache::fetch, DataCache::fetch.
pub(in squashfs) struct Decompressor {
    compression_id: u16,
}

impl Decompressor {
    /// Caller: fs::Squashfs::open. Callee: none.
    pub(in squashfs) fn new(compression_id: u16) -> Self {
        todo!()
    }

    /// Caller: MetadataCache::fetch, DataCache::fetch. Callee: none.
    pub(in squashfs) fn decompress(&self, stored: &[u8], out: &mut [u8]) -> Result<usize> {
        todo!()
    }
}
