// SPDX-License-Identifier: MPL-2.0

//! The durable lower-source identity record.
//!
//! This module owns the stateless `trusted.overlay.origin` record end-to-end
//! (encode / persist / decode / read): [`LowerIdRecord`] is a pure value type
//! carrying the pre-copy-up provenance triplet `(container_dev_id,
//! lower_layer_root_ino, real_ino)` of the lower source — the origin layer's
//! `st_dev`, configured root inode, and real inode number — the
//! module-private `ORIGIN_*` consts freeze the native 32-byte wire layout
//! (not Linux-wire-compatible; no export-style file handle exists in
//! Asterinas), and the two [`OverlayFs`] methods (`store_lower_id` /
//! `read_lower_id`) bridge the record to the upper inode's xattr surface.
//!
//! Copy-up publishes the record through [`OverlayFs::store_lower_id`], and
//! identity projection consumes it through [`OverlayFs::read_lower_id`]; the
//! xattr value is the single durable source of the lower identity, and
//! encode/decode are pure value transforms. `store_lower_id` writes only when
//! the upper filesystem can store private xattrs; `read_lower_id` returns a
//! record only when its origin pair resolves to a current lower layer.
//!
//! # Structure
//!
//! | Item | Owns |
//! |---|---|
//! | [`LowerIdRecord`] | The pure value type with encode/decode. |
//! | `ORIGIN_*` consts | The native wire layout (header + payload). |
//! | `OverlayFs::store_lower_id` / `read_lower_id` | The xattr bridge on the upper inode. |
//!
//! # References
//!
//! - Overlayfs origin xattr and file-handle verification (Linux):
//!   <https://elixir.bootlin.com/linux/latest/source/fs/overlayfs/xattrs.c>
//! - Overlayfs file-handle format (`struct ovl_fh`, `ovl_check_fb_len`):
//!   <https://elixir.bootlin.com/linux/latest/source/fs/overlayfs/export.c>

use device_id::DeviceId;

use super::{entry::RealObject, inode::OverlayObjectFacts};
use crate::{
    fs::{
        fs_impls::overlayfs::mount::OverlayFs,
        vfs::{
            inode::Inode,
            xattr::{XattrName, XattrSetFlags},
        },
    },
    prelude::*,
};

/// The durable lower-source identity record: a stateless value type carrying
/// the origin layer's `container_dev_id`, configured lower root inode, and
/// real inode number (pre-copy-up provenance).
#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::fs::fs_impls::overlayfs) struct LowerIdRecord {
    /// Durable underlying-fs identity of the origin layer: the layer root's
    /// `st_dev` (`OverlayLayer::container_dev_id`), replacing the mount-local
    /// `fsid` ordinal.
    container_dev_id: DeviceId,
    /// Configured lower-layer root inode for pair-only record resolution.
    lower_layer_root_ino: u64,
    /// Real inode number of the lower source (pre-copy-up provenance).
    real_ino: u64,
}

const ORIGIN_XATTR_FULL_NAME: &str = "trusted.overlay.origin";

/// Wire version. Every older or unknown version decodes as no origin.
const ORIGIN_WIRE_VERSION: u8 = 3;

/// The wire magic marking a native origin-record buffer; any other magic
/// decodes as "no origin".
const ORIGIN_WIRE_MAGIC: u32 = 0x0000_00fb;

/// Wire header length: magic (4) + version/flags/type/reserved (4).
const ORIGIN_WIRE_HEADER_LEN: usize = 8;

/// Wire payload length: `container_dev_id`, `lower_layer_root_ino`, and
/// `real_ino` (8 bytes each), all native endian.
const ORIGIN_WIRE_PAYLOAD_LEN: usize = 24;

const ORIGIN_WIRE_TOTAL_LEN: usize = ORIGIN_WIRE_HEADER_LEN + ORIGIN_WIRE_PAYLOAD_LEN;

/// The only known flag bits; unknown flag bits mean "origin unknown"
/// (`Ok(None)`).
const ORIGIN_WIRE_FLAGS_KNOWN: u8 = 0;

impl LowerIdRecord {
    /// Constructs the record from a lower [`RealObject`].
    ///
    /// The per-mount `fsid` is deliberately not persisted — it is derived at
    /// read time from the device/root pair.
    pub(super) fn try_from_lower(lower: &RealObject, lower_layer_root_ino: u64) -> Result<Self> {
        Ok(Self {
            container_dev_id: lower.container_dev_id(),
            lower_layer_root_ino,
            real_ino: lower.real_inode().ino(),
        })
    }

