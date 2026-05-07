// SPDX-License-Identifier: MPL-2.0

//! Implements volume-administration operations such as forced shutdown and volume labels.

use alloc::{string::String, vec::Vec};

use super::{
    direntry::DIRECTORY_ENTRY_SIZE, fs::ExfatFs, invalid_on_disk_layout, invalid_operation_input,
    not_mounted, read_only_conflict,
};
use crate::{
    fs::vfs::file_system::FsFlags, prelude::*, process::credentials::capabilities::CapSet,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum VolumeAdminRequest {
    ForceShutdown,
    WriteVolumeLabel(Option<String>),
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
            set_forced_shutdown(fs)
        }
        VolumeAdminRequest::WriteVolumeLabel(label) => {
            ensure_privileged_fn()?;
            write_volume_label(fs, label)
        }
    }
}

pub(super) fn read_volume_label(fs: &ExfatFs) -> Result<Option<String>> {
    let block_device = fs.immutable_block_device();
    let boot_region = fs.immutable_boot_region();
    let root_inode = fs
        .root_inode
        .read()
        .as_ref()
        .ok_or_else(not_mounted)?
        .clone();
    let directory_bytes = root_inode.read_root_directory_bytes(&block_device, &boot_region)?;
    let label = decode_volume_label_entry(&directory_bytes)?;
    match label {
        Some(label) => String::from_utf16(&label)
            .map(Some)
            .map_err(|_| invalid_on_disk_layout()),
        None => Ok(None),
    }
}

pub(super) fn write_volume_label(fs: &ExfatFs, label: Option<String>) -> Result<()> {
    let label = match label {
        None => None,
        Some(label) if label.is_empty() => None,
        Some(label) => {
            let label: Vec<u16> = label.encode_utf16().collect();
            if label.len() > VOLUME_LABEL_MAX_CODE_UNITS {
                return_errno_with_message!(Errno::EINVAL, "invalid exFAT volume label");
            }
            Some(label)
        }
    };
    let mutation_mount_state = fs.mount_state_write_guard()?;
    let block_device = fs.immutable_block_device();
    let boot_region = fs.immutable_boot_region();
    if mutation_mount_state.forced_shutdown {
        return_errno!(Errno::EIO);
    }
    if mutation_mount_state
        .options
        .fs_flags
        .contains(FsFlags::RDONLY)
    {
        return Err(read_only_conflict());
    }
    let root_inode = fs
        .root_inode
        .read()
        .as_ref()
        .ok_or_else(not_mounted)?
        .clone();
    let mut directory_bytes = root_inode.read_root_directory_bytes(&block_device, &boot_region)?;
    encode_volume_label_entry(&mut directory_bytes, label.as_deref())?;
    root_inode.rewrite_root_directory_bytes(&block_device, &boot_region, &directory_bytes)?;
    Ok(())
}

pub(super) fn set_forced_shutdown(fs: &ExfatFs) -> Result<()> {
    let mut mount_state = fs.mount_state.write();
    let mount_state = mount_state.as_mut().ok_or_else(not_mounted)?;
    mount_state.forced_shutdown = true;
    Ok(())
}

fn decode_volume_label_entry(directory_bytes: &[u8]) -> Result<Option<Vec<u16>>> {
    let Some(entry_index) = locate_volume_label_entry(directory_bytes)? else {
        return Ok(None);
    };
    let entry_offset = entry_index
        .checked_mul(DIRECTORY_ENTRY_SIZE)
        .ok_or_else(invalid_on_disk_layout)?;
    let entry_end = entry_offset
        .checked_add(DIRECTORY_ENTRY_SIZE)
        .ok_or_else(invalid_on_disk_layout)?;
    let entry = directory_bytes
        .get(entry_offset..entry_end)
        .ok_or_else(invalid_on_disk_layout)?;
    let label_length = usize::from(entry[VOLUME_LABEL_ENTRY_LENGTH_OFFSET]);
    if label_length > VOLUME_LABEL_MAX_CODE_UNITS {
        return Err(invalid_on_disk_layout());
    }
    if label_length == 0 {
        return Ok(None);
    }

    let label_end = VOLUME_LABEL_UTF16_OFFSET
        .checked_add(
            label_length
                .checked_mul(2)
                .ok_or_else(invalid_on_disk_layout)?,
        )
        .ok_or_else(invalid_on_disk_layout)?;
    let mut label = Vec::with_capacity(label_length);
    for code_unit_bytes in entry[VOLUME_LABEL_UTF16_OFFSET..label_end].chunks_exact(2) {
        label.push(u16::from_le_bytes([code_unit_bytes[0], code_unit_bytes[1]]));
    }
    Ok(Some(label))
}

