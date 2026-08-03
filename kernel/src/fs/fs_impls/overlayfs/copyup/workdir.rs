// SPDX-License-Identifier: MPL-2.0

//! The workdir temporary lifecycle — `P1-34`.
//!
//! This module owns the workdir-temp retry contract — `P1-34`.
//!
//! [`WorkdirTempRequest`] describes the closed set of staging operations and
//! [`WorkdirTemp`] preserves the successful name/inode pair. The
//! [`OverlayFs::create_workdir_temp`] entry retries only `EEXIST`, regenerates
//! the name for every attempt, and leaves publication or cleanup to its caller.
//!
//! The workdir is a private staging area on the upper filesystem, never a
//! layer: temporaries never enter lookup/readdir, unique naming keeps them out
//! of the overlay namespace, and a failure leaves a recorded cleanup
//! obligation, never a visible entry (invariant I7, BC-4 §40/§45.1). A temp
//! handle belongs only to the winner's copy-up transaction (BC-4 §40.2): it is
//! never returned to the VFS, never stored on the inode, and never a
//! page-cache forwarding target. The P1-35 claim guarantees no cross-mount
//! collision (a workdir cannot be claimed by two live mounts), so the
//! composite name needs only per-mount uniqueness.
//!
//! Lock contract (spec §3.0): workdir temp naming is uniqueness-based, not
//! lock-based — no Overlay lock is acquired or held by any method here, and
//! the underlying upper-filesystem calls run against that filesystem's own
//! locking (proven non-re-entrant into Overlay, spec §3.3 Hazard 2). The EROFS
//! gate precedes every workdir/upper side effect (I10): the private
//! [`OverlayFs::workdir_root`] resolver returns `Err(Errno::EROFS)` when no
//! writable claim exists (spec §2 Case 4).
//!
//! [`OverlayFs::workdir_root`] remains the single workdir-root claim resolver
//! of the overlayfs tree.

use alloc::format;

use crate::{
    fs::{
        file::{InodeMode, InodeType},
        fs_impls::overlayfs::mount::OverlayFs,
        utils::NAME_MAX,
        vfs::inode::{Inode, MknodType},
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
        source: Arc<dyn Inode>,
    },
}

/// A successful private workdir-temp creation.
pub(in crate::fs::fs_impls::overlayfs) struct WorkdirTemp {
    name: String,
    inode: Arc<dyn Inode>,
}

const MAX_WORKDIR_TEMP_CREATE_ATTEMPTS: usize = 8;

impl WorkdirTemp {
    pub(in crate::fs::fs_impls::overlayfs) fn name(&self) -> &str {
        &self.name
    }

    pub(in crate::fs::fs_impls::overlayfs) fn inode(&self) -> &Arc<dyn Inode> {
        &self.inode
    }

    pub(in crate::fs::fs_impls::overlayfs) fn into_parts(self) -> (String, Arc<dyn Inode>) {
        (self.name, self.inode)
    }
}

impl WorkdirTempRequest<'_> {
    fn create_in(&self, workdir: &Arc<dyn Inode>, temp_name: &str) -> Result<Arc<dyn Inode>> {
        match self {
            Self::Create { kind, mode } => workdir.create(temp_name, *kind, *mode),
            Self::Mknod { mode, node } => {
                let node = match node {
                    MknodType::NamedPipe => MknodType::NamedPipe,
                    MknodType::CharDevice(device_id) => MknodType::CharDevice(*device_id),
                    MknodType::BlockDevice(device_id) => MknodType::BlockDevice(*device_id),
                };
                workdir.mknod(temp_name, *mode, node)
            }
            Self::Link { source } => {
                workdir.link(source, temp_name)?;
                Ok(source.clone())
            }
        }
    }
}

