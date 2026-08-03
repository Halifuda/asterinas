// SPDX-License-Identifier: MPL-2.0

//! The module root of the `visibility_projection_identity` meso (meso-02).
//!
//! This module declares the six `projection/*` submodules and hosts the
//! `OverlayFs`-extension impl blocks of the frozen meso-02 spec §4: the
//! `bindings()`/`inodes()`/`identity()` field accessors and the
//! `lookup_binding`/`project_inode`/`publish_binding` lookup orchestration
//! (caller-side projection, spec §3.3 lookup-flow note). The overlayfs-ceiling
//! carriers are re-exported at `pub(in crate::fs::fs_impls::overlayfs)` so the
//! frozen consumers in sibling module trees (`mount::build` step 10,
//! `mount::superblock` field types) can name them: [`OverlayInode`],
//! [`OverlayObjectFacts`], [`BindingCache`], [`InodeCache`], and
//! [`IdentityPolicy`] (wave-2 review item 2 widened the cross-meso carriers;
//! the projection-local carriers — `Binding`/`RealObject`/`RealObjectKey`/
//! `BindingKey`/`LayerLookup`/`OverlayObjectId`/`LowerIdRecord`/… — stay
//! reachable only inside this module tree).

mod binding_cache;
mod entry;
mod identity;
mod inode;
mod inode_cache;
mod lower_id;

pub(in crate::fs::fs_impls::overlayfs) use binding_cache::BindingCache;
use binding_cache::{Binding, BindingKey, PositiveBinding, PositiveKind};
use entry::{LayerLookup, RealObject};
pub(in crate::fs::fs_impls::overlayfs) use identity::IdentityPolicy;
pub(in crate::fs::fs_impls::overlayfs) use inode::{OverlayInode, OverlayObjectFacts};
pub(in crate::fs::fs_impls::overlayfs) use inode_cache::InodeCache;
use inode_cache::RealObjectKey;

use super::mount::OverlayFs;
use crate::{
    fs::vfs::inode::{Extension, Inode},
    prelude::*,
};

impl OverlayFs {
    /// Returns the mount-wide binding cache (spec §4 accessor).
    ///
    /// The cache is the first source for `(parent, name)` lookup results
    /// (`Binding-first` invariant); its internal `RwMutex` is an internal data
    /// lock used sequentially under the caller's parent `DIR` transaction.
    pub(super) fn bindings(&self) -> &BindingCache {
        &self.bindings
    }

    /// Returns the mount-wide inode identity-reuse cache (spec §4 accessor).
    pub(super) fn inodes(&self) -> &InodeCache {
        &self.inodes
    }

    /// Returns the immutable identity policy of this mount (spec §4 accessor).
    pub(super) fn identity(&self) -> &IdentityPolicy {
        &self.identity
    }

    /// Resolves one `name` under `parent_facts` into a published binding.
    ///
    /// Called from `OverlayInode::lookup` (inode.rs) under the parent `DIR`
    /// transaction lock (spec §3.3). The flow is binding-first: a cached
    /// `(parent_id, name)` snapshot is returned directly; on a miss the
    /// layer-ordered lookup (`lookup_in_layers`, entry.rs) produces the single
    /// private intermediate, the positive branch is projected into a shared
    /// [`OverlayInode`] (`project_inode`), and the assembled binding is
    /// published before `DIR` release (`publish_binding`). Negative variants
    /// stay private evidence and surface as `ENOENT` at the caller.
    pub(super) fn lookup_binding(
        &self,
        parent_facts: &OverlayObjectFacts,
        name: &str,
    ) -> Result<Binding> {
        let parent_id = RealObjectKey::from_source(visible_source(parent_facts));
        if let Some(binding) = self.bindings().get(&parent_id, name) {
            return Ok(binding.as_ref().clone());
        }
        let binding = match self.lookup_in_layers(parent_facts, name)? {
            LayerLookup::Positive(facts) => {
                let kind = facts.kind;
                let inode = self.project_inode(&facts);
                Binding::Positive(PositiveBinding { kind, inode })
            }
            LayerLookup::Negative(negative) => Binding::Negative(negative),
        };
        self.publish_binding(&parent_id, name, binding.clone());
        Ok(binding)
    }

