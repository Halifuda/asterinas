// SPDX-License-Identifier: MPL-2.0

//! The copy-up coordination (`CUL`) payload — `P1-01`.
//!
//! This module owns the two frozen `P1-01` types of the meso-04 spec §4
//! `copyup/coordination.rs`: [`CopyUpTransition`] — the durable per-object
//! publication coordinate (`publication_parent` + `name` + `phase`) — and
//! its [`CopyUpPhase`] transition marker. The payload is exactly the
//! promotion coordinate plus phase; no unrelated fields and no stored
//! "copy-up completed" history marker (BC-4 §37.2: the upper authority in
//! the facts record is the durable outcome).
//!
//! Owner/guard: the payload lives under
//! `OverlayInode::copyup_transition: Mutex<Option<CopyUpTransition>>`, the
//! `CUL` level-3 domain (frozen Wave-3 seam in `projection/inode.rs`).
//! `None` only before the first positive-binding publication (invariant I3);
//! the guard is a sleep-capable `ostd::sync::Mutex` (promotion can BIO under
//! it — spec §4 lock-carrier table), and waiters hold nothing while blocked
//! on `lock()` (invariant I2). The coordinate is recorded once at the first
//! positive-binding publication and read by every winner; `phase` transitions
//! advance the coordinate's marker, never the authority (BC-4 §37.2).
//!
//! Visibility: both types are declared at the overlayfs ceiling
//! (`pub(in crate::fs::fs_impls::overlayfs)`) because the frozen Wave-3
//! `OverlayInode::copyup_transition` field in the sibling `projection` tree
//! names [`CopyUpTransition`] (packet override: the spec's unqualified
//! `pub(super)` is read through the meso-01/02 visibility audit, "cross-module
//! items within `overlayfs`"). The struct fields stay `pub(super)`, visible
//! across the `copyup` module tree — the only consumers:
//! `record_copyup_transition` (`copyup/mod.rs`) writes the coordinate,
//! `ensure_upper_authority` (`copyup/trigger.rs`) reads it under `CUL`, and
//! `promote` (`copyup/promote.rs`) advances the phase.

use crate::{
    fs::fs_impls::overlayfs::projection::OverlayInode,
    prelude::*,
};

/// The copy-up publication coordinate and phase of one logical overlay
/// object (`P1-01`, meso-04 spec §4 `copyup/coordination.rs`).
///
/// The `CUL`-domain payload stored at
/// `OverlayInode::copyup_transition`: the promotion coordinate plus phase,
/// recorded exactly once at the first positive-binding publication and read
/// by every subsequent winner (invariant I3 — the coordinate is immutable
/// after the first record; only `phase` transitions). The strong [`Arc`] pin
/// in [`Self::publication_parent`] forms the publication-parent chain
/// (acyclic, root-terminated; no cycle), so the trigger's top-down ancestor
/// walk terminates at the upper-backed root and never re-enters the same
/// instance.
///
/// The spec's `#[derive(Debug)]` is dropped: `OverlayInode` carries no
/// `Debug` impl (its frozen `Weak<OverlayFs>` field cannot satisfy a derived
/// `Debug` bound — wave-3 `projection/inode.rs` precedent), so the derive is
/// unsatisfiable and no shape hint is lost.
pub(in crate::fs::fs_impls::overlayfs) struct CopyUpTransition {
    /// The logical parent overlay inode (may still be lower-backed; the
    /// parent's upper existence is resolved by the trigger's ancestor walk,
    /// never assumed ready — meso-04 spec §4 override 3).
    pub(super) publication_parent: Arc<OverlayInode>,
    /// The exact publication name under `publication_parent`; non-empty.
    pub(super) name: String,
    /// The transition marker consumed by the next winner entry; the upper
    /// authority in the facts record, not this marker, is the durable outcome
    /// (BC-4 §37.2).
    pub(super) phase: CopyUpPhase,
}

/// The transition marker of one copy-up coordination (`P1-01`, meso-04 spec
/// §4 `copyup/coordination.rs`).
///
/// Semantic mapping (BC-4 §37.3): lower-authoritative = `facts.upper` none +
/// [`CopyUpPhase::Idle`]; promotion-in-progress = the `CUL` guard held by the
/// winner (observable only as mutex contention); upper-authoritative =
/// `facts.upper` some; retryable failure = the error returned to the caller
/// (authority unchanged, no durable marker needed); reconcile-required =
/// [`CopyUpPhase::ReconcilePending`]. No "copy-up completed" history marker
/// is stored.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::fs::fs_impls::overlayfs) enum CopyUpPhase {
    /// No unfinished transition; a lower authority (if any) is clean.
    Idle,
    /// Physical publication happened but semantic publication failed; the
    /// upper object at `(publication_parent, name)` must be verified before
    /// reuse (BC-4 §38.3/§45.2).
    ReconcilePending,
}
