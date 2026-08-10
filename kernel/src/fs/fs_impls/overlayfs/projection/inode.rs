// SPDX-License-Identifier: MPL-2.0

//! The Overlay inode carrier and its canonical VFS trait surface.
//!
//! This module owns the [`OverlayInode`] struct (the published logical inode
//! carrier), its per-object payload [`OverlayObjectFacts`], the root-carrier
//! constructor ([`OverlayInode::new_root`], consumed by `OverlayFs::new` step
//! 10),
//! and the sole `Inode` and `FileOps` implementations. Those canonical trait
//! methods directly forward each module's behavior to its current helper
//! owner: projection lookup/metadata/identity/revalidation helpers stay here;
//! copy-up, readdir, metadata-security, and directory-mutation behavior stays
//! in their existing module helpers, including `dir/`'s namespace mutations.
//!
//! Lock contract: `dir_transaction_lock` is the payload-less per-directory
//! `DIR` transaction lock (`Some` for directories only); `facts` is the
//! per-object `INODE` domain, accessed only through brief snapshot locks
//! (`facts_snapshot`: clone and release) and never held across an underlying
//! call. The only nested order is `DIR -> INODE` plus the mount-wide cache
//! locks used sequentially under `DIR` inside `OverlayFs::lookup_binding`.

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
        utils::DirentVisitor,
        vfs::{
            file_system::FileSystem,
            inode::{
                Extension, FallocMode, FileOps, Inode, Metadata, MknodType, RenameMode,
                RevalidationPolicy, SymbolicLink,
            },
            xattr::{XattrName, XattrNamespace, XattrSetFlags},
        },
    },
    prelude::*,
    process::{Gid, Uid},
    vm::page_cache::Vmo,
};

/// The logical Overlay inode carrier exposed to the VFS.
///
/// One [`OverlayInode`] represents one logical overlay object and is shared by
/// every name bound to it: hard links reuse a single carrier through the
/// inode cache, and a `PositiveBinding` references the shared inode with zero
/// per-name fact duplication. The real-object facts live exactly once in
/// [`OverlayObjectFacts`], and the precomputed `object_id` publishes the
/// projected `st_dev`/`st_ino` (for copied-up objects it already carries the
/// lower-id derived identity, so no re-derivation happens at stat time).
///
/// Invariants: `fs` is a `Weak<OverlayFs>` so no `fs -> inode -> fs` strong
/// cycle exists (ramfs precedent); `dir_transaction_lock` is `Some` iff the
/// object is a directory; `facts` is fixed at creation and only transitions
/// via copy-up (replaced, never mutated in place).
///
/// `readdir_index` is `Some` iff the object is a directory (empty initial
/// index), and `copyup_transition` starts `None` (copy-up records the first
/// transition at the positive-binding publication point).
pub(in crate::fs::fs_impls::overlayfs) struct OverlayInode {
    /// The owning mount; a weak reference so a live inode never pins the
    /// mount (ramfs `Arc::new_cyclic` + `Weak<RamFs>` precedent).
    pub(super) fs: Weak<OverlayFs>,
    /// The inode-cache key of the visible-metadata source.
    ///
    /// Interior-mutable so the copy-up transition
    /// ([`OverlayInode::replace_facts`]) can commit the carrier's new
    /// visible-source key atomically with its inode-cache alias: the
    /// fallible `alias_key` runs first (the old-key mapping is retained until
    /// the dead-pin sweep reclaims it), then this field is updated — the
    /// published `key()` never disagrees with `facts_snapshot()`. The field
    /// is touched only under the object's `DIR` transaction.
    pub(super) key: Mutex<RealObjectKey>,
    /// The per-object payload: the real-object facts, fixed at creation
    /// (brief snapshot locks only).
    pub(super) facts: Mutex<OverlayObjectFacts>,
    /// The payload-less `DIR` transaction lock; `Some` iff this object is a
    /// directory.
    pub(super) dir_transaction_lock: Option<Mutex<()>>,
    /// The precomputed projected `st_dev`/`st_ino`.
    pub(super) object_id: OverlayObjectId,
    /// The VFS inode extension groups (fs event publisher / fs lock context).
    pub(super) extension: Extension,
    /// The per-directory merged-readdir index; `Some` iff this object is a
    /// directory.
    ///
    /// The initial value is the empty index (`ReaddirIndex::new()` —
    /// `NeedsRebuild`, no entries), maintained by `readdir_at`,
    /// `invalidate_readdir_index`, and the namespace-mutation update entries.
    pub(in crate::fs::fs_impls::overlayfs) readdir_index: Option<Mutex<ReaddirIndex>>,
    /// The copy-up transition coordinate; `None` until copy-up records the
    /// first positive-binding publication.
    ///
    /// Read by the `record_copyup_transition` hook and the
    /// `ensure_upper_authority` winner/waiter flow.
    pub(in crate::fs::fs_impls::overlayfs) copyup_transition: Mutex<Option<CopyUpTransition>>,
}