impl OverlayFs {
    /// Generates a uniquely-named workdir temp name for a copy-up target
    /// (`P1-34`, meso-04 spec §4 `copyup/workdir.rs`).
    ///
    /// The composite is `#{target_name}#{parent_ino}#{serial}`: the target's
    /// publication name, the upper-parent real inode number ([`Inode::ino`]),
    /// and one per-mount saturating workdir serial
    /// ([`OverlayFs::workdir_temp_serial`]). The target-name component is
    /// capped so the composite stays within [`crate::fs::utils::NAME_MAX`]
    /// for any legal target name. The retry entry regenerates the name before
    /// each attempt as the collision backstop.
    pub(in crate::fs::fs_impls::overlayfs) fn generate_workdir_temp_name(
        &self,
        target_name: &str,
        upper_parent: &Arc<dyn Inode>,
    ) -> String {
        let parent_ino = upper_parent.ino();
        let serial = self.workdir_temp_serial();
        const TEMP_NAME_SEPARATORS: usize = 3;
        const U64_DEC_DIGITS_MAX: usize = 20;
        const TEMP_NAME_FIXED_OVERHEAD: usize = TEMP_NAME_SEPARATORS + 2 * U64_DEC_DIGITS_MAX;
        const TEMP_NAME_TARGET_CAP: usize = NAME_MAX - TEMP_NAME_FIXED_OVERHEAD;
        let target_component =
            &target_name[..target_name.floor_char_boundary(TEMP_NAME_TARGET_CAP)];
        format!("#{target_component}#{parent_ino}#{serial}")
    }

    /// Creates a private workdir temp object for copy-up staging (`P1-34`).
    ///
    /// Each attempt generates a fresh name and dispatches the same typed
    /// request. Only `EEXIST` retries; on exhaustion the final underlying
    /// `EEXIST` is returned, while all other errors propagate unchanged.
    pub(in crate::fs::fs_impls::overlayfs) fn create_workdir_temp(
        &self,
        target_name: &str,
        upper_parent: &Arc<dyn Inode>,
        request: WorkdirTempRequest<'_>,
    ) -> Result<WorkdirTemp> {
        let workdir = self.workdir_root()?;
        let mut final_eexist = None;

        for _ in 0..MAX_WORKDIR_TEMP_CREATE_ATTEMPTS {
            let name = self.generate_workdir_temp_name(target_name, upper_parent);
            match request.create_in(&workdir, &name) {
                Ok(inode) => return Ok(WorkdirTemp { name, inode }),
                Err(err) if err.error() == Errno::EEXIST => final_eexist = Some(err),
                Err(err) => return Err(err),
            }
        }

        match final_eexist {
            Some(err) => Err(err),
            None => unreachable!("the nonzero retry bound must attempt workdir creation"),
        }
    }

    /// Removes a workdir temp object (`P1-34`).
    ///
    /// The recipe calls this best-effort on any pre-publication failure; a
    /// cleanup failure propagates as the recorded P3-09 workdir-cleanup
    /// obligation and never becomes a visible namespace entry (invariant I7,
    /// BC-4 §45.1).
    pub(in crate::fs::fs_impls::overlayfs) fn cleanup_workdir_temp(
        &self,
        temp_name: &str,
    ) -> Result<()> {
        self.workdir_root()?.unlink(temp_name)
    }

    /// Resolves the pinned workdir root inode of this writable mount.
    ///
    /// The single workdir-root claim resolver of the overlayfs tree (wave-4
    /// round-2 repair item 5): every workdir-root consumer — the three
    /// helpers in this file, `OverlayInode::workdir_root`
    /// (`copyup/promote.rs`), the meso-06 dir/ recipes, and the two
    /// `dir/whiteout.rs` sites — funnels through this one entry, so the
    /// claim-resolution shape and the EROFS error text exist exactly once.
    /// The workdir root is reachable via the meso-01 `claims()` seam (spec §2
    /// pre-condition P1-34: "the workdir root and upper parent real inode are
    /// reachable via meso-01 `claims()`"). A missing claim means the mount is
    /// effectively read-only (or the claims were released), so the EROFS gate
    /// fires here — before any workdir/upper side effect (I10, spec §2
    /// Case 4).
    pub(in crate::fs::fs_impls::overlayfs) fn workdir_root(&self) -> Result<Arc<dyn Inode>> {
        let claim = self.claims().ok_or_else(|| {
            Error::with_message(Errno::EROFS, "the overlay mount has no workdir claim")
        })?;
        Ok(claim.workdir_inode().clone())
    }
}
