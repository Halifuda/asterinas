// SPDX-License-Identifier: MPL-2.0

//! Real-object projection and the upper-first layer lookup core (`P0-07`..`P0-11`).
//!
//! This module owns [`RealObject`] — the pinned real (underlying) object of one
//! layer — its private whiteout/opaque marker reads, and the module-private
//! [`LayerLookup`] intermediate produced by [`OverlayFs::lookup_in_layers`].
//! The lookup scan is upper-first and matches the Linux `ovl_lookup_single`
//! merge-stop semantics (verified against the Linux source tree
//! `fs/overlayfs/namei.c`, function `ovl_lookup_single`; wave-2 review items
//! 3/4): the first non-directory hit terminates
//! as `Single`; directory hits accumulate into the lower stack until a
//! barrier — a whiteout (negative, BC-2 §18.2), an opaque directory found at
//! the name (the merge stops below it, namei.c:324-331), or a non-directory
//! below an accumulated directory (the merge stops, namei.c:298-299) — or
//! the upper-miss opaque-parent case (negative, Case 3). Meso-02 spec §4
//! `projection/entry.rs`.

use device_id::DeviceId;

use super::{
    binding_cache::{HiddenEvidence, NegativeBinding, PositiveKind},
    inode::OverlayObjectFacts,
};
use crate::{
    fs::{
        file::InodeType,
        fs_impls::overlayfs::mount::OverlayFs,
        vfs::{inode::Inode, xattr::XattrName},
    },
    prelude::*,
};

/// The xattr name of the xattr-based whiteout marker (Linux `OVL_XATTR_XWHITEOUT`).
const WHITEOUT_XATTR_FULL_NAME: &str = "trusted.overlay.whiteout";

/// The xattr name of the opaque-directory marker (Linux `OVL_XATTR_OPAQUE`).
const OPAQUE_XATTR_FULL_NAME: &str = "trusted.overlay.opaque";

/// One pinned real (underlying) object of an overlay layer (`P0-07`).
///
/// `layer_index` is the object's position in the overlay layer stack (`0` =
/// upper, `1..` = lower position); `fsid` is the per-unique-underlying-
/// superblock identifier published by meso-01; `container_dev_id` is the
/// `st_dev` evidence of the same layer. The real inode is a strong pin:
/// `RealObject` values inside facts are immutable while published — facts are
/// replaced, never mutated in place (BC-3 §33).
///
/// Invariants: the pin is strong; the fields are fixed for the lifetime of the
/// value. The named constructor (`RealObject::new`) is a Wave-3 seam; until it
/// lands, values are built through the `pub(super)` fields (the mechanism the
/// frozen meso-03 consumption note sanctions alongside the constructor), so the
/// within-wave construction graph (root facts in `inode.rs`, the lookup scan
/// here, `RealObjectKey::from_source`) can build same-module values.
#[derive(Clone, Debug)]
pub(super) struct RealObject {
    pub(super) layer_index: usize,
    pub(super) real_inode: Arc<dyn Inode>,
    pub(super) fsid: u64,
    pub(super) container_dev_id: DeviceId,
}

impl RealObject {
    /// Returns the position of this real object in the overlay layer stack.
    pub(super) fn layer_index(&self) -> usize {
        self.layer_index
    }

    /// Returns the pinned underlying inode of this real object.
    pub(super) fn real_inode(&self) -> &Arc<dyn Inode> {
        &self.real_inode
    }

    /// Returns the layer filesystem identifier of this real object.
    pub(super) fn fsid(&self) -> u64 {
        self.fsid
    }

    /// Returns the `st_dev` of the container filesystem of this real object.
    pub(super) fn container_dev_id(&self) -> DeviceId {
        self.container_dev_id
    }

