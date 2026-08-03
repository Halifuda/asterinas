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
//! `mount::superblock` field types, and the Wave-4 leaf meso modules
//! `readdir_index.rs`/`copyup/`/`dir/`) can name them: [`OverlayInode`],
//! [`OverlayObjectFacts`], [`BindingCache`], [`InodeCache`], [`IdentityPolicy`],
//! and the wave-3 widened visibility chain — [`Binding`], [`BindingKey`],
//! [`PositiveBinding`], [`NegativeBinding`], [`HiddenEvidence`],
//! [`PositiveKind`], [`RealObject`], [`RealObjectKey`], and
//! [`OverlayObjectId`] (wave-2 review item 2 widened the cross-meso cache and
//! policy carriers; wave-3 review item 3 completed the leaf-consumer chain to
//! the same ceiling). Only the module-private intermediates (`LayerLookup`,
//! `LowerIdRecord`) and the `pub(super)` carrier fields stay reachable only
//! inside this module tree.
//!
//! Wave-3 shared-carrier seams (handoff §2.3 item 4; parent N/A, no feature
//! claims): `OverlayFs::project_new_upper` (meso-06 §4.1 consumption seam;
//! reuses the `project_inode` path) and the `record_copyup_transition`
//! invocation at the positive-binding assembly point (meso-04 §3.4 item 2 /
//! §4.1 hook; the method body lands in Wave 4 `copyup/mod.rs`), plus the
//! `readdir_index`/`copyup_transition` initializations in the `project_inode`
//! constructor literal (handoff §2.3 item 1; the sibling inode.rs pass landed
//! the fields on `OverlayInode`).

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
pub(in crate::fs::fs_impls::overlayfs) use entry::RealObject;
pub(in crate::fs::fs_impls::overlayfs) use identity::{IdentityPolicy, OverlayObjectId};
pub(in crate::fs::fs_impls::overlayfs) use inode::{OverlayInode, OverlayObjectFacts};
pub(in crate::fs::fs_impls::overlayfs) use inode_cache::{InodeCache, RealObjectKey};

use super::mount::OverlayFs;
use crate::{
    fs::{
        fs_impls::overlayfs::readdir_index::ReaddirIndex,
        vfs::{
            file_system::FileSystem,
            inode::{Extension, Inode},
        },
    },
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
        let parent_id = RealObjectKey::from_facts(parent_facts);
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
        // meso-04 §3.4 item 2 / §4.1 cross-meso hook: at the
        // positive-binding assembly point (the assembled binding is
        // published below via `publish_binding` before `DIR` release),
        // record the copy-up transition coordinate — `publication_parent`
        // + `name` — on the projected inode. The once-per-inode guard
        // (first positive binding wins; `try_lock`, skip when contended —
        // invariant I3) lives in the Wave-4 method body (`copyup/mod.rs`),
        // so re-publications after a binding invalidation are no-ops.
        if let Binding::Positive(positive) = &binding {
            // `publication_parent` resolves the live parent carrier by its
            // visible-source key (`P0-16`; the fallible copy-up transition
            // aliases the registration first — the old-key mapping is
            // retained until the dead-pin sweep reclaims it — so the probe
            // cannot miss for a live parent). A miss is an invariant
            // violation surfaced loudly (debug-assert + log) and degrades
            // recoverably: the coordinate recording is skipped rather than
            // fabricating a duplicate carrier (wave-3 round-2 repair item 1).
            if let Ok(parent) = self.publication_parent(parent_facts) {
                positive.inode.record_copyup_transition(parent, name);
            }
        }
        self.publish_binding(&parent_id, name, binding.clone());
        Ok(binding)
    }

    /// Creates or reuses the shared [`OverlayInode`] for `facts`.
    ///
    /// The positive branch only (spec §4): the visible-metadata source keys
    /// the inode-cache entry (`RealObjectKey::from_facts`, `P0-16`), and the
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
        let key = RealObjectKey::from_facts(facts);
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
                key: Mutex::new(key),
                facts: Mutex::new(facts),
                dir_transaction_lock: if is_directory {
                    Some(Mutex::new(()))
                } else {
                    None
                },
                // Wave-3 seam fields (handoff §2.3 item 1): the readdir
                // index is `Some` iff this object is a directory (meso-03
                // spec §4; the empty initial index — `ReaddirIndex::new()`
                // — lands in Wave 4 `readdir_index.rs`); the copy-up
                // transition coordinate starts `None` (meso-04 records the
                // first positive-binding publication in Wave 4).
                readdir_index: if is_directory {
                    Some(Mutex::new(ReaddirIndex::new()))
                } else {
                    None
                },
                copyup_transition: Mutex::new(None),
                object_id,
                extension: Extension::new(),
            })
        })
    }

    /// Creates or reuses the shared [`OverlayInode`] for a freshly created
    /// upper object (meso-06 §4.1 consumption seam).
    ///
    /// The `namespace_mutation_whiteout` recipes call this frozen seam
    /// inline for the new upper object before `BindingCache::insert`. It
    /// reuses the exact `project_inode` semantics — inode-cache
    /// `get_or_create` by `RealObjectKey`, facts/object_id initialization
    /// — so a second projection of the same upper real object reuses the
    /// same `P0-16` identity carrier.
    #[expect(
        dead_code,
        reason = "frozen meso-06 §4.1 consumption seam; consumed by the Wave-4 namespace-mutation Creator (dir/)"
    )]
    pub(in crate::fs::fs_impls::overlayfs) fn project_new_upper(
        &self,
        facts: &OverlayObjectFacts,
    ) -> Arc<OverlayInode> {
        self.project_inode(facts)
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

    /// Returns the published `Arc<OverlayInode>` of the parent directory
    /// whose facts are `parent_facts` — the lookup receiver (meso-04 §3.4
    /// item 2 hook support).
    ///
    /// Every live carrier — the root (registered by `OverlayInode::new_root`
    /// alongside the mount slot, wave-3 review item 2), every non-root
    /// carrier (`project_inode`), and every copied-up parent
    /// ([`OverlayInode::replace_facts`], which aliases the registration so
    /// the old and new visible-source keys both resolve to the one carrier,
    /// wave-3 round-3 repair item 1) — is registered in `InodeCache` under
    /// its *current* visible-source key.
    /// The read-only probe therefore returns the actual lookup parent — the
    /// one carrier registered for the visible-source key — with exactly one
    /// carrier per key on every path (`P0-16`); no key-equality root check
    /// exists (wave-3 review item 2) and no duplicate is ever projected.
    ///
    /// A probe miss violates the registration invariant (a live parent is
    /// never missing from the cache): the miss is surfaced with a
    /// `debug_assert!` and an error log and returns `Err` so the hook caller
    /// degrades recoverably (skipping the coordinate recording) instead of
    /// silently minting a second carrier (wave-3 round-2 review). The probe
    /// holds no upgradeable-reader slot, so it never re-enters the inode
    /// cache's single upgradeable slot.
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
                    "overlay parent identity inconsistency: no inode-cache carrier for \
                     visible-source key {:?}",
                    key
                );
                Err(Error::with_message(
                    Errno::EIO,
                    "the overlay parent carrier is not registered under its visible-source key",
                ))
            }
        }
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
/// enforced in exactly one place (wave-2 review item 9 dedupe). The key
/// derivation is the second half of that rule and is centralized in
/// `RealObjectKey::from_facts` (wave-3 round-3 repair, `dry`).
pub(super) fn visible_source(facts: &OverlayObjectFacts) -> &RealObject {
    match &facts.upper {
        Some(upper) => upper,
        None => &facts.lowers[0],
    }
}