    /// Serializes the record into the native 32-byte wire buffer.
    pub(super) fn serialize(&self) -> Vec<u8> {
        let mut wire = Vec::with_capacity(ORIGIN_WIRE_TOTAL_LEN);
        wire.extend_from_slice(&ORIGIN_WIRE_MAGIC.to_ne_bytes());
        wire.push(ORIGIN_WIRE_VERSION);
        wire.push(ORIGIN_WIRE_FLAGS_KNOWN);
        wire.push(0);
        wire.push(0); // reserved header byte
        wire.extend_from_slice(&self.container_dev_id.as_encoded_u64().to_ne_bytes());
        wire.extend_from_slice(&self.lower_layer_root_ino.to_ne_bytes());
        wire.extend_from_slice(&self.real_ino.to_ne_bytes());
        wire
    }

    /// Reads one native-endian `u64` payload field at the 8-byte slot `slot`
    /// of the wire payload (slot 0 = `container_dev_id`).
    fn read_payload_u64(bytes: &[u8], slot: usize) -> u64 {
        let offset = ORIGIN_WIRE_HEADER_LEN + slot * 8;
        u64::from_ne_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
            bytes[offset + 4],
            bytes[offset + 5],
            bytes[offset + 6],
            bytes[offset + 7],
        ])
    }

    /// Conservatively decodes a wire buffer into a record: `Ok(None)` on any
    /// structural mismatch (wrong length, bad magic, bad version, or unknown
    /// flag bits); `Err` is reserved and unreachable.
    pub(super) fn decode(bytes: &[u8]) -> Result<Option<Self>> {
        if bytes.len() != ORIGIN_WIRE_TOTAL_LEN {
            return Ok(None);
        }
        if u32::from_ne_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) != ORIGIN_WIRE_MAGIC {
            return Ok(None);
        }
        let version = bytes[4];
        let flags = bytes[5];
        let type_ = bytes[6];
        // Retired v1/v2 formats are never decoded.
        if version != ORIGIN_WIRE_VERSION {
            return Ok(None);
        }
        if flags & !ORIGIN_WIRE_FLAGS_KNOWN != 0 || type_ != 0 || bytes[7] != 0 {
            return Ok(None);
        }
        let Some(container_dev_id) = DeviceId::from_encoded_u64(Self::read_payload_u64(bytes, 0))
        else {
            return Ok(None);
        };
        let lower_layer_root_ino = Self::read_payload_u64(bytes, 1);
        let real_ino = Self::read_payload_u64(bytes, 2);
        Ok(Some(Self {
            container_dev_id,
            lower_layer_root_ino,
            real_ino,
        }))
    }

    pub(in crate::fs::fs_impls::overlayfs) fn container_dev_id(&self) -> DeviceId {
        self.container_dev_id
    }

    pub(in crate::fs::fs_impls::overlayfs) fn lower_layer_root_ino(&self) -> u64 {
        self.lower_layer_root_ino
    }

    pub(in crate::fs::fs_impls::overlayfs) fn real_ino(&self) -> u64 {
        self.real_ino
    }
}

impl OverlayFs {
    /// Persists the lower-source identity record on the upper inode with a
    /// single `set_xattr(..., CREATE_OR_REPLACE)` call; a missing
    /// capability or `EOPNOTSUPP` is a gated no-op.
    pub(in crate::fs::fs_impls::overlayfs) fn store_lower_id(
        &self,
        upper: &Arc<dyn Inode>,
        lower: &RealObject,
    ) -> Result<()> {
        let lower_layer_root_ino = self.lower_layer_root_ino_for_origin(lower)?;
        let Some(capabilities) = self.policy().upper_capabilities() else {
            return Ok(());
        };
        if !capabilities.can_store_private_xattr() {
            return Ok(());
        }
        let record = LowerIdRecord::try_from_lower(lower, lower_layer_root_ino)?;
        let name = XattrName::try_from_full_name(ORIGIN_XATTR_FULL_NAME).ok_or_else(|| {
            Error::with_message(Errno::EINVAL, "invalid overlay origin xattr name")
        })?;
        let value = record.serialize();
        let mut reader = VmReader::from(value.as_slice()).to_fallible();
        match upper.set_xattr(name, &mut reader, XattrSetFlags::CREATE_OR_REPLACE) {
            // `EOPNOTSUPP` is a gated no-op: `Ok(())` with no record.
            Err(err) if err.error() == Errno::EOPNOTSUPP => Ok(()),
            result => result,
        }
    }

