// SPDX-License-Identifier: MPL-2.0

#![short_vis_path::add(overlayfs)]
//! The real-object reference model beneath the overlay namespace.
//!
//! A real object is one underlying filesystem entry as seen from one layer.
//! [`RealObject`] anchors it with the owning layer index plus the anchoring
//! dentry; the layer's identity fields (fsid, container device id) stay on
//! the owning layer and are not copied per object, and the anchored inode is
//! read infallibly because the dentry owns it. A full dentry-anchored path
//! is rebuilt on demand through the owning layer's clone view.
//!
//! Anchor validity follows the overlay lifetime: the owning layer strongly
//! holds its private clone view, so a reachable logical object never
//! observes a dead anchor. [`RealObjectKey`] pairs the visible source's
//! layer fsid with its real inode number; it is the identity-reuse key of
//! the inode cache.

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
