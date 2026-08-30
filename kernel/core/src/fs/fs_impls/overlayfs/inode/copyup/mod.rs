// SPDX-License-Identifier: MPL-2.0

#![short_vis_path::add(overlayfs)]
//! The copy-up authority: per-object coordination, winner/waiter trigger,
//! and object-kind promotion.
//!
//! [`OverlayInode::copy_up`] is the single promotion entry. Each lower-backed
//! object carries its pending publication coordinate from projection time;
//! the entry promotes ancestors before the child and serializes winners
//! through the `copyup` mutex.
//!
//! ## Locking
//!
//! `InodeCache` is a leaf lock. `publish_rekey` runs under `entries.write()`
//! alone: it never waits, never acquires another overlay lock, and never
//! calls back into projection or copy-up. Within one copy-up frame the locks
//! nest strictly as `CUL(self) -> DIR(publication parent) ->
//! InodeCache.entries.write()`: the coordinate read acquires and releases
//! `CUL` together with the `PARENT` read guard before the ancestor walk (no
//! `CUL(child) -> CUL(parent)` edge exists), the winner then holds `CUL`
//! continuously from re-acquisition through the commit tail and the
//! coordinate retirement, `publish_by_rename` takes the parent `DIR` lock
//! inside that `CUL` hold, and the cache write is the innermost leaf. No
//! lock is released and reacquired inside the cache guard, and no new lock
//! domain or lock edge is introduced. The pre-existing orders `CUL -> DIR`,
//! `DIR -> PARENT`, and `CUL -> PARENT` are unchanged.
//!
//! ## References
//!
//! - Linux `ovl_real_file_path` follow-copy-up:
//!   <https://elixir.bootlin.com/linux/latest/source/fs/overlayfs/file.c#L128-L171>
//! - Linux `ovl_set_attr` (symlink mode skip):
//!   <https://elixir.bootlin.com/linux/latest/source/fs/overlayfs/copy_up.c#L392-L416>

use core::cmp::min;

use self::workdir::{WorkdirTemp, WorkdirTempRequest};
use crate::{
    fs::{
        file::{InodeType, StatusFlags},
        fs_impls::overlayfs::{
            fs::OverlayFs,
            inode::{Lookup, OverlayInode, xattr::XattrCopyPolicy},
            real::RealObject,
        },
        vfs::inode::{Inode, MknodType, RenameMode, SymbolicLink},
    },
    prelude::*,
};

pub(super) mod workdir;

/// The maximum depth of the copy-up ancestor recursion.
///
/// Each frame keeps only two live `Arc`s and no guard, so 1024 frames fit
/// within the default kernel task stack; a deeper chain fails closed with
/// `ELOOP` instead of risking a stack overflow.
const MAX_COPYUP_DEPTH: usize = 1024;

/// The chunk size of the regular-file data stream during copy-up.
///
/// The lower file is streamed through one reused kernel buffer; the chunk
/// bounds each `read_at`/`write_at` pair so a short read still makes bounded
/// progress.
const COPY_CHUNK_SIZE: usize = 64 * 1024;

impl OverlayInode {
    /// Promotes this logical object to upper authority, winning or waiting on
    /// the per-object `copyup` mutex.
    ///
    /// Returns `Ok(())` once the object is upper-backed (idempotent fast path,
    /// waiter leg, this task's own completed promotion, or no pending
    /// coordinate), and propagates failures unchanged: a dead publication parent
    /// fails closed with `EIO`, an inconsistent retired coordinate under the
    /// winner guard with `EIO`, and an ancestor chain deeper than
    /// [`MAX_COPYUP_DEPTH`] with `Errno::ELOOP`. Every failure is pre-rename.
    pub(super) fn copy_up(&self) -> Result<()> {
        self.copy_up_inner(0)
    }

