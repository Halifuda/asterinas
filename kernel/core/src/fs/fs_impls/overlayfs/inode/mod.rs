// SPDX-License-Identifier: MPL-2.0
#![short_vis_path::add(overlayfs)]

//! The logical overlay inode and its VFS trait surface.
//!
//! [`OverlayInode`] is the published logical inode shared by every name bound
//! to the same overlay object. It owns the per-object real-object facts, the
//! per-directory transaction lock, the precomputed projected identity, and
//! the copy-up coordination state.
//!
//! # Locking
//!
//! `lock` is the per-inode transaction lock; directories carry a
//! [`ReaddirIndex`] in its payload, while non-directories use it as a plain
//! serialization token. [`OverlayInode::append_write`] holds this lock across
//! the underlying `size()` + `write_at` so concurrent appends serialize on the
//! post-write size.

mod copyup;
mod data;
mod dir;
mod identity;
mod inode_cache;
mod lookup;
mod metadata;
mod permission;
mod readdir;
mod xattr;

use core::time::Duration;

use spin::Once;

pub(super) use self::{
    dir::whiteout::WhiteoutCache,
    identity::{IdentityPolicy, collect_layer_devs},
    inode_cache::InodeCache,
};
use self::{
    identity::ObjectId,
    lookup::{Lookup, NegativeLookup, is_opaque_directory, is_whiteout_inode},
    permission::AccessType,
    readdir::ReaddirIndex,
};
use crate::{
    fs::{
        file::{AccessMode, InodeMode, InodeType, PerOpenFileOps, Permission, StatusFlags},
        fs_impls::overlayfs::{
            fs::OverlayFs,
            inode::copyup::CopyUpTransition,
            layer::RealObjectStack,
            real::{RealObject, RealObjectKey},
        },
        utils::DirentVisitor,
        vfs::{
            file_system::FileSystem,
            inode::{
                Extension, FallocMode, FileOps, Inode, Metadata, MknodType, RenameMode,
                SymbolicLink,
            },
            path::Path,
            xattr::{XattrName, XattrNamespace, XattrSetFlags},
        },
    },
    prelude::*,
    process::{Gid, Uid},
    vm::page_cache::Vmo,
};

/// The logical overlay inode exposed to the VFS: one logical overlay object
/// shared by every name bound to it, with an immutable lower stack and an
/// upper object published at most once by copy-up.
pub(super) struct OverlayInode {
    /// The owning mount.
    fs: Weak<OverlayFs>,
    /// The immutable lower real-object stack, topmost first.
    lowers: Vec<RealObject>,
    /// The upper real object; unset until copy-up publishes one.
    upper: Once<RealObject>,
    /// The per-inode transaction lock. Directories carry their merged-readdir
    /// index in the payload; non-directories use the lock as a pure
    /// serialization token with `None`.
    lock: Mutex<Option<ReaddirIndex>>,
    /// The precomputed projected `st_dev`/`st_ino`.
    object_id: ObjectId,
    /// The VFS inode extension groups (fs event publisher / fs lock context).
    extension: Extension,
    /// The copy-up transition coordinate; `publication_parent`/`name` are
    /// `None` until the first positive-binding publication records them.
    copyup_transition: Mutex<CopyUpTransition>,
}

