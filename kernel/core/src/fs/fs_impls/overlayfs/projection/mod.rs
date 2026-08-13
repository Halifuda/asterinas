// SPDX-License-Identifier: MPL-2.0

//! The overlayfs projection and identity subsystem.
//!
//! # Concepts
//!
//! A **projection** is the deterministic mapping from a name's real
//! (underlying) layer object — or, for a merged directory, its upper real
//! object together with its lower stack — to the overlay's visible identity
//! for it: the object kind, the projected dev/ino, and the reusable
//! [`OverlayInode`] — instead of a copy of the real object. A **binding** is the remembered result of one
//! `(parent, name)` lookup: `Positive` (a pinned [`OverlayInode`]) or
//! `Negative` (why the name is hidden or absent).
//!
//! This module owns the lookup path: it resolves a name upper-first across
//! the layer stack, projects the winning real object, and publishes the
//! binding in the mount-wide [`BindingCache`].
//!
//! # Structure
//!
//! | Submodule | Owns |
//! |---|---|
//! | `binding_cache` | The `Binding` type and the mount-wide binding cache. |
//! | `entry` | Real-object projection and the upper-first layer lookup core. |
//! | `identity` | Dev/ino identity projection. |
//! | `inode` | The overlay inode and its VFS trait surface. |
//! | `inode_cache` | Inode identity-reuse cache. |
//! | `lower_id` | The durable lower-source identity record. |
//!
//! # References
//!
//! - Overlayfs (Linux overlay filesystem):
//!   <https://www.kernel.org/doc/html/latest/filesystems/overlayfs.html>

mod binding_cache;
mod entry;
mod identity;
mod inode;
mod inode_cache;
mod lower_id;

pub(in crate::fs::fs_impls::overlayfs) use binding_cache::{
    Binding, BindingCache, BindingKey, HiddenEvidence, NegativeBinding, PositiveBinding,
    PositiveKind,
};
use entry::LayerLookup;
pub(in crate::fs::fs_impls::overlayfs) use entry::{RealObject, is_whiteout_inode};
pub(in crate::fs::fs_impls::overlayfs) use identity::{IdentityPolicy, LowerLayerIdentity};
pub(in crate::fs::fs_impls::overlayfs) use inode::{OverlayInode, OverlayObjectFacts};
pub(in crate::fs::fs_impls::overlayfs) use inode_cache::{InodeCache, RealObjectKey};

use super::mount::OverlayFs;
use crate::{
    fs::{fs_impls::overlayfs::readdir_index::ReaddirIndex, vfs::inode::Extension},
    prelude::*,
};

/// The result of one `(parent_id, name)` lookup.
///
/// `is_stale_upper` is true when the fresh layer truth no longer contains the
/// upper entry of a previously published upper-backed positive binding and no
/// whiteout covers the name. Most consumers ignore the signal; the remove
/// path consumes it to surface `ESTALE` instead of re-exposing the lower
/// counterpart.
pub(in crate::fs::fs_impls::overlayfs) struct LookupOutcome {
    /// A verified cached binding, or the freshly rebuilt binding from the
    /// layer truth.
    pub(in crate::fs::fs_impls::overlayfs) binding: Binding,
    /// Whether this lookup observed the stale-upper class.
    pub(in crate::fs::fs_impls::overlayfs) is_stale_upper: bool,
}

impl OverlayFs {
    pub(super) fn bindings(&self) -> &BindingCache {
        &self.bindings
    }

    pub(super) fn inodes(&self) -> &InodeCache {
        &self.inodes
    }

    pub(super) fn identity(&self) -> &IdentityPolicy {
        &self.identity
    }

    /// Resolves one `name` under `parent_facts` into a [`LookupOutcome`].
    ///
    /// The flow is verify-then-serve: the layer-ordered lookup re-observes
    /// the fresh layer truth, and a cached binding is served only when it
    /// matches that truth; otherwise the binding is rebuilt and published.
    pub(super) fn lookup_binding(
        &self,
        parent_facts: &OverlayObjectFacts,
        name: &str,
    ) -> Result<LookupOutcome> {
        let parent_id = RealObjectKey::from_facts(parent_facts);
        let truth = self.lookup_in_layers(parent_facts, name)?;
        let is_stale_upper = if let Some(binding) = self.bindings().get(&parent_id, name) {
            if binding.matches_truth(&truth) {
                return Ok(LookupOutcome {
                    binding: binding.as_ref().clone(),
                    is_stale_upper: false,
                });
            }
            binding.is_stale_upper(&truth)
        } else {
            false
        };
        let binding = match truth {
            LayerLookup::Positive(facts) => {
                let inode = self.project_inode(&facts);
                Binding::Positive(PositiveBinding::new(inode))
            }
            LayerLookup::Negative(negative) => Binding::Negative(negative),
        };
        // Record the copy-up transition coordinate — the `(parent, name)`
        // under which this inode first appeared on the upper. The
        // per-inode guard keeps the first positive binding's coordinate;
        // later lookups leave it unchanged.
        if let Binding::Positive(positive) = &binding {
            if let Ok(parent) = self.publication_parent(parent_facts) {
                positive.inode.record_copyup_transition(parent, name);
            }
        }
        self.publish_binding(&parent_id, name, binding.clone());
        Ok(LookupOutcome {
            binding,
            is_stale_upper,
        })
    }

