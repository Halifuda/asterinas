// SPDX-License-Identifier: MPL-2.0

#![short_vis_path::add(overlayfs)]
//! The layer-model types of an overlay mount.
//!
//! [`Layer`] is one pinned real directory root of the mount: the writable
//! upper (at most one) or one of the read-only lowers. Each layer is the
//! sole strong holder of a private, unregistered clone view rooted at its
//! resolved directory, so the layer root need not be the underlying mount's
//! root and stays alive for the mount's lifetime.
//!
//! [`LayerStack`] is the ordered, mount-fixed layer collection: the upper
//! first, then the lowers topmost-first, immutable after assembly. The
//! mount-time assembly and validation of these types is not part of this
//! module. [`RealObjectStack`] is the per-object half of the model: the
//! real-object composition behind one logical overlay object, with the
//! visible-metadata source defined as the upper when present, else the
//! topmost lower.

use device_id::DeviceId;

use super::real::RealObject;
use crate::{
    fs::vfs::path::{Dentry, Mount, Path},
    prelude::*,
};

#[derive(Debug)]
pub(super) struct Layer {
    pub(super) mount: Arc<Mount>,
    pub(super) fsid: u64,
    pub(super) container_dev_id: DeviceId,
}

impl Layer {
    pub(super) fn root_dentry(&self) -> &Arc<Dentry> {
        self.mount.root_dentry()
    }

    pub(super) fn child_real_object(&self, child_path: &Path) -> RealObject {
        RealObject::new(0, child_path.dentry().clone())
    }

    pub(super) fn root_path(&self) -> Path {
        Path::new(self.mount.clone(), self.root_dentry().clone())
    }
}

#[derive(Debug)]
pub(super) struct LayerStack {
    pub(super) upper: Option<Layer>,
    pub(super) lowers: Vec<Layer>,
}

impl LayerStack {
    pub(super) fn validate_layer_overlap(new: &Layer, others: &[&Layer]) -> Result<()> {
        let new_dentry = new.root_dentry();
        for other in others {
            let other_dentry = other.root_dentry();
            if Arc::ptr_eq(new_dentry, other_dentry)
                || Arc::ptr_eq(new.root_dentry().inode(), other.root_dentry().inode())
            {
                return_errno_with_message!(
                    Errno::EINVAL,
                    "overlay layer roots must be distinct directories"
                );
            }
            if new_dentry.is_equal_or_descendant_of(other_dentry)
                || other_dentry.is_equal_or_descendant_of(new_dentry)
            {
                return_errno_with_message!(
                    Errno::EINVAL,
                    "overlay layer roots must not be each other's ancestor or descendant"
                );
            }
        }
        Ok(())
    }

    pub(super) fn upper_layer(&self) -> Result<&Layer> {
        self.upper.as_ref().ok_or_else(|| {
            Error::with_message(Errno::EROFS, "the overlay mount has no upper layer")
        })
    }

    pub(super) fn lower_layers(&self) -> &[Layer] {
        &self.lowers
    }

    pub(super) fn validate_workdir_against_lowers(&self, workdir_path: &Path) -> Result<()> {
        let workdir_dentry = workdir_path.dentry();
        for lower in &self.lowers {
            let lower_dentry = lower.root_dentry();
            if Arc::ptr_eq(lower_dentry, workdir_dentry)
                || Arc::ptr_eq(lower.root_dentry().inode(), workdir_path.inode())
            {
                return_errno_with_message!(
                    Errno::EINVAL,
                    "workdir must be distinct from every lower layer root"
                );
            }
            if workdir_dentry.is_equal_or_descendant_of(lower_dentry)
                || lower_dentry.is_equal_or_descendant_of(workdir_dentry)
            {
                return_errno_with_message!(
                    Errno::EINVAL,
                    "workdir must not be an ancestor or descendant of a lower layer root"
                );
            }
        }
        Ok(())
    }

    pub(super) fn lower_layer_root_ino_for_origin(&self, layer_index: usize) -> Result<u64> {
        let lower_layer = layer_index
            .checked_sub(1)
            .and_then(|index| self.lowers.get(index))
            .ok_or_else(|| {
                Error::with_message(
                    Errno::EINVAL,
                    "the origin source does not identify a configured lower layer",
                )
            })?;
        Ok(lower_layer.root_dentry().inode().ino())
    }
}

#[derive(Clone, Debug)]
pub(super) struct RealObjectStack {
    pub(super) upper: Option<RealObject>,
    pub(super) lowers: Vec<RealObject>,
}

impl RealObjectStack {
    pub(super) fn new(upper: Option<RealObject>, lowers: Vec<RealObject>) -> Self {
        debug_assert!(upper.is_some() || !lowers.is_empty());
        Self { upper, lowers }
    }

    pub(super) fn upper_only(upper: RealObject) -> Self {
        Self {
            upper: Some(upper),
            lowers: Vec::new(),
        }
    }

    pub(super) fn lower_only(lower: RealObject) -> Self {
        Self {
            upper: None,
            lowers: vec![lower],
        }
    }

    pub(super) fn is_merged(&self) -> bool {
        (self.upper.is_some() && !self.lowers.is_empty()) || self.lowers.len() > 1
    }

    pub(super) fn visible_source(&self) -> &RealObject {
        match &self.upper {
            Some(upper) => upper,
            None => &self.lowers[0],
        }
    }
}
