// SPDX-License-Identifier: MPL-2.0
#![short_vis_path::add(overlayfs)]

//! Dev/ino identity projection of the overlay namespace.
//!
//! This module owns the immutable per-mount [`IdentityPolicy`], the published
//! [`ObjectId`], and the durable lower-source identity record
//! ([`LowerIdOrigin`]).
//!
//! The **xino matrix** decides, for one layer and one real inode, which
//! `st_dev`/`st_ino` pair the overlay publishes:
//!
//! - **same-fs passthrough** — every layer shares one underlying
//!   filesystem, so `st_ino` matches the underlying inode and `st_dev` is
//!   uniform;
//! - **xino effective** — the overlay publishes its own `st_dev` and an
//!   encoded `st_ino` (layer `fsid` in the high `xino_shift` bits, real
//!   ino in the payload);
//! - **xino off** — directories report the overlay `st_dev` plus a
//!   saturating allocated ino; non-directories report the underlying
//!   dev/ino; an ino that does not fit the xino payload falls back to the
//!   xino-off behavior (explicit, never silently wrong).
//!
//! A **lower-id record** is the durable `(container_dev_id,
//! lower_layer_root_ino, real_ino)` provenance that copy-up persists on the
//! upper inode. [`IdentityPolicy::project_object_id_from_lower_id`] feeds
//! such a record back through the same xino matrix so the object keeps a
//! constant `st_ino` across copy-up (authority-continuity); the record's
//! device/root pair is resolved to a per-mount `fsid` from the immutable
//! lower-layer snapshot.

use core::sync::atomic::{AtomicU64, Ordering};

use device_id::DeviceId;

use super::xattr::origin_xattr_name;
use crate::{
    fs::{
        fs_impls::overlayfs::{
            fs::{OverlayFs, policy::XinoMode},
            layer::{Layer, LayerStack, RealObjectStack},
            real::RealObject,
        },
        vfs::{inode::Inode, xattr::XattrSetFlags},
    },
    prelude::*,
};

/// Total bit width of a `u64` inode value.
const U64_BITS: u32 = u64::BITS;

/// The published `st_dev`/`st_ino` identity of one overlay object.
///
/// The pair is precomputed once by [`IdentityPolicy`] at inode creation and
/// stored on the `OverlayInode`; it identifies one logical overlay object
/// wherever that object is reached.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct ObjectId {
    /// Published `st_dev`.
    pub(super) dev: DeviceId,
    /// Published `st_ino`.
    pub(super) ino: u64,
}

/// One published layer's identity triplet.
#[derive(Clone, Copy, Debug)]
pub(in overlayfs) struct LowerLayerIdentity {
    /// The per-mount layer ordinal.
    fsid: u64,
    /// The backend container device id of the layer.
    container_dev_id: DeviceId,
    /// The layer root's real inode number.
    lower_layer_root_ino: u64,
}

/// Collects the construction-local layer identity inputs for
/// [`IdentityPolicy::new`].
///
/// Returns the per-published-layer [`LowerLayerIdentity`] list (upper first
/// when present) with the upper's entry position. The exclusion is by
/// position, not by value: an upper sharing an underlying filesystem with a
/// lower must not also drop the lower's entry.
pub(in overlayfs) fn collect_layer_devs(
    layer_stack: &LayerStack,
) -> (Vec<LowerLayerIdentity>, Option<usize>) {
    let layer_capacity = layer_stack.lowers.len() + usize::from(layer_stack.upper.is_some());
    let mut layer_devs: Vec<LowerLayerIdentity> = Vec::with_capacity(layer_capacity);
    let upper_layer_dev_index = if let Some(upper) = layer_stack.upper.as_ref() {
        let index = layer_devs.len();
        layer_devs.push(LowerLayerIdentity {
            fsid: upper.fsid,
            container_dev_id: upper.container_dev_id,
            lower_layer_root_ino: upper.root_dentry().inode().ino(),
        });
        Some(index)
    } else {
        None
    };
    for lower in &layer_stack.lowers {
        layer_devs.push(LowerLayerIdentity {
            fsid: lower.fsid,
            container_dev_id: lower.container_dev_id,
            lower_layer_root_ino: lower.root_dentry().inode().ino(),
        });
    }
    (layer_devs, upper_layer_dev_index)
}

