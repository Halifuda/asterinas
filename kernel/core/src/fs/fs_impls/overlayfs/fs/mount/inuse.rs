// SPDX-License-Identifier: MPL-2.0

#![short_vis_path::add(overlayfs)]
//! Upper/workdir exclusivity claims and the unified 64-bit overlay identity.
//!
//! This module implements the inode `Extension` runtime lease that carries
//! the claim. Each claimed root inode hosts a VFS-owned `OverlayInuseSlot`.
//! The non-zero unified [`Uuid`] value serves two roles:
//!
//! - claim token: the per-slot compare-and-swap (CAS) on the slot's owner
//!   token guards this value so only one overlay can hold the claim;
//! - persisted overlay UUID: when effective, the value is stored under the
//!   mount's selected private-prefix uuid record (`trusted.overlay.uuid` by
//!   default, `user.overlay.uuid` in `userxattr` mode) on the upper root.

use super::super::policy::UuidMode;
use crate::{
    fs::{
        file::{InodeMode, InodeType},
        fs_impls::overlayfs::{
            inode::{OverlayInode, OverlayRecordName, OverlayXattrPrefix, overlay_record_name},
            read_child_names,
        },
        vfs::{inode::Inode, inode_ext::InodeExt, path::Path, xattr::XattrSetFlags},
    },
    prelude::*,
};

pub(super) const OVERLAY_UUID_SIZE: usize = 8;

const WORKDIR_NAME: &str = "work";

const WORKDIR_MODE: InodeMode = InodeMode::from_bits_truncate(0o700);

const WORKDIR_CLEANUP_MAX_DEPTH: usize = 2;

/// The unified 64-bit identity of one writable overlay mount.
///
/// The value is never zero. It serves as the claim token for
/// [`InuseGuard`]; when effective, it is also the overlay UUID
/// persisted as `trusted.overlay.uuid` and published through
/// `MountPolicy::uuid()`/`SuperBlock::fsid`.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(in super::super) struct Uuid(u64);

impl Uuid {
    /// Creates a [`Uuid`], rejecting the zero value with `EINVAL`.
    fn try_new(value: u64) -> Result<Self> {
        if value == 0 {
            return_errno_with_message!(Errno::EINVAL, "the overlay uuid must be non-zero");
        }
        Ok(Self(value))
    }

    pub(in super::super) fn value(&self) -> u64 {
        self.0
    }

    /// Generates a fresh non-zero identity from the kernel CSPRNG.
    pub(super) fn generate() -> Self {
        loop {
            let mut bytes = [0u8; OVERLAY_UUID_SIZE];
            crate::util::random::getrandom(&mut bytes);
            let value = u64::from_le_bytes(bytes);
            if let Ok(uuid) = Self::try_new(value) {
                return uuid;
            }
        }
    }
}

/// A runtime lease on one root inode's `OverlayInuseSlot`.
///
/// The guard pins the claimed inode so the slot cannot be evicted while the
/// claim is held and holds the unified non-zero token.
#[derive(Debug)]
struct InuseGuard {
    inode: Arc<dyn Inode>,
    token: Uuid,
}

impl InuseGuard {
    /// Claims the inode's `OverlayInuseSlot` with `identity` as the token.
    ///
    /// Returns `EBUSY` when the slot is already claimed by another holder.
    fn try_claim(inode: Arc<dyn Inode>, identity: Uuid) -> Result<Self> {
        inode.overlay_inuse_slot().try_claim(identity.value())?;
        Ok(Self {
            inode,
            token: identity,
        })
    }
}

impl Drop for InuseGuard {
    fn drop(&mut self) {
        self.inode.overlay_inuse_slot().release(self.token.value());
    }
}

/// The exclusively claimed upper/workdir pair of a writable overlay mount.
#[derive(Debug)]
pub(in overlayfs) struct UpperWorkdirInuse {
    workdir: InuseGuard,
    upper: InuseGuard,
    identity: Uuid,
    workspace: Option<Path>,
}

impl UpperWorkdirInuse {
    /// Validates the upper/workdir pair structurally.
    ///
    /// Both roots must be directories on one underlying filesystem (`st_dev`
    /// evidence) sharing one mount node; the workdir must not overlap the
    /// upperdir. Failures map to `ENOTDIR` / `EINVAL`.
    pub(super) fn validate_pair(upper: &Path, workdir: &Path) -> Result<()> {
        if !upper.type_().is_directory() {
            return_errno_with_message!(Errno::ENOTDIR, "upperdir is not a directory");
        }
        if !workdir.type_().is_directory() {
            return_errno_with_message!(Errno::ENOTDIR, "workdir is not a directory");
        }
        if !Arc::ptr_eq(upper.mount_node(), workdir.mount_node()) {
            return_errno_with_message!(
                Errno::EINVAL,
                "workdir and upperdir must reside under the same mount"
            );
        }
        if upper.metadata()?.container_dev_id != workdir.metadata()?.container_dev_id {
            return_errno_with_message!(
                Errno::EINVAL,
                "workdir and upperdir must be on the same underlying filesystem"
            );
        }
        if Arc::ptr_eq(upper.dentry(), workdir.dentry()) {
            return_errno_with_message!(Errno::EINVAL, "workdir must be distinct from upperdir");
        }
        if Arc::ptr_eq(upper.inode(), workdir.inode()) {
            return_errno_with_message!(Errno::EINVAL, "workdir must be distinct from upperdir");
        }
        if workdir.dentry().is_equal_or_descendant_of(upper.dentry())
            || upper.dentry().is_equal_or_descendant_of(workdir.dentry())
        {
            return_errno_with_message!(
                Errno::EINVAL,
                "workdir must not be an ancestor or descendant of upperdir"
            );
        }
        Ok(())
    }

