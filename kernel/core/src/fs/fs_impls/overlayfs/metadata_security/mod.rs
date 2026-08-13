// SPDX-License-Identifier: MPL-2.0

//! The module root and entry surface of the metadata, permission, and xattr
//! policy subsystem: it supplies the two shared helpers
//! ([`OverlayInode::delegate_to_real`] and [`OverlayFs::xattr_policy`])
//! across the three layers it gathers — the two-stage permission pipeline,
//! metadata setters, and xattr-name classification and entry bounds.
//!

use self::xattr::OverlayXattrPolicy;
use super::{mount::OverlayFs, projection::OverlayInode};
use crate::{fs::vfs::inode::Inode, prelude::*};

mod metadata;
mod permission;
pub(super) mod xattr;

impl OverlayInode {
    /// Runs the single private delegation helper of this module tree.
    ///
    /// Runs `operation_fn` against the current real authority under the
    /// mount's creator-credential scope. Precondition: the permission stage
    /// has already admitted the operation (or the entry is a pure read
    /// delegation); metadata setters whose underlying real ops do not
    /// self-evaluate additionally ran the explicit real check first.
    ///
    /// The generic `T` is deliberate: the delegated operations return
    /// heterogeneous types (`usize` for `list_xattr`, `()` for the setters,
    /// and other `Inode`-trait results), so one call helper avoids dedicated
    /// per-kind carriers.
    fn delegate_to_real<T>(
        &self,
        operation_fn: impl FnOnce(&Arc<dyn Inode>) -> Result<T>,
    ) -> Result<T> {
        let fs = self.fs_arc()?;
        let real = self.select_real_inode();
        fs.policy()
            .credential_policy()
            .with_creator_credentials_fn(|| operation_fn(&real))
    }
}

impl OverlayFs {
    pub(super) fn xattr_policy(&self) -> &OverlayXattrPolicy {
        &self.xattr_policy
    }
}