/// The immutable per-mount dev/ino projection policy.
///
/// Invariants: `xino_shift <= 63` (enforced by [`IdentityPolicy::new`]);
/// `fallback_ino_allocator` never wraps (saturating); `is_all_layers_same_fs`
/// is fixed at construction; `lower_layer_devs` is an fsid-sorted immutable
/// snapshot with one entry per configured lower — never re-probed at runtime.
#[derive(Debug)]
pub(in overlayfs) struct IdentityPolicy {
    /// The `xino=` mode; consumed from the mount policy at construction.
    xino_mode: XinoMode,
    /// The overlay's own `st_dev` (`AnonDeviceId`), acquired in the extended
    /// `OverlayFs::new`.
    overlay_dev_id: DeviceId,
    /// High-bit encoding width of the xino layer id (e.g. `64 - 16` = 48-bit
    /// payload).
    xino_shift: u32,
    /// Whether every layer shares one underlying filesystem (fast path);
    /// derived at construction from the published layer dev ids.
    is_all_layers_same_fs: bool,
    /// Immutable LOWER-only identity snapshot for durable origin records.
    lower_layer_devs: Box<[LowerLayerIdentity]>,
    /// Saturating fallback ino allocator for directories / anon inos when
    /// xino is not applicable.
    fallback_ino_allocator: AtomicU64,
}

impl IdentityPolicy {
    /// The xino layer id occupies the high 16 bits; the inode payload is the
    /// low 48 bits.
    pub(in overlayfs) const XINO_SHIFT: u32 = 16;

    /// Constructs the immutable projection policy from the published layer
    /// snapshot.
    ///
    /// The policy keeps a LOWER-only identity snapshot: the published layer
    /// list minus the upper's entry. Exclusion is by position, not by
    /// value — an upper sharing an underlying filesystem with a lower must
    /// keep the lower's entry so no lower is dropped only because its
    /// device id matches the upper's.
    pub(in overlayfs) fn new(
        overlay_dev_id: DeviceId,
        layer_devs: &[LowerLayerIdentity],
        upper_layer_dev_index: Option<usize>,
        xino_shift: u32,
        xino_mode: XinoMode,
    ) -> Result<Self> {
        if xino_shift > 63 {
            return_errno_with_message!(Errno::EINVAL, "invalid overlay xino shift");
        }
        let is_all_layers_same_fs = layer_devs.first().is_some_and(|first| {
            layer_devs
                .iter()
                .all(|layer| layer.container_dev_id == first.container_dev_id)
        });
        let mut lower_layer_devs: Vec<LowerLayerIdentity> = layer_devs
            .iter()
            .copied()
            .enumerate()
            .filter(|(index, _)| Some(*index) != upper_layer_dev_index)
            .map(|(_, layer)| layer)
            .collect();
        lower_layer_devs.sort_by_key(|layer| layer.fsid);
        Ok(Self {
            xino_mode,
            overlay_dev_id,
            xino_shift,
            is_all_layers_same_fs,
            lower_layer_devs: lower_layer_devs.into_boxed_slice(),
            fallback_ino_allocator: AtomicU64::new(0),
        })
    }

    pub(super) fn is_xino_effective(&self) -> bool {
        if self.is_all_layers_same_fs {
            return false;
        }
        matches!(self.xino_mode, XinoMode::Auto | XinoMode::On)
    }

    /// Projects the dev/ino identity of a real object from its owning
    /// layer's evidence.
    ///
    /// This is the entry for callers that already hold a [`RealObject`] (the
    /// real ino is read from the object, while the layer fsid and container
    /// dev come from the owning [`Layer`]), as opposed to the lower-id entry
    /// that starts from a durable record, so callers need not unpack the
    /// layer evidence into `project` themselves.
    pub(super) fn project_object_id(
        &self,
        layer: &Layer,
        real: &RealObject,
        is_directory: bool,
    ) -> ObjectId {
        self.project(
            layer.fsid,
            real.real_inode().ino(),
            layer.container_dev_id,
            is_directory,
        )
    }

