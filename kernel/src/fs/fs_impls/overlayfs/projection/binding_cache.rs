// SPDX-License-Identifier: MPL-2.0

//! The unified binding algebra and the mount-wide binding cache
//! (`P0-08`/`P0-09`/`P0-10`/`P0-11` supporting surface).
//!
//! This module implements the frozen §4 `binding_cache.rs` surface of the
//! `visibility_projection_identity` meso spec (revision 07): one `Binding`
//! type used for both cache entries and lookup results, the per-name positive
//! binding (`{ kind, inode }` with zero per-name fact duplication), the
//! private negative reasons that all surface as `ENOENT`, and the mount-wide
//! `BindingCache` — the first source for `(parent, name)` lookup results
//! (`Binding-first` invariant). `BindingCache` is not a second layer registry
//! or identity table: it holds only strong pins — a positive entry pins its
//! `OverlayInode`, a negative entry pins its barrier object via
//! [`HiddenEvidence`] (BC-2 lifetime rule).
//!
//! Wave-2 repair item 8 (`minimize-copies`): the cache stores each parent's
//! name bindings in a per-parent inner map keyed by `Box<str>`, so a hit-path
//! probe borrows the name (`Box<str>: Borrow<str>`) and allocates nothing.
//! The review's suggested `Borrow<(RealObjectKey, str)> for BindingKey` is
//! not implementable — a tuple cannot be unsized into `(RealObjectKey, str)`
//! and `borrow()` must return a reference to a real value — so the nested
//! per-parent map is the equivalent allocation-free shape (recorded
//! deviation). Wave-2 repair item 2 widens [`BindingCache`] to the
//! overlayfs ceiling and adds [`BindingCache::new`] so `mount/build.rs`
//! stops building struct literals over private fields.

use hashbrown::HashMap;

use super::{inode::OverlayInode, inode_cache::RealObjectKey};
use crate::{fs::vfs::inode::Inode, prelude::*};

/// Outer positive/negative binding algebra (BC-2 §19) — one type for cache
/// AND lookup results.
///
/// No `Debug` derive (round-2 review item 2): a derived `Debug` would require
/// `OverlayInode: Debug`, which the frozen carrier deliberately does not
/// implement (inode.rs dev note — the `fs: Weak<OverlayFs>` field cannot
/// satisfy the derive); nothing in the tree formats a binding.
#[derive(Clone)]
pub(super) enum Binding {
    Positive(PositiveBinding),
    Negative(NegativeBinding),
}

/// A positive per-name binding: the per-name view classification plus the
/// shared overlay inode.
///
/// The real-object facts live once, in the shared inode's
/// `OverlayObjectFacts`; a `PositiveBinding` carries zero per-name fact
/// duplication (revision-04 model).
///
/// No `Debug` derive (round-2 review item 2): the `Arc<OverlayInode>` field
/// makes the derive unsatisfiable (`OverlayInode` deliberately has no `Debug`).
#[derive(Clone)]
pub(super) struct PositiveBinding {
    /// The per-name view classification of the bound name.
    pub(super) kind: PositiveKind,
    /// The shared inode for the bound name.
    pub(super) inode: Arc<OverlayInode>,
}

/// The per-name view classification of a positive binding.
///
/// `Single` = one real object; `Merged` = a directory merging upper + lower
/// observations.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum PositiveKind {
    /// One real object backs the name.
    Single,
    /// A directory merging upper + lower observations backs the name.
    Merged,
}

/// A negative per-name binding.
///
/// Every variant surfaces as `ENOENT` to VFS while the reason stays private
/// (BC-2 §18.2/§22); hidden bindings pin their barrier via
/// [`HiddenEvidence`] for lifetime + revalidation of the cached negative
/// answer.
#[derive(Clone, Debug)]
pub(super) enum NegativeBinding {
    /// The name is absent from every layer.
    Absent,
    /// The name is hidden by a whiteout barrier.
    HiddenByWhiteout(HiddenEvidence),
    /// The name is hidden by an opaque-directory barrier.
    HiddenByOpaque(HiddenEvidence),
}

