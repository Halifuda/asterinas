// SPDX-License-Identifier: MPL-2.0

//! The Overlay inode carrier and its canonical VFS trait surface
//! (`P0-04`/`P0-06`/`P0-12`/`P0-17`).
//!
//! This module owns the [`OverlayInode`] struct (the published logical inode
//! carrier of the `visibility_projection_identity` meso), its `INODE`-domain
//! payload [`OverlayObjectFacts`], the frozen root-carrier seam
//! ([`OverlayInode::new_root`], consumed by `OverlayFs::new` step 10), and the
//! sole `Inode` and `FileOps` implementations. Those canonical trait methods
//! directly forward each Meso's behavior to its current helper owner:
//! projection lookup/metadata/identity/revalidation helpers stay here;
//! copy-up, readdir, metadata-security, and directory-mutation behavior stays
//! in their existing Meso helpers, including `dir/`'s namespace mutations.
//!
//! Lock contract (spec §3.0/§3.1/§3.3): `dir_transaction_lock` is the
//! payload-less per-directory `DIR` transaction lock (`Some` for directories
//! only); `facts` is the per-object `INODE` domain, accessed only through
//! brief snapshot locks (`facts_snapshot`: clone and release) and never held
//! across an underlying call. The only nested order is `DIR -> INODE` plus
//! the mount-wide cache locks used sequentially under `DIR` inside
//! `OverlayFs::lookup_binding`.
//!
//! Visibility: items that cross the `projection`/`mount` boundary (the
//! published carriers `OverlayInode`/`OverlayObjectFacts`, the frozen
//! `new_root` seam, and the published `key`/`object_id` accessors) are
//! declared `pub(in crate::fs::fs_impls::overlayfs)` — the spec's "overlayfs
//! ceiling" — because the frozen consumers live in sibling module trees
//! (`mount::build` step 10, and later sibling meso modules); items consumed
//! only within `projection` stay at `pub(super)`. The frozen §4 listings'
//! unqualified `pub(super)` is read through the spec's own visibility audit
//! ("`pub(super)` for all cross-module items within `overlayfs`"), and the
//! dispatch packet explicitly allows either `pub(super)` or
//! `pub(in crate::fs::fs_impls::overlayfs)` as the ceiling.
//!
//! Wave-3 shared-carrier seams (handoff §2.3 item 1) add the two carrier
//! fields (`readdir_index`, `copyup_transition`) at the same ceiling, because
//! the sibling Wave-4 leaf modules (`readdir_index.rs`, `copyup/`, `dir/`)
//! host the impl blocks that consume them. The `OverlayObjectFacts` content
//! fields stay at `pub(super)` and are surfaced at the ceiling through the
//! read-only accessors (`kind`/`upper`/`lowers`) plus the checked constructor
//! and the `replace_facts` copy-up transition seam (wave-3 review item 4), so
//! sibling mesos enumerate layers without being able to mint invalid facts.

use core::time::Duration;

use super::{
    binding_cache::PositiveKind, entry::RealObject, identity::OverlayObjectId,
    inode_cache::RealObjectKey, visible_source,
};
use crate::{
    fs::{
        file::{AccessMode, InodeMode, InodeType, PerOpenFileOps, Permission, StatusFlags},
        fs_impls::overlayfs::{
            AccessType, copyup::coordination::CopyUpTransition, mount::OverlayFs,
            readdir_index::ReaddirIndex,
        },
        vfs::{
            file_system::FileSystem,
            inode::{
                Extension, FallocMode, FileOps, Inode, Metadata, MknodType, RenameMode,
                RevalidationPolicy, SymbolicLink,
            },
            xattr::{XattrName, XattrNamespace, XattrSetFlags},
        },
        utils::DirentVisitor,
    },
    prelude::*,
    process::{Gid, Uid},
    vm::page_cache::PageCache,
};