    /// Projects the dev/ino identity from the durable lower-id record
    /// through the shared [`IdentityPolicy::project`] matrix; an unresolved
    /// origin pair leaves the caller on the visible-source fallback.
    pub(super) fn project_object_id_from_lower_id(
        &self,
        lower_id: &LowerIdOrigin,
        is_directory: bool,
    ) -> Option<ObjectId> {
        let layer_id = self.resolve_layer_id_for_record(
            lower_id.container_dev_id(),
            lower_id.lower_layer_root_ino(),
        )?;
        Some(self.project(
            layer_id,
            lower_id.real_ino(),
            lower_id.container_dev_id(),
            is_directory,
        ))
    }

    /// Projects one `(layer_id, real_ino, origin_dev)` identity input to
    /// its published `st_dev`/`st_ino`.
    ///
    /// The branches make every published `st_ino` unambiguous: shared-fs
    /// passes the origin through, an encodable pair is packed into the xino
    /// payload, and an unencodable pair takes the fallback. The fit test
    /// rejects truncation that would alias two layers; checked arithmetic
    /// keeps the `payload_bits == U64_BITS` case from shifting by the full width.
    fn project(
        &self,
        layer_id: u64,
        real_ino: u64,
        origin_dev: DeviceId,
        is_directory: bool,
    ) -> ObjectId {
        if self.is_all_layers_same_fs {
            return ObjectId {
                dev: origin_dev,
                ino: real_ino,
            };
        }
        // Xino encoding applies when both the real ino and the layer id
        // fit the encoded space.
        if self.is_xino_effective() && self.xino_fits(layer_id, real_ino) {
            let payload_bits = U64_BITS - self.xino_shift;
            let encoded_ino = if payload_bits == U64_BITS {
                real_ino
            } else {
                (layer_id << payload_bits) | real_ino
            };
            return ObjectId {
                dev: self.overlay_dev_id,
                ino: encoded_ino,
            };
        }
        // No xino encoding (or the ino overflowed the payload):
        // directories take the overlay dev plus an allocated ino, so they
        // stay stable without an encodable payload; non-directories pass
        // through the origin dev/ino unchanged.
        if is_directory {
            ObjectId {
                dev: self.overlay_dev_id,
                ino: self.allocate_fallback_ino(),
            }
        } else {
            ObjectId {
                dev: origin_dev,
                ino: real_ino,
            }
        }
    }

    /// Returns whether the `(layer_id, real_ino)` pair fits the xino-encoded
    /// ino space.
    ///
    /// Checked arithmetic skips the degenerate `payload_bits == U64_BITS`
    /// (`xino_shift == 0`) case, so it never shifts by the full bit width.
    fn xino_fits(&self, layer_id: u64, real_ino: u64) -> bool {
        let payload_bits = U64_BITS - self.xino_shift;
        payload_bits == U64_BITS
            || (real_ino >> payload_bits == 0 && layer_id >> self.xino_shift == 0)
    }

    /// Allocates a fallback ino for directories / anon objects when xino is
    /// not applicable.
    ///
    /// Saturating by construction (the counter never wraps). The first
    /// allocation returns `1` (ino 0 is not handed out).
    fn allocate_fallback_ino(&self) -> u64 {
        match self.fallback_ino_allocator.try_update(
            Ordering::Relaxed,
            Ordering::Relaxed,
            |current| Some(current.saturating_add(1)),
        ) {
            Ok(previous) => previous.saturating_add(1),
            Err(_) => u64::MAX,
        }
    }

