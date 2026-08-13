// SPDX-License-Identifier: MPL-2.0

//! The copy-up winner body: object-kind promotion and upper publication.
//!
//! This module hosts the private winner body [`OverlayInode::promote`] and
//! its helpers: the object-kind recipe arms, metadata/xattr transfer, the
//! ReconcilePending verification, and the publication steps.
//!
//! ## References
//!
//! - Linux `ovl_set_attr` (symlink mode skip):
//!   <https://elixir.bootlin.com/linux/latest/source/fs/overlayfs/copy_up.c#L392-L416>

use core::cmp::min;

use super::{
    coordination::{CopyUpPhase, CopyUpTransition},
    workdir::WorkdirTempRequest,
};
use crate::{
    fs::{
        file::{InodeType, StatusFlags},
        fs_impls::overlayfs::{
            metadata_security::xattr::XattrCopyPolicy,
            mount::{OverlayFs, RealPath},
            projection::{OverlayInode, OverlayObjectFacts, PositiveKind, RealObject},
        },
        vfs::{
            inode::{Inode, MknodType, RenameMode, SymbolicLink},
            path::Path,
        },
    },
    prelude::*,
};

/// The chunk size of the regular-file data stream during copy-up.
///
/// The lower file is streamed through one reused kernel buffer; the chunk
/// bounds each `read_at`/`write_at` pair so a short read still makes bounded
/// progress.
const COPY_CHUNK_SIZE: usize = 64 * 1024;