/// The logical Overlay inode carrier exposed to the VFS (`P0-06`).
///
/// One [`OverlayInode`] represents one logical overlay object and is shared by
/// every name bound to it: hard links reuse a single carrier through the
/// inode cache (`P0-16`), and a `PositiveBinding` references the shared
/// inode with zero per-name fact duplication (revision-04 model). The
/// real-object facts live exactly once in [`OverlayObjectFacts`], and the
/// precomputed `object_id` publishes the projected `st_dev`/`st_ino`
/// (`P2-01`/`P0-12`; for copied-up objects it already carries the lower-id
/// derived identity, so no re-derivation happens at stat time).
///
/// Invariants: `fs` is a `Weak<OverlayFs>` so no `fs -> inode -> fs` strong
/// cycle exists (B/C-2 lifetime rule, ramfs precedent); `dir_transaction_lock`
/// is `Some` iff the object is a directory; `facts` is fixed at creation and
/// only transitions via meso-04 copy-up (replaced, never mutated in place).
///
/// Wave-3 shared-carrier seams (handoff §2.3 item 1): `readdir_index` is
/// `Some` iff the object is a directory (empty initial index, meso-03), and
/// `copyup_transition` starts `None` (meso-04 records the first transition at
/// the positive-binding publication point).
///
/// # Dev note (recorded deviation)
///
/// `#[derive(Debug)]` is dropped: the frozen `fs: Weak<OverlayFs>` field
/// cannot satisfy a derived `Debug` bound (`OverlayFs` carries no `Debug`
/// impl), and the spec §4 shape hint explicitly allows dropping an
/// unsatisfiable derive.
///
/// The type is published at the overlayfs ceiling (`pub(in
/// crate::fs::fs_impls::overlayfs)`) because the frozen consumers sit outside
/// the `projection` tree (`mount::build` step 10, sibling meso modules).
/// The carrier fields stay at `pub(super)` — constructible by the sibling
/// `projection::mod`/`entry` creators only — with two exceptions: the Wave-3
/// seam fields `readdir_index` and `copyup_transition` live at the ceiling
/// (the sibling Wave-4 leaf modules own their payload types), and the
/// published accessors (`key`, `object_id`, `facts_snapshot`, `dir`) are
/// ceiling methods.
pub(in crate::fs::fs_impls::overlayfs) struct OverlayInode {
    /// The owning mount; a weak reference so a live inode never pins the
    /// mount (B/C-2 lifetime rule, ramfs `Arc::new_cyclic` + `Weak<RamFs>`
    /// precedent).
    pub(super) fs: Weak<OverlayFs>,
    /// The inode-cache key of the visible-metadata source (`P0-16`).
    ///
    /// Interior-mutable so the meso-04 copy-up transition
    /// ([`OverlayInode::replace_facts`]) can commit the carrier's new
    /// visible-source key atomically with its inode-cache alias: the
    /// fallible `alias_key` runs first (the old-key mapping is retained until
    /// the dead-pin sweep reclaims it, wave-3 round-3/5 repair), then this
    /// field is updated — the published `key()` never disagrees with
    /// `facts_snapshot()` (wave-3 round-2 repair item 1). The field is
    /// touched only under the object's `DIR` transaction.
    pub(super) key: Mutex<RealObjectKey>,
    /// The `INODE`-domain payload (level 4): the real-object facts, fixed at
    /// creation (brief snapshot locks only, spec §3.1).
    pub(super) facts: Mutex<OverlayObjectFacts>,
    /// The payload-less `DIR` transaction lock (level 2); `Some` iff this
    /// object is a directory (spec §3.0, §4 lock-carrier table).
    pub(super) dir_transaction_lock: Option<Mutex<()>>,
    /// The precomputed projected `st_dev`/`st_ino` (`P2-01`/`P0-12`).
    pub(super) object_id: OverlayObjectId,
    /// The VFS inode extension groups (fs event publisher / fs lock context).
    pub(super) extension: Extension,
    /// The per-directory merged-readdir index (`INODE`-domain, level 4);
    /// `Some` iff this object is a directory (Wave-3 seam, meso-03 spec §4).
    ///
    /// The payload type lands in Wave 4 `readdir_index.rs` (frozen name
    /// `ReaddirIndex`); the initial value is the empty index
    /// (`ReaddirIndex::new()` — `NeedsRebuild`, no entries), maintained by
    /// the meso-03 `readdir_at`/`invalidate_readdir_index` seams and the
    /// meso-06 decision seams in Wave 4.
    pub(in crate::fs::fs_impls::overlayfs) readdir_index: Option<Mutex<ReaddirIndex>>,
    /// The `CUL`-domain (level 3) copy-up transition coordinate; `None` until
    /// meso-04 records the first positive-binding publication (Wave-3 seam,
    /// meso-04 spec §4.1).
    ///
    /// The payload type lands in Wave 4 `copyup/coordination.rs` (frozen name
    /// `CopyUpTransition`); the `record_copyup_transition` hook and the
    /// `ensure_upper_authority` winner/waiter flow read it then.
    pub(in crate::fs::fs_impls::overlayfs) copyup_transition: Mutex<Option<CopyUpTransition>>,
}

