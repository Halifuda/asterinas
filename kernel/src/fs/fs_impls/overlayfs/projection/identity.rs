// SPDX-License-Identifier: MPL-2.0

//! Dev/ino identity projection of the overlay namespace (`P2-01`/`P0-12`).
//!
//! This module owns the immutable per-mount [`IdentityPolicy`] (mounted as
//! `OverlayFs::identity`) and the published [`OverlayObjectId`] carrier. It
//! implements the frozen `P2-01` dev/ino matrix of spec §4
//! `projection/identity.rs`:
//!
//! - **same-fs passthrough** — when every layer shares one underlying
//!   filesystem, `st_dev` is uniform and `st_ino` matches the underlying
//!   inode (`P0-12` fast path);
//! - **xino effective** — overlay `st_dev` plus an encoded `st_ino` (the
//!   layer `fsid` in the high `xino_shift` bits, real ino in the payload);
//! - **xino off** — directories report the overlay `st_dev` plus a saturating
//!   allocated ino; non-directories report the underlying dev/ino;
//! - **per-object overflow** — an ino that does not fit the xino payload
//!   falls back to the xino-off behavior (explicit fallback, never silently
//!   wrong).
//!
//! Revision 07 adds the `P1-07` consumption seam
//! [`IdentityPolicy::project_object_id_from_lower_id`]: the durable lower-id
//! record is projected through the SAME frozen matrix with the record's
//! `(fsid, real_ino)` as the identity input — constant `st_ino` across
//! copy-up (authority-continuity invariant). It is a new input to the
//! existing projection, never a replacement of `RealObjectKey`/the xino
//! matrix.
//!
//! # Locking
//!
//! [`IdentityPolicy`] is immutable policy inside `OverlayFs::identity`; the
//! only mutable state is the genuinely independent saturating
//! `fallback_ino_allocator` counter (priors `careful-atomics`), and
//! `layer_devs` is policy input, not runtime state and not a lock. The
//! projection functions are pure, lock-free transforms; they are called from
//! inode creation under the caller's `DIR` transaction (or lock-free at stat
//! time) and hold no Overlay lock (spec §3.3, §4 Lock Carriers).

use core::sync::atomic::{AtomicU64, Ordering};

use device_id::DeviceId;

use super::{entry::RealObject, lower_id::LowerIdRecord};
use crate::prelude::*;

/// The published `st_dev`/`st_ino` identity of one overlay object
/// (`P2-01`/`P0-12`).
///
/// The pair is precomputed once by [`IdentityPolicy`] at inode creation and
/// stored on the `OverlayInode`; stat reuses it without re-derivation. It is
/// an identity projection, never a reverse name map.
///
/// Wave-3 review item 3 widened the type to the overlayfs ceiling: it is the
/// return type of the published `OverlayInode::object_id()` accessor, so
/// sibling mesos must be able to name it (private-in-public closure).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::fs::fs_impls::overlayfs) struct OverlayObjectId {
    /// Published `st_dev`.
    pub(in crate::fs::fs_impls::overlayfs) dev: DeviceId,
    /// Published `st_ino`.
    pub(in crate::fs::fs_impls::overlayfs) ino: u64,
}

/// The `xino=` mount-option mode (`Off`/`Auto`/`On`).
///
/// Dependency note (spec §4 Enums + §3.5 item 2): the frozen spec places this
/// enum on meso-01's `OverlayMountOptions` surface and consumes it via the
/// (unpublished) `MountPolicy::xino_mode()` accessor. That publication is a
/// **recorded contract gap** — meso-01 does not define the type this wave —
/// so this declaration lives in the identity projection's own module and
/// [`IdentityPolicy::xino_mode`] is fixed to `Auto`. When meso-01 publishes
/// the accessor, this declaration should move to `mount/options.rs` and be
/// consumed from there (exit-plan condition, see the Creator report).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum XinoMode {
    /// xino encoding disabled; non-directories report the underlying dev/ino.
    #[expect(
        dead_code,
        reason = "frozen xino=off variant (spec §4); unreachable this wave — the xino= option publication gap (spec §3.5 item 2) fixes the policy to Auto"
    )]
    Off,
    /// xino enabled when feasible; this wave's fixed mode (recorded gap).
    Auto,
    /// xino encoding always enabled.
    #[expect(
        dead_code,
        reason = "frozen xino=on variant (spec §4); unreachable this wave — the xino= option publication gap (spec §3.5 item 2) fixes the policy to Auto"
    )]
    On,
}