impl OverlayInode {
    /// Constructs the overlay mount root inode on demand.
    ///
    /// The root facts merge the upper root with all lower roots; the root is
    /// always a directory. Construction is delegated to the shared
    /// [`OverlayFs::project_inode`] path so the root uses the same inode-cache
    /// and identity projection as every other logical object.
    pub(super) fn new_root(fs: Weak<OverlayFs>) -> Arc<dyn Inode> {
        let fs = match fs.upgrade() {
            Some(fs) => fs,
            None => unreachable!(
                "the root inode is constructed only through a live mount Arc; \
                 the mount reference is always alive at this call site"
            ),
        };
        let layer_stack = &fs.layer_stack();
        let upper = layer_stack.upper_layer().ok().map(|layer| {
            let root_path = layer
                .root_path
                .upgrade()
                .expect("the pinned layer root path must stay alive for the mount lifetime");
            RealObject::from_layer_path(0, &root_path, layer.fsid, layer.container_dev_id)
        });
        let lowers: Vec<_> = layer_stack
            .lower_layers()
            .iter()
            .enumerate()
            .map(|(layer_index, layer)| {
                let root_path = layer
                    .root_path
                    .upgrade()
                    .expect("the pinned layer root path must stay alive for the mount lifetime");
                RealObject::from_layer_path(
                    layer_index + 1,
                    &root_path,
                    layer.fsid,
                    layer.container_dev_id,
                )
            })
            .collect();
        fs.project_inode(&RealObjectStack::new(upper, lowers))
    }

    /// Returns the inode-cache key derived from the current visible source.
    fn key(&self) -> RealObjectKey {
        RealObjectKey::from_source(self.visible_source())
    }

    /// Returns the current visible real-object source.
    fn visible_source(&self) -> &RealObject {
        self.upper.get().unwrap_or_else(|| {
            self.lowers
                .first()
                .expect("a real-object stack is never empty")
        })
    }

    /// Returns whether `real_inode` is this object's visible source or one of
    /// its retained lowers.
    fn contains_real_inode(&self, real_inode: &Arc<dyn Inode>) -> bool {
        Arc::ptr_eq(self.visible_source().real_inode(), real_inode)
            || self
                .lowers
                .iter()
                .any(|lower| Arc::ptr_eq(lower.real_inode(), real_inode))
    }

    /// Materializes the current real-object stack as an owned value.
    fn real_object_stack(&self) -> RealObjectStack {
        RealObjectStack::new(self.upper.get().cloned(), self.lowers.clone())
    }

    /// Returns the precomputed projected `st_dev`/`st_ino`.
    ///
    /// Copy-up re-projection keeps the lower-id-derived identity, so the
    /// value is stable across copy-up (authority-continuity invariant).
    fn object_id(&self) -> ObjectId {
        self.object_id
    }

    fn select_real_inode(&self) -> Arc<dyn Inode> {
        self.visible_source().real_inode().clone()
    }

    /// Returns the dentry-anchored path of the promoted upper real parent
    /// directory.
    ///
    /// After promotion the object is guaranteed to have an upper object that
    /// is always dentry-anchored, so the checked `real_path()` accessor
    /// succeeds; `EROFS`/`EIO` propagate when that guarantee does not hold.
    fn upper_parent_path(&self) -> Result<Path> {
        let upper = self.upper.get().ok_or_else(|| {
            Error::with_message(Errno::EROFS, "the overlay object has no upper real parent")
        })?;
        upper.real_path()
    }

    /// Returns `Err` when the inode does not belong to an overlay filesystem.
    fn fs_arc(&self) -> Result<Arc<OverlayFs>> {
        let fs = self.fs();
        Arc::downcast::<OverlayFs>(fs).map_err(|_| {
            Error::with_message(
                Errno::EIO,
                "the inode does not belong to an overlay filesystem",
            )
        })
    }

