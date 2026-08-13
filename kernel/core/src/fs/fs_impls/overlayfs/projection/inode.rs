// SPDX-License-Identifier: MPL-2.0

//! The Overlay inode and its canonical VFS trait surface.
//!
//! An [`OverlayInode`] is the published logical inode: one overlay object
//! shared by every name bound to it. [`OverlayObjectFacts`] is the
//! per-object real-object facts — its per-name kind, the upper real object
//! (the visible-metadata source for merged directories), and the
//! topmost-first lower stack — replaced only by the copy-up transition.
//! The module also owns the root-inode constructor
//! ([`OverlayInode::new_root`]) and the sole `Inode` and `FileOps`
//! implementations of the overlay.
//!
//! # Locking
//!
//! `dir_transaction_lock` serializes directory mutations (present only on
//! directories). `facts` guards the per-object facts and is normally held
//! only briefly; the one non-obvious hold is [`OverlayInode::append_write`],
//! which keeps the `facts` guard across the underlying `size()` + `write_at`
//! so concurrent appends serialize on the post-write size.
//!
//! # Structure
//!
//! | Item | Owns |
//! |---|---|
//! | [`OverlayInode`] | The published logical inode and its `Inode`/`FileOps` surfaces. |
//! | [`OverlayObjectFacts`] | The immutable real-object facts. |
//!
//! # References
//!
//! - Overlayfs (Linux overlay filesystem):
//!   <https://www.kernel.org/doc/html/latest/filesystems/overlayfs.html>

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

/// The logical Overlay inode exposed to the VFS: one logical overlay object
/// shared by every name bound to it, with the real-object facts living once
/// in [`OverlayObjectFacts`].
pub(in crate::fs::fs_impls::overlayfs) struct OverlayInode {
    /// The owning mount.
    pub(super) fs: Weak<OverlayFs>,
    /// The inode-cache key of the visible-metadata source.
    pub(super) key: Mutex<RealObjectKey>,
    /// The per-object real-object facts, replaced only by the copy-up
    /// transition.
    pub(super) facts: Mutex<OverlayObjectFacts>,
    /// The per-directory transaction lock; `Some` iff this object is a
    /// directory.
    pub(super) dir_transaction_lock: Option<Mutex<()>>,
    /// The precomputed projected `st_dev`/`st_ino`.
    pub(super) object_id: OverlayObjectId,
    /// The VFS inode extension groups (fs event publisher / fs lock context).
    pub(super) extension: Extension,
    /// The per-directory merged-readdir index; `Some` iff this object is a
    /// directory.
    pub(in crate::fs::fs_impls::overlayfs) readdir_index: Option<Mutex<ReaddirIndex>>,
    /// The copy-up transition coordinate; `None` until copy-up records the
    /// first positive-binding publication.
    pub(in crate::fs::fs_impls::overlayfs) copyup_transition: Mutex<Option<CopyUpTransition>>,
}

/// The immutable real-object facts of one logical overlay object.
///
/// Invariant: `upper.is_some() || !lowers.is_empty()`, enforced at the
/// construction paths (the in-tree `projection` builders and the checked
/// [`OverlayObjectFacts::try_new`] constructor).
#[derive(Clone, Debug)]
pub(in crate::fs::fs_impls::overlayfs) struct OverlayObjectFacts {
    /// The per-name view classification of this object.
    pub(super) kind: PositiveKind,
    /// The upper real object; the visible-metadata source for merged
    /// directories.
    pub(super) upper: Option<RealObject>,
    /// The lower stack, topmost first; non-empty for lower-only/merged
    /// objects.
    pub(super) lowers: Vec<RealObject>,
}

impl OverlayObjectFacts {
    pub(in crate::fs::fs_impls::overlayfs) fn kind(&self) -> PositiveKind {
        self.kind
    }

    pub(in crate::fs::fs_impls::overlayfs) fn upper(&self) -> Option<&RealObject> {
        self.upper.as_ref()
    }

    pub(in crate::fs::fs_impls::overlayfs) fn lowers(&self) -> &[RealObject] {
        &self.lowers
    }