/// The immutable real-object facts of one logical overlay object (`P0-06`).
///
/// Carries the BC-2 `OverlayObjectState` role, content-named per the meso-01
/// naming rule. The visible-metadata source is `upper` when present, else the
/// topmost lower (`lowers[0]`); merged directories report upper metadata only
/// (`P0-12` §2 Case 4).
///
/// Invariants: `upper.is_some() || !lowers.is_empty()`; the facts are fixed
/// at creation and only transition via meso-04 copy-up (replaced, never
/// mutated in place — BC-3 §33).
///
/// Wave-3 seam (handoff §2.3 item 1; wave-3 review item 4): the content
/// fields stay at `pub(super)` so the `upper.is_some() || !lowers.is_empty()`
/// invariant is enforced at the only construction paths — the in-tree
/// `projection` builders and the checked [`OverlayObjectFacts::try_new`]
/// constructor, the sole construction path outside `projection`. The sibling
/// meso-03 merged scan (`readdir_index.rs`) branches on `kind()` and
/// enumerates the layer `RealObject`s through the ceiling read-only accessors
/// (`kind`/`upper`/`lowers`).
#[derive(Clone, Debug)]
pub(in crate::fs::fs_impls::overlayfs) struct OverlayObjectFacts {
    /// `Single` = one real object; `Merged` = a directory merging upper and
    /// lower observations.
    pub(super) kind: PositiveKind,
    /// The upper real object; the visible-metadata source for merged
    /// directories.
    pub(super) upper: Option<RealObject>,
    /// The lower stack, topmost first; non-empty for lower-only/merged
    /// objects.
    pub(super) lowers: Vec<RealObject>,
}

impl OverlayObjectFacts {
    /// Returns the per-name view classification of this object's facts
    /// (ceiling read-only accessor; wave-3 review item 4).
    pub(in crate::fs::fs_impls::overlayfs) fn kind(&self) -> PositiveKind {
        self.kind
    }

    /// Returns the upper real object, if this object has an upper component
    /// (ceiling read-only accessor; wave-3 review item 4).
    pub(in crate::fs::fs_impls::overlayfs) fn upper(&self) -> Option<&RealObject> {
        self.upper.as_ref()
    }

    /// Returns the lower stack, topmost first (ceiling read-only accessor;
    /// wave-3 review item 4).
    pub(in crate::fs::fs_impls::overlayfs) fn lowers(&self) -> &[RealObject] {
        &self.lowers
    }

    /// Constructs an [`OverlayObjectFacts`], returning `None` when both
    /// `upper` and `lowers` are empty.
    ///
    /// The checked fallible constructor — the only construction path for
    /// [`OverlayObjectFacts`] outside the `projection` tree (wave-3 review
    /// item 4; renamed per the `mount/claims.rs::try_new` precedent, wave-3
    /// round-2 review). Enforces the frozen invariant
    /// `upper.is_some() || !lowers.is_empty()` (spec §4) at construction, so
    /// a sibling meso can never mint facts whose `visible_source` indexing
    /// (`lowers[0]`) would panic on the lookup/metadata hot path.
    pub(in crate::fs::fs_impls::overlayfs) fn try_new(
        kind: PositiveKind,
        upper: Option<RealObject>,
        lowers: Vec<RealObject>,
    ) -> Option<Self> {
        if upper.is_some() || !lowers.is_empty() {
            Some(Self { kind, upper, lowers })
        } else {
            None
        }
    }
}

