// SPDX-License-Identifier: MPL-2.0

//! An opened image and the one bound every read of it obeys.
//!
//! A Squashfs image is a byte range of a block device,
//! and its superblock names that range's length.
//! Every later read of the image is bounded by that number rather than by the device's size,
//! because a device may hold more than one image,
//! and bytes past the image belong to whatever follows it.
//!
//! This module is also where a mount starts.

#![expect(unused_variables, reason = "every body here is still `todo!()`")]
#![short_vis_path::add(squashfs)]

use alloc::sync::Arc;

use aster_block::BlockDevice;

use super::super::format::superblock::{ImageOffset, SuperBlock};
use crate::prelude::*;

/// One opened image: the device it lives on and the superblock that describes it.
///
/// The filesystem owns the one `Image` of a mount and every reader borrows it, so
/// the device handle and the validated record exist once per mount. The device is
/// shared with the rest of the mount rather than copied, and it is never written:
/// the image is read-only, so a cached read of the same range always returns the
/// same bytes. Reads are checked against the image's own length instead of the
/// device's, and a read that would leave the image is reported as an I/O error
/// because the image, not the caller, is malformed.
pub(in squashfs) struct Image {
    device: Arc<dyn BlockDevice>,
    super_block: SuperBlock,
}

impl Image {
    /// Opens the image on `device` and returns it with the superblock it states.
    ///
    /// Fails with an invalid argument when the record describes no mountable image
    /// or claims more bytes than the device holds, and with an I/O error when the
    /// device cannot deliver the record's bytes.
    pub(in squashfs) fn open(device: Arc<dyn BlockDevice>) -> Result<Self> {
        // Read the record at position zero first; it states the bound every read obeys.
        todo!()
    }

    /// Returns the superblock the mount validated.
    pub(in squashfs) fn super_block(&self) -> &SuperBlock {
        todo!()
    }

    /// Reads exactly `out.len()` bytes starting at an image position.
    ///
    /// Fails with an I/O error when the range leaves the image or the device
    /// returns fewer bytes than asked for.
    pub(super) fn read_exact_at(&self, position: ImageOffset, out: &mut [u8]) -> Result<()> {
        todo!()
    }
}
