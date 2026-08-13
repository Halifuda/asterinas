// SPDX-License-Identifier: MPL-2.0

//! The remove recipes: the shared unlink/rmdir recipe on [`OverlayInode`],
//! parameterized by [`RemoveKind`].
//!
//! [`RemoveKind::{Unlink, Rmdir}`] names the operation; `remove_target` is
//! the shared recipe, with `clear_empty_exchange` and `translate_stale_upper_enoent` as helpers.
//!
//! Lock contract: the caller holds the parent directory transaction lock;
//! this module enters the per-object copy-up coordination lock only through
//! the copy-up step of `check_permission`, and never touches the whiteout
//! cache lock.

//!
//! ## References
//!
//! - <https://elixir.bootlin.com/linux/v6.17/source/fs/overlayfs/dir.c#L763-L807>
//!   (Linux `ovl_remove_and_whiteout` whiteout-publish removal)
//! - <https://elixir.bootlin.com/linux/v6.17/source/fs/overlayfs/dir.c#L809-L859>
//!   (Linux `ovl_remove_upper` direct upper removal)
//! - <https://elixir.bootlin.com/linux/v6.17/source/fs/overlayfs/namei.c#L1418-L1480>
//!   (Linux `ovl_lower_positive` lower-presence check)
//! - <https://elixir.bootlin.com/linux/v6.17/source/fs/overlayfs/dir.c#L758>
//!   (Linux `ovl_matches_upper` stale-upper check)

use super::whiteout;
use crate::{
    fs::{
        file::{InodeType, Permission},
        fs_impls::overlayfs::{
            AccessType,
            copyup::WorkdirTempRequest,
            metadata_security::xattr::{
                OPAQUE_MARKER_VALUE, OPAQUE_XATTR_FULL_NAME, XattrCopyPolicy,
            },
            mount::OverlayFs,
            projection::{
                Binding, BindingKey, HiddenEvidence, NegativeBinding, OverlayInode,
                OverlayObjectFacts,
            },
        },
        vfs::{
            inode::{Inode, RenameMode},
            path::{self, Path},
            xattr::{XattrName, XattrSetFlags},
        },
    },
    prelude::*,
};

/// The remove operation kind of [`OverlayInode::remove_target`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum RemoveKind {
    Unlink,
    Rmdir,
}

