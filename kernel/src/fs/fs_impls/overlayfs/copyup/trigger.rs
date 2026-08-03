// SPDX-License-Identifier: MPL-2.0

//! The copy-up promotion trigger — the single winner/waiter entry that gives
//! an existing logical overlay object upper authority (`P1-02`/`P1-01`).
//!
//! [`OverlayInode::ensure_upper_authority`] is the only entry into
//! lower-to-upper authority promotion. Mutating entries (write-intent `open`,
//! `resize`, `fallocate`, and the meso-05 `setattr` seam) derive a coarse
//! mutating-vs-read-only class from the VFS surface, pass the EROFS gate
//! upstream, and call this method; read-only operations never reach it
//! (meso-04 spec §4.2 recipe flow). It takes **no trusted intent parameter**:
//! the promotion scope — how many ancestor parents to promote and which
//! object-kind recipe to run — is decided inside the per-object `CUL` domain
//! by inspection of the facts, the per-object publication coordinate
//! (`copyup_transition`), and the parent chain (BC-4 §37.2/§37.3).
//!
//! Winner/waiter flow (BC-4 §37.3): the top-down ancestor walk promotes the
//! parent chain strictly parent-`CUL`-before-child-`CUL`, terminating at the
//! upper-backed root. After the walk, the winner's arbitration guard
//! (`copyup_transition`) is acquired and — wave-4 repair item 4 — HELD
//! through publication: under the guard the winner re-snapshots the facts
//! (Case 3 waiter leg), re-reads the coordinate once, and runs the promotion
//! body ([`OverlayInode::promote`], promote.rs) with the coordinate carried
//! as parameters. `promote`'s helpers consume the passed coordinate and never
//! re-acquire `copyup_transition`, so the non-reentrant `ostd::sync::Mutex`
//! is never re-entered (spec §3.3 Hazard 2) and no concurrent winner can
//! interleave between the re-snapshot and the semantic publication — the
//! double copy-up TOCTOU (BC-4 §38.3) is closed. Waiters block on the
//! sleep-capable `CUL` lock and re-observe authority immediately after
//! acquisition (fast-path re-snapshot), never holding `CUL`/`INODE`/`UPPER`
//! while sleeping (invariant I2). The ReconcilePending marker (Case 6
//! recovery) is derived from the coordinate phase inside `promote` under the
//! held guard — no redundant bool crosses this boundary (wave-4 round-2
//! repair item 3). The outlet returns with every Overlay lock released;
//! `Ok(())` is the sole success carrier and the caller re-observes authority
//! via `facts_snapshot` (spec §2 post-conditions).

use crate::{
    fs::fs_impls::overlayfs::projection::OverlayInode,
    prelude::*,
};

impl OverlayInode {
    /// Promotes this logical object to upper authority, winning or waiting on
    /// the per-object `CUL` (`copyup_transition`).
    ///
    /// Returns `Ok(())` when the object is already upper-backed (idempotent
    /// fast path, Case 2), when it became upper-backed while waiting for the
    /// `CUL` (Case 3 waiter leg), or after this task won and completed the
    /// promotion; returns `Err(Errno::ENOENT)` when no publication coordinate
    /// is recorded (Case 7 defensive guard), and propagates any underlying
    /// recipe failure unchanged (Cases 5/6).
    ///
    /// Lock contract (meso-04 spec §4 steps 1-9, reconciled by wave-4 repair
    /// item 4): the brief `CUL` read that captures `publication_parent`
    /// releases its guard before the recursive ancestor walk, so the parent
    /// `CUL` is always acquired strictly before the child `CUL`; the
    /// arbitration guard is then acquired and held THROUGH the winner body —
    /// the re-snapshot (Case 3), the coordinate re-read, and
    /// [`OverlayInode::promote`] (which carries the coordinate and never
    /// re-acquires the guard; `ostd::sync::Mutex` is non-reentrant, spec §3.3
    /// Hazard 2). No Overlay lock crosses the return boundary (outlet, spec
    /// §3.1).
    pub(in crate::fs::fs_impls::overlayfs) fn ensure_upper_authority(&self) -> Result<()> {
        // Step 1 — mount-lifetime pin: the `Weak<OverlayFs>` upgrade proves
        // the mount is alive and pins it for the trigger's duration (the
        // recorded post-teardown platform-lifetime note, spec §3.4 item 5 —
        // no `.unwrap()`/`.expect()`).
        let _fs = self.fs_arc()?;

        // Step 2 — idempotent upper fast path (Case 2): facts inspection only,
        // no `CUL`, no second temporary, no second transfer (BC-4 §38.2).
        if self.facts_snapshot().upper().is_some() {
            return Ok(());
        }

        // Step 3 — the publication coordinate (Case 7 guard): the brief `CUL`
        // read clones the logical parent out of the coordinate so the guard
        // is released before the recursive ancestor walk (invariant I3:
        // `Some` after the first positive-binding publication).
        let publication_parent = {
            let transition = self.copyup_transition.lock();
            let Some(coordinate) = transition.as_ref() else {
                return Err(Error::with_message(
                    Errno::ENOENT,
                    "the overlay object has no recorded copy-up publication coordinate",
                ));
            };
            coordinate.publication_parent.clone()
        };

        // Step 4 — top-down ancestor walk (P1-03): the parent promotes its own
        // ancestors first, so the parent `CUL` is strictly acquired before the
        // child `CUL`; the recursion terminates at the upper-backed root and
        // never re-enters the same instance (acyclic chain, I3).
        publication_parent.ensure_upper_authority()?;

        // Step 5 — winner/waiter serialization (I2): the sleep-capable `CUL`
        // wait. Waiters hold nothing while blocked on `lock()`; the guard is
        // then held for the arbitration, the Case-3 re-snapshot, and — wave-4
        // repair item 4 — the whole winner body (promote runs under the
        // guard, so no second winner can interleave between the re-snapshot
        // and the semantic publication).
        let mut transition = self.copyup_transition.lock();

        // Step 6 — re-snapshot under the guard: another task won and promoted
        // while this task waited; re-observe upper authority and return the
        // same `Ok(())` success carrier (Case 3 waiter leg).
        if self.facts_snapshot().upper().is_some() {
            return Ok(());
        }

        // Step 7 — winner body under the held guard (wave-4 repair item 4):
        // read the coordinate once and run `promote`, which verifies a
        // pending reconcile (Case 6) and runs the object-kind recipe
        // (file/symlink/dir/special) through publication. The phase
        // transitions (ReconcilePending on Case 6, Idle on success) are
        // written through the same coordinate borrow; promote's helpers take
        // the passed coordinate and never re-read `copyup_transition` (no
        // non-reentrant deadlock, spec §3.3 Hazard 2).
        let coordinate = match transition.as_mut() {
            Some(coordinate) => coordinate,
            None => {
                return Err(Error::with_message(
                    Errno::ENOENT,
                    "the overlay object has no recorded copy-up publication coordinate",
                ));
            }
        };
        let publication_parent = coordinate.publication_parent.clone();
        let name = coordinate.name.clone();

        // Step 8 — the winner body; Step 9 (release `CUL`) is the guard drop
        // at this function's return. The ReconcilePending marker is derived
        // inside `promote` from the passed coordinate's phase under the held
        // guard (wave-4 round-2 repair item 3 — no redundant bool crosses
        // the trigger boundary).
        self.promote(&publication_parent, &name, coordinate)?;
        Ok(())
    }
}
