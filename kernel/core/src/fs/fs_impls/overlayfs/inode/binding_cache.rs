// SPDX-License-Identifier: MPL-2.0

#![short_vis_path::add(overlayfs)]

//! Temporary binding-cache compatibility types.
//!
//! The Phase 2 design removes `BindingCache`, `Binding`, and related
//! projection-cache types. Until then this file keeps the existing
//! mount-wide per-name lookup cache and the `LayerLookup` intermediate used
//! by `OverlayFs::lookup_binding` available to the current directory
//! mutation recipes without changing their behavior.

use hashbrown::HashMap;

use super::OverlayInode;
use crate::{
    fs::{
        fs_impls::overlayfs::{layer::RealObjectStack, real::RealObjectKey},
        vfs::inode::Inode,
    },
    prelude::*,
};

type BindingsByName = HashMap<Box<str>, Arc<Binding>>;
type BindingsByParent = HashMap<RealObjectKey, BindingsByName>;

/// The intermediate layer-truth result used by the binding cache.
#[derive(Clone)]
pub(super) enum LayerLookup {
    Positive(RealObjectStack),
    Negative(NegativeBinding),
}

/// A cached lookup result: positive or negative.
#[derive(Clone)]
pub(in overlayfs) enum Binding {
    Positive(PositiveBinding),
    Negative(NegativeBinding),
}

/// A positive per-name binding: the shared overlay inode.
#[derive(Clone)]
pub(in overlayfs) struct PositiveBinding {
    pub(super) inode: Arc<OverlayInode>,
}

impl PositiveBinding {
    pub(in overlayfs) fn new(inode: Arc<OverlayInode>) -> Self {
        Self { inode }
    }

    pub(in overlayfs) fn inode(&self) -> Arc<OverlayInode> {
        self.inode.clone()
    }
}

/// A negative per-name binding.
///
/// Every variant surfaces as `ENOENT` to VFS while the reason stays private;
/// hidden bindings pin their barrier via [`HiddenEvidence`] for lifetime +
/// revalidation of the cached negative answer.
#[derive(Clone, Debug)]
pub(in overlayfs) enum NegativeBinding {
    /// The name is absent from every layer.
    Absent,
    /// The name is hidden by a whiteout barrier.
    HiddenByWhiteout(HiddenEvidence),
    /// The name is hidden by an opaque-directory barrier.
    HiddenByOpaque(HiddenEvidence),
}

impl NegativeBinding {
    /// Compares this negative binding against `other` for identity.
    pub(super) fn is_same_negative(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Absent, Self::Absent) => true,
            (Self::HiddenByWhiteout(left), Self::HiddenByWhiteout(right))
            | (Self::HiddenByOpaque(left), Self::HiddenByOpaque(right)) => {
                left.layer_index == right.layer_index
                    && Arc::ptr_eq(&left.real_inode, &right.real_inode)
            }
            _ => false,
        }
    }
}

/// The barrier evidence of a hidden name.
#[derive(Clone, Debug)]
pub(in overlayfs) struct HiddenEvidence {
    pub(super) layer_index: usize,
    /// The real object whose marker hides the name.
    pub(super) real_inode: Arc<dyn Inode>,
}

impl HiddenEvidence {
    pub(in overlayfs) fn new(layer_index: usize, real_inode: Arc<dyn Inode>) -> Self {
        Self {
            layer_index,
            real_inode,
        }
    }
}

/// The publication key of one per-name binding: the parent directory
/// identity plus the exact name in the parent.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(in overlayfs) struct BindingKey {
    pub(super) parent_id: RealObjectKey,
    pub(super) name: Box<str>,
}

impl BindingKey {
    pub(in overlayfs) fn new(parent_id: RealObjectKey, name: String) -> Self {
        Self {
            parent_id,
            name: name.into(),
        }
    }
}

