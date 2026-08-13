// SPDX-License-Identifier: MPL-2.0

//! The create-object recipes.
//!
//! This module hosts [`OverlayInode::create_object`] (dispatcher),
//! [`OverlayInode::create_upper_only`], and
//! [`OverlayInode::create_over_whiteout`] (over-whiteout/opaque branch).
//!
//! Lock contract: the caller holds the parent directory transaction lock;
//! this module enters the per-object copy-up coordination lock only through
//! the copy-up step of `check_permission`, and never touches the whiteout
//! cache lock.

use crate::{
    fs::{
        file::{InodeMode, InodeType, Permission},
        fs_impls::overlayfs::{
            AccessType,
            copyup::WorkdirTempRequest,
            metadata_security::xattr::{OPAQUE_MARKER_VALUE, OPAQUE_XATTR_FULL_NAME},
            mount::{OverlayFs, RealPath},
            projection::{
                Binding, NegativeBinding, OverlayInode, OverlayObjectFacts, PositiveBinding,
                PositiveKind, RealObject,
            },
        },
        vfs::{
            inode::{MknodType, RenameMode},
            xattr::{XattrName, XattrSetFlags},
        },
    },
    prelude::*,
};

impl OverlayInode {
    /// Dispatches one create-family request (create/mkdir/mknod/symlink)
    /// from the fresh `(parent, name)` projection under the parent
    /// directory transaction lock.
    ///
    /// Decides on current `BindingCache` evidence, never the stale VFS
    /// negative dentry that triggered the call.
    pub(super) fn create_object(
        &self,
        name: &str,
        type_: InodeType,
        mode: InodeMode,
        mknod_type: Option<MknodType>,
    ) -> Result<Arc<OverlayInode>> {
        let fs = self.fs_arc()?;
        let parent_facts = self.facts_snapshot();
        let binding = fs.lookup_binding(&parent_facts, name)?.binding;
        match binding {
            Binding::Negative(NegativeBinding::Absent)
            | Binding::Negative(NegativeBinding::HiddenByOpaque(_)) => {
                self.create_upper_only(name, type_, mode, mknod_type)
            }
            Binding::Negative(NegativeBinding::HiddenByWhiteout(_)) => {
                self.create_over_whiteout(name, type_, mode, mknod_type)
            }
            Binding::Positive(_) => Err(Error::with_message(
                Errno::EEXIST,
                "the overlay target already exists and is visible",
            )),
        }
    }

    /// Creates a genuinely absent object directly in the upper parent — no
    /// workdir, no whiteout.
    fn create_upper_only(
        &self,
        name: &str,
        type_: InodeType,
        mode: InodeMode,
        mknod_type: Option<MknodType>,
    ) -> Result<Arc<OverlayInode>> {
        // The entry already ran the EROFS and local DAC permission checks.
        self.check_permission(AccessType::Mutating, Permission::MAY_WRITE)?;
        let fs = self.fs_arc()?;
        let upper_parent_path = self.upper_parent_path()?;
        let object_type = mknod_type
            .as_ref()
            .map(super::mknod_object_type)
            .unwrap_or(type_);
        let new_upper_path = match mknod_type {
            Some(mknod) => upper_parent_path.mknod(name, mode, mknod)?,
            None => upper_parent_path.new_fs_child(name, type_, mode)?,
        };
        let upper_layer = fs.layer_stack().upper.as_ref().ok_or_else(|| {
            Error::with_message(Errno::EROFS, "the overlay mount has no upper layer")
        })?;
        let new_facts = OverlayObjectFacts::try_new(
            PositiveKind::Single,
            Some(RealObject::with_path(
                0,
                RealPath::from_path(&new_upper_path),
                upper_layer.fsid,
                upper_layer.container_dev_id,
            )),
            Vec::new(),
        )
        .ok_or_else(|| {
            Error::with_message(
                Errno::EIO,
                "the new upper object facts are not constructible",
            )
        })?;
        let inode = fs.project_new_upper(&new_facts);
        self.publish_positive_binding(&fs, name, inode.clone(), object_type);
        Ok(inode)
    }

