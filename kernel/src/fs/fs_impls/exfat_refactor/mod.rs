// SPDX-License-Identifier: MPL-2.0

//! Experimental exFAT refactor module.
//!
//! This module is compiled into the kernel so the refactor can evolve in-tree,
//! but it is not registered as a filesystem type yet.
//! The legacy `exfat` module remains the active implementation and test baseline
//! until the refactored implementation is ready to take over deliberately.

mod boot_sector;
mod dentry;
mod fat;
mod io;
mod super_block;

#[cfg(ktest)]
mod test_support;
