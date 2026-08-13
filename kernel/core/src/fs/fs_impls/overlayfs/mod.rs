// SPDX-License-Identifier: MPL-2.0

//! Overlayfs filesystem implementation for Asterinas.
//!
//! This module is the entry point for overlay filesystem support: `init`
//! registers `mount::OverlayFsType`, after which the VFS can mount overlays
//! and access them through the standard filesystem trait interfaces. A mount
//! merges one writable upper layer with one or more read-only lower layers;
//! [`AccessType`] classifies each projected request as read-only or mutating
//! for permission checks and copy-up triggering.
//!
//! # Module structure
//!
//! | Module | Responsibility |
//! |---|---|
//! | `mount` | Mount options, layer-stack assembly, claims, and the `OverlayFs`/`OverlayFsType` carriers. |
//! | `projection` | Upper-first lookup, identity projection, and the overlay inode. |
//! | `dir` | Namespace mutation (create/remove/link/rename) and whiteouts. |
//! | `copyup` | Copy-up coordination, trigger, and workdir promotion. |
//! | `metadata_security` | Permission checks and overlay xattr policy. |
//! | `readdir_index` | Per-directory merged readdir index. |
//!
//! # References
//!
//! - Overlay filesystem:
//!   <https://www.kernel.org/doc/html/latest/filesystems/overlayfs.html>

#![short_vis_path::add(overlayfs)]

mod copyup;
mod dir;
mod metadata_security;
mod mount;
mod projection;
mod readdir_index;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in overlayfs) enum AccessType {
    ReadOnly,
    Mutating,
}

pub(super) fn init() {
    crate::fs::vfs::registry::register(&mount::OverlayFsType).unwrap();
}