/// The immutable per-mount dev/ino projection policy (`P2-01`/`P0-12`).
///
/// Invariants: `xino_shift <= 63` (enforced by [`IdentityPolicy::new`]);
/// `fallback_ino_allocator` never wraps (saturating, see
/// [`IdentityPolicy::allocate_fallback_ino`]); `is_all_layers_same_fs` is
/// fixed at construction; `layer_devs` is fixed at construction, fsid-sorted,
/// with one entry per published layer — never re-probed at runtime. Owner/
/// guard: immutable policy inside `OverlayFs::identity`; the allocator is a
/// genuinely independent counter (priors `careful-atomics`); `layer_devs` is
/// policy input, not runtime state and not a lock.
#[derive(Debug)]
pub(in crate::fs::fs_impls::overlayfs) struct IdentityPolicy {
    /// The `xino=` mode; fixed to `Auto` this wave (recorded meso-01
    /// publication gap, spec §3.5 item 2).
    xino_mode: XinoMode,
    /// The overlay's own `st_dev` (`AnonDeviceId`), acquired in the extended
    /// `OverlayFs::new` (spec §3.5 item 1).
    overlay_dev_id: DeviceId,
    /// High-bit encoding width of the xino layer id (e.g. `64 - 16` = 48-bit
    /// payload).
    xino_shift: u32,
    /// Whether every layer shares one underlying filesystem (`P0-12` fast
    /// path); derived at construction from the published layer dev ids.
    is_all_layers_same_fs: bool,
    /// fsid → origin-layer device table (`P1-07` revision 07); built at
    /// construction from the published layer snapshot, immutable.
    layer_devs: Box<[(u64, DeviceId)]>,
    /// Saturating fallback ino allocator for directories / anon inos when
    /// xino is not applicable.
    fallback_ino_allocator: AtomicU64,
}

impl IdentityPolicy {
    /// Constructs the immutable projection policy from the published layer
    /// snapshot (`P2-01`/`P0-12`/`P1-07`).
    ///
    /// `overlay_dev_id` is the overlay `AnonDeviceId` acquired in the extended
    /// `OverlayFs::new` (spec §3.5 item 1); `layer_devs` carries one
    /// `(fsid, container_dev_id)` entry per published layer, from the same
    /// snapshot that feeds `is_all_layers_same_fs` (recorded dependency 7 of
    /// the revision-07 ledger), and is normalized to fsid order here so the
    /// frozen "fsid-sorted" invariant holds regardless of caller order.
    /// `xino_mode` is fixed to `Auto` this wave (the `xino=` option /
    /// `MountPolicy::xino_mode()` publication is a recorded meso-01 contract
    /// gap, spec §3.5 item 2). The frozen invariant `xino_shift <= 63` is
    /// enforced at construction: a violating shift is a mount-policy
    /// programming error and is rejected instead of building a broken policy.
    ///
    /// Construction surface note: the frozen spec freezes no constructor
    /// signature for this private-field carrier while §3.5 item 1 mandates its
    /// construction in the extended `OverlayFs::new`; this associated
    /// constructor is the minimal surface that makes that contract
    /// implementable (whitelist Rule D — stable invariant carrier
    /// construction), with overlayfs-tree visibility so the `mount/build.rs`
    /// extension packet can invoke it. See the Creator report §5.
    pub(in crate::fs::fs_impls::overlayfs) fn new(
        overlay_dev_id: DeviceId,
        layer_devs: Box<[(u64, DeviceId)]>,
        xino_shift: u32,
    ) -> Result<Self> {
        if xino_shift > 63 {
            return_errno_with_message!(Errno::EINVAL, "invalid overlay xino shift");
        }
        let mut layer_devs = layer_devs;
        layer_devs.sort_by_key(|(fsid, _)| *fsid);
        let is_all_layers_same_fs = layer_devs
            .first()
            .is_some_and(|(_, first_dev)| layer_devs.iter().all(|(_, dev)| dev == first_dev));
        Ok(Self {
            xino_mode: XinoMode::Auto,
            overlay_dev_id,
            xino_shift,
            is_all_layers_same_fs,
            layer_devs,
            fallback_ino_allocator: AtomicU64::new(0),
        })
    }

