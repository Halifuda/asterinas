// SPDX-License-Identifier: MPL-2.0

#![short_vis_path::add(overlayfs)]
//! Real (underlying) object references used by overlayfs.
//!
//! [`RealPath`] is a dentry-anchored path to a real filesystem object and
//! deliberately does not cache its inode: the inode is obtained through
//! [`Path::inode`] after upgrading the anchor. [`RealObject`] combines that
//! optional path with the identity fields needed by overlayfs, and
//! [`RealObjectKey`] is the identity pair used by the inode cache.

use device_id::DeviceId;

use crate::{
    fs::vfs::{
        inode::Inode,
        path::{Dentry, Mount, Path},
    },
    prelude::*,
};

#[derive(Clone, Debug)]
pub(in overlayfs) struct RealPath {
    mount: Weak<Mount>,
    dentry: Arc<Dentry>,
}

impl RealPath {
    pub(in overlayfs) fn from_path(path: &Path) -> Self {
        Self {
            mount: Arc::downgrade(path.mount_node()),
            dentry: path.dentry().clone(),
        }
    }

    /// Returns `Err(EIO)` when the anchor mount is no longer alive (the
    /// parent overlay was unmounted while a stored path survived).
    pub(in overlayfs) fn upgrade(&self) -> Result<Path> {
        let mount = self.mount.upgrade().ok_or_else(|| {
            Error::with_message(
                Errno::EIO,
                "the anchor mount of the stored real path is no longer alive",
            )
        })?;
        Ok(Path::new(mount, self.dentry.clone()))
    }
}

#[derive(Clone, Debug)]
pub(in overlayfs) struct RealObject {
    pub(super) layer_index: usize,
    pub(super) real_inode: Arc<dyn Inode>,
    /// Dentry-anchored real-object [`RealPath`] value.
    pub(super) real_path: Option<RealPath>,
    pub(super) fsid: u64,
    pub(super) container_dev_id: DeviceId,
}

impl RealObject {
    /// Builds a path-less, identity-only real object.
    ///
    /// The readdir `..` projection constructs one of these per visible
    /// child when it only needs the child's identity (dev/ino) — and not a
    /// dentry path — so the object carries no stored `real_path`.
    pub(in overlayfs) fn identity_only(
        layer_index: usize,
        real_inode: Arc<dyn Inode>,
        fsid: u64,
        container_dev_id: DeviceId,
    ) -> Self {
        Self {
            layer_index,
            real_inode,
            real_path: None,
            fsid,
            container_dev_id,
        }
    }

    /// Builds the dentry-anchored real object for a resolved layer path.
    ///
    /// The inode is taken from `path` before it is pinned into a [`RealPath`],
    /// so `RealPath` does not need to cache a redundant inode.
    pub(in overlayfs) fn from_layer_path(
        layer_index: usize,
        path: &Path,
        fsid: u64,
        container_dev_id: DeviceId,
    ) -> Self {
        Self {
            layer_index,
            real_inode: path.inode().clone(),
            real_path: Some(RealPath::from_path(path)),
            fsid,
            container_dev_id,
        }
    }

    /// Builds the dentry-anchored real object for one layer hit of a child
    /// lookup: the resolved child path at `layer_index` pinned through the
    /// hit layer's identity (`fsid` / `container_dev_id`).
    pub(in overlayfs) fn child_hit(
        layer_index: usize,
        child_path: &Path,
        layer_real: &RealObject,
    ) -> Self {
        Self::from_layer_path(
            layer_index,
            child_path,
            layer_real.fsid(),
            layer_real.container_dev_id(),
        )
    }

    pub(in overlayfs) fn layer_index(&self) -> usize {
        self.layer_index
    }

    pub(in overlayfs) fn real_inode(&self) -> &Arc<dyn Inode> {
        &self.real_inode
    }

    /// Returns the dentry-anchored real-object `Path`.
    ///
    /// `Err(EIO)` when no path is stored or the anchor mount is no longer
    /// alive.
    pub(in overlayfs) fn real_path(&self) -> Result<Path> {
        self.real_path
            .as_ref()
            .ok_or_else(|| {
                Error::with_message(
                    Errno::EIO,
                    "the real object carries no dentry-anchored path",
                )
            })?
            .upgrade()
    }

    pub(in overlayfs) fn fsid(&self) -> u64 {
        self.fsid
    }

    pub(in overlayfs) fn container_dev_id(&self) -> DeviceId {
        self.container_dev_id
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(in overlayfs) struct RealObjectKey {
    /// Layer fsid of the visible-metadata source (upper, else topmost lower).
    fsid: u64,
    /// Real inode number of the visible-metadata source.
    real_ino: u64,
}

impl RealObjectKey {
    pub(in overlayfs) fn from_source(real: &RealObject) -> Self {
        Self {
            fsid: real.fsid(),
            real_ino: real.real_inode().ino(),
        }
    }
}
