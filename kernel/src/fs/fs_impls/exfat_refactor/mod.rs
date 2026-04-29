// SPDX-License-Identifier: MPL-2.0

//! Refactor-owned exFAT runtime pieces.

mod bitmap;
mod boot;
mod direntry;
mod fat;
mod fs;
mod inode;
mod upcase;
mod volume;

pub(super) use fs::init;
