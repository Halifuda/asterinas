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
mod volume;

pub(super) use fs::init;

fn device_io() -> Error {
    Error::new(Errno::EIO)
}

fn inconsistent_bitmap_accounting() -> Error {
    Error::with_message(Errno::EUCLEAN, "exFAT bitmap accounting mismatch")
}

fn invalid_mount_input() -> Error {
    Error::new(Errno::EINVAL)
}

fn invalid_on_disk_layout() -> Error {
    Error::with_message(Errno::EUCLEAN, "corrupt exFAT on-disk layout")
}

fn invalid_operation_input() -> Error {
    Error::new(Errno::EINVAL)
}

fn no_space() -> Error {
    Error::new(Errno::ENOSPC)
}

fn read_only_conflict() -> Error {
    Error::new(Errno::EROFS)
}

fn not_mounted() -> Error {
    Error::with_message(Errno::EINVAL, "filesystem is not mounted")
}

fn unsupported_remount_delta() -> Error {
    Error::with_message(Errno::EINVAL, "unsupported exFAT remount delta")
}