/// The immutable real-object facts of one logical overlay object.
///
/// The visible-metadata source is `upper` when present, else the topmost
/// lower (`lowers[0]`); merged directories report upper metadata only.
///
/// Invariants: `upper.is_some() || !lowers.is_empty()`; the facts are fixed
/// at creation and only transition via copy-up (replaced, never mutated in
/// place).
///
/// The `upper.is_some() || !lowers.is_empty()` invariant is enforced at the
/// construction paths — the in-tree `projection` builders and the checked
/// [`OverlayObjectFacts::try_new`] constructor. The merged-directory scan
/// (`readdir_index.rs`) branches on `kind()` and enumerates the layer
/// `RealObject`s through the read-only methods (`kind`/`upper`/`lowers`).
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
    /// Returns the per-name view classification of this object's facts.
    pub(in crate::fs::fs_impls::overlayfs) fn kind(&self) -> PositiveKind {
        self.kind
    }

    /// Returns the upper real object, if this object has an upper component.
    pub(in crate::fs::fs_impls::overlayfs) fn upper(&self) -> Option<&RealObject> {
        self.upper.as_ref()
    }

    /// Returns the lower stack, topmost first.
    pub(in crate::fs::fs_impls::overlayfs) fn lowers(&self) -> &[RealObject] {
        &self.lowers
    }

    /// Constructs an [`OverlayObjectFacts`], returning `None` when both
    /// `upper` and `lowers` are empty.
    ///
    /// The checked fallible constructor — the only construction path for
    /// [`OverlayObjectFacts`] outside the `projection` tree. Enforces the
    /// invariant `upper.is_some() || !lowers.is_empty()` at construction, so
    /// a sibling module can never mint facts whose `visible_source` indexing
    /// (`lowers[0]`) would panic on the lookup/metadata hot path.
    pub(in crate::fs::fs_impls::overlayfs) fn try_new(
        kind: PositiveKind,
        upper: Option<RealObject>,
        lowers: Vec<RealObject>,
    ) -> Option<Self> {
        if upper.is_some() || !lowers.is_empty() {
            Some(Self {
                kind,
                upper,
                lowers,
            })
        } else {
            None
        }
    }

    /// Compares this object's facts against `other` for visible identity.
    ///
    /// Kind-aware positive-identity comparison for the memo verification:
    /// the kinds must match; the upper identities must match (`Arc::ptr_eq`,
    /// or both absent); `Single` objects compare only the visible source
    /// (post-copy-up carriers legitimately retain bookkeeping lowers that the
    /// layer scan no longer reports), while `Merged` objects compare the full
    /// lower composition strictly so a silent lower-layer add/remove is
    /// detected.
    pub(in crate::fs::fs_impls::overlayfs) fn same_visible_identity(
        &self,
        other: &Self,
    ) -> bool {
        if self.kind() != other.kind() {
            return false;
        }
        let same_upper = match (self.upper(), other.upper()) {
            (Some(left), Some(right)) => Arc::ptr_eq(left.real_inode(), right.real_inode()),
            (None, None) => true,
            _ => false,
        };
        if !same_upper {
            return false;
        }
        match self.kind() {
            PositiveKind::Single => {
                Arc::ptr_eq(visible_source(self).real_inode(), visible_source(other).real_inode())
            }
            PositiveKind::Merged => {
                self.lowers().len() == other.lowers().len()
                    && self.lowers().iter().zip(other.lowers()).all(|(left, right)| {
                        Arc::ptr_eq(left.real_inode(), right.real_inode())
                    })
            }
        }
    }

    /// Returns whether `real_inode` is the same logical object as this
    /// object's visible source or any of its retained lowers.
    ///
    /// "Same logical object" = `Arc::ptr_eq` against the visible source OR
    /// any retained lower (covers legal aliases: stale-facts copy-up lookups
    /// resolve the old lower object, and post-copy-up lookups resolve the new
    /// upper object; excludes ino-reuse newcomers).
    pub(in crate::fs::fs_impls::overlayfs) fn contains_real_inode(
        &self,
        real_inode: &Arc<dyn Inode>,
    ) -> bool {
        Arc::ptr_eq(visible_source(self).real_inode(), real_inode)
            || self
                .lowers()
                .iter()
                .any(|lower| Arc::ptr_eq(lower.real_inode(), real_inode))
    }
}

