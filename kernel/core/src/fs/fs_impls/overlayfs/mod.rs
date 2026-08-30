// SPDX-License-Identifier: MPL-2.0

//! Overlayfs filesystem implementation for Asterinas.
//!
//! This module is the entry point for overlay filesystem support: `init`
//! registers [`fs_type::OverlayFsType`], after which the VFS can mount
//! overlays and access them through the standard filesystem trait
//! interfaces. A mount merges one writable upper layer with one or more
//! read-only lower layers.
//!
//! # References
//!
//! - Overlay filesystem:
//!   <https://www.kernel.org/doc/html/latest/filesystems/overlayfs.html>

#![short_vis_path::add(overlayfs)]

mod fs;
mod fs_type;
mod inode;
mod layer;
mod real;

use alloc::format;

use ostd::task::{CurrentTask, Task};

use crate::{
    fs::{
        file::InodeType,
        utils::NAME_MAX,
        vfs::{
            inode::{Inode, MknodType},
            path::{self, Path},
        },
    },
    prelude::*,
    process::posix_thread::{AsPosixThread, PosixThread},
};

/// Runs `operation_fn` with the current task's POSIX thread.
///
/// `None` means a kernel-internal operation (no task / no POSIX thread);
/// callers map `None` to their own default.
pub(in overlayfs) fn with_current_posix_thread<T>(
    operation_fn: impl FnOnce(&CurrentTask, &PosixThread) -> T,
) -> Option<T> {
    let task = Task::current()?;
    let posix_thread = task.as_posix_thread()?;
    Some(operation_fn(&task, posix_thread))
}

/// Returns the pinned child path `parent_path`/`name` through the base VFS
/// dentry lookup; lookup errors propagate unchanged.
pub(in overlayfs) fn lookup_child_path(parent_path: &Path, name: &str) -> Result<Path> {
    let child_dentry = parent_path
        .dentry()
        .as_dir_dentry_or_err()?
        .lookup_child(name)?;
    Ok(Path::new(parent_path.mount_node().clone(), child_dentry))
}

/// Collects the non-`.`/non-`..` child names of a real directory inode,
/// draining `readdir_at` until it reports no consumed entries.
pub(in overlayfs) fn read_child_names(real_dir: &Arc<dyn Inode>) -> Result<Vec<String>> {
    let mut names: Vec<String> = Vec::new();
    let mut offset = 0;
    loop {
        match real_dir.readdir_at(offset, &mut names)? {
            0 => break,
            visited => offset += visited,
        }
    }
    names.retain(|name| !path::is_dot_or_dotdot(name));
    Ok(names)
}

/// Maps the `mknod` kind request to the overlay-visible object type.
///
/// `MknodType` has no `InodeType` conversion, so this match is the only
/// mapping.
pub(in overlayfs) fn mknod_object_type(mknod: &MknodType) -> InodeType {
    match mknod {
        MknodType::NamedPipe => InodeType::NamedPipe,
        MknodType::CharDevice(_) => InodeType::CharDevice,
        MknodType::BlockDevice(_) => InodeType::BlockDevice,
    }
}

/// The fixed-length random hex suffix of a workdir temp name (8 CSPRNG
/// bytes rendered as 16 hex digits).
const TEMP_NAME_RANDOM_SUFFIX_LEN: usize = 16;

/// Generates a uniquely-named workdir temp name for `target_name`.
///
/// Uniqueness comes from a CSPRNG random suffix rather than a serial;
/// `create_workdir_temp` already retries `EEXIST` with a fresh name, so a
/// collision is harmless. The target component is capped so the composite
/// stays within [`crate::fs::utils::NAME_MAX`] for any legal target name.
pub(in overlayfs) fn workdir_temp_name(target_name: &str) -> String {
    let mut random_bytes = [0u8; 8];
    crate::util::random::getrandom(&mut random_bytes);
    const TEMP_NAME_SEPARATORS: usize = 2;
    const TEMP_NAME_TARGET_CAP: usize =
        NAME_MAX - TEMP_NAME_SEPARATORS - TEMP_NAME_RANDOM_SUFFIX_LEN;
    let target_component = &target_name[..target_name.floor_char_boundary(TEMP_NAME_TARGET_CAP)];
    format!(
        "#{target_component}#{:016x}",
        u64::from_le_bytes(random_bytes)
    )
}

// The persisted overlay UUID record lives in the xattr module's record
// table (`inode/xattr.rs`): `overlay_record_name(OverlayRecordName::Uuid,
// prefix)` replaces the former `TRUSTED_OVERLAY_UUID`/`uuid_xattr_name`
// root items.

pub(super) fn init() {
    crate::fs::vfs::registry::register(&fs_type::OverlayFsType).unwrap();
}