    /// Determines the overlay identity for the given `uuid_mode`.
    ///
    /// The existing-identity read measures the mount's selected private
    /// prefix (`prefix`, threaded from `OverlayFs::new`).
    pub(super) fn determine_identity(
        upper_inode: &Arc<dyn Inode>,
        uuid_mode: UuidMode,
        prefix: OverlayXattrPrefix,
    ) -> Result<Uuid> {
        match uuid_mode {
            UuidMode::On => match Self::read_identity_from_upper(upper_inode, prefix)? {
                Some(existing) => Ok(existing),
                None => Ok(Uuid::generate()),
            },
            UuidMode::Auto => match Self::read_identity_from_upper(upper_inode, prefix) {
                Ok(Some(existing)) => Ok(existing),
                Ok(None) | Err(_) => Ok(Uuid::generate()),
            },
            UuidMode::Off | UuidMode::Null => Ok(Uuid::generate()),
        }
    }

    pub(super) fn claim(
        upper_inode: Arc<dyn Inode>,
        workdir_inode: Arc<dyn Inode>,
        identity: Uuid,
    ) -> Result<Self> {
        let upper = InuseGuard::try_claim(upper_inode, identity)?;
        let workdir = match InuseGuard::try_claim(workdir_inode, identity) {
            Ok(workdir) => workdir,
            Err(err) => {
                drop(upper);
                return Err(err);
            }
        };
        Ok(Self {
            workdir,
            upper,
            identity,
            workspace: None,
        })
    }

    /// Ensures the `<workdir>/work` staging workspace exists empty,
    /// recreating it and replacing any residue.
    ///
    /// Other entries under the workdir root are left untouched.
    pub(super) fn prepare_workdir(&mut self, workdir_path: &Path) -> Result<()> {
        match self.workdir.inode.lookup(WORKDIR_NAME) {
            Ok(residue) if residue.type_().is_directory() => {
                self.remove_work_entries(&residue, 0)?;
                workdir_path.rmdir(WORKDIR_NAME)?;
            }
            Ok(_) => {
                workdir_path.unlink(WORKDIR_NAME)?;
            }
            Err(err) if err.error() == Errno::ENOENT => {}
            Err(err) => return Err(err),
        }
        let workspace = workdir_path.new_fs_child(WORKDIR_NAME, InodeType::Dir, WORKDIR_MODE)?;
        self.workspace = Some(workspace);
        Ok(())
    }

    fn remove_work_entries(&self, dir: &Arc<dyn Inode>, level: usize) -> Result<()> {
        let names = read_child_names(dir)?;
        for name in names {
            let child = dir.lookup(&name)?;
            if child.type_().is_directory() {
                if level < WORKDIR_CLEANUP_MAX_DEPTH {
                    self.remove_work_entries(&child, level + 1)?;
                }
                dir.rmdir(&name)?;
            } else {
                dir.unlink(&name)?;
            }
        }
        Ok(())
    }

    /// Persists the overlay identity under the mount's selected private
    /// prefix (`prefix`, threaded from `OverlayFs::new`). The write routes
    /// through the xattr private path
    /// ([`OverlayInode::set_overlay_xattr`]), so the record name is never
    /// escaped.
    pub(super) fn persist_identity(&self, prefix: OverlayXattrPrefix) -> Result<()> {
        let name = overlay_record_name(OverlayRecordName::Uuid, prefix)?;
        let value = self.identity.value().to_le_bytes();
        let mut reader = VmReader::from(value.as_slice()).to_fallible();
        OverlayInode::set_overlay_xattr(
            &self.upper.inode,
            name,
            &mut reader,
            XattrSetFlags::CREATE_OR_REPLACE,
        )
    }

    /// Returns the pinned `<workdir>/work` staging workspace inode.
    pub(in overlayfs) fn workdir_workspace(&self) -> Result<&Arc<dyn Inode>> {
        self.workspace
            .as_ref()
            .map(|workspace| workspace.inode())
            .ok_or_else(|| {
                Error::with_message(
                    Errno::EROFS,
                    "the overlay workdir workspace is not prepared",
                )
            })
    }

    /// Returns the pinned `<workdir>/work` staging workspace path.
    pub(in overlayfs) fn workdir_workspace_path(&self) -> Result<&Path> {
        self.workspace.as_ref().ok_or_else(|| {
            Error::with_message(
                Errno::EROFS,
                "the overlay workdir workspace is not prepared",
            )
        })
    }

    /// Reads an existing persisted identity from the upper root.
    ///
    /// Returns `Ok(None)` when no selected-prefix uuid record exists
    /// (`ENODATA`); a malformed value fails with `EINVAL`.
    fn read_identity_from_upper(
        upper_inode: &Arc<dyn Inode>,
        prefix: OverlayXattrPrefix,
    ) -> Result<Option<Uuid>> {
        let name = overlay_record_name(OverlayRecordName::Uuid, prefix)?;
        let mut value = [0u8; OVERLAY_UUID_SIZE];
        let mut writer = VmWriter::from(value.as_mut_slice()).to_fallible();
        match upper_inode.get_xattr(name, &mut writer) {
            Ok(written) if written == OVERLAY_UUID_SIZE => {
                Ok(Some(Uuid::try_new(u64::from_le_bytes(value))?))
            }
            Ok(_) => return_errno_with_message!(
                Errno::EINVAL,
                "the persisted overlay uuid has a malformed value"
            ),
            Err(err) if err.error() == Errno::ENODATA => Ok(None),
            Err(err) => Err(err),
        }
    }
}
