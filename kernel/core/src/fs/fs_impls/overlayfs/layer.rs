// SPDX-License-Identifier: MPL-2.0

#![short_vis_path::add(overlayfs)]
//! Layer stack and real-object stack types for overlayfs.
//!
//! [`Layer`] and [`LayerStack`] describe the static layer roots of one mount.
//! [`RealObjectStack`] describes the real-object composition behind one
//! logical overlay object: an optional upper object plus the ordered lower
//! objects.

use device_id::DeviceId;

use super::real::{RealObject, RealObjectKey, RealPath};
use crate::{
    fs::vfs::{file_system::FileSystem, inode::Inode, path::Path},
    prelude::*,
};

/// One pinned real layer root of an overlay mount.
#[derive(Debug)]
pub(in overlayfs) struct Layer {
    pub(in overlayfs) root_path: RealPath,
    pub(in overlayfs) fs: Arc<dyn FileSystem>,
    /// Per-unique-underlying-superblock identifier assigned at assembly.
    pub(in overlayfs) fsid: u64,
    /// `st_dev` of the layer root, used for same-filesystem comparisons.
    pub(in overlayfs) container_dev_id: DeviceId,
}

impl Layer {
    /// Builds the upper-layer (index 0) real object for `child_path`.
    pub(in overlayfs) fn child_real_object(&self, child_path: &Path) -> RealObject {
        RealObject::from_layer_path(0, child_path, self.fsid, self.container_dev_id)
    }
}

/// The ordered, immutable layer stack of an overlay mount.
#[derive(Debug)]
pub(in overlayfs) struct LayerStack {
    pub(super) upper: Option<Layer>,
    pub(super) lowers: Vec<Layer>,
}

impl LayerStack {
    /// Rejects an overlap between `new` and every already-assembled layer root.
    ///
    /// - Same directory: identical dentry or inode objects.
    /// - Ancestor/descendant: one root lies within the other's hierarchy.
    /// - Mount boundary: parent chains never cross a mount root.
    ///
    /// Only layer roots are compared, so legal nested subdirectories are never rejected;
    /// violations return `EINVAL`.
    pub(super) fn validate_layer_overlap(new: &Layer, others: &[&Layer]) -> Result<()> {
        let new_path = new.root_path.upgrade()?;
        let new_dentry = new_path.dentry();
        for other in others {
            let other_path = other.root_path.upgrade()?;
            let other_dentry = other_path.dentry();
            if Arc::ptr_eq(new_dentry, other_dentry)
                || Arc::ptr_eq(new_path.inode(), other_path.inode())
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

    /// Returns the writable upper layer, or `EROFS` when the stack has none.
    pub(in overlayfs) fn upper_layer(&self) -> Result<&Layer> {
        self.upper.as_ref().ok_or_else(|| {
            Error::with_message(Errno::EROFS, "the overlay mount has no upper layer")
        })
    }

    /// Returns the ordered lower layers.
    pub(in overlayfs) fn lower_layers(&self) -> &[Layer] {
        &self.lowers
    }

    /// Rejects a workdir root that is the same as, an ancestor of, or a
    /// descendant of any lower layer root.
    ///
    /// The workdir is not a layer, so [`LayerStack::validate_layer_overlap`]
    /// cannot cover it; a nested workdir would place the staging workspace
    /// inside the lower tree. Violations return `EINVAL`.
    pub(super) fn validate_workdir_against_lowers(&self, workdir_path: &Path) -> Result<()> {
        let workdir_dentry = workdir_path.dentry();
        for lower in &self.lowers {
            let lower_path = lower.root_path.upgrade()?;
            let lower_dentry = lower_path.dentry();
            if Arc::ptr_eq(lower_dentry, workdir_dentry)
                || Arc::ptr_eq(lower_path.inode(), workdir_path.inode())
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

    /// Converts a copy-up origin layer index to the configured lower index.
    ///
    /// `layer_index()` counts the upper as position 0, so when the stack has
    /// an upper the origin's own lower position is `layer_index - 1`; both
    /// out-of-range forms fail with `EINVAL`.
    pub(in overlayfs) fn lower_layer_root_ino_for_origin(&self, layer_index: usize) -> Result<u64> {
        let lower_index = if self.upper.is_some() {
            layer_index.checked_sub(1).ok_or_else(|| {
                Error::with_message(
                    Errno::EINVAL,
                    "the origin source does not identify a configured lower layer",
                )
            })?
        } else {
            layer_index
        };
        let lower_layer = self.lowers.get(lower_index).ok_or_else(|| {
            Error::with_message(
                Errno::EINVAL,
                "the origin source does not identify a configured lower layer",
            )
        })?;
        Ok(lower_layer.root_path.upgrade()?.inode().ino())
    }
}

/// The real-object composition behind one logical overlay object.
///
/// Invariant: `upper.is_some() || !lowers.is_empty()`, enforced by the
/// construction paths.
#[derive(Clone, Debug)]
pub(in overlayfs) struct RealObjectStack {
    /// The upper real object; the visible-metadata source for merged
    /// directories.
    pub(super) upper: Option<RealObject>,
    /// The lower stack, topmost first; non-empty for lower-only/merged
    /// objects.
    pub(super) lowers: Vec<RealObject>,
}

impl RealObjectStack {
    /// Returns whether this stack represents a merged directory view.
    ///
    /// A stack is merged when it has both an upper and lower contribution, or
    /// when more than one lower layer contributes.
    pub(in overlayfs) fn is_merged(&self) -> bool {
        (self.upper.is_some() && !self.lowers.is_empty()) || self.lowers.len() > 1
    }

    /// Returns the visible-metadata source: the upper real object when present,
    /// else the topmost lower (`lowers[0]`).
    ///
    /// Precondition: `upper.is_some() || !lowers.is_empty()`.
    pub(in overlayfs) fn visible_source(&self) -> &RealObject {
        match &self.upper {
            Some(upper) => upper,
            None => &self.lowers[0],
        }
    }

    /// Returns the cache key derived from the visible-metadata source.
    pub(in overlayfs) fn key(&self) -> RealObjectKey {
        RealObjectKey::from_source(self.visible_source())
    }

    /// Returns whether `real_inode` is the same logical object as this
    /// stack's visible source or any of its retained lowers.
    pub(in overlayfs) fn contains_real_inode(&self, real_inode: &Arc<dyn Inode>) -> bool {
        Arc::ptr_eq(self.visible_source().real_inode(), real_inode)
            || self
                .lowers
                .iter()
                .any(|lower| Arc::ptr_eq(lower.real_inode(), real_inode))
    }

    /// Returns whether `self` and `other` describe the same physical layer
    /// composition by durable value identity (`fsid` + real inode number),
    /// not by cached inode pointer identity.
    pub(in overlayfs) fn same_real_object_stack(&self, other: &Self) -> bool {
        let same_upper = match (self.upper.as_ref(), other.upper.as_ref()) {
            (Some(left), Some(right)) => {
                left.fsid() == right.fsid() && left.real_inode().ino() == right.real_inode().ino()
            }
            (None, None) => true,
            _ => false,
        };
        same_upper
            && self.lowers.len() == other.lowers.len()
            && self
                .lowers
                .iter()
                .zip(other.lowers.iter())
                .all(|(left, right)| {
                    left.fsid() == right.fsid()
                        && left.real_inode().ino() == right.real_inode().ino()
                })
    }

    /// Returns the current real authority for one delegated call.
    pub(in overlayfs) fn select_real_inode(&self) -> Arc<dyn Inode> {
        self.visible_source().real_inode().clone()
    }
}
