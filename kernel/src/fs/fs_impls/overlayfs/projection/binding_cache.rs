// SPDX-License-Identifier: MPL-2.0

//! The unified binding algebra and the mount-wide binding cache.
//!
//! This module implements one `Binding` type used for both cache entries and
//! lookup results, the per-name positive binding (an inode with zero per-name
//! fact duplication), the private negative reasons that all surface as
//! `ENOENT`, and the mount-wide `BindingCache` — the first source for
//! `(parent, name)` lookup results. `BindingCache` is not a second layer
//! registry or identity table: it holds only strong pins — a positive entry
//! pins its `OverlayInode`, a negative entry pins its barrier object via
//! [`HiddenEvidence`].
//!
//! The cache stores each parent's name bindings in a per-parent inner map
//! keyed by `Box<str>`, so a hit-path probe borrows the name
//! (`Box<str>: Borrow<str>`) and allocates nothing. A tuple-based borrowed
//! key is not implementable — a tuple cannot be unsized into
//! `(RealObjectKey, str)` and `borrow()` must return a reference to a real
//! value — so the nested per-parent map is the equivalent allocation-free
//! shape.

use hashbrown::HashMap;

use super::{inode::OverlayInode, inode_cache::RealObjectKey};
use crate::{fs::vfs::inode::Inode, prelude::*};

type BindingsByName = HashMap<Box<str>, Arc<Binding>>;
type BindingsByParent = HashMap<RealObjectKey, BindingsByName>;

/// Outer positive/negative binding algebra — one type for cache AND lookup
/// results.
///
/// The leaf modules dispatch on the algebra (`readdir_index.rs` via
/// `into_inode`, `dir/` via the positive/negative variants).
#[derive(Clone)]
pub(in crate::fs::fs_impls::overlayfs) enum Binding {
    Positive(PositiveBinding),
    Negative(NegativeBinding),
}

/// A positive per-name binding: the shared overlay inode.
///
/// The real-object facts live once, in the shared inode's
/// `OverlayObjectFacts`; a `PositiveBinding` carries zero per-name fact
/// duplication.
#[derive(Clone)]
pub(in crate::fs::fs_impls::overlayfs) struct PositiveBinding {
    /// The shared inode for the bound name.
    pub(super) inode: Arc<OverlayInode>,
}

impl PositiveBinding {
    /// Constructs a positive per-name binding.
    ///
    /// The namespace-mutation module publishes positive bindings through this
    /// constructor; the inode-only shape avoids per-name fact duplication.
    pub(in crate::fs::fs_impls::overlayfs) fn new(inode: Arc<OverlayInode>) -> Self {
        Self { inode }
    }

    /// Returns the inode carrier bound to this positive name.
    pub(in crate::fs::fs_impls::overlayfs) fn inode(&self) -> Arc<OverlayInode> {
        self.inode.clone()
    }
}

/// The per-name view classification of a positive binding.
///
/// `Single` = one real object; `Merged` = a directory merging upper + lower
/// observations.
///
/// The leaf modules branch on the per-name view classification.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::fs::fs_impls::overlayfs) enum PositiveKind {
    /// One real object backs the name.
    Single,
    /// A directory merging upper + lower observations backs the name.
    Merged,
}

/// A negative per-name binding.
///
/// Every variant surfaces as `ENOENT` to VFS while the reason stays private;
/// hidden bindings pin their barrier via [`HiddenEvidence`] for lifetime +
/// revalidation of the cached negative answer.
#[derive(Clone, Debug)]
pub(in crate::fs::fs_impls::overlayfs) enum NegativeBinding {
    /// The name is absent from every layer.
    Absent,
    /// The name is hidden by a whiteout barrier.
    HiddenByWhiteout(
        #[expect(
            dead_code,
            reason = "retained hidden-whiteout evidence carries barrier provenance and a lifetime pin for cached negative bindings"
        )]
        HiddenEvidence,
    ),
    /// The name is hidden by an opaque-directory barrier.
    HiddenByOpaque(
        #[expect(
            dead_code,
            reason = "retained hidden-opaque evidence carries barrier provenance and a lifetime pin for cached negative bindings"
        )]
        HiddenEvidence,
    ),
}

/// The barrier evidence of a hidden name: the layer whose barrier hid the
/// name and a strong pin to the barrier object.
///
/// A live negative binding pins its barrier object; the pin serves lifetime
/// and revalidation of the cached negative answer.
#[derive(Clone, Debug)]
pub(in crate::fs::fs_impls::overlayfs) struct HiddenEvidence {
    /// The layer whose barrier hid the name.
    #[expect(
        dead_code,
        reason = "retained hidden-barrier provenance supports conservative cached-negative revalidation"
    )]
    pub(super) layer_index: usize,
    /// Strong pin to the barrier object.
    #[expect(
        dead_code,
        reason = "strong pin retains the hidden barrier for the cached-negative binding lifetime"
    )]
    pub(super) real_inode: Arc<dyn Inode>,
}

