// SPDX-License-Identifier: MPL-2.0

#![short_vis_path::add(overlayfs)]
//! Mount build-time subtree: options, layer assembly, claims, and policy.
//!
//! This module contains only mount construction state. The per-mount
//! overlay filesystem object lives in `fs::mod` and VFS registration lives
//! in the top-level `fs_type` module.

pub(super) mod capabilities;
pub(in overlayfs) mod inuse;
mod layer_parts;
pub(super) mod options;

use self::{
    capabilities::UpperFilesystemCapabilities,
    inuse::{UpperWorkdirInuse, Uuid},
    layer_parts::{resolve_root_path, verify_inode_instance_stability},
    options::MountOptions,
};
use super::{
    OverlayFs,
    policy::{MountPolicy, UuidMode},
};
use crate::{
    fs::{
        fs_impls::overlayfs::{
            inode::{IdentityPolicy, InodeCache, WhiteoutCache, collect_layer_devs},
            layer::LayerStack,
        },
        pseudofs::AnonDeviceId,
        vfs::{
            file_system::{FsEventSubscriberStats, FsFlags},
            registry::FsCreationCtx,
        },
    },
    prelude::*,
};

impl OverlayFs {
    /// Constructs and publishes a fully prepared overlay filesystem.
    pub(in overlayfs) fn new(fs_creation_ctx: &FsCreationCtx) -> Result<Arc<Self>> {
        let options = MountOptions::parse(fs_creation_ctx.args(), fs_creation_ctx.flags())?;

        let layer_stack = LayerStack::assemble(
            options.upper_dir.clone(),
            options.lower_dirs.clone(),
            options.is_forced_read_only,
        )?;

        let is_effective_read_only = match &layer_stack.upper {
            Some(upper) => {
                options.is_forced_read_only || upper.fs.flags().contains(FsFlags::RDONLY)
            }
            None => true,
        };

        let mut upper_workdir_pair = None;
        let mut upper_capabilities = None;
        let mut uuid = None;
        if let Some(upper) = &layer_stack.upper {
            // The parse invariant guarantees both option strings are present
            // for an upper-backed overlay; the conversions below are defensive.
            let upper_dir = options.upper_dir.as_deref().ok_or_else(|| {
                Error::with_message(Errno::EINVAL, "internal error: missing upperdir option")
            })?;
            let work_dir = options.work_dir.as_deref().ok_or_else(|| {
                Error::with_message(Errno::EINVAL, "internal error: missing workdir option")
            })?;

            let upper_path = resolve_root_path(upper_dir)?;
            let workdir_path = resolve_root_path(work_dir)?;
            UpperWorkdirInuse::validate_pair(&upper_path, &workdir_path)?;
            // The workdir is not a layer, so `assemble`'s pairwise check
            // cannot cover it.
            layer_stack.validate_workdir_against_lowers(&workdir_path)?;
            verify_inode_instance_stability(upper_dir, upper.root_path.upgrade()?.inode())?;
            verify_inode_instance_stability(work_dir, workdir_path.inode())?;

            let uuid_mode = options.uuid_mode.unwrap_or(UuidMode::Auto);
            let identity = if is_effective_read_only {
                Ok(Uuid::generate())
            } else {
                UpperWorkdirInuse::determine_identity(upper.root_path.upgrade()?.inode(), uuid_mode)
            }?;

            let mut claimed_pair = UpperWorkdirInuse::claim(
                upper.root_path.upgrade()?.inode().clone(),
                workdir_path.inode().clone(),
                identity,
            )?;

            if !is_effective_read_only {
                claimed_pair.prepare_workdir(&workdir_path)?;

                let capabilities = UpperFilesystemCapabilities::probe(
                    upper.root_path.upgrade()?.inode(),
                    claimed_pair.workdir_workspace()?,
                )?;
                let is_uuid_effective = capabilities.validate_uuid_support(uuid_mode)?;

                if is_uuid_effective {
                    match claimed_pair.persist_identity() {
                        Ok(()) => {
                            uuid = Some(identity);
                        }
                        Err(persist_err) => match uuid_mode {
                            UuidMode::On => {
                                return_errno_with_message!(
                                    Errno::EOPNOTSUPP,
                                    "failed to persist the overlay uuid"
                                );
                            }
                            UuidMode::Auto => {
                                warn!(
                                    "overlay uuid persistence failed; degrading to not-effective: {:?}",
                                    persist_err
                                );
                            }
                            UuidMode::Off | UuidMode::Null => {}
                        },
                    }
                }

                upper_capabilities = Some(capabilities);
            }
            upper_workdir_pair = Some(claimed_pair);
        }

        let policy =
            MountPolicy::assemble(is_effective_read_only, &options, uuid, upper_capabilities);

        let anon_device_id = AnonDeviceId::acquire().ok_or_else(|| {
            Error::with_message(
                Errno::ENOSPC,
                "no anonymous device ID is available for the overlay mount",
            )
        })?;
        let overlay_dev_id = anon_device_id.id();

        let (layer_devs, upper_layer_dev_index) = collect_layer_devs(&layer_stack);

        let identity = IdentityPolicy::new(
            overlay_dev_id,
            &layer_devs,
            upper_layer_dev_index,
            IdentityPolicy::XINO_SHIFT,
            policy.xino_mode(),
        )?;

        let inodes = InodeCache::new();

        let overlay_fs = Arc::new_cyclic(move |weak| OverlayFs {
            layer_stack,
            upper_workdir_pair,
            policy,
            fs_event_stats: FsEventSubscriberStats::new(),
            self_weak: weak.clone(),
            inodes,
            identity,
            _anon_device_id: anon_device_id,
            whiteout_cache: Mutex::new(WhiteoutCache::new()),
        });
        Ok(overlay_fs)
    }
}
