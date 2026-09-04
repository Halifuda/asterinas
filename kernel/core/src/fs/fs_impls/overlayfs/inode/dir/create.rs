// SPDX-License-Identifier: MPL-2.0

//! The create-object recipe family.
//!
//! One dispatcher routes on the fresh layer lookup of the target name: an
//! absent or opaque-hidden name creates directly in the upper parent, a
//! whiteout-hidden name replaces the whiteout through a completely prepared
//! private workdir temp, and a fresh positive target fails with `ESTALE`.
//!
use crate::{
    fs::{
        file::{InodeMode, InodeType},
        fs_impls::overlayfs::{
            inode::{
                Lookup, NegativeLookup, OverlayInode, ProjectionBinding, ReaddirIndex,
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
            Lookup::Negative(NegativeLookup::Absent) => {
                self.create_upper_only(name, type_, mode, mknod_type, index)
            }
            Lookup::Negative(NegativeLookup::HiddenByWhiteout) => {
                self.create_over_whiteout(name, type_, mode, mknod_type, index)
            }
            Lookup::Positive(_) => Err(Error::new(Errno::ESTALE)),
        }
    }

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
        let parent_arc = self.cached_self_arc()?;
        let inode = fs.project_inode(
            &new_facts,
            ProjectionBinding::Child {
                parent: &parent_arc,
                name,
            },
        );
        self.readdir_index_insert(name, inode.clone(), object_type, index);
        Ok(inode)
    }

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
                // Part of complete publication: the opaque marker is written
                // on the temp before the atomic swap, so the whiteout is
                // never deleted first.
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
            let upper_layer = fs.layer_stack().upper_layer()?;
            let new_facts =
                RealObjectStack::upper_only(upper_layer.child_real_object(&published_path));
            let parent_arc = self.cached_self_arc()?;
            let inode = fs.project_inode(
                &new_facts,
                ProjectionBinding::Child {
                    parent: &parent_arc,
                    name,
                },
            );
            self.readdir_index_insert(name, inode.clone(), object_type, index);
            Ok(inode)
        })();
        match result {
            Ok(inode) => Ok(inode),
            Err(err) => {
                if committed {
                    self.invalidate_readdir_index(index);
                } else {
                    let _ = fs.cleanup_workdir_temp(temp.name(), temp.kind());
                }
                Err(err)
            }
        }
    }
}