impl OverlayInode {
    /// Constructs the root overlay carrier (the frozen meso-01 step-10 seam,
    /// `P0-04`; wave-2 review item 1 reconciliation).
    ///
    /// Infallible and eager: every fallible preparation completed inside
    /// `OverlayFs::new`, so the seam performs no fallible VFS operation and
    /// takes no Overlay lock (spec §3.2 inlet / §3.3 root construction). The
    /// seam now accepts the canonical `Weak<OverlayFs>` (recorded deviation
    /// from the provisional `new_root(fs: Arc<OverlayFs>)` signature) and is
    /// called by `OverlayFs::new` AFTER the `Arc` is published (the
    /// `Arc::new_cyclic` closure cannot upgrade its `&Weak` — the strong
    /// count stays 0 until the closure returns), so the upgrade below is
    /// guaranteed. The root facts merge the upper root (writable mounts)
    /// with all lower roots (topmost first, `P0-02` non-empty order); the
    /// root is always a directory, so `dir_transaction_lock` is `Some`;
    /// `object_id` is projected by `fs.identity()` from the visible-metadata
    /// source.
    pub(in crate::fs::fs_impls::overlayfs) fn new_root(fs: Weak<OverlayFs>) -> Arc<dyn Inode> {
        // The reconciliation (spec §3.0.5 item 8, wave-2 review item 1):
        // `OverlayFs::new` publishes the `Arc` first (via `Arc::new_cyclic`)
        // and calls this seam immediately afterwards, so the upgrade always
        // succeeds; the failure arm is genuinely unreachable and panics
        // rather than fabricating a mount-less root carrier (never silently
        // wrong, no `.unwrap()`/`.expect()`).
        let fs = match fs.upgrade() {
            Some(fs) => fs,
            None => unreachable!(
                "OverlayFs::new materializes the root carrier right after publishing \
                 the Arc; the mount reference is always alive at this call site"
            ),
        };
        let layer_stack = fs.layer_stack();
        let upper = layer_stack.upper.as_ref().map(|layer| RealObject {
            layer_index: 0,
            real_inode: layer.root_inode.clone(),
            fsid: layer.fsid,
            container_dev_id: layer.container_dev_id,
        });
        let lowers: Vec<_> = layer_stack
            .lowers
            .iter()
            .enumerate()
            .map(|(layer_index, layer)| RealObject {
                layer_index: layer_index + 1,
                real_inode: layer.root_inode.clone(),
                fsid: layer.fsid,
                container_dev_id: layer.container_dev_id,
            })
            .collect();
        // Merged-root classification (§2 Case 1): a writable root merges the
        // upper with the lowers; a read-only root merges its lower stack when
        // more than one lower directory participates (the Linux
        // `ovl_multilayer` analog — the spec freezes the upper+lower case and
        // the multi-lower case is its direct extension of the frozen
        // `PositiveKind::Merged` definition "directory merging upper and
        // lower observations").
        let kind = if upper.is_some() || lowers.len() > 1 {
            PositiveKind::Merged
        } else {
            PositiveKind::Single
        };
        let facts = OverlayObjectFacts {
            kind,
            upper,
            lowers,
        };
        // `P0-02` invariant: the layer stack always carries at least one
        // lower layer, so `visible_source` never indexes an empty `lowers`.
        let visible = visible_source(&facts);
        let key = RealObjectKey::from_facts(&facts);
        let object_id = fs.identity().project_object_id(visible, true);
        let inode = Arc::new(OverlayInode {
            fs: Arc::downgrade(&fs),
            key: Mutex::new(key),
            facts: Mutex::new(facts),
            dir_transaction_lock: Some(Mutex::new(())),
            object_id,
            extension: Extension::new(),
            // Wave-3 seam fields (handoff §2.3 item 1): the root is always a
            // directory, so the readdir index is `Some` (empty initial index,
            // `ReaddirIndex::new()` lands in Wave 4); the copy-up transition
            // starts `None` (meso-04 records the first transition in Wave 4).
            readdir_index: Some(Mutex::new(ReaddirIndex::new())),
            copyup_transition: Mutex::new(None),
        });
        // Wave-3 repair (review item 2): register the root carrier in the
        // inode cache alongside the mount slot, so every live carrier — root
        // included — resolves by its visible-source key (`P0-16`).
        // `publication_parent` then needs no key-equality root special case
        // (a non-root directory aliasing the root's real object resolves to
        // the same carrier on every path), and `project_inode` can never mint
        // a duplicate root carrier. The registration is a brief internal-data
        // cache lock at single-threaded mount construction (no Overlay lock).
        fs.inodes().get_or_create(key, || inode.clone());
        inode
    }

