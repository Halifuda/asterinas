// SPDX-License-Identifier: MPL-2.0

//! Real-object projection and the upper-first layer lookup core.
//!
//! This module owns [`RealObject`] — the pinned real (underlying) object of
//! one layer — its private whiteout/opaque marker reads, and the
//! module-private [`LayerLookup`] intermediate produced by
//! [`OverlayFs::lookup_in_layers`].
//!
//! # Lookup scan
//!
//! The lookup scan is upper-first with overlayfs merge-stop semantics:
//! the first non-directory hit terminates as `Single`; directory hits
//! accumulate into the lower stack until a barrier — a whiteout (negative),
//! an opaque directory found at the name, or a non-directory below an
//! accumulated directory — or the upper-miss opaque-parent case (negative).
//!
//! # References
//!
//! - Overlayfs layer lookup (`ovl_lookup_single` merge-stop rules):
//!   <https://elixir.bootlin.com/linux/latest/source/fs/overlayfs/namei.c#L298-L299>
//!   and <https://elixir.bootlin.com/linux/latest/source/fs/overlayfs/namei.c#L324-L331>
//! - Overlayfs whiteout/opaque marker xattrs (`OVL_XATTR_XWHITEOUT` /
//!   `OVL_XATTR_OPAQUE`, `ovl_is_whiteout` / `ovl_check_xwhiteout`):
//!   <https://elixir.bootlin.com/linux/latest/source/fs/overlayfs/xattrs.c>

use device_id::DeviceId;

use super::{
    binding_cache::{HiddenEvidence, NegativeBinding, PositiveKind},
    inode::OverlayObjectFacts,
};
use crate::{
    fs::{
        file::InodeType,
        fs_impls::overlayfs::mount::{OverlayFs, RealPath},
        vfs::{inode::Inode, path::Path, xattr::XattrName},
    },
    prelude::*,
};

const WHITEOUT_XATTR_FULL_NAME: &str = "trusted.overlay.whiteout";

const OPAQUE_XATTR_FULL_NAME: &str = "trusted.overlay.opaque";

/// Returns whether `real_inode` is a whiteout.
///
/// `true` when either: the object is a character device with device
/// number `0:0` (backends report it as `Some(DeviceId::null())`, or as
/// `None` for a zero device number such as ramfs); or the
/// `trusted.overlay.whiteout` xattr value is exactly `'y'`. An `ERANGE`,
/// `ENODATA`, or `EOPNOTSUPP` marker read is not a whiteout (`false`);
/// any other error propagates.
pub(in crate::fs::fs_impls::overlayfs) fn is_whiteout_inode(
    real_inode: &Arc<dyn Inode>,
) -> Result<bool> {
    let metadata = real_inode.metadata()?;
    if metadata.type_ == InodeType::CharDevice
        && metadata.self_dev_id.is_none_or(|dev_id| dev_id.is_null())
    {
        return Ok(true);
    }
    let name = XattrName::try_from_full_name(WHITEOUT_XATTR_FULL_NAME).ok_or_else(|| {
        Error::with_message(Errno::EINVAL, "invalid overlay whiteout marker xattr name")
    })?;
    let mut value = [0u8; 1];
    let mut writer = VmWriter::from(value.as_mut_slice()).to_fallible();
    match real_inode.get_xattr(name, &mut writer) {
        Ok(written) => Ok(written == 1 && value[0] == b'y'),
        Err(err) if err.error() == Errno::ERANGE => Ok(false),
        Err(err) if err.error() == Errno::ENODATA || err.error() == Errno::EOPNOTSUPP => Ok(false),
        Err(err) => Err(err),
    }
}

