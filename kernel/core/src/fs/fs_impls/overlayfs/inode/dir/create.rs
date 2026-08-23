// SPDX-License-Identifier: MPL-2.0

//! The create-object recipes.
//!
//! This module hosts [`OverlayInode::create_object`] (dispatcher),
//! [`OverlayInode::create_upper_only`], and
//! [`OverlayInode::create_over_whiteout`] (over-whiteout/opaque branch).
//!
use crate::{
    fs::{
        file::{InodeMode, InodeType},
        fs_impls::overlayfs::{
            inode::{
                Lookup, NegativeLookup, OverlayInode, ReaddirIndex,
                copyup::workdir::WorkdirTempRequest,
            },
            layer::RealObjectStack,
            mknod_object_type,
        },
        vfs::inode::{MknodType, RenameMode},
    },
    prelude::*,
};

impl OverlayInode {
    /// Dispatches one create-family request (create/mkdir/mknod/symlink)
    /// from the fresh `(parent, name)` projection under the parent
    /// directory transaction lock.
    ///
    /// Decides on the fresh layer lookup, never the stale VFS negative
    /// dentry that triggered the call.
    pub(super) fn create_object(
        &self,
        name: &str,
        type_: InodeType,
        mode: InodeMode,
        mknod_type: Option<MknodType>,
        index: &mut Option<ReaddirIndex>,
    ) -> Result<Arc<OverlayInode>> {
        let fs = self.fs_arc()?;
        let lookup = fs.lookup(self, name)?;
        match lookup {
            Lookup::Negative(NegativeLookup::Absent | NegativeLookup::HiddenByOpaque) => {
                self.create_upper_only(name, type_, mode, mknod_type, index)
            }
            Lookup::Negative(NegativeLookup::HiddenByWhiteout) => {
                self.create_over_whiteout(name, type_, mode, mknod_type, index)
            }
            // create expects a negative target; a fresh positive means the
            // negative expectation became stale (upper changed underneath us).
            Lookup::Positive(_) => Err(Error::new(Errno::ESTALE)),
        }
    }

    /// Creates a genuinely absent object directly in the upper parent — no
    /// workdir, no whiteout.
    ///
    /// Precondition: the caller holds this parent's directory transaction
    /// lock and has already run
    /// `check_permission(AccessType::Mutating, Permission::MAY_WRITE)`.
    fn create_upper_only(
        &self,
        name: &str,
        type_: InodeType,
        mode: InodeMode,
        mknod_type: Option<MknodType>,
        index: &mut Option<ReaddirIndex>,
    ) -> Result<Arc<OverlayInode>> {
        let fs = self.fs_arc()?;
        let upper_parent_path = self.upper_parent_path()?;
        let object_type = mknod_type.as_ref().map(mknod_object_type).unwrap_or(type_);
        let new_upper_path = match mknod_type {
            Some(mknod) => upper_parent_path.mknod(name, mode, mknod)?,
            None => upper_parent_path.new_fs_child(name, type_, mode)?,
        };
        let upper_layer = fs.layer_stack().upper_layer()?;
        let new_facts = RealObjectStack::upper_only(upper_layer.child_real_object(&new_upper_path));
        let inode = fs.project_inode(&new_facts);
        self.readdir_index_insert(name, inode.clone(), object_type, index);
        Ok(inode)
    }

    /// Replaces a whiteout-hidden name with a completely prepared private
    /// workdir temp, then publishes it.
    ///
    /// A failure before the atomic upper commit best-effort-cleans the temp;
    /// a failure after the commit invalidates the parent readdir index.
    fn create_over_whiteout(
        &self,
        name: &str,
        type_: InodeType,
        mode: InodeMode,
        mknod_type: Option<MknodType>,
        index: &mut Option<ReaddirIndex>,
    ) -> Result<Arc<OverlayInode>> {
        let fs = self.fs_arc()?;
        let upper_parent_path = self.upper_parent_path()?;
        let object_type = mknod_type.as_ref().map(mknod_object_type).unwrap_or(type_);
        let temp = match &mknod_type {
            Some(node) => fs.create_workdir_temp(name, WorkdirTempRequest::Mknod { mode, node })?,
            None => {
                fs.create_workdir_temp(name, WorkdirTempRequest::Create { kind: type_, mode })?
            }
        };
        let workdir_path = self.workdir_root_path()?;
        let mut committed = false;
        let result: Result<Arc<OverlayInode>> = (|| {
            if object_type == InodeType::Dir {
                // Opaque branch: the opaque record is part of the
                // replacement directory's complete publication; the
                // marker write is gated by the private-xattr capability
                // and runs on the temp before the atomic swap — the
                // whiteout is never deleted first.
                fs.set_opaque_marker(
                    temp.inode(),
                    "the upper filesystem cannot store the opaque marker \
                     required for a directory over a whiteout",
                )?;
            }
            let published_path = if object_type.is_directory() {
                let published =
                    fs.publish_temp(&temp, &upper_parent_path, name, RenameMode::Exchange)?;
                committed = true;
                workdir_path.unlink(temp.name())?;
                published
            } else {
                let published =
                    fs.publish_temp(&temp, &upper_parent_path, name, RenameMode::Replace)?;
                committed = true;
                published
            };
            // Semantic publication: the temp path is the published object at
            // `(upper_parent_path, name)` (inode identity is stable across the
            // rename).
            let upper_layer = fs.layer_stack().upper_layer()?;
            let new_facts =
                RealObjectStack::upper_only(upper_layer.child_real_object(&published_path));
            let inode = fs.project_inode(&new_facts);
            self.readdir_index_insert(name, inode.clone(), object_type, index);
            Ok(inode)
        })();
        match result {
            Ok(inode) => Ok(inode),
            Err(err) => {
                if committed {
                    self.invalidate_readdir_index(index);
                } else {
                    // Pre-commit failure (pre-publication arm): best-effort
                    // kind-aware temp cleanup; residue is a known cleanup
                    // debt, never a visible source.
                    let _ = fs.cleanup_workdir_temp(temp.name(), temp.kind());
                }
                Err(err)
            }
        }
    }
}