impl OverlayInode {
    /// Constructs the root overlay carrier (the construction step-10
    /// constructor).
    ///
    /// Infallible and eager: every fallible preparation completed inside
    /// `OverlayFs::new`, so it performs no fallible VFS operation and takes
    /// no Overlay lock. It accepts the canonical `Weak<OverlayFs>` and is
    /// called by `OverlayFs::new` AFTER the `Arc` is published (the
    /// `Arc::new_cyclic` closure cannot upgrade its `&Weak` — the strong count
    /// stays 0 until the closure returns), so the upgrade below is
    /// guaranteed. The root facts merge the upper root (writable mounts) with
    /// all lower roots (topmost first); each root `RealObject` is built from
    /// its layer's dentry-anchored `RealPath` anchor
    /// ([`RealObject::with_path`]), so the root carriers pin the base-mount
    /// dentry layer for the mount lifetime. The root is always a directory,
    /// so `dir_transaction_lock` is `Some`; `object_id` is projected by
    /// `fs.identity()` from the visible-metadata source.
    pub(in crate::fs::fs_impls::overlayfs) fn new_root(fs: Weak<OverlayFs>) -> Arc<dyn Inode> {
        // Construction order: `OverlayFs::new` publishes the `Arc` first (via
        // `Arc::new_cyclic`) and calls this seam immediately afterwards, so
        // the upgrade always succeeds; the failure arm is genuinely
        // unreachable and panics rather than fabricating a mount-less root
        // carrier (never silently wrong, no `.unwrap()`/`.expect()`).
        let fs = match fs.upgrade() {
            Some(fs) => fs,
            None => unreachable!(
                "OverlayFs::new materializes the root carrier right after publishing \
                 the Arc; the mount reference is always alive at this call site"
            ),
        };
        let layer_stack = fs.layer_stack();
        let upper = layer_stack.upper.as_ref().map(|layer| {
            RealObject::with_path(
                0,
                layer.root_path.clone(),
                layer.fsid,
                layer.container_dev_id,
            )
        });
        let lowers: Vec<_> = layer_stack
            .lowers
            .iter()
            .enumerate()
            .map(|(layer_index, layer)| {
                RealObject::with_path(
                    layer_index + 1,
                    layer.root_path.clone(),
                    layer.fsid,
                    layer.container_dev_id,
                )
            })
            .collect();
        // Merged-root classification: a writable root merges the upper with
        // the lowers; a read-only root merges its lower stack when more than
        // one lower directory participates (the Linux `ovl_multilayer`
        // analog — the upper+lower case and the multi-lower case are both
        // `PositiveKind::Merged` "directory merging upper and lower
        // observations").
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
        // The layer stack always carries at least one lower layer, so
        // `visible_source` never indexes an empty `lowers`.
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
            // The root is always a directory, so the readdir index is `Some`
            // (empty initial index); the copy-up transition starts `None`
            // (copy-up records the first transition).
            readdir_index: Some(Mutex::new(ReaddirIndex::new())),
            copyup_transition: Mutex::new(None),
        });
        // Register the root carrier in the inode cache alongside the mount
        // slot, so every live carrier — root included — resolves by its
        // visible-source key. `publication_parent` then needs no key-equality
        // root special case (a non-root directory aliasing the root's real
        // object resolves to the same carrier on every path), and
        // `project_inode` can never mint a duplicate root carrier. The
        // registration is a brief internal-data cache lock at single-threaded
        // mount construction.
        // no-op identity check: the root carrier is registered at mount construction
        fs.inodes().get_or_create(key, |_| true, || inode.clone());
        inode
    }

    /// Returns the inode-cache key of the visible-metadata source.
    ///
    /// Published for sibling modules (merged-directory consumption of
    /// directory inputs/IDs).
    pub(in crate::fs::fs_impls::overlayfs) fn key(&self) -> RealObjectKey {
        *self.key.lock()
    }

    /// Returns the precomputed projected `st_dev`/`st_ino`.
    ///
    /// Published for sibling modules; copy-up re-projection keeps the
    /// lower-id-derived identity, so the value is stable across copy-up
    /// (authority-continuity invariant). Consumed by `readdir_index.rs`
    /// `parent_fallback` (the `d_ino("..") == d_ino(".")` route).
    pub(in crate::fs::fs_impls::overlayfs) fn object_id(&self) -> OverlayObjectId {
        self.object_id
    }

    /// Returns a clone of the fixed real-object facts under a brief `INODE`
    /// lock.
    ///
    /// The snapshot read runs in `DIR -> INODE` order, clones the facts, and
    /// releases the guard before any lock-free use, so the `INODE` domain is
    /// never held across an underlying call.
    pub(in crate::fs::fs_impls::overlayfs) fn facts_snapshot(&self) -> OverlayObjectFacts {
        self.facts.lock().clone()
    }

    /// Returns the payload-less `DIR` transaction lock, if this object is a
    /// directory.
    ///
    /// The lock carries no payload: its purpose is one-`DIR` transaction
    /// serialization, not data protection.
    pub(in crate::fs::fs_impls::overlayfs) fn dir(&self) -> Option<&Mutex<()>> {
        self.dir_transaction_lock.as_ref()
    }

    /// Replaces the real-object facts of this inode — the copy-up transition
    /// (replaced, never mutated in place).
    ///
    /// The transition is self-consistent and fallible: the inode-cache
    /// registration is aliased under the new visible-source key while the
    /// old-key mapping is retained (`InodeCache::alias_key`, both keys → the
    /// one carrier), then the payload is swapped and the published
    /// [`OverlayInode::key`] is re-derived from the new visible source. The
    /// fallible alias runs FIRST and its `Err` propagates with `facts`/`key`
    /// untouched, so a displacement (a different live carrier already
    /// registered at the new key) fails the transition and the copy-up caller
    /// can fail or retry instead of proceeding with a split. The caller
    /// passes the post-transition visible source (`new_visible_source`) so
    /// `alias_key` can tell the two live-occupant cases apart: a same-object
    /// concurrent displacement keeps the error, while an ino-reuse stale
    /// occupant at the new key is replaced and self-healed. After the
    /// commit, a directory carrier also drops the per-parent binding table
    /// published under the old parent identity (`invalidate_parent`),
    /// releasing the stale bindings' strong pins; no inode-cache guard is
    /// held when that leaf write lock is taken. A live parent
    /// resolves to exactly one carrier on every path and
    /// `OverlayFs::publication_parent`'s probe cannot miss for a live parent.
    /// Validity (`upper.is_some() || !lowers.is_empty()`) is guaranteed
    /// because the only construction path outside `projection` is the checked
    /// [`OverlayObjectFacts::try_new`], so the replacement cannot mint invalid
    /// facts. Copy-up calls this under the object's `DIR` transaction lock and
    /// must serialize the transition against concurrent projections of the
    /// same real object (hold the object's and the parents' `DIR`s across
    /// `replace_facts`): the old-key alias closes the stale-facts race,
    /// `alias_key` surfaces — never silently orphans — a live carrier already
    /// projected at the new key, and the retained old-key alias carries a
    /// strong keep-alive pin of the pre-transition real inode so it cannot be
    /// recycled while the alias exists. The post-condition is verified with
    /// `Arc::ptr_eq` against this carrier.
    pub(in crate::fs::fs_impls::overlayfs) fn replace_facts(
        self: &Arc<Self>,
        facts: OverlayObjectFacts,
        new_visible_source: &RealObject,
    ) -> Result<()> {
        let new_key = RealObjectKey::from_facts(&facts);
        // Capture the pre-transition visible-source key AND its real inode
        // under one brief `INODE` lock: the old real inode becomes the
        // keep-alive pin of the retained old-key alias (`alias_key`), so it
        // cannot be recycled while the alias exists.
        let (old_key, old_real_inode) = {
            let old_facts = self.facts.lock();
            (
                RealObjectKey::from_facts(&old_facts),
                visible_source(&old_facts).real_inode().clone(),
            )
        };
        // A live carrier cannot outlive its mount; the teardown arm swaps the
        // payload locally and skips the cache alias (no live lookup can
        // observe this carrier then).
        let Some(fs) = self.fs.upgrade() else {
            *self.facts.lock() = facts;
            *self.key.lock() = new_key;
            return Ok(());
        };
        // The fallible alias runs first: both key views (old and new) still
        // resolve to this one carrier, so a displacement at `new_key` fails
        // the transition with `facts`/`key` untouched and the copy-up caller
        // can fail or retry. Only then is the carrier's own state committed
        // (the old-key alias stays for stale-facts in-flight projections and
        // is retired by the dead-pin sweep).
        fs.inodes().alias_key(old_key, new_key, old_real_inode, new_visible_source)?;
        *self.facts.lock() = facts;
        *self.key.lock() = new_key;
        debug_assert!(
            fs.inodes()
                .get(new_key)
                .is_some_and(|probe| Arc::ptr_eq(&probe, self)),
            "after replace_facts the inode cache maps the new visible-source key to THIS carrier"
        );
        // Clean the stale per-parent binding table of the old parent identity
        // after the semantic commit; directory-only (a non-directory carrier
        // never owns bindings, so the write would be a no-op). No inode-cache
        // guard is held at this point, so the binding-cache write stays a
        // leaf acquisition (the two cache locks are never held together).
        if self.dir_transaction_lock.is_some() {
            fs.bindings().invalidate_parent(&old_key);
        }
        Ok(())
    }
}

