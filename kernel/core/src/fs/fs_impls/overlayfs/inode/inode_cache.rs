// SPDX-License-Identifier: MPL-2.0
#![short_vis_path::add(overlayfs)]

//! Inode identity-reuse cache of the overlay inode module.
//!
//! [`InodeCache`] maps each real-object identity ([`RealObjectKey`]) to the
//! shared [`OverlayInode`]. The hard-link invariant holds: while any
//! reference to an overlay inode lives, every lookup that resolves the same
//! real object (same `fsid`, same real inode number) reuses the same inode
//! instead of constructing a duplicate one.

use core::{
    fmt::Debug,
    sync::atomic::{AtomicU64, Ordering},
};

use hashbrown::HashMap;

use crate::{
    fs::{
        fs_impls::overlayfs::{inode::OverlayInode, real::RealObjectKey},
        vfs::inode::Inode,
    },
    prelude::*,
};

const SWEEP_INTERVAL: u64 = 1024;

#[derive(Clone)]
struct InodeCacheEntry {
    /// Weak pin to the shared [`OverlayInode`].
    carrier: Weak<OverlayInode>,
    /// Strong keep-alive of the real inode denoted by this entry's key when
    /// the inode's facts no longer pin it (stale alias); `None` otherwise.
    keep_alive: Option<Arc<dyn Inode>>,
}

impl Debug for InodeCacheEntry {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("InodeCacheEntry")
            .field("carrier", &self.carrier)
            .field(
                "keep_alive",
                &self.keep_alive.as_ref().map(|_| "<real-inode keep-alive>"),
            )
            .finish()
    }
}

/// The mount-wide inode identity-reuse cache.
///
/// Invariants: one real object maps to one [`OverlayInode`] while any
/// reference lives. After a copy-up facts transition the inode is also
/// registered under a retained old-key alias, retired by the dead-pin sweep
/// once the inode drops.
#[derive(Debug)]
pub(in overlayfs) struct InodeCache {
    /// Weak inode pins (with optional stale-alias keep-alives).
    entries: RwMutex<HashMap<RealObjectKey, InodeCacheEntry>>,
    /// Miss-path insert counter driving the `SWEEP_INTERVAL`-based dead-entry
    /// sweep.
    misses_since_sweep: AtomicU64,
}

impl InodeCache {
    pub(in overlayfs) fn new() -> Self {
        Self {
            entries: RwMutex::new(HashMap::new()),
            misses_since_sweep: AtomicU64::new(0),
        }
    }

    /// Returns the cached overlay inode for `key`, if a live inode is
    /// registered.
    pub(super) fn get(&self, key: RealObjectKey) -> Option<Arc<OverlayInode>> {
        self.entries
            .read()
            .get(&key)
            .and_then(|entry| entry.carrier.upgrade())
    }

    /// Publishes the committing instance under its post-copy-up key and
    /// re-takes the old key as its stale alias.
    ///
    /// Infallible by construction: the committing pin is a parameter (never
    /// derived from the old-key slot, which a sibling's mint may transiently
    /// hold), both inserts are unconditional, and a same-object or ino-reuse
    /// occupant at the new key is superseded, not failed. Leaf lock: runs
    /// under `entries.write()` alone, waits for nothing, calls nothing that
    /// locks.
    pub(super) fn publish_rekey(
        &self,
        old_key: RealObjectKey,
        new_key: RealObjectKey,
        old_real_inode: Arc<dyn Inode>,
        pin: &Arc<OverlayInode>,
    ) {
        let mut guard = self.entries.write();
        // Diagnostic classification of a live different-instance occupant at
        // the new key. The unconditional publication below supersedes every
        // class; the log only records which convergence happened.
        if let Some(existing) = guard.get(&new_key)
            && existing.carrier.strong_count() > 0
            && !Weak::ptr_eq(&existing.carrier, &Arc::downgrade(pin))
            && let Some(occupant) = existing.carrier.upgrade()
        {
            if occupant.contains_real_inode(pin.visible_source().real_inode()) {
                // The same real object was projected early under
                // the new key by a concurrent lookup; the
                // committing copy-up inode supersedes it.
                notice!(
                    "overlay inode-cache convergence at the post-transition key \
                     {:?}: an early projection of the same real object is \
                     superseded by the committing inode",
                    new_key
                );
            } else {
                // A different object (ino reuse) stale-occupies
                // the new key; its registration is replaced.
                error!(
                    "overlay inode-cache stale identity at the post-transition key \
                     {:?}: replacing the occupant with the committing inode \
                     (ino reuse)",
                    new_key
                );
            }
            // The occupant died racing this check: superseded as a
            // dead pin by the publication below.
        }
        guard.insert(
            new_key,
            InodeCacheEntry {
                carrier: Arc::downgrade(pin),
                keep_alive: None,
            },
        );
        if new_key != old_key {
            guard.insert(
                old_key,
                InodeCacheEntry {
                    carrier: Arc::downgrade(pin),
                    keep_alive: Some(old_real_inode),
                },
            );
        }
    }

    /// Returns the cached overlay inode for `key`, or creates and publishes
    /// one via `create_fn` on a miss.
    ///
    /// On a live hit, `is_same_object` validates the cached inode; a stale
    /// inode (backing-fs ino reuse) is evicted and replaced so the key is
    /// never served a different real object. The check-then-publish sequence
    /// is atomic.
    pub(super) fn get_or_create(
        &self,
        key: RealObjectKey,
        is_same_object: impl FnOnce(&Arc<OverlayInode>) -> bool,
        create_fn: impl FnOnce() -> Arc<OverlayInode>,
    ) -> Arc<OverlayInode> {
        let guard = self.entries.upread();
        if let Some(inode) = guard.get(&key).and_then(|entry| entry.carrier.upgrade()) {
            if is_same_object(&inode) {
                return inode;
            }
            error!(
                "overlay inode-cache stale identity at key {:?}: the cached inode no \
                 longer denotes the same real object (ino reuse); replacing it",
                key
            );
        }
        let inode = create_fn();
        let mut guard = guard.upgrade();
        // O(1) per-key eviction: clears any stale weak pin before the fresh
        // inode replaces it.
        guard.remove(&key);
        // Amortized full sweep: every `SWEEP_INTERVAL`-th miss.
        let misses = self.misses_since_sweep.fetch_add(1, Ordering::Relaxed) + 1;
        if misses.is_multiple_of(SWEEP_INTERVAL) {
            // Reclaims dead inode pins AND their stale-alias keep-alives:
            // dropping the entry drops the keep-alive `Arc`.
            guard.retain(|_, entry| entry.carrier.strong_count() > 0);
        }
        guard.insert(
            key,
            InodeCacheEntry {
                carrier: Arc::downgrade(&inode),
                keep_alive: None,
            },
        );
        inode
    }
}