    /// Creates or reuses the shared [`OverlayInode`] for `facts`.
    ///
    /// The `object_id` is precomputed from `IdentityPolicy` before the
    /// inode-cache check-and-create, because the upper-source lower-id read
    /// may block on the underlying xattr and must never run inside the
    /// cache's upgraded guard.
    fn project_inode(&self, facts: &OverlayObjectFacts) -> Arc<OverlayInode> {
        let source = visible_source(facts);
        let key = RealObjectKey::from_facts(facts);
        let is_directory =
            facts.kind == PositiveKind::Merged || source.real_inode().type_().is_directory();
        let fallback_fn = || self.identity().project_object_id(source, is_directory);
        let object_id = if source.layer_index() == 0 {
            match self.read_lower_id(source.real_inode()) {
                // Defensive: the record was device-validated at the read boundary,
                // so `None` here is the absent/ambiguous-device corner.
                Ok(Some(record)) => {
                    // The record is accepted only when its real inode is
                    // consistent with the retained same-layer lower of the
                    // fresh facts.
                    if self.origin_real_ino_resolves(&record, facts) {
                        self.identity()
                            .project_object_id_from_lower_id(&record, is_directory)
                            .unwrap_or_else(fallback_fn)
                    } else {
                        fallback_fn()
                    }
                }
                Ok(None) => fallback_fn(),
                Err(err) => {
                    warn!(
                        "failed to read the lower-id record of the upper source; \
                         falling back to the visible-source projection: {:?}",
                        err
                    );
                    fallback_fn()
                }
            }
        } else {
            fallback_fn()
        };
        // Clone the visible source before the closures move `facts`: the
        // get-or-create predicate validates a cached hit against this real
        // inode, replacing an ino-reuse stale occupant. The fresh-truth
        // upper presence is captured here as well, because the predicate
        // must distinguish a lower-only fresh truth (below) from an
        // upper-backed one.
        let source_inode = visible_source(facts).real_inode().clone();
        let fresh_is_lower_only = facts.upper().is_none();
        let fs = self.self_weak.clone();
        let facts = facts.clone();
        self.inodes().get_or_create(
            key,
            move |carrier| {
                if fresh_is_lower_only {
                    // Reuse only an inode whose visible source is exactly
                    // this lower; a stale-upper inode must not be reused even
                    // though `contains_real_inode` matches the retained
                    // lower, because its dead-upper metadata would be wrong.
                    Arc::ptr_eq(
                        visible_source(&carrier.facts_snapshot()).real_inode(),
                        &source_inode,
                    )
                } else {
                    carrier.facts_snapshot().contains_real_inode(&source_inode)
                }
            },
            move || {
                Arc::new(OverlayInode {
                    fs,
                    key: Mutex::new(key),
                    facts: Mutex::new(facts),
                    dir_transaction_lock: if is_directory {
                        Some(Mutex::new(()))
                    } else {
                        None
                    },
                    object_id,
                    extension: Extension::new(),
                    readdir_index: if is_directory {
                        Some(Mutex::new(ReaddirIndex::new()))
                    } else {
                        None
                    },
                    copyup_transition: Mutex::new(None),
                })
            },
        )
    }

    /// Creates or reuses the shared [`OverlayInode`] for a freshly created
    /// upper object.
    ///
    /// This distinct entry exists for callers that already hold the upper
    /// object's facts (create/mkdir/mknod/symlink paths): it supplies the
    /// just-built upper facts directly instead of re-running a layer
    /// lookup; the caller inserts the returned inode as a positive binding
    /// itself.
    pub(in crate::fs::fs_impls::overlayfs) fn project_new_upper(
        &self,
        facts: &OverlayObjectFacts,
    ) -> Arc<OverlayInode> {
        self.project_inode(facts)
    }

    /// Publishes `binding` for `(parent_id, name)` into the binding cache.
    pub(in crate::fs::fs_impls::overlayfs) fn publish_binding(
        &self,
        parent_id: &RealObjectKey,
        name: &str,
        binding: Binding,
    ) {
        let key = BindingKey {
            parent_id: *parent_id,
            name: name.into(),
        };
        self.bindings().insert(key, Arc::new(binding));
    }

    /// Returns the published `Arc<OverlayInode>` of the parent directory
    /// whose facts are `parent_facts` — the lookup receiver (copy-up hook
    /// support).
    ///
    /// Every live inode is registered in `InodeCache` under its current
    /// visible-source key, so the probe returns the actual lookup parent. A
    /// miss violates that invariant and returns `Err` so the hook caller
    /// degrades recoverably instead of minting a second inode.
    fn publication_parent(&self, parent_facts: &OverlayObjectFacts) -> Result<Arc<OverlayInode>> {
        let key = RealObjectKey::from_facts(parent_facts);
        match self.inodes().get(key) {
            Some(parent) => Ok(parent),
            None => {
                debug_assert!(
                    false,
                    "a live overlay parent is always registered under its current visible-source key"
                );
                error!(
                    "overlay parent identity inconsistency: no inode-cache entry for \
                     visible-source key {:?}",
                    key
                );
                Err(Error::with_message(
                    Errno::EIO,
                    "the overlay parent inode is not registered under its visible-source key",
                ))
            }
        }
    }
}

/// Returns the visible-metadata source of `facts`: the upper real object when
/// present, else the topmost lower (`lowers[0]`).
///
/// Precondition: `upper.is_some() || !lowers.is_empty()`.
pub(super) fn visible_source(facts: &OverlayObjectFacts) -> &RealObject {
    match &facts.upper {
        Some(upper) => upper,
        None => &facts.lowers[0],
    }
}