impl OverlayInode {
    fn size_impl(&self) -> usize {
        let facts = self.facts_snapshot();
        visible_source(&facts).real_inode().size()
    }

    fn metadata_impl(&self) -> Result<Metadata> {
        let facts = self.facts_snapshot();
        let mut metadata = visible_source(&facts).real_inode().metadata()?;
        // The precomputed `object_id` replaces dev/ino: copied-up objects
        // already carry the lower-id-derived identity, so no re-derivation
        // happens here. Merged directories report upper metadata only (the
        // visible source is the upper).
        metadata.ino = self.object_id.ino;
        metadata.container_dev_id = self.object_id.dev;
        Ok(metadata)
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
        // publication) runs inside this one guard.
        let dir = self.dir().ok_or_else(|| {
            Error::with_message(Errno::ENOTDIR, "the overlay inode is not a directory")
        })?;
        let _dir_guard = dir.lock();
        // Brief `INODE` snapshot (`DIR -> INODE` order), then lock-free use.
        let facts = self.facts_snapshot();
        let fs = self.fs.upgrade().ok_or_else(|| {
            Error::with_message(Errno::EIO, "the overlay mount is no longer alive")
        })?;
        let binding = fs.lookup_binding(&facts, name)?.binding;
        match binding.into_inode() {
            Some(inode) => Ok(inode),
            // Every negative variant (`Absent`/`HiddenByWhiteout`/
            // `HiddenByOpaque`) surfaces as `ENOENT` to the VFS.
            None => Err(Error::new(Errno::ENOENT)),
        }
    }

    fn fs_impl(&self) -> Arc<dyn FileSystem> {
        match self.fs.upgrade() {
            Some(fs) => fs,
            // A live `OverlayInode` pins its real objects and is itself
            // reachable only while the mount lives, so the upgrade succeeds
            // by contract; the post-teardown fallback is not invented here —
            // no `.unwrap()`/`.expect()` is introduced.
            None => unreachable!("a live OverlayInode keeps its OverlayFs alive"),
        }
    }

    fn revalidation_policy_impl(&self) -> RevalidationPolicy {
        match self.type_() {
            InodeType::Dir => RevalidationPolicy::REVALIDATE_ABSENT,
            _ => RevalidationPolicy::empty(),
        }
    }

    fn revalidate_absent_impl(&self, _name: &str) -> bool {
        // Cheap and conservative: a negative dentry hit is always
        // re-looked-up. No locks and no I/O.
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

    fn metadata(&self) -> Result<Metadata> {
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

    fn page_cache(&self) -> Option<Arc<Vmo>> {
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
