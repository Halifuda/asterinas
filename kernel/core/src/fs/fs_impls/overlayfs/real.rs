// SPDX-License-Identifier: MPL-2.0

#![short_vis_path::add(overlayfs)]
//! Real (underlying) object references used by overlayfs.
//!
//! [`RealObject`] anchors one real filesystem object: its `layer_index`
//! names the owning layer of the mount's layer stack and its dentry anchors
//! the real entry. [`RealObject::real_inode`] reads the dentry-owned inode
//! infallibly; a full dentry-anchored [`Path`] is rebuilt on demand through
//! the owning layer's private clone view via `OverlayFs::real_object_path`.
//! The owning layer carries the identity fields (fsid / container device
//! id); they are not copied per object. [`RealObjectKey`] is the identity
//! pair used by the inode cache.

use crate::{
    fs::vfs::{inode::Inode, path::Dentry},
    prelude::*,
};

/// One dentry-anchored real object of a known layer.
///
/// The anchor's validity follows the overlay lifetime: the owning
/// [`Layer`](super::layer::Layer) strongly holds the layer's clone view,
/// and the view strongly holds the dentry, so a reachable logical object
/// never observes a dead anchor.
#[derive(Clone, Debug)]
pub(super) struct RealObject {
    /// The owning layer's index in the layer stack.
    layer_index: usize,
    /// The dentry anchoring the real entry.
    dentry: Arc<Dentry>,
}

impl RealObject {
    /// Builds the real object for a dentry resolved at `layer_index`.
    pub(super) fn new(layer_index: usize, dentry: Arc<Dentry>) -> Self {
        Self {
            layer_index,
            dentry,
        }
    }

    pub(super) fn layer_index(&self) -> usize {
        self.layer_index
    }

    pub(super) fn dentry(&self) -> &Arc<Dentry> {
        &self.dentry
    }

    /// Returns the anchored dentry's inode; infallible because the dentry
    /// strongly owns it.
    pub(super) fn real_inode(&self) -> &Arc<dyn Inode> {
        self.dentry.inode()
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(super) struct RealObjectKey {
    /// Layer fsid of the visible-metadata source (upper, else topmost lower).
    fsid: u64,
    /// Real inode number of the visible-metadata source.
    real_ino: u64,
}

impl RealObjectKey {
    pub(super) fn from_source(fsid: u64, real: &RealObject) -> Self {
        Self {
            fsid,
            real_ino: real.real_inode().ino(),
        }
    }
}