    /// Replaces a whiteout-hidden name with a completely prepared private
    /// workdir temp, then publishes it.
    ///
    /// A failure before the atomic upper commit best-effort-cleans the temp;
    /// a failure after the commit reconciles the affected `(parent, name)`
    /// projection as a unit via the shared
    /// [`OverlayInode::invalidate_stale_cache`] entry.
    fn create_over_whiteout(
        &self,
        name: &str,
        type_: InodeType,
        mode: InodeMode,
        mknod_type: Option<MknodType>,
    ) -> Result<Arc<OverlayInode>> {
        self.check_permission(AccessType::Mutating, Permission::MAY_WRITE)?;
        let fs = self.fs_arc()?;
        let upper_parent_path = self.upper_parent_path()?;
        let object_type = mknod_type
            .as_ref()
            .map(super::mknod_object_type)
            .unwrap_or(type_);
        let temp = match &mknod_type {
            Some(node) => fs.create_workdir_temp(
                name,
                &upper_parent_path,
                WorkdirTempRequest::Mknod { mode, node },
            )?,
            None => fs.create_workdir_temp(
                name,
                &upper_parent_path,
                WorkdirTempRequest::Create { kind: type_, mode },
            )?,
        };
        let temp_kind = temp.kind();
        let (temp_name, temp) = temp.into_parts();
        let workdir_path = self.workdir_root_path()?;
        self.run_recipe(
            &fs,
            Some((&temp_name, temp_kind)),
            || self.invalidate_stale_cache(&[(self, name)]),
            |marker| {
                if object_type == InodeType::Dir {
                    // Opaque branch: the opaque record is part of the
                    // replacement directory's complete publication; the
                    // marker write is gated by the private-xattr capability
                    // and runs on the temp before the atomic swap — the
                    // whiteout is never deleted first.
                    let can_store_private_xattr = fs
                        .policy()
                        .upper_capabilities()
                        .is_some_and(|caps| caps.can_store_private_xattr());
                    if !can_store_private_xattr {
                        return Err(Error::with_message(
                            Errno::EOPNOTSUPP,
                            "the upper filesystem cannot store the opaque marker \
                             required for a directory over a whiteout",
                        ));
                    }
                    let marker_name = XattrName::try_from_full_name(OPAQUE_XATTR_FULL_NAME)
                        .ok_or_else(|| {
                            Error::with_message(
                                Errno::EINVAL,
                                "invalid overlay opaque marker xattr name",
                            )
                        })?;
                    let mut marker_reader = VmReader::from(OPAQUE_MARKER_VALUE).to_fallible();
                    temp.set_xattr(
                        marker_name,
                        &mut marker_reader,
                        XattrSetFlags::CREATE_OR_REPLACE,
                    )?;
                }
                if object_type.is_directory() {
                    workdir_path.rename(
                        &temp_name,
                        &upper_parent_path,
                        name,
                        RenameMode::Exchange,
                    )?;
                    marker.commit();
                    workdir_path.unlink(&temp_name)?;
                } else {
                    workdir_path.rename(
                        &temp_name,
                        &upper_parent_path,
                        name,
                        RenameMode::Replace,
                    )?;
                    marker.commit();
                }
                // Semantic publication: the temp handle is the published object at
                // `(upper_parent_path, name)` (inode identity is stable across the
                // rename).
                let upper_layer = fs.layer_stack().upper.as_ref().ok_or_else(|| {
                    Error::with_message(Errno::EROFS, "the overlay mount has no upper layer")
                })?;
                let new_facts = OverlayObjectFacts::try_new(
                    PositiveKind::Single,
                    Some(RealObject::with_path(
                        0,
                        RealPath::from_path(&temp),
                        upper_layer.fsid,
                        upper_layer.container_dev_id,
                    )),
                    Vec::new(),
                )
                .ok_or_else(|| {
                    Error::with_message(
                        Errno::EIO,
                        "the new upper object facts are not constructible",
                    )
                })?;
                let inode = fs.project_new_upper(&new_facts);
                self.publish_positive_binding(&fs, name, inode.clone(), object_type);
                Ok(inode)
            },
        )
    }

    fn publish_positive_binding(
        &self,
        fs: &OverlayFs,
        name: &str,
        inode: Arc<OverlayInode>,
        kind: InodeType,
    ) {
        fs.publish_binding(
            &self.key(),
            name,
            Binding::Positive(PositiveBinding::new(inode.clone())),
        );
        self.readdir_index_insert(name, inode, kind);
    }
}