    /// Constructs an [`OverlayObjectFacts`], returning `None` when both
    /// `upper` and `lowers` are empty.
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
    /// Kinds and upper identities must match; `Single` objects compare only
    /// the visible source (post-copy-up inodes retain bookkeeping lowers),
    /// `Merged` objects compare the full lower composition strictly.
    pub(in crate::fs::fs_impls::overlayfs) fn same_visible_identity(&self, other: &Self) -> bool {
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
            PositiveKind::Single => Arc::ptr_eq(
                visible_source(self).real_inode(),
                visible_source(other).real_inode(),
            ),
            PositiveKind::Merged => {
                self.lowers().len() == other.lowers().len()
                    && self
                        .lowers()
                        .iter()
                        .zip(other.lowers())
                        .all(|(left, right)| Arc::ptr_eq(left.real_inode(), right.real_inode()))
            }
        }
    }

    /// Returns whether `real_inode` is the same logical object as this
    /// object's visible source or any of its retained lowers.
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
    /// Constructs the root overlay inode — the mount-root inode published in
    /// `OverlayFs::new`.
    ///
    /// The root facts merge the upper root with all lower roots; the root is
    /// always a directory.
    pub(in crate::fs::fs_impls::overlayfs) fn new_root(fs: Weak<OverlayFs>) -> Arc<dyn Inode> {
        let fs = match fs.upgrade() {
            Some(fs) => fs,
            None => unreachable!(
                "OverlayFs::new materializes the root inode right after publishing \
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
        // one lower directory participates.
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
            readdir_index: Some(Mutex::new(ReaddirIndex::new())),
            copyup_transition: Mutex::new(None),
        });
        // Register the root inode in the inode cache so every live inode
        // resolves by its visible-source key; `publication_parent` then needs
        // no root special case and `project_inode` never mints a duplicate.
        fs.inodes().get_or_create(key, |_| true, || inode.clone());
        inode
    }

    pub(in crate::fs::fs_impls::overlayfs) fn key(&self) -> RealObjectKey {
        *self.key.lock()
    }

    /// Returns the precomputed projected `st_dev`/`st_ino`.
    ///
    /// Copy-up re-projection keeps the lower-id-derived identity, so the
    /// value is stable across copy-up (authority-continuity invariant).
    pub(in crate::fs::fs_impls::overlayfs) fn object_id(&self) -> OverlayObjectId {
        self.object_id
    }

    pub(in crate::fs::fs_impls::overlayfs) fn facts_snapshot(&self) -> OverlayObjectFacts {
        self.facts.lock().clone()
    }

    /// Serializes an `O_APPEND` write as one atomic size-read + write.
    ///
    /// The `facts` lock is held across both steps because the underlying fs
    /// does not process `O_APPEND` itself. This is the one exception to
    /// holding `facts` only briefly, and it serializes concurrent appends
    /// on the post-write size.
    pub(in crate::fs::fs_impls::overlayfs) fn append_write(
        &self,
        reader: &mut VmReader,
        status_flags: StatusFlags,
    ) -> Result<usize> {
        let guard = self.facts.lock();
        let real = match guard.upper() {
            Some(upper) => upper.real_inode().clone(),
            None => guard.lowers()[0].real_inode().clone(),
        };
        let offset = real.size();
        real.write_at(offset, reader, status_flags)
    }

    pub(in crate::fs::fs_impls::overlayfs) fn dir(&self) -> Option<&Mutex<()>> {
        self.dir_transaction_lock.as_ref()
    }

    /// Replaces the real-object facts of this inode — the copy-up transition.
    ///
    /// The transition is fallible and self-consistent: the inode-cache
    /// registration is aliased under the new visible-source key while the
    /// old-key mapping is retained, then the facts and published `key` are
    /// swapped. The alias runs first, so a displacement fails rather than
    /// silently orphaning the inode.
    pub(in crate::fs::fs_impls::overlayfs) fn replace_facts(
        self: &Arc<Self>,
        facts: OverlayObjectFacts,
        new_visible_source: &RealObject,
    ) -> Result<()> {
        let new_key = RealObjectKey::from_facts(&facts);
        // Capture the pre-transition visible-source key AND its real inode
        // under one brief `facts` lock: the old real inode becomes the
        // keep-alive pin of the retained old-key alias (`alias_key`), so it
        // cannot be recycled while the alias exists.
        let (old_key, old_real_inode) = {
            let old_facts = self.facts.lock();
            (
                RealObjectKey::from_facts(&old_facts),
                visible_source(&old_facts).real_inode().clone(),
            )
        };
        // A live inode cannot outlive its mount; the teardown arm swaps the
        // facts locally and skips the cache alias (no live lookup can
        // observe this inode then).
        let Some(fs) = self.fs.upgrade() else {
            *self.facts.lock() = facts;
            *self.key.lock() = new_key;
            return Ok(());
        };
        // The fallible alias runs first; only after it succeeds is the
        // inode's own state committed.
        fs.inodes()
            .alias_key(old_key, new_key, old_real_inode, new_visible_source)?;
        *self.facts.lock() = facts;
        *self.key.lock() = new_key;
        debug_assert!(
            fs.inodes()
                .get(new_key)
                .is_some_and(|probe| Arc::ptr_eq(&probe, self)),
            "after replace_facts the inode cache maps the new visible-source key to THIS inode"
        );
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
        let dir = self.dir().ok_or_else(|| {
            Error::with_message(Errno::ENOTDIR, "the overlay inode is not a directory")
        })?;
        let _dir_guard = dir.lock();
        let facts = self.facts_snapshot();
        let fs = self.fs.upgrade().ok_or_else(|| {
            Error::with_message(Errno::EIO, "the overlay mount is no longer alive")
        })?;
        let binding = fs.lookup_binding(&facts, name)?.binding;
        match binding.into_inode() {
            Some(inode) => Ok(inode),
            None => Err(Error::new(Errno::ENOENT)),
        }
    }

    fn fs_impl(&self) -> Arc<dyn FileSystem> {
        match self.fs.upgrade() {
            Some(fs) => fs,
            None => unreachable!("a live OverlayInode keeps its OverlayFs alive"),
        }
    }

    /// Returns the revalidation policy for this inode.
    ///
    /// Directories use `REVALIDATE_ABSENT`: an absent name may have appeared
    /// behind the overlay since the last lookup (a lower-layer change or a
    /// concurrent mutation), so a cached negative dentry must be re-checked.
    /// Non-directories return the empty policy: their existence is pinned by
    /// the binding, so no absent-name revalidation applies.
    fn revalidation_policy_impl(&self) -> RevalidationPolicy {
        match self.type_() {
            InodeType::Dir => RevalidationPolicy::REVALIDATE_ABSENT,
            _ => RevalidationPolicy::empty(),
        }
    }

    fn revalidate_absent_impl(&self, _name: &str) -> bool {
        // A negative dentry hit is always re-looked-up.
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

impl OverlayInode {
    /// Resolves the identity published for this directory's `..` entry.
    ///
    /// Serves the child-source-layer real parent identity (exact when the
    /// overlay parent's visible source is on the same layer, otherwise an
    /// approximation), falling back to the stable `d_ino("..") ==
    /// d_ino(".")` self-parent when no disclosure-safe projection exists.
    pub(in crate::fs::fs_impls::overlayfs) fn resolve_parent_object_id(
        &self,
        facts: &OverlayObjectFacts,
    ) -> OverlayObjectId {
        let fs = match self.fs_arc() {
            Ok(fs) => fs,
            Err(err) => {
                warn!(
                    "overlay readdir: the owning mount is unavailable ({:?}); \
                     falling back to d_ino(\"..\") == d_ino(\".\")",
                    err
                );
                return self.parent_fallback();
            }
        };
        // Overlay-root special case: `..` is the root itself (Unix
        // self-parent); the underlying `lookup("..")` is skipped.
        if self.is_mount_root(&fs) {
            return self.parent_fallback();
        }
        // Determinism short-circuit: on a multi-fs xino-off mount the
        // projection matrix takes the xino-off/overflow directory branch for
        // EVERY parent (a fresh fallback ino per call — unstable), so the
        // whole route is predetermined to serve the stable self-parent
        // approximation; skip the underlying `lookup("..")`/origin read whose
        // result would only be discarded.
        if !fs.identity().is_xino_effective() && !fs.identity().is_all_layers_same_fs() {
            return self.parent_fallback();
        }
        let visible = visible_source(facts);
        let parent_real_inode = match visible.real_inode().lookup("..") {
            Ok(parent) => parent,
            Err(err) => {
                warn!(
                    "overlay readdir: `..` resolution on the visible source failed \
                     ({:?}); falling back to d_ino(\"..\") == d_ino(\".\")",
                    err
                );
                return self.parent_fallback();
            }
        };
        // Upper-backed real parent: prefer the durable lower-id record so the
        // `..` identity matches the parent's record-derived `stat("..")`,
        // gated on deterministic projection.
        if visible.layer_index() == 0
            && let Some(object_id) = self.project_parent_from_lower_record(&fs, &parent_real_inode)
        {
            return object_id;
        }
        if !fs
            .identity()
            .is_directory_projection_deterministic(visible.fsid(), parent_real_inode.ino())
        {
            return self.parent_fallback();
        }
        let parent_real = RealObject::new(
            visible.layer_index(),
            parent_real_inode,
            visible.fsid(),
            visible.container_dev_id(),
        );
        fs.identity().project_object_id(&parent_real, true)
    }

    /// Projects the upper-backed real parent's identity from its durable
    /// origin record, gated on deterministic projection.
    ///
    /// Returns `None` when no readable record resolves to a current lower
    /// layer or the projection would be non-deterministic; the caller then
    /// attempts the visible-source projection. The underlying `read_lower_id`
    /// is caller-credential-gated, so `d_ino("..")` may differ between
    /// privileged and unprivileged readers (logged at `debug!`).
    pub(in crate::fs::fs_impls::overlayfs) fn project_parent_from_lower_record(
        &self,
        fs: &OverlayFs,
        parent_real_inode: &Arc<dyn Inode>,
    ) -> Option<OverlayObjectId> {
        match fs.read_lower_id(parent_real_inode) {
            Ok(Some(record)) => {
                // When all layers share one filesystem, projection passes the origin
                // through without a layer id, so this caller skips the layer resolution.
                if !fs.identity().is_all_layers_same_fs() {
                    let layer_id = fs.identity().resolve_layer_id_for_record(
                        record.container_dev_id(),
                        record.lower_layer_root_ino(),
                    )?;
                    if !fs
                        .identity()
                        .is_directory_projection_deterministic(layer_id, record.real_ino())
                    {
                        return None;
                    }
                }
                fs.identity().project_object_id_from_lower_id(&record, true)
            }
            Ok(None) => None,
            Err(err) if matches!(err.error(), Errno::EACCES | Errno::EPERM) => {
                debug!(
                    "overlay readdir: the parent's origin record is \
                     credential-gated ({:?}); d_ino(\"..\") may differ between \
                     privileged and unprivileged readers until the VFS can \
                     read xattrs with the caller's credentials; falling back to the \
                     visible-source projection",
                    err
                );
                None
            }
            Err(err) => {
                debug!(
                    "overlay readdir: the parent's origin record is unreadable \
                     ({:?}); falling back to the visible-source projection",
                    err
                );
                None
            }
        }
    }

    /// Returns whether this inode is the overlay mount root (the self-parent
    /// special case of the `..` route).
    ///
    /// The check compares the root inode's inode-cache key against
    /// `self.key()` and fails closed (serves the self-parent fallback) when
    /// the root is not an `OverlayInode`, never disclosing the backing-store
    /// parent.
    pub(in crate::fs::fs_impls::overlayfs) fn is_mount_root(&self, fs: &OverlayFs) -> bool {
        match Arc::downcast::<OverlayInode>(fs.root_inode()) {
            Ok(root_carrier) => root_carrier.key() == self.key(),
            Err(_) => {
                warn!(
                    "overlay readdir: the mount root inode is not an OverlayInode; \
                     serving the self-parent fallback"
                );
                true
            }
        }
    }

    /// Returns the `d_ino("..") == d_ino(".")` approximation: the stable
    /// fallback identity served when the real parent cannot be resolved
    /// disclosure-safely or deterministically (overlay root, xino-off /
    /// overflow directory branch, unresolvable real parent, or unavailable
    /// owning mount).
    pub(in crate::fs::fs_impls::overlayfs) fn parent_fallback(&self) -> OverlayObjectId {
        self.object_id()
    }
}
