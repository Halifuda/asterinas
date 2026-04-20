// SPDX-License-Identifier: MPL-2.0

//! Refactor-owned exFAT runtime pieces.

mod fs;
mod inode;
mod ondisk;

pub(super) use fs::init;