fn encode_volume_label_entry(directory_bytes: &mut [u8], label: Option<&[u16]>) -> Result<()> {
    let existing_entry_index = locate_volume_label_entry(directory_bytes)?;
    let Some(label) = label.filter(|label| !label.is_empty()) else {
        if let Some(existing_entry_index) = existing_entry_index {
            let entry_offset = existing_entry_index
                .checked_mul(DIRECTORY_ENTRY_SIZE)
                .ok_or_else(invalid_on_disk_layout)?;
            let entry_end = entry_offset
                .checked_add(DIRECTORY_ENTRY_SIZE)
                .ok_or_else(invalid_on_disk_layout)?;
            let entry = directory_bytes
                .get_mut(entry_offset..entry_end)
                .ok_or_else(invalid_on_disk_layout)?;
            entry[0] &= !ENTRY_TYPE_IN_USE_BIT;
        }
        return Ok(());
    };
    if label.len() > VOLUME_LABEL_MAX_CODE_UNITS {
        return Err(invalid_operation_input());
    }

    let destination_entry_index = match existing_entry_index {
        Some(existing_entry_index) => existing_entry_index,
        None => {
            let mut destination_entry_index = None;
            for (entry_index, entry) in directory_bytes
                .chunks_exact(DIRECTORY_ENTRY_SIZE)
                .enumerate()
            {
                if entry[0] == END_OF_DIRECTORY_ENTRY_TYPE || entry[0] & ENTRY_TYPE_IN_USE_BIT == 0
                {
                    destination_entry_index = Some(entry_index);
                    break;
                }
            }
            destination_entry_index.ok_or_else(invalid_on_disk_layout)?
        }
    };

    let mut encoded_entry = [0u8; DIRECTORY_ENTRY_SIZE];
    encoded_entry[0] = VOLUME_LABEL_ENTRY_TYPE;
    encoded_entry[VOLUME_LABEL_ENTRY_LENGTH_OFFSET] =
        u8::try_from(label.len()).map_err(|_| invalid_operation_input())?;
    for (index, code_unit) in label.iter().enumerate() {
        let code_unit_offset = VOLUME_LABEL_UTF16_OFFSET
            .checked_add(index.checked_mul(2).ok_or_else(invalid_on_disk_layout)?)
            .ok_or_else(invalid_on_disk_layout)?;
        encoded_entry[code_unit_offset..code_unit_offset + 2]
            .copy_from_slice(&code_unit.to_le_bytes());
    }

    let entry_offset = destination_entry_index
        .checked_mul(DIRECTORY_ENTRY_SIZE)
        .ok_or_else(invalid_on_disk_layout)?;
    let entry_end = entry_offset
        .checked_add(DIRECTORY_ENTRY_SIZE)
        .ok_or_else(invalid_on_disk_layout)?;
    let entry = directory_bytes
        .get_mut(entry_offset..entry_end)
        .ok_or_else(invalid_on_disk_layout)?;
    entry.copy_from_slice(&encoded_entry);
    Ok(())
}

fn locate_volume_label_entry(directory_bytes: &[u8]) -> Result<Option<usize>> {
    if directory_bytes.len() % DIRECTORY_ENTRY_SIZE != 0 {
        return Err(invalid_on_disk_layout());
    }

    let mut label_entry_index = None;
    for (entry_index, entry) in directory_bytes
        .chunks_exact(DIRECTORY_ENTRY_SIZE)
        .enumerate()
    {
        if entry[0] == END_OF_DIRECTORY_ENTRY_TYPE {
            break;
        }
        if entry[0] == VOLUME_LABEL_ENTRY_TYPE {
            if label_entry_index.replace(entry_index).is_some() {
                return Err(invalid_on_disk_layout());
            }
        }
    }
    Ok(label_entry_index)
}

const END_OF_DIRECTORY_ENTRY_TYPE: u8 = 0x00;
const VOLUME_LABEL_ENTRY_TYPE: u8 = 0x83;
const ENTRY_TYPE_IN_USE_BIT: u8 = 0x80;
const VOLUME_LABEL_ENTRY_LENGTH_OFFSET: usize = 1;
const VOLUME_LABEL_UTF16_OFFSET: usize = 2;
const VOLUME_LABEL_MAX_CODE_UNITS: usize = 11;