    /// Reads the persisted lower-source identity record from the upper inode.
    ///
    /// Returns `Ok(None)` for absent, malformed, foreign, or ambiguous
    /// evidence, preserving the visible-source fallback; genuine xattr-read
    /// errors propagate.
    pub(in crate::fs::fs_impls::overlayfs) fn read_lower_id(
        &self,
        upper: &Arc<dyn Inode>,
    ) -> Result<Option<LowerIdRecord>> {
        let name = XattrName::try_from_full_name(ORIGIN_XATTR_FULL_NAME).ok_or_else(|| {
            Error::with_message(Errno::EINVAL, "invalid overlay origin xattr name")
        })?;
        let mut value = [0u8; ORIGIN_WIRE_TOTAL_LEN];
        let mut writer = VmWriter::from(value.as_mut_slice()).to_fallible();
        match upper.get_xattr(name, &mut writer) {
            Ok(written) => {
                let Some(record) = LowerIdRecord::decode(&value[..written])? else {
                    return Ok(None);
                };
                let resolves = self
                    .identity()
                    .resolve_layer_id_for_record(
                        record.container_dev_id(),
                        record.lower_layer_root_ino(),
                    )
                    .is_some();
                Ok(resolves.then_some(record))
            }
            Err(err) if err.error() == Errno::ENODATA => Ok(None),
            Err(err) if err.error() == Errno::EOPNOTSUPP => Ok(None),
            // Fail-safe: the origin wire is fixed-length, so a value that
            // does not fit the 32-byte buffer cannot be a canonical v3
            // record and reads as "no record".
            Err(err) if err.error() == Errno::ERANGE => Ok(None),
            Err(err) => Err(err),
        }
    }

    /// Returns whether the persisted lower-source record is consistent with
    /// the retained same-layer lower of `facts`: the record's real inode must
    /// equal the retained lower's, so a forged or stale record falls back to
    /// the visible-source projection (identity authenticity wins).
    ///
    // TODO(origin-verify): once the VFS gains an ino-to-inode / file-handle
    // resolution surface, upgrade this cross-check to a full origin
    // verification and drop the retained-lower approximation.
    pub(super) fn origin_real_ino_resolves(
        &self,
        record: &LowerIdRecord,
        facts: &OverlayObjectFacts,
    ) -> bool {
        // The record's layer is the unique current lower fsid matching its
        // device/root pair; the retained lower at that layer is the
        // same-layer evidence in the fresh facts.
        let Some(layer_fsid) = self
            .identity()
            .resolve_layer_id_for_record(record.container_dev_id(), record.lower_layer_root_ino())
        else {
            return false;
        };
        match facts
            .lowers()
            .iter()
            .find(|lower| lower.fsid() == layer_fsid)
        {
            // Accepted only when the record's real inode equals the retained
            // same-layer lower; a mismatch (the lower was replaced since
            // copy-up) or an absent retained lower (the lower no longer
            // participates in the name) rejects the record.
            Some(retained_lower) => record.real_ino() == retained_lower.real_inode().ino(),
            None => false,
        }
    }

    /// Returns the configured lower root inode for a copy-up origin source.
    ///
    /// `layer_index()` counts the upper as position 0, so when the stack has
    /// an upper the origin's own lower position is `layer_index - 1`; the
    /// checked subtraction rejects an origin that claims the upper's
    /// position. The bounds check rejects a position that names no configured
    /// lower layer. Both rejections are `EINVAL` — a copy-up programming
    /// error, not a runtime condition.
    fn lower_layer_root_ino_for_origin(&self, lower: &RealObject) -> Result<u64> {
        let layer_stack = self.layer_stack();
        let lower_index = if layer_stack.upper.is_some() {
            lower.layer_index().checked_sub(1).ok_or_else(|| {
                Error::with_message(
                    Errno::EINVAL,
                    "the origin source does not identify a configured lower layer",
                )
            })?
        } else {
            lower.layer_index()
        };
        let lower_layer = layer_stack.lowers.get(lower_index).ok_or_else(|| {
            Error::with_message(
                Errno::EINVAL,
                "the origin source does not identify a configured lower layer",
            )
        })?;
        Ok(lower_layer.root_inode.ino())
    }
}