    /// Returns whether the xino encoding branch of the frozen matrix applies
    /// (`P2-01`).
    ///
    /// Same-fs passthrough takes precedence: when every layer shares one
    /// underlying filesystem, raw underlying dev/ino is already uniform, so
    /// xino is not effective (the frozen matrix's first branch is selected
    /// before this check at the call sites). Otherwise `Auto`+feasible or
    /// `On` is effective; `Off` is not. Feasibility for `Auto` gates on every
    /// underlying filesystem providing persistent inode identity — Asterinas
    /// has no export-style FH surface this wave (recorded absence, spec
    /// [RELY]), so no feasibility probe exists and `Auto` is treated as
    /// feasible (spec §3.5 item 2: this wave always uses `Auto` semantics).
    pub(super) fn is_xino_effective(&self) -> bool {
        if self.is_all_layers_same_fs {
            return false;
        }
        matches!(self.xino_mode, XinoMode::Auto | XinoMode::On)
    }

    /// Projects the dev/ino identity of the visible-metadata source
    /// (`P2-01`/`P0-12`).
    ///
    /// The frozen matrix (spec §2 Case 8 / §4) is implemented once in the
    /// shared [`IdentityPolicy::project`] helper (wave-2 review item 9
    /// dedupe); this method supplies the visible source's `(fsid, real ino,
    /// origin dev)` and delegates.
    pub(in crate::fs::fs_impls::overlayfs) fn project_object_id(
        &self,
        real: &RealObject,
        is_directory: bool,
    ) -> OverlayObjectId {
        self.project(
            real.fsid(),
            real.real_inode().ino(),
            real.container_dev_id(),
            is_directory,
        )
    }

    /// Projects the dev/ino identity from the durable lower-id record
    /// (`P1-07` revision 07 consumption).
    ///
    /// The SAME frozen matrix as [`IdentityPolicy::project_object_id`]
    /// (implemented once in the shared [`IdentityPolicy::project`] helper)
    /// with the record's `(fsid, real_ino)` as the identity input and the
    /// origin-layer dev resolved from the immutable `layer_devs` table.
    /// Consumed on copied-up objects so `st_ino` stays constant across
    /// copy-up (authority-continuity invariant, spec §2 Case 10). The record
    /// is layer-validated before consumption (`read_lower_id`, wave-2 review
    /// item 5), so `layer_dev` never sees a foreign `fsid` on this path.
    pub(in crate::fs::fs_impls::overlayfs) fn project_object_id_from_lower_id(
        &self,
        lower_id: &LowerIdRecord,
        is_directory: bool,
    ) -> OverlayObjectId {
        self.project(
            lower_id.fsid(),
            lower_id.real_ino(),
            self.layer_dev(lower_id.fsid()),
            is_directory,
        )
    }

