// SPDX-License-Identifier: MPL-2.0

//! Construction orchestration for the overlay filesystem ([`OverlayFs::new`]).
//!
//! [`OverlayFs::new`] is the single constructor that builds the mount
//! resource/policy state in this order:
//!
//! - Parse options and capture creator credentials.
//! - Assemble and validate/claim the layer/upper/workdir state.
//! - On writable mounts, prepare the workdir, probe capabilities, and
//!   persist the effective UUID.
//! - Assemble [`MountPolicy`].
//! - Wire projection state and publish the `Arc<OverlayFs>`.
//!
//! Failure releases claimed resources via RAII.
//!
//! # References
//!
//! - <https://elixir.bootlin.com/linux/v7.0/source/fs/overlayfs/super.c#L1545>
//!   (Linux `ovl_fill_super` mount orchestration)
//! - <https://elixir.bootlin.com/linux/v7.0/source/fs/overlayfs/super.c#L667>
//!   (Linux `ovl_make_workdir`)

use core::sync::atomic::AtomicU64;

use super::{
    OVERLAY_FS_NAME,
    claims::{self, OverlayUuid, UpperWorkdirClaim},
    layers::{self, OverlayLayerStack},
    options::{OverlayMountOptions, UuidMode},
    policy::{CreatorCredentialPolicy, MountPolicy, UpperFilesystemCapabilities},
    superblock::{MountLifecycle, MountPhase, OverlayFs},
};
use crate::{
    fs::{
        fs_impls::overlayfs::{
            dir::whiteout::WhiteoutCache,
            metadata_security::xattr::OverlayXattrPolicy,
            projection::{
                BindingCache, IdentityPolicy, InodeCache, LowerLayerIdentity, OverlayInode,
            },
        },
        pseudofs::AnonDeviceId,
        vfs::{
            file_system::{FsEventSubscriberStats, FsFlags},
            inode::Inode,
            registry::FsCreationCtx,
        },
    },
    prelude::*,
};