impl OverlayInode {
    /// Runs the winner promotion body for this object.
    ///
    /// Called by the trigger with the `copyup_transition` arbitration guard
    /// held. The object kind is dispatched internally; success commits the
    /// phase to [`CopyUpPhase::Idle`], and recipe-arm failures classify as
    /// cleanup-before-publication vs `ReconcilePending`.
    pub(super) fn promote(
        &self,
        publication_parent: &Arc<OverlayInode>,
        name: &str,
        coordinate: &mut CopyUpTransition,
    ) -> Result<()> {
        // 1) Idempotent upper fast path: a waiter may have completed the
        //    transition while this task waited for the arbitration guard.
        if self.facts_snapshot().upper().is_some() {
            return Ok(());
        }

        // 2) ReconcilePending verification (recovery): the parent was
        //    promoted by the ancestor walk, so its real object is the upper
        //    directory; the verify helper consumes the passed `name`.
        if coordinate.phase == CopyUpPhase::ReconcilePending {
            let upper_dir_path = publication_parent.upper_parent_path()?;
            self.verify_upper_target(&upper_dir_path, name)?;
        }

        let upper_dir = publication_parent.select_real_inode();
        let upper_dir_path = publication_parent.upper_parent_path()?;
        let fs = self.fs_arc()?;
        // Impurity marker: every promoted object makes its publication
        // parent impure — persist the marker before the object-kind dispatch
        // and the physical upper commit (strict, pre-commit; read-first
        // idempotence makes an already-marked parent a no-op).
        fs.xattr_policy().set_impure_marker(&upper_dir)?;
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
                    &upper_dir_path,
                    WorkdirTempRequest::Create {
                        kind: InodeType::Dir,
                        mode,
                    },
                )?;
                let temp_kind = temp.kind();
                let (temp_name, temp) = temp.into_parts();
                self.run_recipe(
                    &fs,
                    Some((&temp_name, temp_kind)),
                    || Self::mark_reconcile_pending(coordinate),
                    |marker| {
                        self.transfer_metadata(temp.inode())?;
                        self.copy_eligible_xattrs(temp.inode(), XattrCopyPolicy::Strict)?;
                        self.transfer_timestamps(temp.inode())?;
                        let workdir_path = self.workdir_root_path()?;
                        self.publish_via_rename(
                            &workdir_path,
                            &temp_name,
                            &upper_dir_path,
                            name,
                            marker,
                            lower.clone(),
                        )
                    },
                )
            }
            InodeType::File => {
                let mode = lower.real_inode().mode()?;
                let temp = fs.create_workdir_temp(
                    name,
                    &upper_dir_path,
                    WorkdirTempRequest::Create {
                        kind: InodeType::File,
                        mode,
                    },
                )?;
                let temp_kind = temp.kind();
                let (temp_name, temp) = temp.into_parts();
                self.run_recipe(
                    &fs,
                    Some((&temp_name, temp_kind)),
                    || Self::mark_reconcile_pending(coordinate),
                    |marker| {
                        self.transfer_metadata(temp.inode())?;
                        self.copy_eligible_xattrs(temp.inode(), XattrCopyPolicy::Strict)?;
                        self.promote_regular_file(temp.inode())?;
                        self.transfer_timestamps(temp.inode())?;
                        // Durability: the data file is synced before
                        // publication.
                        temp.inode().sync_all()?;
                        // Atomic publication: rename the private workdir temp
                        // onto the upper target name; a whiteout at the name
                        // is impossible for authority-only promotion.
                        let workdir_path = self.workdir_root_path()?;
                        self.publish_via_rename(
                            &workdir_path,
                            &temp_name,
                            &upper_dir_path,
                            name,
                            marker,
                            lower.clone(),
                        )
                    },
                )
            }
            InodeType::SymLink => {
                // Symlink promotion side: a workdir symlink temp recreated
                // from the lower target, then xattrs and the atomic rename
                // (the symlink object itself is copied; its target is left
                // unreferenced).
                let mode = lower.real_inode().mode()?;
                let temp = fs.create_workdir_temp(
                    name,
                    &upper_dir_path,
                    WorkdirTempRequest::Create {
                        kind: InodeType::SymLink,
                        mode,
                    },
                )?;
                let temp_kind = temp.kind();
                let (temp_name, temp) = temp.into_parts();
                self.run_recipe(
                    &fs,
                    Some((&temp_name, temp_kind)),
                    || Self::mark_reconcile_pending(coordinate),
                    |marker| {
                        self.promote_symlink(temp.inode())?;
                        self.transfer_metadata(temp.inode())?;
                        self.copy_eligible_xattrs(temp.inode(), XattrCopyPolicy::Strict)?;
                        self.transfer_timestamps(temp.inode())?;
                        let workdir_path = self.workdir_root_path()?;
                        self.publish_via_rename(
                            &workdir_path,
                            &temp_name,
                            &upper_dir_path,
                            name,
                            marker,
                            lower.clone(),
                        )
                    },
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
                    &upper_dir_path,
                    WorkdirTempRequest::Mknod {
                        mode,
                        node: &mknod_type,
                    },
                )?;
                let temp_kind = temp.kind();
                let (temp_name, temp) = temp.into_parts();
                self.run_recipe(
                    &fs,
                    Some((&temp_name, temp_kind)),
                    || Self::mark_reconcile_pending(coordinate),
                    |marker| {
                        self.transfer_metadata(temp.inode())?;
                        self.copy_eligible_xattrs(temp.inode(), XattrCopyPolicy::Strict)?;
                        self.transfer_timestamps(temp.inode())?;
                        // The workdir staging workspace resolves inside the
                        // recipe closure: a resolution failure is a
                        // pre-commit failure, so `run_recipe` best-effort
                        // cleans the staged temp instead of leaking it.
                        let workdir_path = self.workdir_root_path()?;
                        self.publish_via_rename(
                            &workdir_path,
                            &temp_name,
                            &upper_dir_path,
                            name,
                            marker,
                            lower.clone(),
                        )
                    },
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

    /// Publishes the staged workdir temp onto the upper target name via an
    /// atomic rename, commits the physical-upper marker, and runs the
    /// semantic authority publication.
    fn publish_via_rename(
        &self,
        workdir_path: &Path,
        temp_name: &str,
        upper_dir_path: &Path,
        name: &str,
        marker: &mut CommitMarker,
        lower: RealObject,
    ) -> Result<()> {
        workdir_path.rename(temp_name, upper_dir_path, name, RenameMode::Replace)?;
        marker.commit();
        let upper_real = self.upper_real_object(upper_dir_path, name)?;
        self.publish_upper_authority(upper_real, lower)
    }

    /// Runs a fallible upper-mutation recipe with the shared commit scaffold.
    ///
    /// The recipe receives the [`CommitMarker`] and calls
    /// [`CommitMarker::commit`] at the physical-upper-commit point; on
    /// success its value is returned unchanged, on failure the scaffold runs
    /// the `reconcile` closure when committed or best-effort cleans the
    /// staged workdir temp otherwise.
    pub(in crate::fs::fs_impls::overlayfs) fn run_recipe<T>(
        &self,
        fs: &Arc<OverlayFs>,
        temp: Option<(&str, InodeType)>,
        reconcile: impl FnOnce(),
        recipe: impl FnOnce(&mut CommitMarker) -> Result<T>,
    ) -> Result<T> {
        let mut marker = CommitMarker::default();
        let recipe_result = recipe(&mut marker);
        match recipe_result {
            Ok(value) => Ok(value),
            Err(err) => {
                if marker.is_committed() {
                    reconcile();
                } else if let Some((temp_name, kind)) = temp {
                    // Pre-commit failure (pre-publication arm): best-effort
                    // kind-aware temp cleanup; residue is a known cleanup
                    // debt, never a visible source.
                    let _ = fs.cleanup_workdir_temp(temp_name, kind);
                }
                Err(err)
            }
        }
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
    fn transfer_metadata(&self, temp: &Arc<dyn Inode>) -> Result<()> {
        let lower = self.lower_source()?;
        let lower_inode = lower.real_inode();
        temp.set_owner(lower_inode.owner()?)?;
        temp.set_group(lower_inode.group()?)?;
        if !matches!(lower_inode.type_(), InodeType::SymLink) {
            temp.set_mode(lower_inode.mode()?)?;
        }
        if lower_inode.type_().is_regular_file() {
            temp.resize(lower_inode.size())?;
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
    fn transfer_timestamps(&self, temp: &Arc<dyn Inode>) -> Result<()> {
        let lower = self.lower_source()?;
        let lower_inode = lower.real_inode();
        temp.set_atime(lower_inode.atime());
        temp.set_mtime(lower_inode.mtime());
        temp.set_ctime(lower_inode.ctime());
        Ok(())
    }

    /// Copies only `Public` xattrs (`User`/`Trusted`/`Security` namespaces)
    /// from the lower source; overlay-private names and the `System`
    /// namespace stay excluded.
    ///
    /// The copy-up policy is strict: a denied source read
    /// (`EACCES`/`EPERM`) propagates and fails the copy-up rather than
    /// silently dropping `security.*`/`trusted.*` metadata.
    fn copy_eligible_xattrs(&self, temp: &Arc<dyn Inode>, policy: XattrCopyPolicy) -> Result<()> {
        let lower = self.lower_source()?;
        let fs = self.fs_arc()?;
        fs.policy()
            .credential_policy()
            .with_creator_credentials_fn(|| {
                fs.xattr_policy()
                    .copy_eligible_xattrs(lower.real_inode(), temp, policy)
            })
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
        let old_facts = self.facts_snapshot();
        // A copied-up DIRECTORY keeps the merged view: the upper directory is
        // created empty, so a directory that still carries a lower stack must
        // stay `Merged` or the pre-existing lower children would vanish from
        // `getdents`. Non-directories keep their pre-copy-up kind; `lowers`
        // are retained regardless so whiteouts keep publishing.
        let kind = if self.type_().is_directory() && !old_facts.lowers().is_empty() {
            PositiveKind::Merged
        } else {
            old_facts.kind()
        };
        // Keep `upper_real` in scope past the facts construction: it is the
        // post-transition visible source passed to `replace_facts`.
        let new_facts = OverlayObjectFacts::try_new(
            kind,
            Some(upper_real.clone()),
            old_facts.lowers().to_vec(),
        )
        .ok_or_else(|| {
            Error::with_message(Errno::EIO, "cannot construct the post-copy-up facts")
        })?;
        let carrier = fs.project_new_upper(&self.facts_snapshot());
        carrier.replace_facts(new_facts, &upper_real)?;
        Ok(())
    }

    /// Verifies the upper entry at the publication coordinate before reuse
    /// (ReconcilePending path).
    ///
    /// Covers the upper entry's object type and basic mode metadata; a
    /// mismatch rejects the reconcile with `EIO`.
    pub(super) fn verify_upper_target(&self, upper_dir_path: &Path, name: &str) -> Result<()> {
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
        let child_path = Path::new(
            upper_dir_path.mount_node().clone(),
            upper_dir_path
                .dentry()
                .as_dir_dentry_or_err()?
                .lookup_child(name)?,
        );
        let fs = self.fs_arc()?;
        let upper_layer = fs.layer_stack().upper.as_ref().ok_or_else(|| {
            Error::with_message(Errno::EROFS, "the overlay mount has no upper layer")
        })?;
        Ok(RealObject::with_path(
            0,
            RealPath::from_path(&child_path),
            upper_layer.fsid,
            upper_layer.container_dev_id,
        ))
    }

    /// Returns the pinned workdir staging workspace path of this mount.
    ///
    /// Lets the copy-up recipe arms resolve the staging workspace without
    /// re-upgrading the mount themselves.
    pub(in crate::fs::fs_impls::overlayfs) fn workdir_root_path(&self) -> Result<Path> {
        self.fs_arc()?.workdir_root_path()
    }

    /// Called on failure after physical publication: the upper object at the
    /// publication coordinate is retained and the next winner entry must
    /// verify it before reuse.
    fn mark_reconcile_pending(coordinate: &mut CopyUpTransition) {
        coordinate.phase = CopyUpPhase::ReconcilePending;
    }

    /// Returns the topmost lower real object of this object (`lowers[0]`).
    ///
    /// Safe by the facts invariant `upper.is_some() || !lowers.is_empty()`;
    /// the checked access surfaces a structural violation as `EIO`.
    fn lower_source(&self) -> Result<RealObject> {
        self.facts_snapshot()
            .lowers()
            .first()
            .cloned()
            .ok_or_else(|| {
                Error::with_message(
                    Errno::EIO,
                    "a lower-backed overlay object has no lower source",
                )
            })
    }
}

/// The physical-upper-commit marker of a [`run_recipe`](OverlayInode::run_recipe)
/// recipe closure.
///
/// A one-way latch over the commit boolean: the recipe calls
/// [`CommitMarker::commit`] exactly once at the physical-upper-commit point,
/// and the scaffold reads [`CommitMarker::is_committed`] to classify a later
/// failure as reconcile vs pre-publication cleanup.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(in crate::fs::fs_impls::overlayfs) struct CommitMarker {
    committed: bool,
}

impl CommitMarker {
    pub(in crate::fs::fs_impls::overlayfs) fn commit(&mut self) {
        self.committed = true;
    }

    pub(in crate::fs::fs_impls::overlayfs) fn is_committed(&self) -> bool {
        self.committed
    }
}