#[derive(Clone, Debug)]
pub(in crate::fs::fs_impls::overlayfs) struct RealObject {
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
    pub(in crate::fs::fs_impls::overlayfs) fn new(
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

    pub(in crate::fs::fs_impls::overlayfs) fn with_path(
        layer_index: usize,
        real_path: RealPath,
        fsid: u64,
        container_dev_id: DeviceId,
    ) -> Self {
        Self {
            layer_index,
            real_inode: real_path.inode().clone(),
            real_path: Some(real_path),
            fsid,
            container_dev_id,
        }
    }

    /// Builds the dentry-anchored real object for one layer hit of a child
    /// lookup: the resolved child path at `layer_index` pinned through the
    /// hit layer's identity (`fsid` / `container_dev_id`).
    fn for_lookup_child(layer_index: usize, child_path: &Path, layer_real: &RealObject) -> Self {
        Self::with_path(
            layer_index,
            RealPath::from_path(child_path),
            layer_real.fsid(),
            layer_real.container_dev_id(),
        )
    }

    pub(in crate::fs::fs_impls::overlayfs) fn layer_index(&self) -> usize {
        self.layer_index
    }

    pub(in crate::fs::fs_impls::overlayfs) fn real_inode(&self) -> &Arc<dyn Inode> {
        &self.real_inode
    }

    /// Returns the dentry-anchored real-object `Path`.
    ///
    /// `Err(EIO)` when no path is stored or the anchor mount is no longer
    /// alive.
    pub(in crate::fs::fs_impls::overlayfs) fn real_path(&self) -> Result<Path> {
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

    pub(in crate::fs::fs_impls::overlayfs) fn fsid(&self) -> u64 {
        self.fsid
    }

    pub(in crate::fs::fs_impls::overlayfs) fn container_dev_id(&self) -> DeviceId {
        self.container_dev_id
    }

    fn is_whiteout(&self) -> Result<bool> {
        is_whiteout_inode(&self.real_inode)
    }

    /// Returns whether this real object is an opaque directory (a
    /// lower-search barrier).
    ///
    /// A non-directory is never opaque (`false`). A directory is opaque
    /// exactly when the `trusted.overlay.opaque` xattr value is `'y'`; an
    /// `ENODATA`, `EOPNOTSUPP`, or `ERANGE` read is not opaque (`false`),
    /// and any other error propagates.
    pub(in crate::fs::fs_impls::overlayfs) fn is_opaque_directory(&self) -> Result<bool> {
        if !self.real_inode.type_().is_directory() {
            return Ok(false);
        }
        let name = XattrName::try_from_full_name(OPAQUE_XATTR_FULL_NAME).ok_or_else(|| {
            Error::with_message(Errno::EINVAL, "invalid overlay opaque marker xattr name")
        })?;
        let mut value = [0u8; 1];
        let mut writer = VmWriter::from(value.as_mut_slice()).to_fallible();
        match self.real_inode.get_xattr(name, &mut writer) {
            Ok(written) => Ok(written == 1 && value[0] == b'y'),
            Err(err)
                if err.error() == Errno::ENODATA
                    || err.error() == Errno::EOPNOTSUPP
                    || err.error() == Errno::ERANGE =>
            {
                Ok(false)
            }
            Err(err) => Err(err),
        }
    }
}

/// The module-private layer-lookup outcome of the lookup path.
///
/// A `Positive` outcome carries [`OverlayObjectFacts`]; the caller consumes
/// those facts after [`OverlayFs::project_inode`] runs to assemble the
/// published [`PositiveBinding`].
pub(super) enum LayerLookup {
    Positive(OverlayObjectFacts),
    Negative(NegativeBinding),
}

impl OverlayFs {
    /// Runs the upper-first layer lookup for `name` inside `parent_facts`'s
    /// real layers, with overlayfs merge-stop semantics.
    pub(super) fn lookup_in_layers(
        &self,
        parent_facts: &OverlayObjectFacts,
        name: &str,
    ) -> Result<LayerLookup> {
        let mut dir_hits: Vec<RealObject> = Vec::new();

        if let Some(upper_real) = &parent_facts.upper {
            let upper_path = upper_real.real_path()?;
            match upper_path
                .dentry()
                .as_dir_dentry_or_err()?
                .lookup_child(name)
            {
                Ok(child_dentry) => {
                    let child_path = Path::new(upper_path.mount_node().clone(), child_dentry);
                    let hit = RealObject::for_lookup_child(0, &child_path, upper_real);
                    if hit.is_whiteout()? {
                        return Ok(LayerLookup::Negative(NegativeBinding::HiddenByWhiteout(
                            HiddenEvidence {
                                layer_index: 0,
                                real_inode: hit.real_inode().clone(),
                            },
                        )));
                    }
                    if !hit.real_inode().type_().is_directory() {
                        return Ok(LayerLookup::Positive(OverlayObjectFacts {
                            kind: PositiveKind::Single,
                            upper: Some(hit),
                            lowers: Vec::new(),
                        }));
                    }
                    if hit.is_opaque_directory()? {
                        return Ok(LayerLookup::Positive(OverlayObjectFacts {
                            kind: PositiveKind::Single,
                            upper: Some(hit),
                            lowers: Vec::new(),
                        }));
                    }
                    dir_hits.push(hit);
                }
                Err(err) if err.error() == Errno::ENOENT => {
                    if upper_real.is_opaque_directory()? {
                        return Ok(LayerLookup::Negative(NegativeBinding::HiddenByOpaque(
                            HiddenEvidence {
                                layer_index: 0,
                                real_inode: upper_real.real_inode().clone(),
                            },
                        )));
                    }
                }
                Err(err) => return Err(err),
            }
        }

        for lower_real in &parent_facts.lowers {
            let layer_index = lower_real.layer_index();
            let lower_path = lower_real.real_path()?;
            match lower_path
                .dentry()
                .as_dir_dentry_or_err()?
                .lookup_child(name)
            {
                Ok(child_dentry) => {
                    let child_path = Path::new(lower_path.mount_node().clone(), child_dentry);
                    let hit = RealObject::for_lookup_child(layer_index, &child_path, lower_real);
                    if hit.is_whiteout()? {
                        // A whiteout is the topmost occurrence of the name:
                        // the name is hidden. Below an already-visible
                        // directory it only ends the downward merge scan.
                        if dir_hits.is_empty() {
                            return Ok(LayerLookup::Negative(NegativeBinding::HiddenByWhiteout(
                                HiddenEvidence {
                                    layer_index,
                                    real_inode: hit.real_inode().clone(),
                                },
                            )));
                        }
                        break;
                    }
                    if !hit.real_inode().type_().is_directory() {
                        if dir_hits.is_empty() {
                            return Ok(LayerLookup::Positive(OverlayObjectFacts {
                                kind: PositiveKind::Single,
                                upper: None,
                                lowers: vec![hit],
                            }));
                        }
                        // A non-directory below an accumulated directory hit
                        // stops the downward merge: every deeper layer stays
                        // hidden.
                        break;
                    }
                    let is_opaque = hit.is_opaque_directory()?;
                    dir_hits.push(hit);
                    if is_opaque {
                        break;
                    }
                }
                Err(err) if err.error() == Errno::ENOENT => continue,
                Err(err) => return Err(err),
            }
        }

        if dir_hits.is_empty() {
            return Ok(LayerLookup::Negative(NegativeBinding::Absent));
        }

        let kind = if dir_hits.len() > 1 {
            PositiveKind::Merged
        } else {
            PositiveKind::Single
        };
        let upper = if dir_hits[0].layer_index() == 0 {
            Some(dir_hits.remove(0))
        } else {
            None
        };
        Ok(LayerLookup::Positive(OverlayObjectFacts {
            kind,
            upper,
            lowers: dir_hits,
        }))
    }
}