    /// Runs the frozen four-branch dev/ino projection matrix (spec §2 Case 8
    /// / §4) for one `(layer_id, real_ino, origin_dev)` identity input.
    ///
    /// Whitelist Rule B: the identical matrix is executed by both
    /// [`IdentityPolicy::project_object_id`] and
    /// [`IdentityPolicy::project_object_id_from_lower_id`] (two call paths
    /// inside this meso), so the branches — including the fit tests — live in
    /// exactly one place (wave-2 review item 9 dedupe).
    ///
    /// Frozen matrix: **1** same-fs passthrough (`origin_dev` + `real_ino`);
    /// **2** xino effective → overlay `st_dev` plus an encoded `st_ino` (the
    /// layer id in the high `xino_shift` bits, real ino in the payload); **3**
    /// xino off → directories get the overlay dev plus an allocated ino,
    /// non-directories report `origin_dev`/`real_ino`; **4** per-object
    /// overflow (the real ino does not fit the payload, or the layer id does
    /// not fit the `xino_shift`-bit layer-id space) → the explicit xino-off
    /// fallback. The layer-id fit test (wave-2 review item 6) closes the
    /// silent-truncation hole: without it, two layer ids differing only above
    /// bit `xino_shift` would encode to the same published `st_ino`. Uses
    /// checked arithmetic: the `payload_bits == 64` short-circuit skips the
    /// degenerate `xino_shift == 0` case and never shifts by the full bit
    /// width.
    fn project(
        &self,
        layer_id: u64,
        real_ino: u64,
        origin_dev: DeviceId,
        is_directory: bool,
    ) -> OverlayObjectId {
        // Frozen matrix branch 1: same-fs passthrough. All layers share one
        // underlying filesystem, so the origin layer's device is the shared
        // underlying dev and the real ino is already uniform.
        if self.is_all_layers_same_fs {
            return OverlayObjectId {
                dev: origin_dev,
                ino: real_ino,
            };
        }
        // Frozen matrix branch 2: xino effective with a fitting real ino AND
        // a fitting layer id (the layer id must fit the `xino_shift`-bit
        // layer-id space; higher bits would be silently dropped by the
        // encode — wave-2 review item 6).
        if self.is_xino_effective() {
            let payload_bits = 64 - self.xino_shift;
            if payload_bits == 64
                || (real_ino >> payload_bits == 0 && layer_id >> self.xino_shift == 0)
            {
                let encoded_ino = if payload_bits == 64 {
                    real_ino
                } else {
                    (layer_id << payload_bits) | real_ino
                };
                return OverlayObjectId {
                    dev: self.overlay_dev_id,
                    ino: encoded_ino,
                };
            }
            // Per-object overflow (real ino and/or layer id does not fit):
            // fall through to the explicit fallback.
        }
        // Frozen matrix branches 3/4: xino off (or per-object overflow
        // fallback): dirs get the overlay dev + an allocated ino; non-dirs
        // report the origin dev/ino.
        if is_directory {
            OverlayObjectId {
                dev: self.overlay_dev_id,
                ino: self.allocate_fallback_ino(),
            }
        } else {
            OverlayObjectId {
                dev: origin_dev,
                ino: real_ino,
            }
        }
    }

    /// Allocates a fallback ino for directories / anon objects when xino is
    /// not applicable.
    ///
    /// Saturating by construction (spec §4 invariant: the counter never
    /// wraps): [`AtomicU64::fetch_update`] commits `saturating_add(1)` and
    /// retries on contention, so the committed counter converges to and stays
    /// at `u64::MAX`; the returned value is the newly committed counter. The
    /// first allocation returns `1` (ino 0 is not handed out).
    fn allocate_fallback_ino(&self) -> u64 {
        match self.fallback_ino_allocator.fetch_update(
            Ordering::Relaxed,
            Ordering::Relaxed,
            |current| Some(current.saturating_add(1)),
        ) {
            // The closure never returns `None`, so `fetch_update` always
            // succeeds; this arm is defensive and unreachable.
            Ok(previous) => previous.saturating_add(1),
            Err(_) => u64::MAX,
        }
    }

    /// Resolves the origin-layer device for `fsid` from the immutable policy
    /// table (`P1-07` revision 07; the xino-off non-samefs branch).
    ///
    /// The table is built at construction from the same published layer
    /// snapshot the fsids come from (one entry per layer). The persisted
    /// lower-id `fsid` is layer-validated at the read boundary
    /// (`read_lower_id`, wave-2 review item 5) before this table is ever
    /// consulted, so a miss is a programming error, not a runtime condition;
    /// the defensive fallback reports the null device instead of fabricating
    /// an identity (never silently wrong).
    fn layer_dev(&self, fsid: u64) -> DeviceId {
        match self
            .layer_devs
            .iter()
            .find(|(candidate, _)| *candidate == fsid)
        {
            Some((_, dev)) => *dev,
            None => DeviceId::null(),
        }
    }
}
