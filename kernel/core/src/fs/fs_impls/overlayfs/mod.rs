// SPDX-License-Identifier: MPL-2.0

mod mount;
mod projection;
mod readdir_index;
mod copyup;
mod metadata_security;
mod dir;

/// The mutating-vs-read-only access class of an overlayfs entry (meso-05
/// shared vocabulary, revision-01 promotion).
///
/// Closed set: encodes the coarse mutating-vs-read-only class that entries
/// derive from the VFS surface (meso-04 vocabulary), replacing a boolean
/// parameter (priors `no-bool-args`). Cross-meso note: meso-04/06 may adopt
/// this vocabulary later (ledger note only; no meso-04/06 edits).
#[expect(
    dead_code,
    reason = "frozen meso-05 shared vocabulary; consumed by the Wave-4 metadata_security Creator"
)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::fs::fs_impls::overlayfs) enum AccessType {
    /// open/access/exec/metadata-read/xattr-read: no EROFS gate, no promotion.
    ReadOnly,
    /// chmod/chown/utimes, xattr set/remove: EROFS gate + `ensure_upper_authority()`.
    Mutating,
}

pub(super) fn init() {
    crate::fs::vfs::registry::register(&mount::OverlayFsType).unwrap();
}
