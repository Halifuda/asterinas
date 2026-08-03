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

use core::time::Duration;

use super::{
    binding_cache::PositiveKind, entry::RealObject, identity::OverlayObjectId,
    inode_cache::RealObjectKey, visible_source,
};
use crate::{
    fs::{
        file::{InodeMode, InodeType},
        fs_impls::overlayfs::mount::OverlayFs,
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
/// # Dev note (recorded deviation)
///
/// `#[derive(Debug)]` is dropped: the frozen `fs: Weak<OverlayFs>` field
/// cannot satisfy a derived `Debug` bound (`OverlayFs` carries no `Debug`
/// impl), and the spec §4 shape hint explicitly allows dropping an
/// unsatisfiable derive.
///
/// The type is published at the overlayfs ceiling (`pub(in
/// crate::fs::fs_impls::overlayfs)`) because the frozen consumers sit outside
/// the `projection` tree (`mount::build` step 10, sibling meso modules);
/// the fields stay at `pub(super)` (constructible by the sibling
/// `projection::mod`/`entry` creators only).
pub(in crate::fs::fs_impls::overlayfs) struct OverlayInode {
    /// The owning mount; a weak reference so a live inode never pins the
    /// mount (B/C-2 lifetime rule, ramfs `Arc::new_cyclic` + `Weak<RamFs>`
    /// precedent).
    pub(super) fs: Weak<OverlayFs>,
    /// The inode-cache key of the visible-metadata source (`P0-16`).
    pub(super) key: RealObjectKey,
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
        let key = RealObjectKey::from_source(visible);
        let object_id = fs.identity().project_object_id(visible, true);
        let inode = Arc::new(OverlayInode {
            fs: Arc::downgrade(&fs),
            key,
            facts: Mutex::new(facts),
            dir_transaction_lock: Some(Mutex::new(())),
            object_id,
            extension: Extension::new(),
        });
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
        self.key
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
    fn facts_snapshot(&self) -> OverlayObjectFacts {
        self.facts.lock().clone()
    }

    /// Returns the payload-less `DIR` transaction lock, if this object is a
    /// directory.
    ///
    /// The lock carries no payload: its purpose is one-`DIR` transaction
    /// serialization, not data protection (spec §3.0).
    fn dir(&self) -> Option<&Mutex<()>> {
        self.dir_transaction_lock.as_ref()
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