impl HiddenEvidence {
    /// Constructs the barrier evidence of a hidden name.
    ///
    /// The namespace-mutation module pins a published whiteout's barrier
    /// object through this constructor; the strong `real_inode` pin serves
    /// the lifetime rule (a live negative binding pins its barrier) and the
    /// revalidation of the cached negative answer.
    pub(in crate::fs::fs_impls::overlayfs) fn new(
        layer_index: usize,
        real_inode: Arc<dyn Inode>,
    ) -> Self {
        Self {
            layer_index,
            real_inode,
        }
    }
}

/// The publication carrier of one per-name binding: the parent directory
/// identity plus the exact name in the parent.
///
/// Keys are per-parent: same-name lookups of one parent serialize under that
/// parent's `DIR` transaction lock. The name is stored as a `Box<str>` so the
/// cache's per-parent inner maps probe it without an allocation; this type is
/// the `insert` carrier — the cache itself is keyed by `(parent_id, name)`
/// through the nested per-parent maps.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(in crate::fs::fs_impls::overlayfs) struct BindingKey {
    /// The parent directory identity (visible-metadata source key).
    pub(super) parent_id: RealObjectKey,
    /// The exact name in the parent.
    pub(super) name: Box<str>,
}

impl BindingKey {
    /// Constructs a publication key for one per-parent name.
    ///
    /// The namespace-mutation module builds keys inline for `BindingCache`
    /// publication under the parent `DIR` transaction. The signature takes
    /// the exact `name` as a `String`; the key stores it as a `Box<str>` so
    /// the cache's per-parent inner maps probe it without an allocation.
    pub(in crate::fs::fs_impls::overlayfs) fn new(parent_id: RealObjectKey, name: String) -> Self {
        Self {
            parent_id,
            name: name.into(),
        }
    }
}

/// The mount-wide binding cache — the first source for `(parent, name)`
/// lookup results.
///
/// Entries are `Arc<Binding>` snapshots (replaced, never mutated in place); a
/// cached positive pins its inode and a cached negative pins its barrier
/// (`HiddenEvidence`). This is not a second layer registry or identity table
/// and holds no `ID -> name` reverse map. Internally the cache is a per-parent
/// map (`parent identity -> name -> binding`) so the hot `get` path probes by
/// borrowed `&str` without allocating.
pub(in crate::fs::fs_impls::overlayfs) struct BindingCache {
    /// Sleep-capable mount-wide cache (read-mostly; an internal data lock);
    /// insert/update happen under the caller's parent `DIR` transaction lock.
    entries: RwMutex<BindingsByParent>,
}

impl BindingCache {
    /// Constructs an empty cache.
    pub(in crate::fs::fs_impls::overlayfs) fn new() -> Self {
        Self {
            entries: RwMutex::new(HashMap::new()),
        }
    }

    /// Returns the cached binding for `(parent_id, name)`, if any.
    ///
    /// The two-level probe borrows the name (`Box<str>: Borrow<str>`, outer
    /// key `Copy`) and allocates nothing on the hit path.
    pub(in crate::fs::fs_impls::overlayfs) fn get(
        &self,
        parent_id: &RealObjectKey,
        name: &str,
    ) -> Option<Arc<Binding>> {
        self.entries.read().get(parent_id)?.get(name).cloned()
    }

    /// Inserts (or replaces) the cached binding for `(parent_id, name)`.
    ///
    /// The caller inserts/updates under the parent's `DIR` transaction lock;
    /// the entry is an immutable `Arc<Binding>` snapshot and is replaced,
    /// never mutated in place.
    pub(in crate::fs::fs_impls::overlayfs) fn insert(
        &self,
        key: BindingKey,
        binding: Arc<Binding>,
    ) {
        let BindingKey { parent_id, name } = key;
        self.entries
            .write()
            .entry(parent_id)
            .or_default()
            .insert(name, binding);
    }

    /// Removes the cached binding for `(parent_id, name)`. An emptied
    /// per-parent map is pruned.
    pub(in crate::fs::fs_impls::overlayfs) fn invalidate(
        &self,
        parent_id: &RealObjectKey,
        name: &str,
    ) {
        let mut guard = self.entries.write();
        if let Some(inner) = guard.get_mut(parent_id) {
            inner.remove(name);
            if inner.is_empty() {
                guard.remove(parent_id);
            }
        }
    }
}

impl Binding {
    /// Returns the shared inode for a positive binding; `None` for a
    /// negative binding.
    ///
    /// The merged-directory scan consumes it per entry.
    pub(in crate::fs::fs_impls::overlayfs) fn into_inode(self) -> Option<Arc<OverlayInode>> {
        match self {
            Binding::Positive(positive) => Some(positive.inode),
            Binding::Negative(_) => None,
        }
    }
}