    /// Creates or reuses the shared [`OverlayInode`] for `facts`.
    ///
    /// The positive branch only (spec §4): the visible-metadata source keys
    /// the inode-cache entry (`RealObjectKey::from_source`, `P0-16`), and the
    /// published `object_id` is precomputed from `IdentityPolicy` BEFORE the
    /// inode-cache check-and-create: the upper-source lower-id read
    /// (`read_lower_id`, `P1-07`) may block on the underlying xattr (Hazard 7)
    /// and must never run inside the cache's upgraded guard, whose create
    /// closure allocates only (Hazard 5). On `read_lower_id` error the
    /// visible-source projection is used (logged; never silently wrong).
    ///
    /// # Mount reference (wave-2 repair item 1)
    ///
    /// The canonical `Weak<OverlayFs>` stamped on every created inode comes
    /// from `OverlayFs::self_weak` — established by `Arc::new_cyclic` in
    /// `OverlayFs::new` (ramfs precedent) — NOT by downcasting the root
    /// carrier (the wave-2 review `coupling-cohesion` finding is removed).
    /// The weak upgrades exactly while the mount lives, so every created
    /// inode's `fs()` upgrade obeys the B/C-2 lifetime rule and the §3.5
    /// item-4 platform-lifetime note.
    fn project_inode(&self, facts: &OverlayObjectFacts) -> Arc<OverlayInode> {
        let source = visible_source(facts);
        let key = RealObjectKey::from_source(source);
        let is_directory =
            facts.kind == PositiveKind::Merged || source.real_inode().type_().is_directory();
        let object_id = if source.layer_index() == 0 {
            match self.read_lower_id(source.real_inode()) {
                Ok(Some(record)) => self
                    .identity()
                    .project_object_id_from_lower_id(&record, is_directory),
                Ok(None) => self.identity().project_object_id(source, is_directory),
                Err(err) => {
                    warn!(
                        "failed to read the lower-id record of the upper source; \
                         falling back to the visible-source projection: {:?}",
                        err
                    );
                    self.identity().project_object_id(source, is_directory)
                }
            }
        } else {
            self.identity().project_object_id(source, is_directory)
        };
        let fs = self.self_weak.clone();
        let facts = facts.clone();
        self.inodes().get_or_create(key, move || {
            Arc::new(OverlayInode {
                fs,
                key,
                facts: Mutex::new(facts),
                dir_transaction_lock: if is_directory {
                    Some(Mutex::new(()))
                } else {
                    None
                },
                object_id,
                extension: Extension::new(),
            })
        })
    }

    /// Publishes `binding` for `(parent_id, name)` into the binding cache.
    ///
    /// Called by `lookup_binding` under the parent `DIR` transaction, so the
    /// check-act-publish sequence stays atomic per directory (one-`DIR` rule);
    /// the entry is an immutable `Arc<Binding>` snapshot (replaced, never
    /// mutated in place, spec §4).
    fn publish_binding(&self, parent_id: &RealObjectKey, name: &str, binding: Binding) {
        let key = BindingKey {
            parent_id: *parent_id,
            name: name.into(),
        };
        self.bindings().insert(key, Arc::new(binding));
    }
}

/// Returns the visible-metadata source of `facts`: the upper real object when
/// present, else the topmost lower (`lowers[0]`).
///
/// Whitelist Rule B: the identical selection runs on the parent facts
/// (binding-key derivation in `lookup_binding`), the child facts (key,
/// directory and identity projection in `project_inode`), and every
/// `OverlayInode` metadata accessor in inode.rs (size/metadata/type_/mode/
/// owner/group/times/root), so the `lowers[0]` indexing invariant
/// (`upper.is_some() || !lowers.is_empty()`, spec §4) is documented and
/// enforced in exactly one place (wave-2 review item 9 dedupe).
pub(super) fn visible_source(facts: &OverlayObjectFacts) -> &RealObject {
    match &facts.upper {
        Some(upper) => upper,
        None => &facts.lowers[0],
    }
}
