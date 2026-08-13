// SPDX-License-Identifier: MPL-2.0

//! Upper/workdir exclusivity claims and the unified 64-bit overlay identity.
//!
//! This module implements the inode `Extension` runtime lease that carries
//! the claim. Each claimed root inode hosts a VFS-owned `OverlayInuseSlot`.
//! The non-zero unified [`OverlayUuid`] value serves two roles:
//!
//! - claim token: the per-slot compare-and-swap (CAS) on the slot's owner
//!   token guards this value so only one overlay can hold the claim;
//! - persisted overlay UUID: when effective, the value is stored as
//!   `trusted.overlay.uuid` on the upper root.

use super::{layers, options::UuidMode};
use crate::{
    fs::{
        file::{InodeMode, InodeType},
        vfs::{
            inode::Inode,
            inode_ext::InodeExt,
            path::{Path, is_dot_or_dotdot},
            xattr::{XattrName, XattrSetFlags},
        },
    },
    prelude::*,
};

pub(super) const OVERLAY_UUID_SIZE: usize = 8;

pub(super) const TRUSTED_OVERLAY_UUID: &str = "trusted.overlay.uuid";

const WORKDIR_NAME: &str = "work";

const WORKDIR_MODE: InodeMode = InodeMode::from_bits_truncate(0o700);

const WORKDIR_CLEANUP_MAX_DEPTH: usize = 2;

/// The unified 64-bit identity of one writable overlay mount.
///
/// The value is never zero. It serves as the claim token for
/// [`InodeClaimGuard`]; when effective, it is also the overlay UUID
/// persisted as `trusted.overlay.uuid` and published through
/// `MountPolicy::uuid()`/`SuperBlock::fsid`.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(in overlayfs) struct OverlayUuid(u64);

impl OverlayUuid {
    /// Creates an [`OverlayUuid`], rejecting the zero value with `EINVAL`.
    pub(super) fn try_new(value: u64) -> Result<Self> {
        if value == 0 {
            return_errno_with_message!(Errno::EINVAL, "the overlay uuid must be non-zero");
        }
        Ok(Self(value))
    }

    pub(super) fn value(&self) -> u64 {
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

    /// Reads an existing persisted identity from the upper root.
    ///
    /// Returns `Ok(None)` when no `trusted.overlay.uuid` xattr exists
    /// (`ENODATA`); a malformed value fails with `EINVAL`.
    fn read_from_upper(upper_inode: &Arc<dyn Inode>) -> Result<Option<Self>> {
        let name = XattrName::try_from_full_name(TRUSTED_OVERLAY_UUID)
            .ok_or_else(|| Error::with_message(Errno::EINVAL, "invalid overlay uuid xattr name"))?;
        let mut value = [0u8; OVERLAY_UUID_SIZE];
        let mut writer = VmWriter::from(value.as_mut_slice()).to_fallible();
        match upper_inode.get_xattr(name, &mut writer) {
            Ok(written) if written == OVERLAY_UUID_SIZE => {
                Ok(Some(Self::try_new(u64::from_le_bytes(value))?))
            }
            Ok(_) => return_errno_with_message!(
                Errno::EINVAL,
                "the persisted overlay uuid has a malformed value"
            ),
            Err(err) if err.error() == Errno::ENODATA => Ok(None),
            Err(err) => Err(err),
        }
    }

    /// Persists this identity as `trusted.overlay.uuid` on the upper root;
    /// callers invoke it only when the identity is effective.
    fn persist_on_upper(&self, upper_inode: &Arc<dyn Inode>) -> Result<()> {
        let name = XattrName::try_from_full_name(TRUSTED_OVERLAY_UUID)
            .ok_or_else(|| Error::with_message(Errno::EINVAL, "invalid overlay uuid xattr name"))?;
        let value = self.value().to_le_bytes();
        let mut reader = VmReader::from(value.as_slice()).to_fallible();
        upper_inode.set_xattr(name, &mut reader, XattrSetFlags::CREATE_OR_REPLACE)
    }
}

/// A runtime lease on one root inode's `OverlayInuseSlot`.
///
/// The guard pins the claimed inode so the slot cannot be evicted while the
/// claim is held and holds the unified non-zero token.
#[derive(Debug)]
pub(super) struct InodeClaimGuard {
    inode: Arc<dyn Inode>,
    token: OverlayUuid,
}

impl InodeClaimGuard {
    /// Claims the inode's `OverlayInuseSlot` with `identity` as the token.
    ///
    /// Returns `EBUSY` when the slot is already claimed by another holder.
    pub(super) fn try_claim(inode: Arc<dyn Inode>, identity: OverlayUuid) -> Result<Self> {
        inode.overlay_inuse_slot().try_claim(identity.value())?;
        Ok(Self {
            inode,
            token: identity,
        })
    }
}

impl Drop for InodeClaimGuard {
    fn drop(&mut self) {
        self.inode.overlay_inuse_slot().release(self.token.value());
    }
}

/// The exclusively claimed upper/workdir pair of a writable overlay mount.
#[derive(Debug)]
pub(in overlayfs) struct UpperWorkdirClaim {
    workdir: InodeClaimGuard,
    upper: InodeClaimGuard,
    identity: OverlayUuid,
    workdir_workspace: Option<WorkdirWorkspace>,
}

/// The prepared `<workdir>/work` staging workspace.
#[derive(Debug)]
struct WorkdirWorkspace {
    inode: Arc<dyn Inode>,
    path: Path,
}

impl UpperWorkdirClaim {
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
    pub(super) fn determine_identity(
        upper_inode: &Arc<dyn Inode>,
        uuid_mode: UuidMode,
    ) -> Result<OverlayUuid> {
        match uuid_mode {
            UuidMode::On => match OverlayUuid::read_from_upper(upper_inode)? {
                Some(existing) => Ok(existing),
                None => Ok(OverlayUuid::generate()),
            },
            UuidMode::Auto => match OverlayUuid::read_from_upper(upper_inode) {
                Ok(Some(existing)) => Ok(existing),
                Ok(None) | Err(_) => Ok(OverlayUuid::generate()),
            },
            UuidMode::Off | UuidMode::Null => Ok(OverlayUuid::generate()),
        }
    }

