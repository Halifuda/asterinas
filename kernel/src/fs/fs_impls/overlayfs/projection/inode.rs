// SPDX-License-Identifier: MPL-2.0

//! The Overlay inode carrier and its VFS `Inode` surface
//! (`P0-04`/`P0-06`/`P0-12`/`P0-17`).
//!
//! This module owns the [`OverlayInode`] struct (the published logical inode
//! carrier of the `visibility_projection_identity` meso), its `INODE`-domain
//! payload [`OverlayObjectFacts`], the frozen root-carrier seam
//! ([`OverlayInode::new_root`], consumed by `OverlayFs::new` step 10), and the
//! `Inode` trait implementation: lookup (`P0-08`/`P0-09`), metadata/identity
//! projection (`P0-12`), revalidation (`P0-17`), and the §2 Case-7 gate stubs
//! owned by later Mesos.
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
        file::{InodeMode, InodeType},
        fs_impls::overlayfs::{
            copyup::coordination::CopyUpTransition, mount::OverlayFs, readdir_index::ReaddirIndex,
        },
        vfs::{
            file_system::FileSystem,
            inode::{
                Extension, FallocMode, HardLinkability, Inode, Metadata, MknodType, RenameMode,
                RevalidationPolicy,
            },
            xattr::{XattrName, XattrSetFlags},
        },
    },
    prelude::*,
    process::{Gid, Uid},
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
    #[expect(
        dead_code,
        reason = "frozen wave-3 seam field (handoff §2.3 item 1); read by meso-03/06 impls in Wave 4"
    )]
    pub(in crate::fs::fs_impls::overlayfs) readdir_index: Option<Mutex<ReaddirIndex>>,
    /// The `CUL`-domain (level 3) copy-up transition coordinate; `None` until
    /// meso-04 records the first positive-binding publication (Wave-3 seam,
    /// meso-04 spec §4.1).
    ///
    /// The payload type lands in Wave 4 `copyup/coordination.rs` (frozen name
    /// `CopyUpTransition`); the `record_copyup_transition` hook and the
    /// `ensure_upper_authority` winner/waiter flow read it then.
    #[expect(
        dead_code,
        reason = "frozen wave-3 seam field (handoff §2.3 item 1); read by meso-04 impls in Wave 4"
    )]
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
    #[expect(
        dead_code,
        reason = "frozen wave-3 read-only accessor; consumed by the Wave-4 meso-03 merged scan (readdir_index.rs)"
    )]
    pub(in crate::fs::fs_impls::overlayfs) fn kind(&self) -> PositiveKind {
        self.kind
    }

    /// Returns the upper real object, if this object has an upper component
    /// (ceiling read-only accessor; wave-3 review item 4).
    #[expect(
        dead_code,
        reason = "frozen wave-3 read-only accessor; consumed by the Wave-4 meso-03 merged scan (readdir_index.rs)"
    )]
    pub(in crate::fs::fs_impls::overlayfs) fn upper(&self) -> Option<&RealObject> {
        self.upper.as_ref()
    }

    /// Returns the lower stack, topmost first (ceiling read-only accessor;
    /// wave-3 review item 4).
    #[expect(
        dead_code,
        reason = "frozen wave-3 read-only accessor; consumed by the Wave-4 meso-03 merged scan (readdir_index.rs)"
    )]
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
    #[expect(
        dead_code,
        reason = "frozen wave-3 construction seam; sibling mesos mint facts through it once the Wave-4 leaf Creators land"
    )]
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
        let lowers = layer_stack
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
    #[expect(
        dead_code,
        reason = "frozen published accessor (spec §4); consumed by sibling mesos once they land"
    )]
    pub(in crate::fs::fs_impls::overlayfs) fn key(&self) -> RealObjectKey {
        *self.key.lock()
    }

    /// Returns the precomputed projected `st_dev`/`st_ino` (`P2-01`/`P0-12`).
    ///
    /// Published for sibling Mesos; copy-up (meso-04) re-projection keeps the
    /// lower-id-derived identity, so the value is stable across copy-up
    /// (authority-continuity invariant).
    #[expect(
        dead_code,
        reason = "frozen published accessor (spec §4); consumed by sibling mesos once they land"
    )]
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
    #[expect(
        dead_code,
        reason = "frozen meso-04 copy-up transition seam; consumed by the Wave-4 copyup Creator"
    )]
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

    /// Rejects the operation with `EROFS` on effective read-only mounts.
    ///
    /// The §2 Case-7 gate: mutating operations owned by later Mesos return
    /// `EROFS` when `MountPolicy::is_effective_read_only()` and `EOPNOTSUPP`
    /// otherwise; this helper is the shared read-only half of that gate.
    fn read_only_gate(&self) -> Result<()> {
        let fs = self.fs.upgrade().ok_or_else(|| {
            Error::with_message(Errno::EIO, "the overlay mount is no longer alive")
        })?;
        if fs.policy().is_effective_read_only() {
            return_errno_with_message!(Errno::EROFS, "the overlay mount is read-only");
        }
        Ok(())
    }
}