    /// The recursive body of [`Self::copy_up`].
    ///
    /// `depth` counts the ancestor recursions already performed and carries
    /// the inherited stack-safety invariant: a publication-parent chain
    /// deeper than [`MAX_COPYUP_DEPTH`] fails closed with `ELOOP`. Each frame
    /// keeps only the coordinate-read parent alive across the recursion; no
    /// lock guard is carried in or out.
    fn copy_up_inner(&self, depth: usize) -> Result<()> {
        // Pins the owning mount for the trigger's duration.
        let fs = self.fs_arc()?;

        // Idempotent fast path: the object is already upper-backed.
        if self.upper.get().is_some() {
            return Ok(());
        }

        // Coordinate read (`CUL -> PARENT`): upgrade the publication parent
        // to a strong reference under the `copyup` mutex, then release both
        // guards before the recursive ancestor walk — no
        // `CUL(child) -> CUL(parent)` edge exists. A dead weak reference
        // fails closed; the parent is never resurrected.
        let (publication_parent, name) = {
            let published = self.lock_copyup();
            let Some(name) = published.as_ref() else {
                // No pending coordinate: the object was already published (or
                // the mount root never carries one). Every mutating entry is
                // EROFS-gated upstream, so this read-only exit is sound.
                return Ok(());
            };
            let Some(parent) = self.recorded_parent.read().upgrade() else {
                return Err(Error::with_message(
                    Errno::EIO,
                    "the copy-up publication parent no longer exists",
                ));
            };
            (parent, name.clone())
        };

        if depth >= MAX_COPYUP_DEPTH {
            return_errno_with_message!(
                Errno::ELOOP,
                "the copy-up ancestor chain exceeds the depth limit"
            );
        }
        publication_parent.copy_up_inner(depth + 1)?;

        // Winner/waiter serialization: the sleep-capable `copyup` lock wait.
        let mut published = self.lock_copyup();

        // Waiter leg: another task completed the promotion while this task
        // waited; re-observe upper authority and return the same `Ok(())`.
        if self.upper.get().is_some() {
            return Ok(());
        }

        // Defensive fail-closed: a retired coordinate with `upper` unset is
        // structurally unreachable — retirement only follows publication,
        // under this same guard.
        if published.is_none() {
            return Err(Error::with_message(
                Errno::EIO,
                "the overlay copy-up coordinate turned inconsistent during arbitration",
            ));
        }

        // Impurity marker: every promoted object makes its publication parent
        // impure — persist the marker before the object-kind dispatch and the
        // physical upper commit (strict, pre-commit; read-first idempotence
        // makes an already-marked parent a no-op).
        OverlayInode::set_impure_marker(
            &publication_parent.select_real_inode(),
            fs.policy().xattr_prefix(),
        )?;

        let staged = self.stage_in_workdir(&name)?;
        self.publish_by_rename(&publication_parent, &name, staged)?;
        // Coordinate retirement through the still-held winner guard. The
        // upper-set/coordinate-pending window is unobservable to other
        // `copyup` observers because every observer holds the guard here.
        *published = None;
        Ok(())
    }

