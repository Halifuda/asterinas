// SPDX-License-Identifier: MPL-2.0

#![short_vis_path::add(overlayfs)]
//! The copy-up authority: per-object coordination, winner/waiter trigger,
//! and object-kind promotion.
//!
//! [`OverlayInode::ensure_upper_authority`] is the single promotion entry.
//! The per-object [`CopyUpTransition`] coordinate is recorded once at the
//! first positive lookup; the trigger promotes ancestors before the child
//! and serializes winners through `copyup_transition`.
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
            inode::{
                Lookup, OverlayInode,
                xattr::{XattrCopyPolicy, copy_eligible_xattrs, set_impure_marker},
            },
            real::RealObject,
        },
        vfs::{
            inode::{Inode, MknodType, RenameMode, SymbolicLink},
            path::Path,
        },
    },
    prelude::*,
};

pub(super) mod workdir;

/// The copy-up publication coordinate and phase of one logical overlay object.
///
/// Recorded exactly once at the first positive lookup; its coordinate fields
/// are fixed and only `phase` can transition thereafter;
/// the upper authority in the real-object state is the durable outcome, so no
/// copy-up-completed history marker exists. The publication-parent chain is
/// acyclic and root-terminated, so the trigger's top-down ancestor walk
/// terminates and never re-enters the same instance.
pub(super) struct CopyUpTransition {
    /// The logical parent overlay inode; `None` until the first positive
    /// lookup records it. Its upper existence is resolved by the trigger's
    /// ancestor walk, which checks the parent's upper existence and may
    /// promote it first.
    publication_parent: Option<Arc<OverlayInode>>,
    name: Option<String>,
    phase: CopyUpPhase,
}

impl CopyUpTransition {
    /// Constructs an unrecorded transition coordinate.
    pub(super) fn new() -> Self {
        Self {
            publication_parent: None,
            name: None,
            phase: CopyUpPhase::Idle,
        }
    }
}