impl Inode for OverlayInode {
    fn size(&self) -> usize {
        let facts = self.facts_snapshot();
        visible_source(&facts).real_inode().size()
    }

    fn resize(&self, _new_size: usize) -> Result<()> {
        self.read_only_gate()?;
        return_errno_with_message!(
            Errno::EOPNOTSUPP,
            "overlay resize is owned by a later meso and is not implemented yet"
        );
    }

    fn metadata(&self) -> Metadata {
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

    fn ino(&self) -> u64 {
        self.object_id.ino
    }

    fn type_(&self) -> InodeType {
        let facts = self.facts_snapshot();
        visible_source(&facts).real_inode().type_()
    }

    fn mode(&self) -> Result<InodeMode> {
        let facts = self.facts_snapshot();
        visible_source(&facts).real_inode().mode()
    }

    fn set_mode(&self, _mode: InodeMode) -> Result<()> {
        self.read_only_gate()?;
        return_errno_with_message!(
            Errno::EOPNOTSUPP,
            "overlay mode mutation is owned by a later meso and is not implemented yet"
        );
    }

    fn owner(&self) -> Result<Uid> {
        let facts = self.facts_snapshot();
        visible_source(&facts).real_inode().owner()
    }

    fn set_owner(&self, _uid: Uid) -> Result<()> {
        self.read_only_gate()?;
        return_errno_with_message!(
            Errno::EOPNOTSUPP,
            "overlay ownership mutation is owned by a later meso and is not implemented yet"
        );
    }

    fn group(&self) -> Result<Gid> {
        let facts = self.facts_snapshot();
        visible_source(&facts).real_inode().group()
    }

    fn set_group(&self, _gid: Gid) -> Result<()> {
        self.read_only_gate()?;
        return_errno_with_message!(
            Errno::EOPNOTSUPP,
            "overlay group mutation is owned by a later meso and is not implemented yet"
        );
    }

    fn atime(&self) -> Duration {
        let facts = self.facts_snapshot();
        visible_source(&facts).real_inode().atime()
    }

    fn set_atime(&self, _time: Duration) {
        // No-op gate stub (§2 Case 7): the trait returns `()`, so the
        // EROFS/EOPNOTSUPP gate cannot be surfaced here; timestamp mutation
        // is owned by a later Meso.
    }

    fn mtime(&self) -> Duration {
        let facts = self.facts_snapshot();
        visible_source(&facts).real_inode().mtime()
    }

    fn set_mtime(&self, _time: Duration) {
        // No-op gate stub (§2 Case 7): timestamp mutation is owned by a
        // later Meso; see `set_atime`.
    }

    fn ctime(&self) -> Duration {
        let facts = self.facts_snapshot();
        visible_source(&facts).real_inode().ctime()
    }

    fn set_ctime(&self, _time: Duration) {
        // No-op gate stub (§2 Case 7): timestamp mutation is owned by a
        // later Meso; see `set_atime`.
    }

    fn create(&self, _name: &str, _type_: InodeType, _mode: InodeMode) -> Result<Arc<dyn Inode>> {
        self.read_only_gate()?;
        return_errno_with_message!(
            Errno::EOPNOTSUPP,
            "overlay create is owned by a later meso and is not implemented yet"
        );
    }

    fn create_tmpfile(
        &self,
        _mode: InodeMode,
        _hard_linkability: HardLinkability,
    ) -> Result<Arc<dyn Inode>> {
        self.read_only_gate()?;
        return_errno_with_message!(
            Errno::EOPNOTSUPP,
            "overlay tmpfile creation is owned by a later meso and is not implemented yet"
        );
    }

    fn mknod(&self, _name: &str, _mode: InodeMode, _type_: MknodType) -> Result<Arc<dyn Inode>> {
        self.read_only_gate()?;
        return_errno_with_message!(
            Errno::EOPNOTSUPP,
            "overlay mknod is owned by a later meso and is not implemented yet"
        );
    }

    fn link(&self, _old: &Arc<dyn Inode>, _name: &str) -> Result<()> {
        self.read_only_gate()?;
        return_errno_with_message!(
            Errno::EOPNOTSUPP,
            "overlay link is owned by a later meso and is not implemented yet"
        );
    }

    fn unlink(&self, _name: &str) -> Result<()> {
        self.read_only_gate()?;
        return_errno_with_message!(
            Errno::EOPNOTSUPP,
            "overlay unlink is owned by a later meso and is not implemented yet"
        );
    }

    fn rmdir(&self, _name: &str) -> Result<()> {
        self.read_only_gate()?;
        return_errno_with_message!(
            Errno::EOPNOTSUPP,
            "overlay rmdir is owned by a later meso and is not implemented yet"
        );
    }

    fn lookup(&self, name: &str) -> Result<Arc<dyn Inode>> {
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

    fn rename(
        &self,
        _old_name: &str,
        _target: &Arc<dyn Inode>,
        _new_name: &str,
        _mode: RenameMode,
    ) -> Result<()> {
        self.read_only_gate()?;
        return_errno_with_message!(
            Errno::EOPNOTSUPP,
            "overlay rename is owned by a later meso and is not implemented yet"
        );
    }

    fn write_link(&self, _target: &str) -> Result<()> {
        self.read_only_gate()?;
        return_errno_with_message!(
            Errno::EOPNOTSUPP,
            "overlay symlink writes are owned by a later meso and are not implemented yet"
        );
    }

    fn fallocate(&self, _mode: FallocMode, _offset: usize, _len: usize) -> Result<()> {
        self.read_only_gate()?;
        return_errno_with_message!(
            Errno::EOPNOTSUPP,
            "overlay fallocate is owned by a later meso and is not implemented yet"
        );
    }

    fn fs(&self) -> Arc<dyn FileSystem> {
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

    fn revalidation_policy(&self) -> RevalidationPolicy {
        match self.type_() {
            InodeType::Dir => RevalidationPolicy::REVALIDATE_ABSENT,
            _ => RevalidationPolicy::empty(),
        }
    }

    fn revalidate_absent(&self, _name: &str) -> bool {
        // Cheap and conservative (`P0-17`): a negative dentry hit is always
        // re-looked-up. No locks and no I/O (spec §3.3 Hazard 4).
        false
    }

    fn extension(&self) -> &Extension {
        &self.extension
    }

    fn set_xattr(
        &self,
        _name: XattrName,
        _value_reader: &mut VmReader,
        _flags: XattrSetFlags,
    ) -> Result<()> {
        self.read_only_gate()?;
        return_errno_with_message!(
            Errno::EOPNOTSUPP,
            "overlay xattr writes are owned by a later meso and are not implemented yet"
        );
    }

    fn remove_xattr(&self, _name: XattrName) -> Result<()> {
        self.read_only_gate()?;
        return_errno_with_message!(
            Errno::EOPNOTSUPP,
            "overlay xattr removal is owned by a later meso and is not implemented yet"
        );
    }
}
