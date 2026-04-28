// SPDX-License-Identifier: MPL-2.0

use super::*;
use crate::process::credentials::capabilities::CapSet;

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
            let (state, block_device, boot_region, _anomaly, _upcase_table, _options) =
                fs.admitted_lookup_state()?;
            let root_inode = state
                .as_ref()
                .ok_or(MountVolumeStateError::UnpublishedState)?
                .root_inode
                .clone();
            let label = root_inode.read_root_directory(
                &block_device,
                &boot_region,
                direntry::read_volume_label,
            )?;
            let label = match label {
                Some(label) => String::from_utf16(&label)
                    .map(Some)
                    .map_err(|_| MountVolumeStateError::InvalidOnDiskLayout.into())?,
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
            let (state, block_device, boot_region, _anomaly, _upcase_table, _options) =
                fs.admitted_mutation_state()?;
            let root_inode = state
                .as_ref()
                .ok_or(MountVolumeStateError::UnpublishedState)?
                .root_inode
                .clone();

            root_inode
                .rewrite_root_directory(&block_device, &boot_region, |directory_bytes| {
                    direntry::write_volume_label(directory_bytes, admitted_label.as_deref())
                })
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

pub(super) fn admit_forced_shutdown(
    fs: &ExfatFs,
) -> core::result::Result<(), MountVolumeStateError> {
    let mut state = fs.state.write();
    let publication = state
        .as_mut()
        .ok_or(MountVolumeStateError::UnpublishedState)?;
    publication.forced_shutdown = true;
    Ok(())
}

pub(super) fn administrative_trim_free_space(fs: &ExfatFs) -> Result<()> {
    let state = fs.state.write();
    let publication = state
        .as_ref()
        .ok_or(MountVolumeStateError::UnpublishedState)?;
    if publication.forced_shutdown {
        return Err(Error::new(Errno::EIO));
    }
    if publication.flags.contains(FsFlags::RDONLY) {
        return Err(MountVolumeStateError::ReadOnlyConflict.into());
    }

    Err(Error::new(Errno::EOPNOTSUPP))
}