    pub(super) fn claim(
        upper_inode: Arc<dyn Inode>,
        workdir_inode: Arc<dyn Inode>,
        identity: OverlayUuid,
    ) -> Result<Self> {
        let upper = InodeClaimGuard::try_claim(upper_inode, identity)?;
        let workdir = match InodeClaimGuard::try_claim(workdir_inode, identity) {
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
            workdir_workspace: None,
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
        self.workdir_workspace = Some(WorkdirWorkspace {
            inode: workspace.inode().clone(),
            path: workspace,
        });
        Ok(())
    }

    fn remove_work_entries(&self, dir: &Arc<dyn Inode>, level: usize) -> Result<()> {
        let mut names: Vec<String> = Vec::new();
        let mut offset = 0;
        loop {
            match dir.readdir_at(offset, &mut names)? {
                0 => break,
                visited => offset += visited,
            }
        }
        names.retain(|name| !is_dot_or_dotdot(name));
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

    pub(super) fn persist_identity(&self) -> Result<()> {
        self.identity.persist_on_upper(&self.upper.inode)
    }

    pub(in overlayfs) fn workdir_workspace(&self) -> Result<&Arc<dyn Inode>> {
        self.workdir_workspace
            .as_ref()
            .map(|workspace| &workspace.inode)
            .ok_or_else(|| {
                Error::with_message(
                    Errno::EROFS,
                    "the overlay workdir workspace is not prepared",
                )
            })
    }

    pub(in crate::fs::fs_impls::overlayfs) fn workdir_workspace_path(&self) -> Result<&Path> {
        self.workdir_workspace
            .as_ref()
            .map(|workspace| &workspace.path)
            .ok_or_else(|| {
                Error::with_message(
                    Errno::EROFS,
                    "the overlay workdir workspace is not prepared",
                )
            })
    }
}

/// Probes that a root path resolves to a backend-instance-stable inode.
///
/// Both resolutions must match `pinned_inode`, so the checked object is the
/// one that [`UpperWorkdirClaim::claim`] later uses. This is a heuristic; a
/// failing backend returns `EOPNOTSUPP`.
pub(super) fn verify_inode_instance_stability(
    raw_path: &str,
    pinned_inode: &Arc<dyn Inode>,
) -> Result<()> {
    let first = layers::resolve_root_path(raw_path)?.inode().clone();
    let second = layers::resolve_root_path(raw_path)?.inode().clone();
    if !Arc::ptr_eq(&first, &second) || !Arc::ptr_eq(&first, pinned_inode) {
        return_errno_with_message!(
            Errno::EOPNOTSUPP,
            "the underlying filesystem does not provide instance-stable inodes for pinned roots"
        );
    }
    Ok(())
}
