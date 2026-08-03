// SPDX-License-Identifier: MPL-2.0

//! Inode identity-reuse cache of the overlay projection (`P0-16`).
//!
//! This module owns the [`RealObjectKey`] identity pair and the mount-wide
//! [`InodeCache`] that maps each real-object identity to the shared
//! [`OverlayInode`] carrier. The `P0-16` hard-link invariant holds: while any
//! reference to an overlay inode lives, every lookup that resolves the same
//! real object (same `fsid`, same real inode number) reuses the same carrier
//! instead of constructing a duplicate one.
//!
//! # Locking
//!
//! [`InodeCache`] is the mount-wide `OverlayFs::inodes` cache; its
//! `ostd::sync::RwMutex` is an internal data lock (not a topology level), and
//! [`InodeCache::get_or_create`] follows the VFS children-cache `upread` →
//! `upgrade` pattern so the check-then-publish sequence is atomic: a writer
//! cannot enter while an upgradeable reader is held, and the upgradeable-reader
//! slot is single, so concurrent creators for one key are serialized. Values
//! are `Weak<OverlayInode>` pins, so the cache never forms an
//! `OverlayFs → OverlayInode → OverlayFs` strong cycle.

use core::sync::atomic::{AtomicU64, Ordering};

use hashbrown::HashMap;

use super::{entry::RealObject, inode::OverlayInode};
use crate::prelude::*;

/// Full-map dead-entry sweep interval (round-2 review item 3): one O(live)
/// sweep per this many miss-path inserts keeps dead `Weak` accumulation
/// bounded with O(1) amortized cost on the per-path-component lookup hot path.
const SWEEP_INTERVAL: u64 = 1024;

/// The identity of the real object that is the visible-metadata source of an
/// overlay inode (`P0-16`).
///
/// The pair is the layer `fsid` of the visible-metadata source (upper, else
/// topmost lower) and that source's real inode number. Hard links to the same
/// real object collapse onto one key, and merged directories key on their
/// visible-metadata source; there is deliberately no `ID -> name` reverse map.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(super) struct RealObjectKey {
    /// Layer fsid of the visible-metadata source (upper, else topmost lower).
    fsid: u64,
    /// Real inode number of the visible-metadata source.
    real_ino: u64,
}

impl RealObjectKey {
    /// Builds the identity-reuse key from the visible-metadata source.
    ///
    /// Returns the `(fsid, real_inode.ino())` pair of `real`; merged
    /// directories are keyed by their visible-metadata source.
    pub(super) fn from_source(real: &RealObject) -> Self {
        Self {
            fsid: real.fsid(),
            real_ino: real.real_inode().ino(),
        }
    }
}

/// The mount-wide inode identity-reuse cache (`P0-16`).
///
/// Invariants: same real object → one key → one [`OverlayInode`] carrier
/// while any reference lives; merged directories key on their visible-metadata
/// source; no `ID -> name` reverse map exists. The values are weak pins so the
/// cache never keeps an inode alive by itself. Widened to the overlayfs
/// ceiling (wave-2 repair item 2) with the [`InodeCache::new`] constructor so
/// `mount/build.rs` can initialize the field.
#[derive(Debug)]
pub(in crate::fs::fs_impls::overlayfs) struct InodeCache {
    /// Weak values; no `OverlayFs → OverlayInode → OverlayFs` cycle.
    by_key: RwMutex<HashMap<RealObjectKey, Weak<OverlayInode>>>,
    /// Miss-path insert counter driving the amortized dead-entry sweep
    /// (round-2 review item 3): a full sweep runs every `SWEEP_INTERVAL`
    /// misses, keeping the round-1 bounded-memory property with O(1)
    /// amortized cost instead of an O(live) sweep on every miss.
    misses_since_sweep: AtomicU64,
}

impl InodeCache {
    /// Constructs an empty cache (wave-2 repair item 2: `mount/build.rs`
    /// initializes the field through this constructor instead of a struct
    /// literal over private fields).
    pub(in crate::fs::fs_impls::overlayfs) fn new() -> Self {
        Self {
            by_key: RwMutex::new(HashMap::new()),
            misses_since_sweep: AtomicU64::new(0),
        }
    }

    /// Returns the cached overlay inode for `key`, or creates and publishes
    /// one via `create_fn` on a miss.
    ///
    /// The check-then-publish sequence is atomic (`upread` → `upgrade`): while
    /// the upgradeable read guard is held no writer can publish another
    /// carrier for the same key, and the single upgradeable-reader slot
    /// serializes concurrent creators, so exactly one carrier per key is ever
    /// published. A stale `Weak` entry whose inode has been dropped is
    /// evicted per-key in O(1) (round-2 review item 3: the earlier whole-map
    /// `retain` sweep was O(live) on every miss, a per-path-component hot
    /// path), and an amortized full sweep every `SWEEP_INTERVAL` misses keeps
    /// the round-1 bounded-memory property (wave-2 repair item 7) without
    /// per-miss linear cost.
    pub(super) fn get_or_create(
        &self,
        key: RealObjectKey,
        create_fn: impl FnOnce() -> Arc<OverlayInode>,
    ) -> Arc<OverlayInode> {
        let guard = self.by_key.upread();
        if let Some(inode) = guard.get(&key).and_then(Weak::upgrade) {
            return inode;
        }
        let inode = create_fn();
        let mut guard = guard.upgrade();
        // O(1) per-key eviction: the single upgradeable-reader slot guarantees
        // no writer published this key between the miss read and the upgrade,
        // so `remove` is a no-op when the key was absent and otherwise clears
        // the stale weak pin before the fresh carrier replaces it.
        guard.remove(&key);
        // Amortized full sweep: every `SWEEP_INTERVAL`-th miss, evict the
        // whole map's dead weak pins under the same write guard (O(live) but
        // only once per interval — O(1) amortized per miss).
        let misses = self.misses_since_sweep.fetch_add(1, Ordering::Relaxed) + 1;
        if misses % SWEEP_INTERVAL == 0 {
            guard.retain(|_, weak| weak.strong_count() > 0);
        }
        guard.insert(key, Arc::downgrade(&inode));
        inode
    }
}
