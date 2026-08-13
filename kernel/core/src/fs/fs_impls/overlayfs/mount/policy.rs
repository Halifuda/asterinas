// SPDX-License-Identifier: MPL-2.0

//! Published mount policy state.
//!
//! A mount policy is the fixed per-mount decision state: whether the mount is
//! effectively read-only or `default_permissions`, the `xino`/UUID modes, and
//! the effective overlay UUID. The creator-credential policy is the stashed
//! mounting-thread credential scope; the upper-filesystem capabilities are the
//! probe-derived limits of the post-claim upper filesystem.
//!
//! This module owns the [`MountPolicy`] assembled by
//! [`OverlayFs`](super::superblock::OverlayFs), the
//! [`CreatorCredentialPolicy`], and the [`UpperFilesystemCapabilities`].

use alloc::format;

use aster_rights::ReadDupOp;

use super::{
    claims::{OVERLAY_UUID_SIZE, OverlayUuid, TRUSTED_OVERLAY_UUID},
    options::{OverlayMountOptions, UuidMode, XinoMode},
};
use crate::{
    fs::{
        file::{InodeMode, InodeType},
        utils::DirentVisitor,
        vfs::{
            inode::{Inode, MknodType},
            path::is_dot_or_dotdot,
            xattr::XattrName,
        },
    },
    prelude::*,
    process::Credentials,
};

const CHAR_DEVICE_PROBE_PREFIX: &str = ".overlay-char-device-probe-";
const D_TYPE_PROBE_PREFIX: &str = ".overlay-dtype-probe-";

/// Generates a uniquely-named workdir staging-workspace temp entry for a
/// capability probe.
fn unique_temp_name(prefix: &str) -> String {
    let mut probe_bytes = [0u8; 8];
    crate::util::random::getrandom(&mut probe_bytes);
    format!("{}{:016x}", prefix, u64::from_le_bytes(probe_bytes))
}

pub(in overlayfs) struct MountPolicy {
    is_effective_read_only: bool,
    #[expect(
        dead_code,
        reason = "the uuid mode policy is not read yet; reserved for the future UUID/fsid policy surface"
    )]
    uuid_mode: UuidMode,
    uuid: Option<OverlayUuid>,
    credential_policy: CreatorCredentialPolicy,
    upper_capabilities: Option<UpperFilesystemCapabilities>,
    is_default_permissions: bool,
    xino_mode: XinoMode,
}

impl MountPolicy {
    pub(super) fn assemble(
        is_effective_read_only: bool,
        credential_policy: CreatorCredentialPolicy,
        options: &OverlayMountOptions,
        uuid: Option<OverlayUuid>,
        upper_capabilities: Option<UpperFilesystemCapabilities>,
    ) -> Self {
        Self {
            is_effective_read_only,
            uuid_mode: options.uuid_mode,
            uuid,
            credential_policy,
            upper_capabilities,
            is_default_permissions: options.is_default_permissions,
            xino_mode: options.xino_mode,
        }
    }

    /// Returns whether this mount is effectively read-only.
    pub(in overlayfs) fn is_effective_read_only(&self) -> bool {
        self.is_effective_read_only
    }

    /// Reports the option value only.
    pub(in overlayfs) fn is_default_permissions(&self) -> bool {
        self.is_default_permissions
    }

    pub(in overlayfs) fn xino_mode(&self) -> XinoMode {
        self.xino_mode
    }

    /// Returns the overlay UUID when effective.
    pub(super) fn uuid(&self) -> Option<&OverlayUuid> {
        self.uuid.as_ref()
    }

    pub(in overlayfs) fn credential_policy(&self) -> &CreatorCredentialPolicy {
        &self.credential_policy
    }

    /// Returns the post-claim upper-filesystem capabilities.
    pub(in overlayfs) fn upper_capabilities(&self) -> Option<&UpperFilesystemCapabilities> {
        self.upper_capabilities.as_ref()
    }
}

/// The creator-credential policy of an overlay mount.
///
/// Stashes the mounting thread's credentials once.
pub(in overlayfs) struct CreatorCredentialPolicy {
    snapshot: Credentials<ReadDupOp>,
    source: CredentialSource,
}

impl CreatorCredentialPolicy {
    pub(super) fn new(snapshot: Credentials<ReadDupOp>) -> Self {
        Self {
            snapshot,
            source: CredentialSource::Creator,
        }
    }

    // TODO: Consume this through a VFS API that runs a closure under the stashed credentials.
    #[expect(dead_code, reason = "the VFS has no scoped creator-credential switch")]
    pub(in crate::fs::fs_impls::overlayfs) fn snapshot(&self) -> &Credentials<ReadDupOp> {
        &self.snapshot
    }

    #[expect(dead_code, reason = "the VFS has no scoped creator-credential switch")]
    pub(in crate::fs::fs_impls::overlayfs) fn source(&self) -> CredentialSource {
        self.source
    }