    /// Returns the inode-cache key of the visible-metadata source (`P0-16`).
    ///
    /// Published for sibling Mesos (merged-directory consumption of directory
    /// inputs/IDs, spec §1 item 3).
    pub(in crate::fs::fs_impls::overlayfs) fn key(&self) -> RealObjectKey {
        *self.key.lock()
    }

    /// Returns the precomputed projected `st_dev`/`st_ino` (`P2-01`/`P0-12`).
    ///
    /// Published for sibling Mesos; copy-up (meso-04) re-projection keeps the
    /// lower-id-derived identity, so the value is stable across copy-up
    /// (authority-continuity invariant). Consumed by `readdir_index.rs`
    /// `parent_fallback` (the frozen `d_ino("..") == d_ino(".")` route).
    pub(in crate::fs::fs_impls::overlayfs) fn object_id(&self) -> OverlayObjectId {
        self.object_id
    }

    /// Returns a clone of the fixed real-object facts under a brief `INODE`
    /// lock.
    ///
    /// Spec §3.1: the snapshot read runs in `DIR -> INODE` order, clones the
    /// facts, and releases the guard before any lock-free use, so the `INODE`
    /// domain is never held across an underlying call.
    ///
    /// Wave-3 seam (handoff §2.3 item 1): widened from private to the
    /// overlayfs ceiling so the sibling meso-03/04/06 modules can take brief
    /// snapshots under their own `DIR` transactions.
    pub(in crate::fs::fs_impls::overlayfs) fn facts_snapshot(&self) -> OverlayObjectFacts {
        self.facts.lock().clone()
    }

    /// Returns the payload-less `DIR` transaction lock, if this object is a
    /// directory.
    ///
    /// The lock carries no payload: its purpose is one-`DIR` transaction
    /// serialization, not data protection (spec §3.0).
    ///
    /// Wave-3 seam (handoff §2.3 item 1): widened from private to the
    /// overlayfs ceiling so the sibling meso-03/06 modules can serialize
    /// namespace transactions under the affected parent's `DIR`.
    pub(in crate::fs::fs_impls::overlayfs) fn dir(&self) -> Option<&Mutex<()>> {
        self.dir_transaction_lock.as_ref()
    }