/// The transition marker of one copy-up coordination.
///
/// Maps the copy-up phase values to their semantic states:
/// - lower-authoritative: the real-object stack has no upper and
///   [`CopyUpPhase::Idle`].
/// - promotion-in-progress: the `copyup_transition` guard is held by
///   the winner (observable only as mutex contention).
/// - upper-authoritative: the real-object stack has an upper.
/// - retryable failure: the error is returned to the caller (authority
///   unchanged, no durable marker needed).
/// - reconcile-required: [`CopyUpPhase::ReconcilePending`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CopyUpPhase {
    /// The coordinate carries no unfinished transition; a lower authority (if
    /// any) is clean.
    Idle,
    /// Physical publication happened but semantic publication failed; the
    /// upper object at `(publication_parent, name)` must be verified before
    /// reuse.
    ReconcilePending,
}

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
    /// Records the copy-up transition coordinate at the first positive
    /// lookup.
    ///
    /// The coordinate is set once — the first positive lookup wins; the
    /// non-blocking `try_lock` skips when contended because a transition
    /// already running has already set it.
    pub(super) fn try_record_copyup_transition(
        &self,
        publication_parent: Arc<OverlayInode>,
        name: &str,
    ) {
        let Some(mut guard) = self.try_lock_copyup_transition() else {
            return;
        };
        if guard.publication_parent.is_some() {
            return;
        }
        guard.publication_parent = Some(publication_parent);
        guard.name = Some(String::from(name));
        guard.phase = CopyUpPhase::Idle;
    }

    /// Promotes this logical object to upper authority, winning or waiting on
    /// the per-object copy-up coordination guard (`copyup_transition`).
    ///
    /// Returns `Ok(())` once the object is upper-backed (idempotent fast
    /// path, waiter leg, or this task's own completed promotion), `Err` when
    /// no publication coordinate is recorded, and propagates any underlying
    /// recipe failure unchanged. A deeper ancestor chain than
    /// [`MAX_COPYUP_DEPTH`] fails closed with `Errno::ELOOP`.
    pub(super) fn ensure_upper_authority(&self) -> Result<()> {
        self.ensure_upper_authority_inner(0)
    }

    /// The recursive body of [`OverlayInode::ensure_upper_authority`]; `depth`
    /// is the number of ancestor recursions already performed (0 at the entry,
    /// incremented once per consecutive lower-backed publication parent).
    fn ensure_upper_authority_inner(&self, depth: usize) -> Result<()> {
        // Pins the owning mount for the trigger's duration.
        let _fs = self.fs_arc()?;

        if self.upper.get().is_some() {
            return Ok(());
        }

        // Publication coordinate: the brief
        // `copyup_transition` read clones the logical parent and name so the
        // guard is released before the recursive ancestor walk; both are
        // fixed once the coordinate is recorded, so the winner body reuses
        // this single coordinate.
        let (publication_parent, name) = {
            let transition = self.lock_copyup_transition();
            let Some(publication_parent) = transition.publication_parent.as_ref().cloned() else {
                return Err(Error::with_message(
                    Errno::ENOENT,
                    "the overlay object has no recorded copy-up publication coordinate",
                ));
            };
            let Some(name) = transition.name.as_ref().cloned() else {
                return Err(Error::with_message(
                    Errno::ENOENT,
                    "the overlay object has no recorded copy-up publication coordinate",
                ));
            };
            (publication_parent, name)
        };

        if depth >= MAX_COPYUP_DEPTH {
            return_errno_with_message!(
                Errno::ELOOP,
                "the copy-up ancestor chain exceeds the depth limit"
            );
        }
        publication_parent.ensure_upper_authority_inner(depth + 1)?;

        // Winner/waiter serialization: the sleep-capable
        // `copyup_transition` lock wait.
        let mut transition = self.lock_copyup_transition();

        // Re-check under the guard: another task won and promoted while this
        // task waited; re-observe upper authority and return the same
        // `Ok(())` success value (waiter path).
        if self.upper.get().is_some() {
            return Ok(());
        }

        // Winner body:
        let coordinate = &mut *transition;
        self.promote(&publication_parent, &name, coordinate)?;
        Ok(())
    }

    /// Runs the winner promotion body for this object.
    ///
    /// Called by the trigger with the `copyup_transition` arbitration guard
    /// held. The object kind is dispatched internally; success commits the
    /// phase to [`CopyUpPhase::Idle`], and recipe-arm failures classify as
    /// cleanup-before-publication vs `ReconcilePending`.
    fn promote(
        &self,
        publication_parent: &Arc<OverlayInode>,
        name: &str,
        coordinate: &mut CopyUpTransition,
    ) -> Result<()> {
        // 1) Idempotent upper fast path: a waiter may have completed the
        //    transition while this task waited for the arbitration guard.
        if self.upper.get().is_some() {
            return Ok(());
        }

        // The publication parent's upper directory path is computed once and
        // shared by the ReconcilePending verification and the promotion body.
        let upper_dir_path = publication_parent.upper_parent_path()?;
        // 2) ReconcilePending verification (recovery): the parent was
        //    promoted by the ancestor walk, so its real object is the upper
        //    directory; the verify helper consumes the passed `name`.
        if coordinate.phase == CopyUpPhase::ReconcilePending {
            self.verify_upper_target(&upper_dir_path, name)?;
        }

        let upper_dir = publication_parent.select_real_inode();
        let fs = self.fs_arc()?;
        // Impurity marker: every promoted object makes its publication
        // parent impure — persist the marker before the object-kind dispatch
        // and the physical upper commit (strict, pre-commit; read-first
        // idempotence makes an already-marked parent a no-op).
        set_impure_marker(&upper_dir)?;
        let lower = self.lower_source()?;
        let result = match lower.real_inode().type_() {
            InodeType::Dir => {
                // Directory copy-up: private workdir temp, metadata/xattr
                // transfer, then atomic `RenameMode::Replace` publication so
                // a stale upper entry is replaced instead of failing `create`
                // with `EEXIST`. Only the directory object itself is copied; its
                // children remain lower-backed.
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
                        copy_eligible_xattrs(
                            lower.real_inode(),
                            temp.inode(),
                            XattrCopyPolicy::Strict,
                        )
                    })
                    .and_then(|_| self.transfer_timestamps(lower.real_inode(), temp.inode()))
                {
                    let _ = fs.cleanup_workdir_temp(temp.name(), temp.kind());
                    return Err(err);
                }
                self.finish_promotion(
                    coordinate,
                    publication_parent,
                    name,
                    &upper_dir_path,
                    lower.clone(),
                    &temp,
                )
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
                        copy_eligible_xattrs(
                            lower.real_inode(),
                            temp.inode(),
                            XattrCopyPolicy::Strict,
                        )
                    })
                    .and_then(|_| self.promote_regular_file(temp.inode()))
                    .and_then(|_| self.transfer_timestamps(lower.real_inode(), temp.inode()))
                    .and_then(|_| temp.inode().sync_all())
                {
                    let _ = fs.cleanup_workdir_temp(temp.name(), temp.kind());
                    return Err(err);
                }
                self.finish_promotion(
                    coordinate,
                    publication_parent,
                    name,
                    &upper_dir_path,
                    lower.clone(),
                    &temp,
                )
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
                        copy_eligible_xattrs(
                            lower.real_inode(),
                            temp.inode(),
                            XattrCopyPolicy::Strict,
                        )
                    })
                    .and_then(|_| self.transfer_timestamps(lower.real_inode(), temp.inode()))
                {
                    let _ = fs.cleanup_workdir_temp(temp.name(), temp.kind());
                    return Err(err);
                }
                self.finish_promotion(
                    coordinate,
                    publication_parent,
                    name,
                    &upper_dir_path,
                    lower.clone(),
                    &temp,
                )
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
                        copy_eligible_xattrs(
                            lower.real_inode(),
                            temp.inode(),
                            XattrCopyPolicy::Strict,
                        )
                    })
                    .and_then(|_| self.transfer_timestamps(lower.real_inode(), temp.inode()))
                {
                    let _ = fs.cleanup_workdir_temp(temp.name(), temp.kind());
                    return Err(err);
                }
                self.finish_promotion(
                    coordinate,
                    publication_parent,
                    name,
                    &upper_dir_path,
                    lower.clone(),
                    &temp,
                )
            }
            InodeType::Unknown => Err(Error::with_message(
                Errno::EINVAL,
                "cannot promote an overlay object of unknown type",
            )),
        };
        result?;
        coordinate.phase = CopyUpPhase::Idle;
        Ok(())
    }

    /// Completes a staged promotion after the per-kind preparation ran.
    ///
    /// The caller has already run the preparation and cleaned up on failure;
    /// this tail performs the physical publication. Failures before commit
    /// clean the workdir temp; failures after the physical rename mark the
    /// coordinate `ReconcilePending` for the next winner. Success leaves the
    /// caller to set the coordinate back to `Idle`.
    ///
    /// The physical rename and the semantic upper-authority commit run while
    /// holding `publication_parent.lock`. This is the single `CUL -> lock`
    /// edge in the overlayfs lock graph: every entry must finish copy-up
    /// promotion before taking any directory transaction lock.
    fn finish_promotion(
        &self,
        coordinate: &mut CopyUpTransition,
        publication_parent: &Arc<OverlayInode>,
        name: &str,
        upper_dir_path: &Path,
        lower: RealObject,
        temp: &WorkdirTemp,
    ) -> Result<()> {
        let fs = self.fs_arc()?;
        let committed;
        let publish_result = {
            let _publication_guard = publication_parent.lock.lock();
            // Liveness recheck under `publication_parent.lock`: if the name
            // was removed (or replaced) while this copy-up was being prepared,
            // abort before the rename would resurrect it.
            match fs.lookup(publication_parent, name)? {
                Lookup::Positive(current) if core::ptr::addr_eq(Arc::as_ptr(&current), self) => {}
                _ => {
                    return Err(Error::with_message(
                        Errno::ENOENT,
                        "the copy-up target name is no longer visible",
                    ));
                }
            }
            let rename_result = fs.publish_temp(temp, upper_dir_path, name, RenameMode::Replace);
            committed = rename_result.is_ok();
            rename_result.and_then(|_| {
                let upper_real = self.upper_real_object(upper_dir_path, name)?;
                self.publish_upper_authority(upper_real, lower)
            })
        };
        if let Err(err) = publish_result {
            if committed {
                Self::mark_reconcile_pending(coordinate);
            } else {
                let _ = fs.cleanup_workdir_temp(temp.name(), temp.kind());
            }
            return Err(err);
        }
        Ok(())
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
    /// Split out of [`OverlayInode::transfer_metadata`] so the copy-up
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

    /// Publishes the upper authority semantically.
    ///
    /// After publication the visible source of this object is the upper real
    /// object, with the lower-derived `object_id` kept (constant `st_ino`).
    /// Recording the lower id is capability-gated: `store_lower_id` returns
    /// `Ok(())` with no record when the mount has no `upper_capabilities` or
    /// when `can_store_private_xattr()` is false.
    fn publish_upper_authority(
        &self,
        upper_real: RealObject,
        lower_real: RealObject,
    ) -> Result<()> {
        let fs = self.fs_arc()?;
        fs.store_lower_id(upper_real.real_inode(), &lower_real)?;
        // `lowers` are immutable across copy-up, so the carrier only publishes
        // the newly created upper object.
        let carrier = fs.project_inode(&self.real_object_stack());
        carrier.replace_facts(upper_real.clone(), &upper_real)?;
        Ok(())
    }

    /// Verifies the upper entry at the publication coordinate before reuse
    /// (ReconcilePending path).
    ///
    /// Covers the upper entry's object type and basic mode metadata; a
    /// mismatch rejects the reconcile with `EIO`.
    fn verify_upper_target(&self, upper_dir_path: &Path, name: &str) -> Result<()> {
        let upper_real = self.upper_real_object(upper_dir_path, name)?;
        let lower = self.lower_source()?;
        if upper_real.real_inode().type_() != lower.real_inode().type_() {
            return_errno_with_message!(
                Errno::EIO,
                "the upper target type does not match the lower source"
            );
        }
        if upper_real.real_inode().mode()? != lower.real_inode().mode()? {
            return_errno_with_message!(
                Errno::EIO,
                "the upper target mode does not match the lower source"
            );
        }
        Ok(())
    }

    /// Resolves the real object now published at the upper target name.
    fn upper_real_object(&self, upper_dir_path: &Path, name: &str) -> Result<RealObject> {
        let child_path = super::super::lookup_child_path(upper_dir_path, name)?;
        let fs = self.fs_arc()?;
        let upper_layer = fs.layer_stack().upper_layer()?;
        Ok(upper_layer.child_real_object(&child_path))
    }

    /// Called on failure after physical publication: the upper object at the
    /// publication coordinate is retained and the next winner entry must
    /// verify it before reuse.
    fn mark_reconcile_pending(coordinate: &mut CopyUpTransition) {
        coordinate.phase = CopyUpPhase::ReconcilePending;
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