    /// Returns whether this real object is a whiteout (`P0-11`).
    ///
    /// A whiteout is either a classic character device `0:0` (Linux
    /// `ovl_is_whiteout`) or an object carrying the
    /// `trusted.overlay.whiteout` marker. The marker read is presence-based
    /// (accepted meso-06 contract): an absent (`ENODATA`) or unsupported
    /// (`EOPNOTSUPP`) marker reads as "not a whiteout", while a value longer
    /// than the 1-byte probe (`ERANGE`) still proves presence. Genuine xattr
    /// errors propagate.
    fn is_whiteout(&self) -> Result<bool> {
        let metadata = self.real_inode.metadata();
        // A classic whiteout is a character device with device number 0:0.
        // Backends report that device number either as
        // `Some(DeviceId::null())` or — when the device number is zero
        // (e.g. ramfs) — as `None`.
        if metadata.type_ == InodeType::CharDevice
            && metadata.self_dev_id.is_none_or(|dev_id| dev_id.is_null())
        {
            return Ok(true);
        }
        let name = XattrName::try_from_full_name(WHITEOUT_XATTR_FULL_NAME).ok_or_else(|| {
            Error::with_message(Errno::EINVAL, "invalid overlay whiteout marker xattr name")
        })?;
        let mut value = [0u8; 1];
        let mut writer = VmWriter::from(&mut value).to_fallible();
        match self.real_inode.get_xattr(name, &mut writer) {
            Ok(_) => Ok(true),
            Err(err) if err.error() == Errno::ERANGE => Ok(true),
            Err(err) if err.error() == Errno::ENODATA || err.error() == Errno::EOPNOTSUPP => {
                Ok(false)
            }
            Err(err) => Err(err),
        }
    }

