// SPDX-License-Identifier: MPL-2.0

//! The workdir temporary lifecycle.
//!
//! [`WorkdirTemp`] preserves the successful name/inode pair, and
//! [`OverlayFs::create_workdir_temp`] retries only `EEXIST`, regenerating the
//! name for each attempt and leaving publication or cleanup to its caller.
//! Naming is uniqueness-based.
//!
//! ## References
//!
//! - Linux `ofs->workdir` dentry-ref parity:
//!   <https://elixir.bootlin.com/linux/latest/source/fs/overlayfs/super.c#L663-L803>

use alloc::format;

use crate::{
    fs::{
        file::{InodeMode, InodeType},
        fs_impls::overlayfs::{dir::mknod_object_type, mount::OverlayFs},
        utils::NAME_MAX,
        vfs::{
            inode::{Inode, MknodType},
            path::Path,
        },
    },
    prelude::*,
};

/// The operation to retry while creating a private workdir temp.
pub(in crate::fs::fs_impls::overlayfs) enum WorkdirTempRequest<'a> {
    Create {
        kind: InodeType,
        mode: InodeMode,
    },
    Mknod {
        mode: InodeMode,
        node: &'a MknodType,
    },
    Link {
        source: Path,
    },
}

/// A successful private workdir-temp creation; the handle carries the
/// request-derived [`InodeType`] needed by the kind-aware cleanup dispatcher.
pub(in crate::fs::fs_impls::overlayfs) struct WorkdirTemp {
    name: String,
    path: Path,
    kind: InodeType,
}

const MAX_WORKDIR_TEMP_CREATE_ATTEMPTS: usize = 8;

impl WorkdirTemp {
    pub(in crate::fs::fs_impls::overlayfs) fn name(&self) -> &str {
        &self.name
    }

    pub(in crate::fs::fs_impls::overlayfs) fn kind(&self) -> InodeType {
        self.kind
    }

    /// Returns the real inode of the staged workdir temp.
    ///
    /// Derived from the dentry-anchored [`Path`], so the inode and the path
    /// always refer to the same workdir object.
    pub(in crate::fs::fs_impls::overlayfs) fn inode(&self) -> &Arc<dyn Inode> {
        self.path.inode()
    }

    /// Consumes the handle into its `(name, path)` parts; the dentry-anchored
    /// path stays valid after the workdir-to-upper rename and doubles as the
    /// published upper object's path.
    pub(in crate::fs::fs_impls::overlayfs) fn into_parts(self) -> (String, Path) {
        (self.name, self.path)
    }
}

impl WorkdirTempRequest<'_> {
    fn kind(&self) -> InodeType {
        match self {
            Self::Create { kind, .. } => *kind,
            Self::Mknod { node, .. } => mknod_object_type(node),
            Self::Link { source } => source.inode().type_(),
        }
    }

    fn create_in(&self, workdir_path: &Path, temp_name: &str) -> Result<Path> {
        match self {
            Self::Create { kind, mode } => workdir_path.new_fs_child(temp_name, *kind, *mode),
            Self::Mknod { mode, node } => {
                let node = match node {
                    MknodType::NamedPipe => MknodType::NamedPipe,
                    MknodType::CharDevice(device_id) => MknodType::CharDevice(*device_id),
                    MknodType::BlockDevice(device_id) => MknodType::BlockDevice(*device_id),
                };
                workdir_path.mknod(temp_name, *mode, node)
            }
            Self::Link { source } => {
                workdir_path.link(source, temp_name)?;
                Ok(Path::new(
                    workdir_path.mount_node().clone(),
                    workdir_path
                        .dentry()
                        .as_dir_dentry_or_err()?
                        .lookup_child(temp_name)?,
                ))
            }
        }
    }
}

impl OverlayFs {
    /// Generates a uniquely-named workdir temp name for a copy-up target.
    ///
    /// The target-name component is capped so the composite stays within
    /// [`crate::fs::utils::NAME_MAX`] for any legal target name.
    pub(in crate::fs::fs_impls::overlayfs) fn generate_workdir_temp_name(
        &self,
        target_name: &str,
        upper_parent: &Path,
    ) -> String {
        let parent_ino = upper_parent.inode().ino();
        let serial = self.workdir_temp_serial();
        const TEMP_NAME_SEPARATORS: usize = 3;
        const U64_DEC_DIGITS_MAX: usize = 20;
        const TEMP_NAME_FIXED_OVERHEAD: usize = TEMP_NAME_SEPARATORS + 2 * U64_DEC_DIGITS_MAX;
        const TEMP_NAME_TARGET_CAP: usize = NAME_MAX - TEMP_NAME_FIXED_OVERHEAD;
        let target_component =
            &target_name[..target_name.floor_char_boundary(TEMP_NAME_TARGET_CAP)];
        format!("#{target_component}#{parent_ino}#{serial}")
    }

    /// Creates a private workdir temp object for copy-up staging, retrying
    /// only `EEXIST` with a fresh name and propagating all other errors.
    pub(in crate::fs::fs_impls::overlayfs) fn create_workdir_temp(
        &self,
        target_name: &str,
        upper_parent_path: &Path,
        request: WorkdirTempRequest<'_>,
    ) -> Result<WorkdirTemp> {
        let workdir_path = self.workdir_root_path()?;
        let mut final_eexist = None;

        for _ in 0..MAX_WORKDIR_TEMP_CREATE_ATTEMPTS {
            let name = self.generate_workdir_temp_name(target_name, upper_parent_path);
            match request.create_in(&workdir_path, &name) {
                Ok(path) => {
                    return Ok(WorkdirTemp {
                        name,
                        path,
                        kind: request.kind(),
                    });
                }
                Err(err) if err.error() == Errno::EEXIST => final_eexist = Some(err),
                Err(err) => return Err(err),
            }
        }

        match final_eexist {
            Some(err) => Err(err),
            None => unreachable!("the nonzero retry bound must attempt workdir creation"),
        }
    }

    /// Removes a workdir temp object, dispatching on its known kind.
    ///
    /// Directories are removed with `rmdir` and every other kind with
    /// `unlink`, because the underlying filesystem refuses to `unlink` a
    /// directory (`EISDIR`) and would otherwise leak directory-temp residue.
    pub(in crate::fs::fs_impls::overlayfs) fn cleanup_workdir_temp(
        &self,
        temp_name: &str,
        kind: InodeType,
    ) -> Result<()> {
        let workdir_path = self.workdir_root_path()?;
        if kind.is_directory() {
            workdir_path.rmdir(temp_name)
        } else {
            workdir_path.unlink(temp_name)
        }
    }

    /// Resolves the pinned workdir staging workspace path of this writable
    /// mount.
    ///
    /// The path is fixed at mount time and never re-resolves the `work` name;
    /// a missing claim or unprepared workspace means the mount is effectively
    /// read-only, so this entry returns `EROFS` before any workdir side effect.
    pub(in crate::fs::fs_impls::overlayfs) fn workdir_root_path(&self) -> Result<Path> {
        let claim = self.claims().ok_or_else(|| {
            Error::with_message(Errno::EROFS, "the overlay mount has no workdir claim")
        })?;
        Ok(claim.workdir_workspace_path()?.clone())
    }
}
