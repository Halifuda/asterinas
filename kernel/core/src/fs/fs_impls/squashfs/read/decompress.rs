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

/// The mount's handle on decompression.
///
/// One instance serves every reader of a mount, because a block has to be
/// decompressed for any of them and a second handle would duplicate whatever
/// state the codec keeps. The id it carries is the image's statement of which
/// codec wrote its blocks; whether this build can run that codec is a question
/// for the mount's own validation, not for the readers that come after it.
pub(in squashfs) struct Decompressor {
    compression_id: u16,
}

impl Decompressor {
    /// Creates the handle for an image written with `compression_id`.
    pub(in squashfs) fn new(compression_id: u16) -> Self {
        todo!()
    }

    /// Writes one stored block's decompressed bytes into `out`.
    ///
    /// Returns how many bytes were written. `out` is sized to the length the
    /// reader expects of this block, so a result shorter than `out` means the
    /// block was stored short, and a block whose payload does not fit is an I/O
    /// error rather than a truncation.
    pub(in squashfs) fn decompress(&self, stored: &[u8], out: &mut [u8]) -> Result<usize> {
        todo!()
    }
}