    /// Locks the per-object copy-up coordination state.
    fn lock_copyup_transition(&self) -> MutexGuard<'_, CopyUpTransition> {
        self.copyup_transition.lock()
    }

    /// Attempts to lock the per-object copy-up coordination state without
    /// blocking; `None` when another coordinator holds the lock.
    fn try_lock_copyup_transition(&self) -> Option<MutexGuard<'_, CopyUpTransition>> {
        self.copyup_transition.try_lock()
    }

    /// Serializes an `O_APPEND` write as one atomic size-read + write.
    ///
    /// The per-inode `lock` is held across both steps because the underlying
    /// fs does not process `O_APPEND` itself. This serializes concurrent
    /// appends on the post-write size.
    fn append_write(
        &self,
        reader: &mut VmReader,
        status_flags: StatusFlags,
    ) -> Result<usize> {
        let _guard = self.lock.lock();
        let real = self.select_real_inode();
        let offset = real.size();
        real.write_at(offset, reader, status_flags)
    }

    /// Publishes the upper real object of this inode — the copy-up
    /// transition.
    ///
    /// The fallible inode-cache alias runs before the `Once` publication, so
    /// a displacement fails rather than silently orphaning the inode. `lowers`
    /// are immutable across copy-up.
    fn replace_facts(
        self: &Arc<Self>,
        new_upper: RealObject,
        new_visible_source: &RealObject,
    ) -> Result<()> {
        let new_key = RealObjectKey::from_source(new_visible_source);
        let old_key = self.key();
        let old_real_inode = self.visible_source().real_inode().clone();
        let Some(fs) = self.fs.upgrade() else {
            // Teardown arm: no live lookup can observe this inode, so only
            // publish the local upper object.
            self.upper.call_once(|| new_upper);
            return Ok(());
        };
        // The fallible alias runs first; only after it succeeds is the
        // upper object published.
        fs.inodes()
            .rekey_keep_old_alias(old_key, new_key, old_real_inode, new_visible_source)?;
        self.upper.call_once(|| new_upper);
        debug_assert!(
            fs.inodes()
                .get(new_key)
                .is_some_and(|probe| Arc::ptr_eq(&probe, self)),
            "after replace_facts the inode cache maps the new visible-source key to THIS inode"
        );
        Ok(())
    }

    /// Runs `operation_fn` directly against the current real authority.
    ///
    /// Precondition: the permission stage has already admitted the operation
    /// (or the entry is a pure read delegation).
    fn delegate_to_real<T>(
        &self,
        operation_fn: impl FnOnce(&Arc<dyn Inode>) -> Result<T>,
    ) -> Result<T> {
        let real = self.select_real_inode();
        operation_fn(&real)
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
        self.visible_source().real_inode().size()
    }

    fn metadata(&self) -> Result<Metadata> {
        let mut metadata = self.visible_source().real_inode().metadata()?;
        metadata.ino = self.object_id.ino;
        metadata.container_dev_id = self.object_id.dev;
        Ok(metadata)
    }

    fn ino(&self) -> u64 {
        self.object_id.ino
    }

    fn type_(&self) -> InodeType {
        self.visible_source().real_inode().type_()
    }

    fn mode(&self) -> Result<InodeMode> {
        self.visible_source().real_inode().mode()
    }

    fn owner(&self) -> Result<Uid> {
        self.visible_source().real_inode().owner()
    }

    fn group(&self) -> Result<Gid> {
        self.visible_source().real_inode().group()
    }

    fn atime(&self) -> Duration {
        self.visible_source().real_inode().atime()
    }

    fn mtime(&self) -> Duration {
        self.visible_source().real_inode().mtime()
    }

    fn ctime(&self) -> Duration {
        self.visible_source().real_inode().ctime()
    }

    fn lookup(&self, name: &str) -> Result<Arc<dyn Inode>> {
        let _dir_guard = self.lock.lock();
        if _dir_guard.is_none() {
            return Err(Error::with_message(
                Errno::ENOTDIR,
                "lookup is supported on overlay directories only",
            ));
        }
        let fs = self.fs.upgrade().ok_or_else(|| {
            Error::with_message(Errno::EIO, "the overlay mount is no longer alive")
        })?;
        match fs.lookup(self, name)? {
            Lookup::Positive(inode) => Ok(inode),
            Lookup::Negative(_) => Err(Error::new(Errno::ENOENT)),
        }
    }

    fn fs(&self) -> Arc<dyn FileSystem> {
        match self.fs.upgrade() {
            Some(fs) => fs,
            None => unreachable!("a live OverlayInode keeps its OverlayFs alive"),
        }
    }

    fn extension(&self) -> &Extension {
        &self.extension
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