    /// Replaces the real-object facts of this inode — the meso-04 copy-up
    /// transition seam (wave-3 review item 4; BC-3 §33: replaced, never
    /// mutated in place).
    ///
    /// The transition is self-consistent and fallible (wave-3 round-2/3/4
    /// repair): the inode-cache registration is aliased under the new
    /// visible-source key while the old-key mapping is retained
    /// (`InodeCache::alias_key`, both keys → the one carrier, `P0-16`), then
    /// the `INODE`-domain payload is swapped and the published
    /// [`OverlayInode::key`] is re-derived from the new visible source. The
    /// fallible alias runs FIRST and its `Err` propagates with `facts`/`key`
    /// untouched, so a `P0-16` displacement (a different live carrier already
    /// registered at the new key) fails the transition and the Wave-4 copy-up
    /// caller can fail or retry instead of proceeding with a split (round-4
    /// repair item 1). A live parent resolves to exactly one carrier on every
    /// path and `OverlayFs::publication_parent`'s probe cannot miss for a
    /// live parent. Validity (`upper.is_some() || !lowers.is_empty()`) is
    /// guaranteed because the only construction path outside `projection` is
    /// the checked [`OverlayObjectFacts::try_new`], so the replacement cannot
    /// mint invalid facts. Wave-4 copy-up calls this under the object's `DIR`
    /// transaction lock and must serialize the transition against concurrent
    /// projections of the same real object (hold the object's and the parents'
    /// `DIR`s across `replace_facts`): the old-key alias closes the
    /// stale-facts race, `alias_key` surfaces — never silently orphans — a
    /// live carrier already projected at the new key (wave-3 round-3 review;
    /// fallible since round 4), and the retained old-key alias carries a
    /// strong keep-alive pin of the pre-transition real inode so it cannot be
    /// recycled while the alias exists (wave-3 round-5 repair item 1). The
    /// post-condition is verified with `Arc::ptr_eq` against this carrier.
    pub(in crate::fs::fs_impls::overlayfs) fn replace_facts(
        self: &Arc<Self>,
        facts: OverlayObjectFacts,
    ) -> Result<()> {
        let new_key = RealObjectKey::from_facts(&facts);
        // Capture the pre-transition visible-source key AND its real inode
        // under one brief `INODE` lock: the old real inode becomes the
        // keep-alive pin of the retained old-key alias (`alias_key`, wave-3
        // round-5 repair item 1), so it cannot be recycled while the alias
        // exists.
        let (old_key, old_real_inode) = {
            let old_facts = self.facts.lock();
            (
                RealObjectKey::from_facts(&old_facts),
                visible_source(&old_facts).real_inode().clone(),
            )
        };
        // A live carrier cannot outlive its mount (meso-02 §3.5 item 4); the
        // teardown arm swaps the payload locally and skips the cache alias
        // (no live lookup can observe this carrier then).
        let Some(fs) = self.fs.upgrade() else {
            *self.facts.lock() = facts;
            *self.key.lock() = new_key;
            return Ok(());
        };
        // The fallible alias runs first: both key views (old and new) still
        // resolve to this one carrier, so a displacement at `new_key` fails
        // the transition with `facts`/`key` untouched (`P0-16`) and the
        // Wave-4 caller can fail or retry. Only then is the carrier's own
        // state committed (the old-key alias stays for stale-facts in-flight
        // projections and is retired by the dead-pin sweep).
        fs.inodes().alias_key(old_key, new_key, old_real_inode)?;
        *self.facts.lock() = facts;
        *self.key.lock() = new_key;
        debug_assert!(
            fs.inodes()
                .get(new_key)
                .is_some_and(|probe| Arc::ptr_eq(&probe, self)),
            "after replace_facts the inode cache maps the new visible-source key to THIS carrier"
        );
        Ok(())
    }
}

impl OverlayInode {
    fn size_impl(&self) -> usize {
        let facts = self.facts_snapshot();
        visible_source(&facts).real_inode().size()
    }

    fn metadata_impl(&self) -> Metadata {
        let facts = self.facts_snapshot();
        let mut metadata = visible_source(&facts).real_inode().metadata();
        // The precomputed `object_id` replaces dev/ino (`P2-01`/`P0-12`):
        // copied-up objects already carry the lower-id-derived identity, so no
        // re-derivation happens here (`P1-07`). Merged directories report
        // upper metadata only (the visible source is the upper).
        metadata.ino = self.object_id.ino;
        metadata.container_dev_id = self.object_id.dev;
        metadata
    }

    fn ino_impl(&self) -> u64 {
        self.object_id.ino
    }

    fn type_impl(&self) -> InodeType {
        let facts = self.facts_snapshot();
        visible_source(&facts).real_inode().type_()
    }

    fn mode_impl(&self) -> Result<InodeMode> {
        let facts = self.facts_snapshot();
        visible_source(&facts).real_inode().mode()
    }

    fn owner_impl(&self) -> Result<Uid> {
        let facts = self.facts_snapshot();
        visible_source(&facts).real_inode().owner()
    }

    fn group_impl(&self) -> Result<Gid> {
        let facts = self.facts_snapshot();
        visible_source(&facts).real_inode().group()
    }

    fn atime_impl(&self) -> Duration {
        let facts = self.facts_snapshot();
        visible_source(&facts).real_inode().atime()
    }