impl OverlayFs {
    /// Constructs and publishes a fully prepared overlay filesystem.
    pub(super) fn new(fs_creation_ctx: &FsCreationCtx) -> Result<Arc<Self>> {
        let options = OverlayMountOptions::parse(fs_creation_ctx.args(), fs_creation_ctx.flags())?;

        let mount_source = fs_creation_ctx
            .source()
            .unwrap_or(OVERLAY_FS_NAME)
            .to_string();

        let credential_policy = super::with_current_posix_thread(|posix_thread| {
            Ok(CreatorCredentialPolicy::new(posix_thread.credentials_dup()))
        })?;

        let layer_stack = OverlayLayerStack::assemble(
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

        let mut claims = None;
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

            let upper_path = layers::resolve_root_path(upper_dir)?;
            let workdir_path = layers::resolve_root_path(work_dir)?;
            UpperWorkdirClaim::validate_pair(&upper_path, &workdir_path)?;
            // The workdir is not a layer, so `assemble`'s pairwise check
            // cannot cover it; a workdir nested in a lower layer root would
            // place the staging workspace inside the lower tree, so it is
            // rejected with `EINVAL`.
            let workdir_dentry = workdir_path.dentry();
            for lower in &layer_stack.lowers {
                let lower_path = lower.root_path.upgrade()?;
                let lower_dentry = lower_path.dentry();
                if Arc::ptr_eq(lower_dentry, workdir_dentry)
                    || Arc::ptr_eq(&lower.root_inode, workdir_path.inode())
                {
                    return_errno_with_message!(
                        Errno::EINVAL,
                        "workdir must be distinct from every lower layer root"
                    );
                }
                if workdir_dentry.is_equal_or_descendant_of(lower_dentry)
                    || lower_dentry.is_equal_or_descendant_of(workdir_dentry)
                {
                    return_errno_with_message!(
                        Errno::EINVAL,
                        "workdir must not be an ancestor or descendant of a lower layer root"
                    );
                }
            }
            claims::verify_inode_instance_stability(upper_dir, &upper.root_inode)?;
            claims::verify_inode_instance_stability(work_dir, workdir_path.inode())?;

            let identity = Self::determine_identity(
                is_effective_read_only,
                &upper.root_inode,
                options.uuid_mode,
            )?;

            let mut claimed_pair = UpperWorkdirClaim::claim(
                upper.root_inode.clone(),
                workdir_path.inode().clone(),
                identity,
            )?;

            if !is_effective_read_only {
                claimed_pair.prepare_workdir(&workdir_path)?;

                let capabilities = UpperFilesystemCapabilities::probe(
                    &upper.root_inode,
                    claimed_pair.workdir_workspace()?,
                )?;
                let is_uuid_effective =
                    Self::apply_capability_gates(&capabilities, options.uuid_mode)?;

                if is_uuid_effective {
                    match claimed_pair.persist_identity() {
                        Ok(()) => {
                            uuid = Some(identity);
                        }
                        Err(persist_err) => match options.uuid_mode {
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
            claims = Some(claimed_pair);
        }

        let policy = MountPolicy::assemble(
            is_effective_read_only,
            credential_policy,
            &options,
            uuid,
            upper_capabilities,
        );

        let anon_device_id = AnonDeviceId::acquire().ok_or_else(|| {
            Error::with_message(
                Errno::ENOSPC,
                "no anonymous device ID is available for the overlay mount",
            )
        })?;
        let overlay_dev_id = anon_device_id.id();

        let (layer_devs, upper_layer_dev_index) = Self::collect_layer_devs(&layer_stack);

        // `XINO_SHIFT` reserves the lower 16 bits for the inode number.
        const XINO_SHIFT: u32 = 16;
        let identity = IdentityPolicy::new(
            overlay_dev_id,
            &layer_devs,
            upper_layer_dev_index,
            XINO_SHIFT,
            policy.xino_mode(),
        )?;

        let bindings = BindingCache::new();
        let inodes = InodeCache::new();

        let overlay_fs = Arc::new_cyclic(move |weak| OverlayFs {
            layer_stack,
            claims,
            policy,
            mount_source,
            root_inode: Mutex::new(None),
            lifecycle: Mutex::new(MountLifecycle {
                phase: MountPhase::Ready,
            }),
            fs_event_stats: FsEventSubscriberStats::new(),
            self_weak: weak.clone(),
            bindings,
            inodes,
            identity,
            _anon_device_id: anon_device_id,
            workdir_temp_serial: AtomicU64::new(0),
            xattr_policy: OverlayXattrPolicy,
            whiteout_cache: Mutex::new(WhiteoutCache::new()),
        });
        let root_inode = OverlayInode::new_root(Arc::downgrade(&overlay_fs));
        *overlay_fs.root_inode.lock() = Some(root_inode);
        Ok(overlay_fs)
    }

    /// Determines the unified overlay identity before the claim step.
    ///
    /// Effective read-only overlays generate a fresh token directly, so
    /// `UuidMode::On` cannot fail on an xattr read that would only
    /// matter for persistence.
    ///
    fn determine_identity(
        is_effective_read_only: bool,
        upper_root_inode: &Arc<dyn Inode>,
        uuid_mode: UuidMode,
    ) -> Result<OverlayUuid> {
        if is_effective_read_only {
            Ok(OverlayUuid::generate())
        } else {
            UpperWorkdirClaim::determine_identity(upper_root_inode, uuid_mode)
        }
    }

    /// Applies the post-claim capability checks and derives whether the UUID
    /// mode is effective.
    ///
    /// Returns whether the UUID is effective; the caller owns the
    /// capabilities probe and the persistence step.
    fn apply_capability_gates(
        capabilities: &UpperFilesystemCapabilities,
        uuid_mode: UuidMode,
    ) -> Result<bool> {
        if !capabilities.can_report_directory_type() {
            return_errno_with_message!(
                Errno::EOPNOTSUPP,
                "the upper filesystem cannot report directory entry types"
            );
        }
        if !capabilities.can_mknod_char() && !capabilities.can_store_private_xattr() {
            return_errno_with_message!(
                Errno::EOPNOTSUPP,
                "the upper filesystem supports no whiteout form"
            );
        }
        match uuid_mode {
            UuidMode::On => {
                if !capabilities.can_store_private_xattr() {
                    return_errno_with_message!(
                        Errno::EOPNOTSUPP,
                        "the upper filesystem cannot persist the overlay uuid"
                    );
                }
                Ok(true)
            }
            UuidMode::Auto => Ok(capabilities.can_store_private_xattr()),
            UuidMode::Off | UuidMode::Null => Ok(false),
        }
    }

    /// Collects the construction-local layer identity inputs for
    /// [`IdentityPolicy::new`].
    ///
    /// Returns the per-published-layer [`LowerLayerIdentity`] list (upper
    /// first when present) with the upper's entry position. The exclusion is
    /// by position, not by value: an upper sharing an underlying filesystem
    /// with a lower must not also drop the lower's entry.
    fn collect_layer_devs(
        layer_stack: &OverlayLayerStack,
    ) -> (Vec<LowerLayerIdentity>, Option<usize>) {
        let layer_capacity =
            layer_stack.lowers.len() + if layer_stack.upper.is_some() { 1 } else { 0 };
        let mut layer_devs: Vec<LowerLayerIdentity> = Vec::with_capacity(layer_capacity);
        let upper_layer_dev_index = if let Some(upper) = layer_stack.upper.as_ref() {
            let index = layer_devs.len();
            layer_devs.push(LowerLayerIdentity {
                fsid: upper.fsid,
                container_dev_id: upper.container_dev_id,
                lower_layer_root_ino: upper.root_inode.ino(),
            });
            Some(index)
        } else {
            None
        };
        for lower in &layer_stack.lowers {
            layer_devs.push(LowerLayerIdentity {
                fsid: lower.fsid,
                container_dev_id: lower.container_dev_id,
                lower_layer_root_ino: lower.root_inode.ino(),
            });
        }
        (layer_devs, upper_layer_dev_index)
    }
}
