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

/// One pinned real layer root of an overlay mount.
///
/// The layer is the sole strong holder of its private clone view; the view
/// is unregistered from every mount namespace (an empty
/// `Weak<MountNamespace>` was passed to `Mount::clone_mount`), so it is
/// reachable only through this layer and its root dentry is the resolved
/// layer root. The view inherits the flags of the mount the layer root was
/// resolved through; lower layers stay read-only by overlay
/// self-discipline — the overlay never issues mutations through a lower
/// view (directly modifying underlying layers is undefined behavior;
/// see `Documentation/filesystems/overlayfs.rst`).
#[derive(Debug)]
pub(super) struct Layer {
    /// The layer's private, unregistered clone view; its root dentry is the
    /// layer root.
    pub(super) mount: Arc<Mount>,
    /// Per-unique-underlying-superblock identifier assigned at assembly.
    pub(super) fsid: u64,
    /// `st_dev` of the layer root, used for same-filesystem comparisons.
    pub(super) container_dev_id: DeviceId,
}

impl Layer {
    /// Returns the layer-root dentry; the dentry owns the root inode and is
    /// carried by the clone view.
    pub(super) fn root_dentry(&self) -> &Arc<Dentry> {
        self.mount.root_dentry()
    }

    /// Builds the upper-layer (index 0) real object for `child_path`.
    pub(super) fn child_real_object(&self, child_path: &Path) -> RealObject {
        RealObject::new(0, child_path.dentry().clone())
    }

    /// Returns the layer root path rebuilt from the clone view.
    pub(super) fn root_path(&self) -> Path {
        Path::new(self.mount.clone(), self.root_dentry().clone())
    }
}

/// The ordered, immutable layer stack of an overlay mount.
#[derive(Debug)]
pub(super) struct LayerStack {
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

    /// Returns the writable upper layer, or `EROFS` when the stack has none.
    pub(super) fn upper_layer(&self) -> Result<&Layer> {
        self.upper.as_ref().ok_or_else(|| {
            Error::with_message(Errno::EROFS, "the overlay mount has no upper layer")
        })
    }

    /// Returns the ordered lower layers.
    pub(super) fn lower_layers(&self) -> &[Layer] {
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

    /// Converts a copy-up origin layer index to the configured lower index
    /// under the uniform layer-index rule: index `0` is the upper when
    /// present, `n >= 1` addresses `lowers[n-1]` — the same rule the root
    /// and lookup construction use.
    ///
    /// Out-of-range forms (an index that is not a configured lower) fail
    /// with `EINVAL`.
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

/// The real-object composition behind one logical overlay object.
///
/// Invariant: `upper.is_some() || !lowers.is_empty()`, enforced by the
/// construction paths.
#[derive(Clone, Debug)]
pub(super) struct RealObjectStack {
    /// The upper real object; the visible-metadata source for merged
    /// directories.
    pub(super) upper: Option<RealObject>,
    /// The lower stack, topmost first; non-empty for lower-only/merged
    /// objects.
    pub(super) lowers: Vec<RealObject>,
}

impl RealObjectStack {
    /// Constructs a stack from an optional upper and an ordered lower list.
    ///
    /// Callers must keep the real-object invariant: at least one of the upper
    /// or lower slots is populated.
    pub(super) fn new(upper: Option<RealObject>, lowers: Vec<RealObject>) -> Self {
        debug_assert!(upper.is_some() || !lowers.is_empty());
        Self { upper, lowers }
    }

    /// Constructs an upper-only real-object stack.
    pub(super) fn upper_only(upper: RealObject) -> Self {
        Self {
            upper: Some(upper),
            lowers: Vec::new(),
        }
    }

    /// Constructs a lower-only real-object stack.
    pub(super) fn lower_only(lower: RealObject) -> Self {
        Self {
            upper: None,
            lowers: vec![lower],
        }
    }

    /// Returns whether this stack represents a merged directory view.
    ///
    /// A stack is merged when it has both an upper and lower contribution, or
    /// when more than one lower layer contributes.
    pub(super) fn is_merged(&self) -> bool {
        (self.upper.is_some() && !self.lowers.is_empty()) || self.lowers.len() > 1
    }

    /// Returns the visible-metadata source: the upper real object when present,
    /// else the topmost lower (`lowers[0]`).
    ///
    /// Precondition: `upper.is_some() || !lowers.is_empty()`.
    pub(super) fn visible_source(&self) -> &RealObject {
        match &self.upper {
            Some(upper) => upper,
            None => &self.lowers[0],
        }
    }
}