    /// Resolves the unique per-mount layer id (fsid) for a durable origin
    /// record's `(container_dev_id, lower_layer_root_ino)` pair among the
    /// current lower layers.
    ///
    /// The LOWER-only table is consulted because origin records only ever
    /// come from lower sources; an absent pair or multiple matching fsids
    /// returns `None` to keep the visible-source fallback.
    pub(super) fn resolve_layer_id_for_record(
        &self,
        container_dev_id: DeviceId,
        lower_layer_root_ino: u64,
    ) -> Option<u64> {
        let mut matched_fsid: Option<u64> = None;
        for layer in self.lower_layer_devs.iter() {
            if layer.container_dev_id == container_dev_id
                && layer.lower_layer_root_ino == lower_layer_root_ino
            {
                match matched_fsid {
                    None => matched_fsid = Some(layer.fsid),
                    Some(existing) if existing == layer.fsid => {}
                    Some(_) => return None,
                }
            }
        }
        matched_fsid
    }
}

/// The durable lower-source identity record: a stateless value type carrying
/// the origin layer's `container_dev_id`, configured lower root inode, and
/// real inode number (pre-copy-up provenance).
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct LowerIdOrigin {
    /// Durable underlying-fs identity of the origin layer: the layer root's
    /// `st_dev` (`Layer::container_dev_id`), replacing the mount-local
    /// `fsid` ordinal.
    container_dev_id: DeviceId,
    /// Configured lower-layer root inode for pair-only record resolution.
    lower_layer_root_ino: u64,
    /// Real inode number of the lower source (pre-copy-up provenance).
    real_ino: u64,
}

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

impl LowerIdOrigin {
    /// Constructs the record from a lower [`RealObject`] and its owning
    /// layer.
    ///
    /// The per-mount `fsid` is deliberately not persisted — it is derived at
    /// read time from the device/root pair.
    fn try_from_lower(
        layer: &Layer,
        lower: &RealObject,
        lower_layer_root_ino: u64,
    ) -> Result<Self> {
        Ok(Self {
            container_dev_id: layer.container_dev_id,
            lower_layer_root_ino,
            real_ino: lower.real_inode().ino(),
        })
    }

    /// Serializes the record into the native 32-byte wire buffer.
    fn serialize(&self) -> Vec<u8> {
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
    fn decode(bytes: &[u8]) -> Result<Option<Self>> {
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

    pub(super) fn container_dev_id(&self) -> DeviceId {
        self.container_dev_id
    }

    pub(super) fn lower_layer_root_ino(&self) -> u64 {
        self.lower_layer_root_ino
    }

    pub(super) fn real_ino(&self) -> u64 {
        self.real_ino
    }
}

impl OverlayFs {
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
        record: &LowerIdOrigin,
        facts: &RealObjectStack,
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
            .lowers
            .iter()
            .find(|lower| self.layer(lower.layer_index()).fsid == layer_fsid)
        {
            // Accepted only when the record's real inode equals the retained
            // same-layer lower; a mismatch (the lower was replaced since
            // copy-up) or an absent retained lower (the lower no longer
            // participates in the name) rejects the record.
            Some(retained_lower) => record.real_ino() == retained_lower.real_inode().ino(),
            None => false,
        }
    }

    /// Persists the lower-source identity record on the upper inode with a
    /// single `set_xattr(..., CREATE_OR_REPLACE)` call; a missing
    /// capability or `EOPNOTSUPP` is a gated no-op.
    pub(super) fn store_lower_id(&self, upper: &Arc<dyn Inode>, lower: &RealObject) -> Result<()> {
        let layer = self.layer(lower.layer_index());
        let lower_layer_root_ino = self
            .layer_stack()
            .lower_layer_root_ino_for_origin(lower.layer_index())?;
        let Some(capabilities) = self.policy().upper_capabilities() else {
            return Ok(());
        };
        if !capabilities.can_store_private_xattr() {
            return Ok(());
        }
        let record = LowerIdOrigin::try_from_lower(layer, lower, lower_layer_root_ino)?;
        let name = origin_xattr_name()?;
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
    pub(super) fn read_lower_id(&self, upper: &Arc<dyn Inode>) -> Result<Option<LowerIdOrigin>> {
        let name = origin_xattr_name()?;
        let mut value = [0u8; ORIGIN_WIRE_TOTAL_LEN];
        let mut writer = VmWriter::from(value.as_mut_slice()).to_fallible();
        match upper.get_xattr(name, &mut writer) {
            Ok(written) => {
                let Some(record) = LowerIdOrigin::decode(&value[..written])? else {
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
}
