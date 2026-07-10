// SPDX-License-Identifier: MPL-2.0

//! Refactor-owned exFAT runtime pieces.

use crate::prelude::*;

mod bitmap;
mod boot;
mod direntry;
mod fat;
mod fs;
mod inode;
mod upcase;

pub(super) use fs::init;

fn device_io() -> Error {
    Error::new(Errno::EIO)
}

fn inconsistent_bitmap_accounting() -> Error {
    Error::with_message(Errno::EUCLEAN, "exFAT bitmap accounting mismatch")
}

fn invalid_on_disk_layout() -> Error {
    Error::with_message(Errno::EUCLEAN, "corrupt exFAT on-disk layout")
}

fn invalid_operation_input() -> Error {
    Error::new(Errno::EINVAL)
}

fn not_mounted() -> Error {
    Error::with_message(Errno::EINVAL, "filesystem is not mounted")
}