    /// Returns whether this real object is an opaque directory (`P0-10`).
    ///
    /// An opaque directory carries `trusted.overlay.opaque == "y"` and acts
    /// as a lower-search barrier. Only directories qualify; the marker is
    /// re-observed on every lookup (no marker cache, per the frozen contract).
    /// Absent, unsupported, or over-long markers read as "not opaque"; genuine
    /// xattr errors propagate.
    fn is_opaque_directory(&self) -> Result<bool> {
        if !self.real_inode.type_().is_directory() {
            return Ok(false);
        }
        let name = XattrName::try_from_full_name(OPAQUE_XATTR_FULL_NAME).ok_or_else(|| {
            Error::with_message(Errno::EINVAL, "invalid overlay opaque marker xattr name")
        })?;
        let mut value = [0u8; 1];
        let mut writer = VmWriter::from(&mut value).to_fallible();
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

/// The module-private layer-lookup outcome (entry.rs) — the only named
/// intermediate of the lookup path (intermediate-hygiene rule).
///
/// The payloads are final types (`OverlayObjectFacts` / `NegativeBinding`);
/// `PositiveBinding` is assembled by the caller (`lookup_binding` in `mod.rs`)
/// after `project_inode` runs, which is why the positive payload is facts
/// rather than a binding.
pub(super) enum LayerLookup {
    Positive(OverlayObjectFacts),
    Negative(NegativeBinding),
}

impl OverlayFs {
    /// Runs the upper-first layer lookup for `name` inside `parent_facts`'s
    /// real layers (`P0-08`/`P0-09`/`P0-10`/`P0-11`).
    ///
    /// Frozen scan contract, matching Linux `ovl_lookup_single` (verified
    /// against namei.c): layers are observed topmost-first
    /// (`parent_facts.upper`, then `parent_facts.lowers`); the first
    /// non-directory hit terminates as `Single`; directory hits accumulate
    /// into the lower stack; a whiteout hit terminates as `HiddenByWhiteout`;
    /// an opaque directory found at the name stops the downward merge at any
    /// layer (`val == 'y'` -> `d->stop`, namei.c:324-331); a non-directory
    /// below an accumulated directory stops the merge (namei.c:298-299); an
    /// opaque parent upper (re-observed `trusted.overlay.opaque == "y"`)
    /// terminates names absent from the upper as `HiddenByOpaque` without a
    /// lower scan (Case 3). The caller holds the parent `DIR` transaction
    /// lock; this function takes no Overlay lock itself.
    pub(super) fn lookup_in_layers(
        &self,
        parent_facts: &OverlayObjectFacts,
        name: &str,
    ) -> Result<LayerLookup> {
        // The accumulation of directory hits (topmost-first) for the merged
        // directory case; a raw local of the lookup, not a named type.
        let mut dir_hits: Vec<RealObject> = Vec::new();

        // Layer 0: the upper component of the parent, when present.
        if let Some(upper_real) = &parent_facts.upper {
            match upper_real.real_inode().lookup(name) {
                Ok(child) => {
                    let hit = RealObject {
                        layer_index: 0,
                        real_inode: child,
                        fsid: upper_real.fsid(),
                        container_dev_id: upper_real.container_dev_id(),
                    };
                    if hit.is_whiteout()? {
                        return Ok(LayerLookup::Negative(NegativeBinding::HiddenByWhiteout(
                            HiddenEvidence {
                                layer_index: 0,
                                real_inode: hit.real_inode().clone(),
                            },
                        )));
                    }
                    if !hit.real_inode().type_().is_directory() {
                        // The first non-directory hit terminates as `Single`
                        // and hides all lower hits (P0-08).
                        return Ok(LayerLookup::Positive(OverlayObjectFacts {
                            kind: PositiveKind::Single,
                            upper: Some(hit),
                            lowers: Vec::new(),
                        }));
                    }
                    dir_hits.push(hit);
                    if hit.is_opaque_directory()? {
                        // An opaque directory found at the name is a merge
                        // barrier at EVERY layer, including the upper (Linux
                        // `ovl_lookup_single`: `val == 'y'` -> `d->stop =
                        // true`; namei.c:324-331): its lower counterparts are
                        // hidden, so the upper directory is the sole visible
                        // layer entry (P0-10).
                        return Ok(LayerLookup::Positive(OverlayObjectFacts {
                            kind: PositiveKind::Single,
                            upper: Some(hit),
                            lowers: Vec::new(),
                        }));
                    }
                }
                Err(err) if err.error() == Errno::ENOENT => {
                    // The name is absent from the upper. An opaque upper
                    // directory is a lower-search barrier (Case 3): the name
                    // is hidden and lower layers are never scanned.
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

        // Lower layers, topmost-first (layer indices `1..`).
        for (offset, lower_real) in parent_facts.lowers.iter().enumerate() {
            let layer_index = offset + 1;
            match lower_real.real_inode().lookup(name) {
                Ok(child) => {
                    let hit = RealObject {
                        layer_index,
                        real_inode: child,
                        fsid: lower_real.fsid(),
                        container_dev_id: lower_real.container_dev_id(),
                    };
                    if hit.is_whiteout()? {
                        // A whiteout is the topmost occurrence of the name:
                        // the name is hidden (P0-11). Below an already-visible
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
                            // The first non-directory hit terminates as
                            // `Single`; lower hits are hidden (P0-08).
                            return Ok(LayerLookup::Positive(OverlayObjectFacts {
                                kind: PositiveKind::Single,
                                upper: None,
                                lowers: vec![hit],
                            }));
                        }
                        // A non-directory below an accumulated directory hit
                        // stops the downward merge: every deeper layer stays
                        // hidden (Linux `ovl_lookup_single`:
                        // `!d_can_lookup(this)` with `d->is_dir` already set
                        // -> `d->stop = true`; namei.c:298-299).
                        break;
                    }
                    dir_hits.push(hit);
                    if hit.is_opaque_directory()? {
                        // An opaque directory found at this layer is the last
                        // entry of the merge: deeper lower directories are
                        // hidden (Linux `ovl_lookup_single`: `val == 'y'` ->
                        // `d->stop = true`; namei.c:324-331).
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