    fn mtime_impl(&self) -> Duration {
        let facts = self.facts_snapshot();
        visible_source(&facts).real_inode().mtime()
    }

    fn ctime_impl(&self) -> Duration {
        let facts = self.facts_snapshot();
        visible_source(&facts).real_inode().ctime()
    }

    fn lookup_impl(&self, name: &str) -> Result<Arc<dyn Inode>> {
        if !self.type_().is_directory() {
            return_errno_with_message!(
                Errno::ENOTDIR,
                "lookup is supported on overlay directories only"
            );
        }
        // Acquire the payload-less parent `DIR` transaction lock; the whole
        // lookup (underlying observations, cache publication, visible-result
        // publication) runs inside this one guard (BC-2 one-`DIR` rule,
        // spec §3.3).
        let dir = self.dir().ok_or_else(|| {
            Error::with_message(Errno::ENOTDIR, "the overlay inode is not a directory")
        })?;
        let _dir_guard = dir.lock();
        // Brief `INODE` snapshot (`DIR -> INODE` order), then lock-free use.
        let facts = self.facts_snapshot();
        let fs = self.fs.upgrade().ok_or_else(|| {
            Error::with_message(Errno::EIO, "the overlay mount is no longer alive")
        })?;
        let binding = fs.lookup_binding(&facts, name)?;
        match binding.into_inode() {
            Some(inode) => Ok(inode),
            // Every negative variant (`Absent`/`HiddenByWhiteout`/
            // `HiddenByOpaque`) surfaces as `ENOENT` to the VFS (BC-2
            // §18.2/§22).
            None => Err(Error::new(Errno::ENOENT)),
        }
    }

    fn fs_impl(&self) -> Arc<dyn FileSystem> {
        match self.fs.upgrade() {
            Some(fs) => fs,
            // A live `OverlayInode` pins its real objects and is itself
            // reachable only while the mount lives, so the upgrade succeeds
            // by contract; the post-teardown fallback is the recorded open
            // platform-lifetime note (spec §3.5 item 4) and is not invented
            // here — no `.unwrap()`/`.expect()` is introduced (exfat `fs()`
            // precedent).
            None => unreachable!(
                "a live OverlayInode keeps its OverlayFs alive (meso-02 spec §3.5 item 4)"
            ),
        }
    }

    fn revalidation_policy_impl(&self) -> RevalidationPolicy {
        match self.type_() {
            InodeType::Dir => RevalidationPolicy::REVALIDATE_ABSENT,
            _ => RevalidationPolicy::empty(),
        }
    }

    fn revalidate_absent_impl(&self, _name: &str) -> bool {
        // Cheap and conservative (`P0-17`): a negative dentry hit is always
        // re-looked-up. No locks and no I/O (spec §3.3 Hazard 4).
        false
    }

    fn extension_impl(&self) -> &Extension {
        &self.extension
    }
}

impl FileOps for OverlayInode {
    fn read_at(
        &self,
        offset: usize,
        writer: &mut VmWriter,
        status_flags: StatusFlags,
    ) -> Result<usize> {
        self.read_at_impl(offset, writer, status_flags)
    }

    fn write_at(
        &self,
        offset: usize,
        reader: &mut VmReader,
        status_flags: StatusFlags,
    ) -> Result<usize> {
        self.write_at_impl(offset, reader, status_flags)
    }

    fn readdir_at(&self, offset: usize, visitor: &mut dyn DirentVisitor) -> Result<usize> {
        self.readdir_at_impl(offset, visitor)
    }
}

impl Inode for OverlayInode {
    fn size(&self) -> usize {
        self.size_impl()
    }

    fn metadata(&self) -> Metadata {
        self.metadata_impl()
    }

    fn ino(&self) -> u64 {
        self.ino_impl()
    }

    fn type_(&self) -> InodeType {
        self.type_impl()
    }

    fn mode(&self) -> Result<InodeMode> {
        self.mode_impl()
    }

    fn owner(&self) -> Result<Uid> {
        self.owner_impl()
    }

    fn group(&self) -> Result<Gid> {
        self.group_impl()
    }

    fn atime(&self) -> Duration {
        self.atime_impl()
    }