/// The barrier evidence of a hidden name: the layer whose barrier hid the
/// name and a strong pin to the barrier object.
///
/// A live negative binding pins its barrier object; the pin serves lifetime
/// (BC-2 lifetime rule) and revalidation of the cached negative answer.
#[derive(Clone, Debug)]
pub(super) struct HiddenEvidence {
    /// The layer whose barrier hid the name.
    pub(super) layer_index: usize,
    /// Strong pin to the barrier object.
    pub(super) real_inode: Arc<dyn Inode>,
}

/// The publication carrier of one per-name binding: the parent directory
/// identity plus the exact name in the parent.
///
/// Keys are per-parent: same-name lookups of one parent serialize under that
/// parent's `DIR` transaction lock. The name is stored as a `Box<str>` so the
/// cache's per-parent inner maps probe it without an allocation (wave-2
/// repair item 8); this type is the `insert` carrier — the cache itself is
/// keyed by `(parent_id, name)` through the nested per-parent maps.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(super) struct BindingKey {
    /// The parent directory identity (visible-metadata source key).
    pub(super) parent_id: RealObjectKey,
    /// The exact name in the parent.
    pub(super) name: Box<str>,
}

/// The mount-wide binding cache — the first source for `(parent, name)`
/// lookup results (`Binding-first` invariant).
///
/// Entries are `Arc<Binding>` snapshots (replaced, never mutated in place); a
/// cached positive pins its inode and a cached negative pins its barrier
/// (`HiddenEvidence`). This is not a second layer registry or identity table
/// and holds no `ID -> name` reverse map. Internally the cache is a per-parent
/// map (`parent identity -> name -> binding`) so the hot `get` path probes by
/// borrowed `&str` without allocating (wave-2 repair item 8).
// No `Debug` derive (round-2 review item 2): it would cascade through
// `Binding` -> `PositiveBinding` -> `OverlayInode`, which deliberately has no
// `Debug`; nothing requires `BindingCache: Debug` (`OverlayFs` itself has no
// `Debug` impl).
pub(in crate::fs::fs_impls::overlayfs) struct BindingCache {
    /// Sleep-capable mount-wide cache (read-mostly; an internal data lock,
    /// not a topology level); insert/update happen under the caller's parent
    /// `DIR` transaction lock.
    entries: RwMutex<HashMap<RealObjectKey, HashMap<Box<str>, Arc<Binding>>>>,
}

impl BindingCache {
    /// Constructs an empty cache (wave-2 repair item 2: `mount/build.rs`
    /// initializes the field through this constructor instead of a struct
    /// literal over private fields).
    pub(in crate::fs::fs_impls::overlayfs) fn new() -> Self {
        Self {
            entries: RwMutex::new(HashMap::new()),
        }
    }

    /// Returns the cached binding for `(parent_id, name)`, if any.
    ///
    /// The two-level probe borrows the name (`Box<str>: Borrow<str>`, outer
    /// key `Copy`) and allocates nothing on the hit path (wave-2 repair item
    /// 8).
    pub(super) fn get(&self, parent_id: &RealObjectKey, name: &str) -> Option<Arc<Binding>> {
        self.entries.read().get(parent_id)?.get(name).cloned()
    }

    /// Inserts (or replaces) the cached binding for `(parent_id, name)`.
    ///
    /// The caller inserts/updates under the parent's `DIR` transaction lock;
    /// the entry is an immutable `Arc<Binding>` snapshot and is replaced,
    /// never mutated in place.
    pub(super) fn insert(&self, key: BindingKey, binding: Arc<Binding>) {
        let BindingKey { parent_id, name } = key;
        self.entries
            .write()
            .entry(parent_id)
            .or_default()
            .insert(name, binding);
    }

    /// Removes the cached binding for `(parent_id, name)` (mutation-Meso
    /// surface). An emptied per-parent map is pruned.
    #[expect(
        dead_code,
        reason = "frozen §4 mutation surface (spec: mutation-Meso surface); consumed by the namespace-mutation Meso once it lands"
    )]
    pub(super) fn invalidate(&self, parent_id: &RealObjectKey, name: &str) {
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
    pub(super) fn into_inode(self) -> Option<Arc<OverlayInode>> {
        match self {
            Binding::Positive(positive) => Some(positive.inode),
            Binding::Negative(_) => None,
        }
    }
}