    /// Runs `operation_fn` with the caller's current credentials.
    ///
    /// This is a passthrough seam: the VFS has no scoped "run with stashed
    /// credentials" API, so the stashed credentials cannot be installed and
    /// callers must not rely on this for permission decisions.
    ///
    // TODO: restore the scope switch once the VFS provides a scoped credentials API.
    pub(in crate::fs::fs_impls::overlayfs) fn with_creator_credentials_fn<T>(
        &self,
        operation_fn: impl FnOnce() -> Result<T>,
    ) -> Result<T> {
        operation_fn()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in overlayfs) enum CredentialSource {
    Creator,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in overlayfs) struct UpperFilesystemCapabilities {
    can_store_private_xattr: bool,
    can_report_directory_type: bool,
    can_mknod_char: bool,
}

impl UpperFilesystemCapabilities {
    /// Probes the upper/workspace capabilities post-claim (writable mounts
    /// only, sleep-capable construction context).
    pub(super) fn probe(
        upper_inode: &Arc<dyn Inode>,
        workspace_inode: &Arc<dyn Inode>,
    ) -> Result<Self> {
        // The d_type and char-device probes create uniquely-named temp
        // entries in the workdir staging workspace and remove them on
        // success/failure.
        let can_store_private_xattr = Self::probe_private_xattr(upper_inode)?;
        let can_report_directory_type = Self::probe_d_type(workspace_inode)?;
        let can_mknod_char = Self::probe_mknod_char(workspace_inode)?;
        Ok(Self {
            can_store_private_xattr,
            can_report_directory_type,
            can_mknod_char,
        })
    }

    fn probe_private_xattr(upper_inode: &Arc<dyn Inode>) -> Result<bool> {
        let name = XattrName::try_from_full_name(TRUSTED_OVERLAY_UUID).ok_or_else(|| {
            Error::with_message(Errno::EINVAL, "invalid overlay xattr probe name")
        })?;
        let mut value = [0u8; OVERLAY_UUID_SIZE];
        let mut writer = VmWriter::from(value.as_mut_slice()).to_fallible();
        match upper_inode.get_xattr(name, &mut writer) {
            Ok(_) => Ok(true),
            Err(err) if err.error() == Errno::ENODATA => Ok(true),
            Err(err) if err.error() == Errno::ERANGE => Ok(true),
            Err(err) if err.error() == Errno::EOPNOTSUPP => Ok(false),
            Err(err) => Err(err),
        }
    }

    fn probe_d_type(workspace_inode: &Arc<dyn Inode>) -> Result<bool> {
        let d_type_probe_name = unique_temp_name(D_TYPE_PROBE_PREFIX);
        workspace_inode.create(&d_type_probe_name, InodeType::File, InodeMode::empty())?;
        let mut d_type_probe = DTypeProbeVisitor::new();
        let mut offset = 0;
        let d_type_scan_result = loop {
            match workspace_inode.readdir_at(offset, &mut d_type_probe) {
                Ok(0) => break Ok(()),
                Ok(visited) => offset += visited,
                Err(err) => break Err(err),
            }
        };
        match d_type_scan_result {
            Ok(()) => {
                workspace_inode.unlink(&d_type_probe_name)?;
                Ok(!d_type_probe.saw_unknown_non_dot)
            }
            Err(err) => {
                let _ = workspace_inode.unlink(&d_type_probe_name);
                Err(err)
            }
        }
    }

    fn probe_mknod_char(workspace_inode: &Arc<dyn Inode>) -> Result<bool> {
        let probe_name = unique_temp_name(CHAR_DEVICE_PROBE_PREFIX);
        match workspace_inode.mknod(&probe_name, InodeMode::empty(), MknodType::CharDevice(0)) {
            Ok(_) => {
                workspace_inode.unlink(&probe_name)?;
                Ok(true)
            }
            Err(err)
                if matches!(
                    err.error(),
                    Errno::EOPNOTSUPP | Errno::EPERM | Errno::EACCES
                ) =>
            {
                Ok(false)
            }
            Err(err) => Err(err),
        }
    }

    /// Consumed by the origin-record store.
    pub(in crate::fs::fs_impls::overlayfs) fn can_store_private_xattr(&self) -> bool {
        self.can_store_private_xattr
    }

    pub(super) fn can_report_directory_type(&self) -> bool {
        self.can_report_directory_type
    }

    /// Reports whether the workdir supports the classic whiteout char device
    /// `0:0`.
    pub(in crate::fs::fs_impls::overlayfs) fn can_mknod_char(&self) -> bool {
        self.can_mknod_char
    }
}

/// A [`DirentVisitor`] that records whether any non-dot entry reports
/// `InodeType::Unknown`.
///
/// The `readdir_at` interface requires a visitor; no existing implementation
/// captures entry types.
struct DTypeProbeVisitor {
    saw_unknown_non_dot: bool,
}

impl DTypeProbeVisitor {
    fn new() -> Self {
        Self {
            saw_unknown_non_dot: false,
        }
    }
}

impl DirentVisitor for DTypeProbeVisitor {
    fn visit(&mut self, name: &str, _ino: u64, type_: InodeType, _offset: usize) -> Result<()> {
        if !is_dot_or_dotdot(name) && type_ == InodeType::Unknown {
            self.saw_unknown_non_dot = true;
        }
        Ok(())
    }
}