    fn mtime(&self) -> Duration {
        self.mtime_impl()
    }

    fn ctime(&self) -> Duration {
        self.ctime_impl()
    }

    fn lookup(&self, name: &str) -> Result<Arc<dyn Inode>> {
        self.lookup_impl(name)
    }

    fn fs(&self) -> Arc<dyn FileSystem> {
        self.fs_impl()
    }

    fn revalidation_policy(&self) -> RevalidationPolicy {
        self.revalidation_policy_impl()
    }

    fn revalidate_absent(&self, name: &str) -> bool {
        self.revalidate_absent_impl(name)
    }

    fn extension(&self) -> &Extension {
        self.extension_impl()
    }

    fn open(
        &self,
        access_mode: AccessMode,
        status_flags: StatusFlags,
    ) -> Option<Result<Box<dyn PerOpenFileOps>>> {
        self.open_impl(access_mode, status_flags)
    }

    fn seek_end(&self) -> Option<usize> {
        self.seek_end_impl()
    }

    fn resize(&self, new_size: usize) -> Result<()> {
        self.resize_impl(new_size)
    }

    fn fallocate(&self, mode: FallocMode, offset: usize, len: usize) -> Result<()> {
        self.fallocate_impl(mode, offset, len)
    }

    fn sync_all(&self) -> Result<()> {
        self.sync_all_impl()
    }

    fn sync_data(&self) -> Result<()> {
        self.sync_data_impl()
    }

    fn read_link(&self) -> Result<SymbolicLink> {
        self.read_link_impl()
    }

    fn page_cache(&self) -> Option<PageCache> {
        self.page_cache_impl()
    }

    fn set_mode(&self, mode: InodeMode) -> Result<()> {
        self.set_mode_impl(mode)
    }

    fn set_owner(&self, uid: Uid) -> Result<()> {
        self.set_owner_impl(uid)
    }

    fn set_group(&self, gid: Gid) -> Result<()> {
        self.set_group_impl(gid)
    }

    fn set_atime(&self, time: Duration) {
        self.set_atime_impl(time)
    }

    fn set_mtime(&self, time: Duration) {
        self.set_mtime_impl(time)
    }

    fn set_ctime(&self, time: Duration) {
        self.set_ctime_impl(time)
    }

    fn check_permission(&self, perm: Permission) -> Result<()> {
        self.check_permission(AccessType::ReadOnly, perm)
    }

    fn get_xattr(&self, name: XattrName, value_writer: &mut VmWriter) -> Result<usize> {
        self.get_xattr_impl(name, value_writer)
    }

    fn set_xattr(
        &self,
        name: XattrName,
        value_reader: &mut VmReader,
        flags: XattrSetFlags,
    ) -> Result<()> {
        self.set_xattr_impl(name, value_reader, flags)
    }

    fn list_xattr(&self, namespace: XattrNamespace, list_writer: &mut VmWriter) -> Result<usize> {
        self.list_xattr_impl(namespace, list_writer)
    }

    fn remove_xattr(&self, name: XattrName) -> Result<()> {
        self.remove_xattr_impl(name)
    }

    fn create(&self, name: &str, type_: InodeType, mode: InodeMode) -> Result<Arc<dyn Inode>> {
        self.create_impl(name, type_, mode)
    }

    fn mknod(&self, name: &str, mode: InodeMode, type_: MknodType) -> Result<Arc<dyn Inode>> {
        self.mknod_impl(name, mode, type_)
    }

    fn write_link(&self, target: &str) -> Result<()> {
        self.write_link_impl(target)
    }

    fn link(&self, old: &Arc<dyn Inode>, name: &str) -> Result<()> {
        self.link_impl(old, name)
    }

    fn unlink(&self, name: &str) -> Result<()> {
        self.unlink_impl(name)
    }

    fn rmdir(&self, name: &str) -> Result<()> {
        self.rmdir_impl(name)
    }

    fn rename(
        &self,
        old_name: &str,
        target: &Arc<dyn Inode>,
        new_name: &str,
        mode: RenameMode,
    ) -> Result<()> {
        self.rename_impl(old_name, target, new_name, mode)
    }
}