    /// Acquires the per-object copy-up mutex; the guard exposes the pending
    /// publication name (`Some`) or the retired coordinate (`None`).
    fn lock_copyup(&self) -> MutexGuard<'_, Option<String>> {
        self.copyup.lock()
    }

    /// Stages a fully prepared private workdir temp for the pending
    /// publication name.
    ///
    /// Called by the winner with the `copyup` guard held; the object is
    /// lower-backed, so [`Self::lower_source`] succeeds. The staging performs
    /// no namespace change: the workdir lives outside every layer root. Per
    /// kind, the lower metadata, the eligible xattrs, and the lower-id origin
    /// record (capability-gated) are written on the temp body before the
    /// timestamp replay (the origin write may refresh `ctime`), and a
    /// regular-file temp is synced before publication, so the record and the
    /// data are durable when the rename publishes the object. On any staging
    /// failure the temp is cleaned up and the error propagates — a failed
    /// staging leaves no residue, and the whole copy-up retries from scratch.
    fn stage_in_workdir(&self, name: &str) -> Result<WorkdirTemp> {
        let fs = self.fs_arc()?;
        let prefix = fs.policy().xattr_prefix();
        let lower = self.lower_source()?;
        match lower.real_inode().type_() {
            InodeType::Dir => {
                // Directory copy-up: private workdir temp, metadata/xattr
                // transfer, then atomic `RenameMode::Replace` publication so
                // a stale upper entry is replaced instead of failing `create`
                // with `EEXIST`. Only the directory object itself is copied;
                // its children remain lower-backed.
                let mode = lower.real_inode().mode()?;
                let temp = fs.create_workdir_temp(
                    name,
                    WorkdirTempRequest::Create {
                        kind: InodeType::Dir,
                        mode,
                    },
                )?;
                if let Err(err) = self
                    .transfer_metadata(lower.real_inode(), temp.inode())
                    .and_then(|_| {
                        OverlayInode::copy_eligible_xattrs(
                            lower.real_inode(),
                            temp.inode(),
                            XattrCopyPolicy::Strict,
                            prefix,
                        )
                    })
                    .and_then(|_| fs.store_lower_id(temp.inode(), &lower))
                    .and_then(|_| self.transfer_timestamps(lower.real_inode(), temp.inode()))
                {
                    let _ = fs.cleanup_workdir_temp(temp.name(), temp.kind());
                    return Err(err);
                }
                Ok(temp)
            }
            InodeType::File => {
                // File copy-up: data is streamed into the workdir temp and
                // synced before the atomic `RenameMode::Replace` publication;
                // durability is guaranteed by `sync_all` preceding the rename.
                let mode = lower.real_inode().mode()?;
                let temp = fs.create_workdir_temp(
                    name,
                    WorkdirTempRequest::Create {
                        kind: InodeType::File,
                        mode,
                    },
                )?;
                if let Err(err) = self
                    .transfer_metadata(lower.real_inode(), temp.inode())
                    .and_then(|_| {
                        OverlayInode::copy_eligible_xattrs(
                            lower.real_inode(),
                            temp.inode(),
                            XattrCopyPolicy::Strict,
                            prefix,
                        )
                    })
                    .and_then(|_| self.promote_regular_file(temp.inode()))
                    .and_then(|_| fs.store_lower_id(temp.inode(), &lower))
                    .and_then(|_| self.transfer_timestamps(lower.real_inode(), temp.inode()))
                    .and_then(|_| temp.inode().sync_all())
                {
                    let _ = fs.cleanup_workdir_temp(temp.name(), temp.kind());
                    return Err(err);
                }
                Ok(temp)
            }
            InodeType::SymLink => {
                // Symlink promotion side: a workdir symlink temp recreated
                // from the lower target, then metadata/xattr transfer and the
                // atomic rename (the symlink object itself is copied; its
                // target is left unreferenced).
                let mode = lower.real_inode().mode()?;
                let temp = fs.create_workdir_temp(
                    name,
                    WorkdirTempRequest::Create {
                        kind: InodeType::SymLink,
                        mode,
                    },
                )?;
                if let Err(err) = self
                    .promote_symlink(temp.inode())
                    .and_then(|_| self.transfer_metadata(lower.real_inode(), temp.inode()))
                    .and_then(|_| {
                        OverlayInode::copy_eligible_xattrs(
                            lower.real_inode(),
                            temp.inode(),
                            XattrCopyPolicy::Strict,
                            prefix,
                        )
                    })
                    .and_then(|_| fs.store_lower_id(temp.inode(), &lower))
                    .and_then(|_| self.transfer_timestamps(lower.real_inode(), temp.inode()))
                {
                    let _ = fs.cleanup_workdir_temp(temp.name(), temp.kind());
                    return Err(err);
                }
                Ok(temp)
            }
            InodeType::CharDevice
            | InodeType::BlockDevice
            | InodeType::NamedPipe
            | InodeType::Socket => {
                // A socket node cannot be recreated through the stable
                // `Inode::mknod` surface (`MknodType` has no socket variant),
                // so it is rejected before any side effect.
                let mknod_type = match lower.real_inode().type_() {
                    InodeType::NamedPipe => MknodType::NamedPipe,
                    InodeType::CharDevice => {
                        let rdev = lower
                            .real_inode()
                            .metadata()?
                            .self_dev_id
                            .ok_or_else(|| {
                                Error::with_message(
                                    Errno::EINVAL,
                                    "the lower char device has no device id",
                                )
                            })?
                            .as_encoded_u64();
                        MknodType::CharDevice(rdev)
                    }
                    InodeType::BlockDevice => {
                        let rdev = lower
                            .real_inode()
                            .metadata()?
                            .self_dev_id
                            .ok_or_else(|| {
                                Error::with_message(
                                    Errno::EINVAL,
                                    "the lower block device has no device id",
                                )
                            })?
                            .as_encoded_u64();
                        MknodType::BlockDevice(rdev)
                    }
                    _ => {
                        return Err(Error::with_message(
                            Errno::EOPNOTSUPP,
                            "socket nodes cannot be copied up",
                        ));
                    }
                };
                let mode = lower.real_inode().mode()?;
                let temp = fs.create_workdir_temp(
                    name,
                    WorkdirTempRequest::Mknod {
                        mode,
                        node: &mknod_type,
                    },
                )?;
                if let Err(err) = self
                    .transfer_metadata(lower.real_inode(), temp.inode())
                    .and_then(|_| {
                        OverlayInode::copy_eligible_xattrs(
                            lower.real_inode(),
                            temp.inode(),
                            XattrCopyPolicy::Strict,
                            prefix,
                        )
                    })
                    .and_then(|_| fs.store_lower_id(temp.inode(), &lower))
                    .and_then(|_| self.transfer_timestamps(lower.real_inode(), temp.inode()))
                {
                    let _ = fs.cleanup_workdir_temp(temp.name(), temp.kind());
                    return Err(err);
                }
                Ok(temp)
            }
            InodeType::Unknown => Err(Error::with_message(
                Errno::EINVAL,
                "cannot promote an overlay object of unknown type",
            )),
        }
    }

    /// Commits the staged promotion: the physical rename and the semantic
    /// facts publication.
    ///
    /// Called by the winner with the `copyup` guard held and no other overlay
    /// lock. `publication_parent` is the coordinate-read parent — upper-backed
    /// after the ancestor walk — passed explicitly rather than re-derived from
    /// `recorded_parent`: a concurrent cross-parent rename may repoint
    /// `recorded_parent` before the DIR lock is taken, and the commit must
    /// target the parent the ancestor walk promoted and the temp was staged
    /// for (the first-bound coordinate rule).
    ///
    /// The body acquires `publication_parent.lock` (the sole `CUL -> DIR`
    /// edge in the overlayfs lock graph; every entry must finish copy-up
    /// promotion before taking any directory transaction lock), runs the
    /// widened liveness recheck, pins the committing instance, performs the
    /// physical rename, and closes with the infallible facts publication; the
    /// DIR guard is released on scope exit. Failures are pre-rename only: the
    /// staged temp is cleaned up and the whole copy-up retries from scratch.
    fn publish_by_rename(
        &self,
        publication_parent: &Arc<OverlayInode>,
        name: &str,
        staged: WorkdirTemp,
    ) -> Result<()> {
        let fs = self.fs_arc()?;
        // Pre-rename resolution: computed once before the DIR scope so the
        // post-rename segment contains no fallible step.
        let upper_dir_path = publication_parent.upper_parent_path()?;
        let upper_layer = fs.layer_stack().upper_layer()?;
        let result: Result<()> = (|| {
            let _publication_guard = publication_parent.lock.lock();
            // Liveness recheck under `publication_parent.lock`: the freshly
            // looked-up child must still denote the same logical publication
            // target. Negative lookups and positive-but-unrelated objects (a
            // fresh recreate after unlink) abort before the rename would
            // resurrect the name.
            let current = match fs.lookup(publication_parent, name)? {
                Lookup::Positive(current) if self.is_same_publication_target(&current, &fs) => {
                    current
                }
                _ => {
                    return Err(Error::with_message(
                        Errno::ENOENT,
                        "the copy-up target name is no longer visible",
                    ));
                }
            };
            // Strong pin of the committing instance, taken before the rename.
            let pin = self.commit_pin(&current)?;
            // The physical rename — the copy-up boundary. `publish_temp`
            // returns the published object's own dentry-anchored path.
            let published_path =
                fs.publish_temp(&staged, &upper_dir_path, name, RenameMode::Replace)?;
            // The upper real object is derived from the published path — no
            // re-resolution, no failure class after the rename. The origin
            // record ran on the temp in staging, so it travels with the
            // published object.
            let upper_real = upper_layer.child_real_object(&published_path);
            self.replace_facts(upper_real, &pin);
            Ok(())
        })();
        if let Err(err) = result {
            // Pre-rename failure: delete the temp; a retry starts from
            // scratch. Best-effort, kind-aware.
            let _ = fs.cleanup_workdir_temp(staged.name(), staged.kind());
            return Err(err);
        }
        Ok(())
    }

    /// Pins the committing instance for the registration step.
    ///
    /// When the recheck hit is this instance's canonical `Arc`, the hit is
    /// the pin. On the sibling arms (the coordinate resolves to another
    /// instance of the same logical object) the pin is resolved through this
    /// instance's visible-source cache key and verified to denote `self`
    /// before it is returned, so the narrow double-mint window where the
    /// cache slot holds a sibling fails closed instead of misregistering.
    /// Pre-rename clean-fallible: both `EIO` arms are the sanctioned
    /// retry-from-scratch class.
    fn commit_pin(&self, current: &Arc<OverlayInode>) -> Result<Arc<OverlayInode>> {
        if core::ptr::addr_eq(Arc::as_ptr(current), self) {
            return Ok(current.clone());
        }
        let pin = self.cached_self_arc()?;
        if !core::ptr::addr_eq(Arc::as_ptr(&pin), self) {
            return Err(Error::with_message(
                Errno::EIO,
                "this instance is no longer the cache-resident identity for its key",
            ));
        }
        Ok(pin)
    }

    /// Judges whether the freshly looked-up child at the publication
    /// coordinate denotes the same logical publication target as this
    /// committing instance.
    ///
    /// Three arms:
    /// 1. the coordinate resolves to this instance;
    /// 2. both instances are lower-backed aliases of the same topmost lower
    ///    real object (an alias split of one logical object);
    /// 3. the looked-up instance is upper-backed and its persisted origin
    ///    record resolves against this instance's retained lower stack — the
    ///    coordinate holds a sibling's upper copy of the same lower object.
    ///    Mounts without the private-xattr capability carry no record, so
    ///    this arm is undetectable there (documented gap).
    ///
    /// An absent record (`Ok(None)`) and a genuine origin-record read error
    /// both fail closed to `false` — a pre-rename clean abort.
    fn is_same_publication_target(&self, current: &Arc<OverlayInode>, fs: &Arc<OverlayFs>) -> bool {
        if core::ptr::addr_eq(Arc::as_ptr(current), self) {
            return true;
        }
        if current.upper.get().is_none()
            && self.upper.get().is_none()
            && let (Some(current_lower), Some(self_lower)) =
                (current.lowers.first(), self.lowers.first())
            && Arc::ptr_eq(current_lower.real_inode(), self_lower.real_inode())
        {
            return true;
        }
        if current.upper.get().is_some() {
            let Ok(Some(record)) = fs.read_lower_id(current.visible_source().real_inode()) else {
                return false;
            };
            return fs.origin_real_ino_resolves(&record, &self.real_object_stack());
        }
        false
    }

    /// Streams the lower regular file's data into the workdir temp.
    ///
    /// The stream runs `read_at`/`write_at` pairs over one reused buffer.
    /// Short reads advance by the read length; a zero-length read before the
    /// declared size or a short write is surfaced as `EIO` — a partial
    /// transfer is never treated as short successful I/O.
    fn promote_regular_file(&self, temp: &Arc<dyn Inode>) -> Result<()> {
        let lower = self.lower_source()?;
        let size = lower.real_inode().size();
        let mut offset = 0usize;
        let mut buffer = vec![0u8; COPY_CHUNK_SIZE];
        while offset < size {
            let chunk = min(COPY_CHUNK_SIZE, size - offset);
            let mut writer = VmWriter::from(&mut buffer[..chunk]).to_fallible();
            let read_len = lower
                .real_inode()
                .read_at(offset, &mut writer, StatusFlags::empty())?;
            if read_len == 0 {
                return_errno_with_message!(
                    Errno::EIO,
                    "the lower source returned a zero-length read before its declared size"
                );
            }
            let mut reader = VmReader::from(&buffer[..read_len]).to_fallible();
            let write_len = temp.write_at(offset, &mut reader, StatusFlags::empty())?;
            if write_len != read_len {
                return_errno_with_message!(
                    Errno::EIO,
                    "the workdir temp accepted a short write during copy-up"
                );
            }
            offset += write_len;
        }
        Ok(())
    }

    /// Recreates the lower symlink target on the workdir temp.
    ///
    /// A `SymbolicLink::Path` target cannot be recreated through the stable
    /// VFS surface and is rejected with `EOPNOTSUPP`.
    fn promote_symlink(&self, temp: &Arc<dyn Inode>) -> Result<()> {
        let lower = self.lower_source()?;
        let target = match lower.real_inode().read_link()? {
            SymbolicLink::Plain(target) => target,
            SymbolicLink::Path(_) => {
                return_errno_with_message!(
                    Errno::EOPNOTSUPP,
                    "a path-style symlink target cannot be copied up"
                );
            }
        };
        temp.write_link(&target)
    }

    /// Transfers the lower metadata onto the upper object: owner, group,
    /// mode, and — for regular files — size.
    ///
    /// The mode transfer skips symlinks because the backing filesystems treat
    /// a symlink `set_mode` as a no-op or reject it, so the copy-up does not
    /// depend on that per-fs behavior.
    pub(super) fn transfer_metadata(
        &self,
        source: &Arc<dyn Inode>,
        temp: &Arc<dyn Inode>,
    ) -> Result<()> {
        temp.set_owner(source.owner()?)?;
        temp.set_group(source.group()?)?;
        if !matches!(source.type_(), InodeType::SymLink) {
            temp.set_mode(source.mode()?)?;
        }
        if source.type_().is_regular_file() {
            temp.resize(source.size())?;
        }
        Ok(())
    }

    /// Replays the lower timestamps (`atime`/`mtime`/`ctime`) onto the upper
    /// object.
    ///
    /// Split out of [`Self::transfer_metadata`] so the copy-up
    /// preserves the lower timestamps instead of publishing the copy-up
    /// instant; it runs last, after every data/metadata/xattr step that
    /// could refresh `mtime`/`ctime`.
    pub(super) fn transfer_timestamps(
        &self,
        source: &Arc<dyn Inode>,
        temp: &Arc<dyn Inode>,
    ) -> Result<()> {
        temp.set_atime(source.atime());
        temp.set_mtime(source.mtime());
        temp.set_ctime(source.ctime());
        Ok(())
    }

    /// Returns the topmost lower real object of this object (`lowers[0]`).
    ///
    /// Safe by the real-object invariant `upper.is_some() || !lowers.is_empty()`;
    /// the checked access surfaces a structural violation as `EIO`.
    fn lower_source(&self) -> Result<RealObject> {
        self.lowers.first().cloned().ok_or_else(|| {
            Error::with_message(
                Errno::EIO,
                "a lower-backed overlay object has no lower source",
            )
        })
    }
}