impl OverlayInode {
    /// Removes one visible name via the shared unlink/rmdir recipe.
    ///
    /// Fresh projection, the type and rmdir-emptiness gates, then direct upper
    /// removal or whiteout publication (clear-empty exchange for lower-backed
    /// directories). `unlink` refuses directories (`EISDIR`) to avoid
    /// whiteout-hiding a lower-backed directory; opaque upper directories are
    /// lower-backed, so `rmdir` publishes a whiteout instead.
    pub(super) fn remove_target(&self, name: &str, kind: RemoveKind) -> Result<()> {
        let fs = self.fs_arc()?;
        let parent_facts = self.facts_snapshot();
        let lookup = fs.lookup_binding(&parent_facts, name)?;
        // Stale-upper routing: a previously published upper-backed binding
        // whose physical upper object vanished behind the overlay (no
        // whiteout left) surfaces `ESTALE` before the rmdir emptiness gate
        // and the unlink `EISDIR` type gate.
        if lookup.is_stale_upper {
            return Err(translate_stale_upper_enoent(Error::with_message(
                Errno::ENOENT,
                "the overlay target became stale behind the overlay",
            )));
        }
        let target_inode = lookup.binding.into_inode().ok_or_else(|| {
            Error::with_message(Errno::ENOENT, "the overlay target does not exist")
        })?;
        let target_facts = target_inode.facts_snapshot();

        if kind == RemoveKind::Rmdir {
            match target_inode.visible_child_count(&target_facts) {
                Ok(0) => {}
                Ok(_) => {
                    return Err(Error::with_message(
                        Errno::ENOTEMPTY,
                        "the overlay directory is not empty",
                    ));
                }
                Err(err) if err.error() == Errno::ENOTDIR => {
                    return Err(err);
                }
                Err(_) => {
                    // `NeedsRebuild`-unresolvable: conservative `ENOTEMPTY`
                    // (never an upper-only emptiness guess).
                    return Err(Error::with_message(
                        Errno::ENOTEMPTY,
                        "the overlay directory emptiness could not be verified",
                    ));
                }
            }
        } else if target_inode.type_().is_directory() {
            return Err(Error::with_message(
                Errno::EISDIR,
                "a directory cannot be unlinked",
            ));
        }

        let is_pure_upper = match target_facts.upper() {
            Some(upper_obj) => {
                target_facts.lowers().is_empty() && !upper_obj.is_opaque_directory()?
            }
            None => false,
        };

        // The VFS entry already ran the EROFS and permission checks; this
        // call repeats the recipe's write-permission check before mutating.
        self.check_permission(AccessType::Mutating, Permission::MAY_WRITE)?;
        let upper_parent_path = self.upper_parent_path()?;

        if is_pure_upper {
            // A pure-upper rmdir may still face physical whiteout residue
            // inside the upper dir (the visible-emptiness gate does not
            // count whiteouts) — sweep it before the physical rmdir. The
            // `EIO` arm is defensive: `is_pure_upper` already implies an
            // upper object.
            if kind == RemoveKind::Rmdir {
                let target_upper_dir = target_facts.upper().ok_or_else(|| {
                    Error::with_message(
                        Errno::EIO,
                        "the pure-upper rmdir target has no upper real directory",
                    )
                })?;
                whiteout::cleanup_upper_whiteouts(&target_upper_dir.real_path()?)?;
            }
            // A physical-upper `ENOENT` means the asserted upper object
            // became stale and maps to `ESTALE`; other upper errors
            // propagate as-is. The binding invalidate and readdir-index
            // remove are both infallible, so no reconcile arm exists here.
            let result = if kind == RemoveKind::Rmdir {
                upper_parent_path.rmdir(name)
            } else {
                upper_parent_path.unlink(name)
            };
            result.map_err(translate_stale_upper_enoent)?;
            fs.bindings().invalidate(&self.key(), name);
            self.readdir_index_remove(name);
            // The removal may have restored purity — refresh the marker
            // best-effort (the mutation already committed; a refresh failure
            // never fails the removal).
            if let Err(err) = self.refresh_impure_marker() {
                warn!(
                    "overlay remove: the impure-marker refresh failed after the \
                     pure-upper removal (best-effort): {:?}",
                    err
                );
            }
            return Ok(());
        }

        // Lower-backed target: preserve the lower result with a published
        // whiteout. The rmdir target type is always `Dir` (the type gate
        // above refused rmdir-on-file with `ENOTDIR` and unlink-on-dir with
        // `EISDIR`).
        let target_type = target_inode.type_();
        let replace_target = target_facts.upper().map(|_| target_type);
        let clear_empty_temp = if kind == RemoveKind::Rmdir {
            match target_facts.upper() {
                Some(upper_obj) => {
                    let mut upper_names: Vec<String> = Vec::new();
                    upper_obj.real_inode().readdir_at(0, &mut upper_names)?;
                    upper_names.retain(|entry| !path::is_dot_or_dotdot(entry));
                    if upper_names.is_empty() {
                        None
                    } else {
                        let mode = upper_obj.real_inode().mode()?;
                        Some(fs.create_workdir_temp(
                            name,
                            &upper_parent_path,
                            WorkdirTempRequest::Create {
                                kind: InodeType::Dir,
                                mode,
                            },
                        )?)
                    }
                }
                None => None,
            }
        } else {
            None
        };
        let staged_temp = clear_empty_temp
            .as_ref()
            .map(|temp| (temp.name(), temp.kind()));
        self.run_recipe(
            &fs,
            staged_temp,
            || self.invalidate_stale_cache(&[(self, name)]),
            |marker| {
                if let Some(temp) = clear_empty_temp.as_ref() {
                    self.clear_empty_exchange(
                        &fs,
                        &target_facts,
                        name,
                        &upper_parent_path,
                        temp.name(),
                        temp.inode(),
                    )?;
                    marker.commit();
                }
                fs.publish_whiteout(&upper_parent_path, name, replace_target)
                    .map_err(|err| {
                        if replace_target.is_some() {
                            translate_stale_upper_enoent(err)
                        } else {
                            err
                        }
                    })?;
                marker.commit();
                // Semantic publication — inline composition: the whiteout
                // is re-observed from the upper (layer 0) so the published
                // `HiddenByWhiteout` binding pins its strong `HiddenEvidence`
                // barrier, then the parent index tombstones the now-hidden
                // name. The re-observation is fallible: on failure the
                // whiteout is already published — reconcile.
                let whiteout_path = Path::new(
                    upper_parent_path.mount_node().clone(),
                    upper_parent_path
                        .dentry()
                        .as_dir_dentry_or_err()?
                        .lookup_child(name)?,
                );
                let whiteout_inode = whiteout_path.inode().clone();
                let evidence = HiddenEvidence::new(0, whiteout_inode);
                fs.bindings().insert(
                    BindingKey::new(self.key(), String::from(name)),
                    Arc::new(Binding::Negative(NegativeBinding::HiddenByWhiteout(
                        evidence,
                    ))),
                );
                self.readdir_index_remove(name);
                Ok(())
            },
        )?;
        if let Err(err) = self.refresh_impure_marker() {
            warn!(
                "overlay remove: the impure-marker refresh failed after the \
                 whiteout publish (best-effort): {:?}",
                err
            );
        }
        Ok(())
    }