/// The mount-wide binding cache.
///
/// Invariant: entries are immutable `Arc<Binding>` snapshots (replaced, never
/// mutated in place) — a cached positive pins its inode, a cached negative
/// pins its barrier (`HiddenEvidence`).
pub(in overlayfs) struct BindingCache {
    entries: RwMutex<BindingsByParent>,
}

impl BindingCache {
    pub(in overlayfs) fn new() -> Self {
        Self {
            entries: RwMutex::new(HashMap::new()),
        }
    }

    /// Returns the cached binding for `(parent_id, name)`, if any.
    pub(super) fn get(&self, parent_id: &RealObjectKey, name: &str) -> Option<Arc<Binding>> {
        self.entries.read().get(parent_id)?.get(name).cloned()
    }

    /// Inserts (or replaces) the cached binding for `(parent_id, name)`.
    pub(in overlayfs) fn insert(&self, key: BindingKey, binding: Arc<Binding>) {
        let BindingKey { parent_id, name } = key;
        self.entries
            .write()
            .entry(parent_id)
            .or_default()
            .insert(name, binding);
    }

    /// Removes the cached binding for `(parent_id, name)`. An emptied
    /// per-parent map is pruned.
    pub(in overlayfs) fn invalidate(&self, parent_id: &RealObjectKey, name: &str) {
        let mut guard = self.entries.write();
        if let Some(inner) = guard.get_mut(parent_id) {
            inner.remove(name);
            if inner.is_empty() {
                guard.remove(parent_id);
            }
        }
    }

    /// Removes the whole per-parent binding table for `parent_id`.
    pub(in overlayfs) fn invalidate_parent(&self, parent_id: &RealObjectKey) {
        self.entries.write().remove(parent_id);
    }
}

/// The result of one `(parent_id, name)` lookup.
pub(super) struct LookupOutcome {
    /// A verified cached binding, or the freshly rebuilt binding from the
    /// layer truth.
    pub(super) binding: Binding,
    /// Whether this lookup observed the stale-upper class.
    pub(super) is_stale_upper: bool,
}

impl Binding {
    /// Returns the shared inode for a positive binding; `None` for a
    /// negative binding.
    pub(in overlayfs) fn inode(&self) -> Option<Arc<OverlayInode>> {
        match self {
            Binding::Positive(positive) => Some(positive.inode.clone()),
            Binding::Negative(_) => None,
        }
    }

    /// Returns whether this cached binding still matches the layer truth.
    pub(super) fn matches_truth(&self, truth: &LayerLookup) -> bool {
        match (self, truth) {
            (Binding::Positive(positive), LayerLookup::Positive(facts)) => positive
                .inode
                .facts_snapshot()
                .same_real_object_stack(facts),
            (Binding::Negative(negative), LayerLookup::Negative(truth_negative)) => {
                negative.is_same_negative(truth_negative)
            }
            _ => false,
        }
    }

    /// Returns whether this cached binding is a "stale upper" for `truth`: a
    /// previously upper-backed positive binding whose physical upper object
    /// vanished behind the overlay with no whiteout left.
    pub(super) fn is_stale_upper(&self, truth: &LayerLookup) -> bool {
        let Binding::Positive(positive) = self else {
            return false;
        };
        if positive.inode.facts_snapshot().upper.is_none() {
            return false;
        }
        match truth {
            // A whiteout now covers the name: the upper was legitimately
            // removed through the overlay; the rebuild serves the negative
            // truth (not stale).
            LayerLookup::Negative(NegativeBinding::HiddenByWhiteout(_)) => false,
            // A fresh positive truth that still carries an upper entry is an
            // overlay-owned replacement at the name (not stale).
            LayerLookup::Positive(fresh) => fresh.upper.is_none(),
            // The name is absent from every layer, or hidden by an opaque
            // barrier with no upper entry at the name: the previously
            // published upper object has vanished behind the overlay with no
            // whiteout left (stale).
            LayerLookup::Negative(_) => true,
        }
    }
}
