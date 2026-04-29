// SPDX-License-Identifier: MPL-2.0

use alloc::{string::String, vec::Vec};

use super::{
    direntry,
    fs::{ExfatFs, ExfatFsError},
};
use crate::{
    fs::vfs::file_system::FsFlags, prelude::*, process::credentials::capabilities::CapSet,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct VolumeIdentityEntries {
    pub(super) guid: Option<[u8; 16]>,
    pub(super) label: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum VolumeIdentityQuery {
    Guid,
    Label,
    LabelAndGuid,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum VolumeIdentityUpdate {
    Guid(Option<[u8; 16]>),
    Label(Option<String>),
    LabelAndGuid {
        guid: Option<[u8; 16]>,
        label: Option<String>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum VolumeAdminRequest {
    ForceShutdown,
    TrimFreeSpace,
    UpdateIdentity(VolumeIdentityUpdate),
}

pub(super) fn handle_volume_admin_request(
    fs: &ExfatFs,
    request: VolumeAdminRequest,
    ctx: &Context,
) -> Result<()> {
    let is_privileged = ctx
        .posix_thread
        .credentials()
        .effective_capset()
        .contains(CapSet::SYS_ADMIN);
    let ensure_privileged_fn = || {
        if is_privileged {
            return Ok(());
        }
        return_errno_with_message!(
            Errno::EPERM,
            "exFAT volume administration requires SYS_ADMIN"
        )
    };
    match request {
        VolumeAdminRequest::ForceShutdown => {
            ensure_privileged_fn()?;
            admit_forced_shutdown(fs).map_err(Error::from)
        }
        VolumeAdminRequest::TrimFreeSpace => {
            ensure_privileged_fn()?;
            administrative_trim_free_space(fs)
        }
        VolumeAdminRequest::UpdateIdentity(update) => {
            ensure_privileged_fn()?;
            update_volume_identity(fs, update)
        }
    }
}

pub(super) fn query_volume_identity(
    fs: &ExfatFs,
    query: VolumeIdentityQuery,
) -> Result<VolumeIdentityEntries> {
    match query {
        VolumeIdentityQuery::Label => {
            let admission = fs.admitted_lookup_state()?;
            let root_inode = admission
                .state_guard
                .as_ref()
                .ok_or(ExfatFsError::UnpublishedState)?
                .root_inode
                .clone();
            let label = root_inode.read_root_directory(
                &admission.block_device,
                &admission.boot_region,
                direntry::read_volume_label,
            )?;
            let label = match label {
                Some(label) => String::from_utf16(&label)
                    .map(Some)
                    .map_err(|_| Error::from(ExfatFsError::InvalidOnDiskLayout))?,
                None => None,
            };
            Ok(VolumeIdentityEntries { guid: None, label })
        }
        VolumeIdentityQuery::Guid | VolumeIdentityQuery::LabelAndGuid => {
            return_errno_with_message!(
                Errno::EOPNOTSUPP,
                "exFAT volume GUID administration is not supported"
            );
        }
    }
}

pub(super) fn update_volume_identity(fs: &ExfatFs, update: VolumeIdentityUpdate) -> Result<()> {
    match update {
        VolumeIdentityUpdate::Label(label) => {
            let admitted_label = match label {
                None => None,
                Some(label) if label.is_empty() => None,
                Some(label) => {
                    let admitted_label: Vec<u16> = label.encode_utf16().collect();
                    if admitted_label.len() > 11 {
                        return_errno_with_message!(Errno::EINVAL, "invalid exFAT volume label");
                    }
                    Some(admitted_label)
                }
            };
            let admission = fs.admitted_mutation_state()?;
            if admission.forced_shutdown {
                return_errno!(Errno::EIO);
            }
            if admission.options.fs_flags.contains(FsFlags::RDONLY) {
                return Err(ExfatFsError::ReadOnlyConflict.into());
            }
            let root_inode = admission
                .state_guard
                .as_ref()
                .ok_or(ExfatFsError::UnpublishedState)?
                .root_inode
                .clone();

            root_inode
                .rewrite_root_directory(
                    &admission.block_device,
                    &admission.boot_region,
                    |directory_bytes| {
                        direntry::write_volume_label(directory_bytes, admitted_label.as_deref())
                    },
                )
                .map_err(Error::from)
        }
        VolumeIdentityUpdate::Guid(_) | VolumeIdentityUpdate::LabelAndGuid { .. } => {
            return_errno_with_message!(
                Errno::EOPNOTSUPP,
                "exFAT volume GUID administration is not supported"
            );
        }
    }
}

pub(super) fn admit_forced_shutdown(fs: &ExfatFs) -> core::result::Result<(), ExfatFsError> {
    let mut state = fs.state.write();
    let publication = state.as_mut().ok_or(ExfatFsError::UnpublishedState)?;
    publication.forced_shutdown = true;
    Ok(())
}

pub(super) fn administrative_trim_free_space(fs: &ExfatFs) -> Result<()> {
    let state = fs.state.write();
    let publication = state.as_ref().ok_or(ExfatFsError::UnpublishedState)?;
    if publication.forced_shutdown {
        return Err(Error::new(Errno::EIO));
    }
    if publication.flags.contains(FsFlags::RDONLY) {
        return Err(ExfatFsError::ReadOnlyConflict.into());
    }

    Err(Error::new(Errno::EOPNOTSUPP))
}