    /// Executes the clear-empty directory exchange of the lower-backed rmdir
    /// recipe.
    ///
    /// Needed when the upper directory holds whiteout-hidden entries that
    /// would make `publish_whiteout` fail with `ENOTEMPTY`. The caller commits
    /// the [`CommitMarker`](crate::fs::fs_impls::overlayfs::copyup::promote::CommitMarker)
    /// after this returns; displaced-dir cleanup is best-effort and pre-commit
    /// failures propagate.
    fn clear_empty_exchange(
        &self,
        fs: &Arc<OverlayFs>,
        target_facts: &OverlayObjectFacts,
        name: &str,
        upper_parent_path: &Path,
        temp_name: &str,
        temp_inode: &Arc<dyn Inode>,
    ) -> Result<()> {
        let Some(upper_obj) = target_facts.upper() else {
            return Err(Error::with_message(
                Errno::EIO,
                "the clear-empty workdir temp has no upper directory",
            ));
        };
        let old_upper_dir = upper_obj.real_inode().clone();
        // The opaque marker is part of the replacement directory's complete
        // preparation: it keeps the name a lower-search barrier at every
        // instant of the swap (crash window included), gated by the
        // private-xattr capability.
        let can_store_private_xattr = fs
            .policy()
            .upper_capabilities()
            .is_some_and(|caps| caps.can_store_private_xattr());
        if !can_store_private_xattr {
            return Err(Error::with_message(
                Errno::EOPNOTSUPP,
                "the upper filesystem cannot store the opaque marker \
                         required for the clear-empty directory exchange",
            ));
        }
        let marker_name =
            XattrName::try_from_full_name(OPAQUE_XATTR_FULL_NAME).ok_or_else(|| {
                Error::with_message(Errno::EINVAL, "invalid overlay opaque marker xattr name")
            })?;
        let mut marker_reader = VmReader::from(OPAQUE_MARKER_VALUE).to_fallible();
        temp_inode.set_xattr(
            marker_name,
            &mut marker_reader,
            XattrSetFlags::CREATE_OR_REPLACE,
        )?;
        // The xattr buffer copy runs before the owner/group/mode are applied,
        // while the temp is still owned by the caller (the creating task), so
        // a non-owner rmdir of a directory carrying xattrs does not fail
        // `EACCES` on the temp `set_xattr`.
        fs.policy()
            .credential_policy()
            .with_creator_credentials_fn(|| {
                fs.xattr_policy().copy_eligible_xattrs(
                    &old_upper_dir,
                    temp_inode,
                    XattrCopyPolicy::BestEffort,
                )
            })?;
        temp_inode.set_owner(old_upper_dir.owner()?)?;
        temp_inode.set_group(old_upper_dir.group()?)?;
        temp_inode.set_mode(old_upper_dir.mode()?)?;
        temp_inode.set_atime(old_upper_dir.atime());
        temp_inode.set_mtime(old_upper_dir.mtime());
        temp_inode.set_ctime(old_upper_dir.ctime());
        let workdir_path = self.workdir_root_path()?;
        workdir_path
            .rename(temp_name, upper_parent_path, name, RenameMode::Exchange)
            .map_err(translate_stale_upper_enoent)?;
        match workdir_path
            .dentry()
            .as_dir_dentry_or_err()?
            .lookup_child(temp_name)
        {
            Ok(displaced_dentry) => {
                let displaced_path = Path::new(workdir_path.mount_node().clone(), displaced_dentry);
                if let Err(cleanup_err) = whiteout::cleanup_upper_whiteouts(&displaced_path) {
                    warn!(
                        "overlay clear-empty: the displaced-directory whiteout \
                         cleanup failed (residue, never a visible source): {:?}",
                        cleanup_err
                    );
                }
                if let Err(cleanup_err) = workdir_path.rmdir(temp_name) {
                    warn!(
                        "overlay clear-empty: workdir cleanup of the displaced \
                         directory {:?} failed (residue, never a visible source): {:?}",
                        temp_name, cleanup_err
                    );
                }
            }
            Err(reobserve_err) => {
                warn!(
                    "overlay clear-empty: re-observation of the displaced \
                     directory {:?} failed (residue, never a visible source): {:?}",
                    temp_name, reobserve_err
                );
            }
        }
        Ok(())
    }
}

/// Translates a physical-upper `ENOENT` into the stale-upper `ESTALE`
/// error; every other errno passes through unchanged.
///
/// This indirect approximation is intentional: a faithful VFS-level dentry
/// check would require a breaking VFS change, so it waits for a
/// non-breaking integration point.
// TODO(stale-upper): replace this approximation with the faithful VFS-level
// dentry check once a non-breaking integration point exists.
fn translate_stale_upper_enoent(err: Error) -> Error {
    if err.error() == Errno::ENOENT {
        Error::with_message(
            Errno::ESTALE,
            "the upper object at the target name became stale",
        )
    } else {
        err
    }
}
