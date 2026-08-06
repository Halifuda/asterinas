// SPDX-License-Identifier: MPL-2.0

//! Construction orchestration for the overlay filesystem ([`OverlayFs::new`]).
//!
//! This module implements the construction sequence of the
//! `mount_resource_policy` component inside the single constructor
//! [`OverlayFs::new`]. The list below follows the contract order, not the
//! textual statement order: the actual execution order is 1 → 4a → 2 → 4b →
//! 4c → 3 → 5 → 6 → 7 → 8 → 9 → 4d → 10 → 11, because tags 4a-4d execute
//! around steps 2-3 for drop-order reasons (the credential snapshot is
//! declared before the layer stack so it drops last), and steps 7-9 execute
//! only on the writable branch. Step 10 is the projection wiring: it acquires
//! the overlay `AnonDeviceId` and constructs `IdentityPolicy` plus the
//! `bindings`/`inodes` caches, all required before the root constructor runs
//! because `OverlayInode::new_root` reads `fs.identity()`, then constructs the
//! root
//! carrier, which accepts the `Weak<OverlayFs>` and fills the late-bound root
//! publication slot right after the `Arc` is published via `Arc::new_cyclic`.
//!
//! 1. parse the mount options (`OverlayMountOptions::parse`);
//! 2. assemble the layer stack (`OverlayLayerStack::assemble`);
//! 3. validate the upper/workdir pair structurally and probe instance
//!    stability (`UpperWorkdirClaim::validate_pair` +
//!    `verify_inode_instance_stability`);
//! 4. compute the policy draft — the creator-credential snapshot, the
//!    effective read-only state, and the write-access accounting (writable
//!    mounts only) — split across tags 4a-4d because the credential snapshot
//!    must be declared before the layer stack so it drops last;
//! 5. determine the unified identity (fresh token for effective read-only
//!    overlays; `UpperWorkdirClaim::determine_identity` reuse-or-generate for
//!    writable overlays);
//! 6. claim the upper/workdir slots (`UpperWorkdirClaim::claim`);
//! 7. probe the upper capabilities and apply the d_type/whiteout gates
//!    (writable mounts only);
//! 8. prepare the workdir (`UpperWorkdirClaim::prepare_workdir`; writable
//!    mounts only);
//! 9. persist the UUID when effective (`UpperWorkdirClaim::persist_identity`;
//!    writable mounts only);
//! 10. perform the projection wiring: (a) acquire the overlay `AnonDeviceId`
//!     (fallible) and construct `IdentityPolicy` (with `overlay_dev_id`) plus
//!     the empty `bindings`/`inodes` caches, then (b) construct the root
//!     carrier via the projection constructor `OverlayInode::new_root` (see
//!     the construction note at the call site);
//! 11. publish the `Arc<OverlayFs>` (the single publication point).
//!
//! On failure the locals drop in reverse declaration order, so the runtime
//! resources release in a fixed order: root carrier / workdir state / workdir
//! claim / upper claim / layer pins / credential snapshot. The step-10 locals
//! (overlay `AnonDeviceId`, the `IdentityPolicy`, and the `bindings`/`inodes`
//! caches) are declared after the policy snapshot, so on rollback they
//! release before the step-1..9 resources — the release order is undisturbed.
//!
//! TODO(doc): the step numbering (1-11 with 4a-4d and 10a/10b substeps) is
//! retained as a construction-order aid; a future revision may fold it into
//! plain prose.

use core::sync::atomic::AtomicU64;

use device_id::DeviceId;

use super::{
    OVERLAY_FS_NAME,
    claims::{OverlayUuid, UpperWorkdirClaim, verify_inode_instance_stability},
    layers::{OverlayLayerStack, resolve_root_path},
    options::{OverlayMountOptions, UuidMode},
    policy::{
        CreatorCredentialPolicy, MountPolicy, UpperFilesystemCapabilities, WriteAccessAccounting,
    },
    superblock::{MountLifecycle, MountPhase, OverlayFs},
};
use crate::{
    fs::{
        fs_impls::overlayfs::{
            dir::whiteout::WhiteoutCache,
            metadata_security::xattr::OverlayXattrPolicy,
            projection::{BindingCache, IdentityPolicy, InodeCache, OverlayInode},
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
    ///
    /// The 11 ordered steps are local statements. The construction resources
    /// are declared in creation order (creator credential policy first, then
    /// the layer stack, then the claims, then the policy snapshot and the
    /// root carrier), so a failure at any point rolls back in reverse
    /// declaration order: root carrier / workdir state / workdir claim /
    /// upper claim / layer pins / credential snapshot.
    pub(super) fn new(fs_creation_ctx: &FsCreationCtx) -> Result<Arc<Self>> {
        // Step 1 — parse the mount options. The parsed fields are consumed
        // here as `pub(super)`-visible construction inputs within the `mount`
        // tree.
        let options = OverlayMountOptions::parse(fs_creation_ctx.args(), fs_creation_ctx.flags())?;

        // The reported mount source; the fs type name is the default when the
        // mount(2) call supplies no source string (single representation via
        // `OVERLAY_FS_NAME`).
        let mount_source = fs_creation_ctx
            .source()
            .unwrap_or(OVERLAY_FS_NAME)
            .to_string();

        // Step 4a (policy draft) — the creator credential snapshot is taken
        // once, at construction, and is declared first so it is dropped last
        // (release-order invariant: the credential snapshot is the final
        // release).
        let credential_policy =
            CreatorCredentialPolicy::new(fs_creation_ctx.task_ctx().posix_thread.credentials_dup());

        // Step 2 — assemble the layer stack. The parsed `is_forced_read_only`
        // flag is passed in instead of being re-derived from
        // `fs_creation_ctx.flags()` inside `assemble`.
        let layer_stack = OverlayLayerStack::assemble(
            fs_creation_ctx,
            options.upper_dir.clone(),
            options.lower_dirs.clone(),
            options.is_forced_read_only,
        )?;

        // Step 4b (policy draft) — effective read-only state, computed before
        // any claim is taken: no upper, forced read-only, or a read-only upper
        // backend.
        let is_effective_read_only = match &layer_stack.upper {
            Some(upper) => {
                options.is_forced_read_only || upper.fs.flags().contains(FsFlags::RDONLY)
            }
            None => true,
        };

        // Steps 3 and 5-9 — upper/workdir handling. The locals are declared
        // after the layer stack so the claims (and their inode guards) release
        // before the layer pins on rollback. Steps 7-9 (capability probe,
        // workdir preparation, UUID persistence) run only for genuinely
        // writable overlays: a read-only overlay never probes, never checks
        // the workdir for emptiness, and never persists, so
        // `write_access`/`upper_capabilities`/`uuid` all stay `None`.
        let mut write_access = None;
        let mut claims = None;
        let mut upper_capabilities = None;
        let mut uuid = None;
        if let Some(upper) = &layer_stack.upper {
            // Step 4c (policy draft) — write-access accounting for genuinely
            // writable mounts (`write_access` is `Some` iff
            // `is_effective_read_only` is false).
            if !is_effective_read_only {
                write_access = Some(WriteAccessAccounting::new(upper.fs.clone()));
            }

            // The parse invariant guarantees both option strings are present
            // for an upper-backed overlay; the conversions below are defensive
            // (no `.unwrap()`/`.expect()` in production paths).
            let upper_dir = options.upper_dir.as_deref().ok_or_else(|| {
                Error::with_message(Errno::EINVAL, "internal error: missing upperdir option")
            })?;
            let work_dir = options.work_dir.as_deref().ok_or_else(|| {
                Error::with_message(Errno::EINVAL, "internal error: missing workdir option")
            })?;

            // Step 3 — structural upper/workdir validation and the
            // instance-stability probe for both roots (pre-claim evidence).
            // The probe compares the layer-pinned inodes (`upper.root_inode`
            // and the resolved workdir inode) against fresh resolutions, so
            // the checked objects are exactly the objects claimed in step 6
            // (check/use alignment). Both paths go through the shared
            // `resolve_root_path` helper.
            let upper_path = resolve_root_path(fs_creation_ctx, upper_dir)?;
            let workdir_path = resolve_root_path(fs_creation_ctx, work_dir)?;
            UpperWorkdirClaim::validate_pair(&upper_path, &workdir_path)?;
            verify_inode_instance_stability(fs_creation_ctx, upper_dir, &upper.root_inode)?;
            verify_inode_instance_stability(fs_creation_ctx, work_dir, workdir_path.inode())?;

            // Step 5 — determine the unified identity before the claim step
            // (the token must be known at claim time).
            //
            // Effective read-only overlays never persist (steps 7-9 are
            // skipped, so `uuid` stays `None`): a fresh non-zero claim token
            // is generated directly, so `UuidMode::On` cannot fail closed on
            // an xattr read that would only matter for persistence. Writable
            // overlays go through the full `determine_identity`
            // (reuse-or-generate, per `uuid_mode`).
            let identity = if is_effective_read_only {
                OverlayUuid::generate()
            } else {
                UpperWorkdirClaim::determine_identity(&upper.root_inode, options.uuid_mode)?
            };

            // Step 6 — claim the upper slot first, then the workdir slot; a
            // workdir conflict rolls back the upper claim.
            let claimed_pair = UpperWorkdirClaim::claim(
                upper.root_inode.clone(),
                workdir_path.inode().clone(),
                identity,
            )?;

            if !is_effective_read_only {
                // Step 7 — probe the upper capabilities post-claim and apply
                // the d_type/whiteout gates (writable overlays only — the
                // whiteout gate is irrelevant to read-only overlays and the
                // char-device probe performs a write).
                let capabilities = UpperFilesystemCapabilities::probe(
                    &upper.root_inode,
                    claimed_pair.workdir_inode(),
                )?;
                if !capabilities.can_report_directory_type() {
                    return_errno_with_message!(
                        Errno::EOPNOTSUPP,
                        "the upper filesystem cannot report directory entry types"
                    );
                }
                // Whiteout-capability gate: a writable overlay needs at least
                // one whiteout form to delete lower-backed names.
                if !capabilities.can_mknod_char() && !capabilities.can_store_private_xattr() {
                    return_errno_with_message!(
                        Errno::EOPNOTSUPP,
                        "the upper filesystem supports no whiteout form"
                    );
                }
                // UUID-mode effectiveness: `On` fails closed without xattr
                // persistence; `Auto` degrades; `Off`/`Null` never persist.
                let is_uuid_effective = match options.uuid_mode {
                    UuidMode::On => {
                        if !capabilities.can_store_private_xattr() {
                            return_errno_with_message!(
                                Errno::EOPNOTSUPP,
                                "the upper filesystem cannot persist the overlay uuid"
                            );
                        }
                        true
                    }
                    UuidMode::Auto => capabilities.can_store_private_xattr(),
                    UuidMode::Off | UuidMode::Null => false,
                };

                // Step 8 — prepare the workdir (`ENOTEMPTY` on residue;
                // skipped for read-only overlays).
                claimed_pair.prepare_workdir()?;

                // Step 9 — persist the UUID when effective. `On` persist
                // failure fails closed; `Auto` degrades to not-effective. A
                // successful persist is a durable identity record and is
                // never rolled back.
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

        // Step 4d (policy draft) — freeze the immutable policy snapshot once
        // all constituents exist (identity from step 5, capabilities from
        // step 7).
        let policy = MountPolicy::assemble(
            is_effective_read_only,
            credential_policy,
            &options,
            uuid,
            upper_capabilities,
            write_access,
        );

        // Step 10a — projection construction wiring: the extended
        // `OverlayFs::new` acquires the overlay `AnonDeviceId` (fallible) and
        // constructs the `IdentityPolicy` (`overlay_dev_id` set here;
        // construction-local layer identity tuples from the published layer
        // snapshot). The `bindings`/`inodes` caches are initialized empty here
        // too — they are the per-mount state fields on the carrier, and the
        // `projection` module publishes them through
        // `OverlayFs::bindings()`/`inodes()`/`identity()`.
        //
        // The overlay `AnonDeviceId` is the mount's own `st_dev` (major-0
        // pseudo device, `pseudofs::AnonDeviceId`). Acquisition is fallible —
        // the minor-number pool can be exhausted — and maps to `ENOSPC`; no
        // `.expect()`/`.unwrap()` in production paths.
        let anon_device_id = AnonDeviceId::acquire().ok_or_else(|| {
            Error::with_message(
                Errno::ENOSPC,
                "no anonymous device ID is available for the overlay mount",
            )
        })?;
        let overlay_dev_id = anon_device_id.id();

        // `layer_devs` is construction-local `(fsid, container_dev_id,
        // lower_layer_root_ino)` input from the published snapshot (upper
        // first when present, then lowers topmost-first). `IdentityPolicy`
        // derives all-layer same-fs state and retains only its fsid-sorted
        // lower snapshot; no all-layer table remains stored after `new`.
        let layer_capacity =
            layer_stack.lowers.len() + if layer_stack.upper.is_some() { 1 } else { 0 };
        let mut layer_devs: Vec<(u64, DeviceId, u64)> = Vec::with_capacity(layer_capacity);
        // The upper's entry position in `layer_devs`, when an upper exists:
        // the LOWER-only snapshot that serves origin-record device/root-pair
        // resolution is derived inside `IdentityPolicy::new`
        // by excluding exactly this entry — one construction, so the two
        // views can never diverge. The exclusion is by position, not by
        // value: an upper sharing an underlying filesystem with a lower must
        // not also drop the lower's entry. The explicit branch keeps the
        // index capture next to the push it describes.
        let upper_layer_dev_index = if let Some(upper) = layer_stack.upper.as_ref() {
            let index = layer_devs.len();
            layer_devs.push((upper.fsid, upper.container_dev_id, upper.root_inode.ino()));
            Some(index)
        } else {
            None
        };
        for lower in &layer_stack.lowers {
            layer_devs.push((lower.fsid, lower.container_dev_id, lower.root_inode.ino()));
        }

        // The xino mask width ("e.g. 64 - 16 = 48-bit payload"); the policy
        // value is owned by the build packet. `new` is fallible only to
        // enforce the `xino_shift <= 63` invariant.
        const XINO_SHIFT: u32 = 16;
        let identity = IdentityPolicy::new(
            overlay_dev_id,
            &layer_devs,
            upper_layer_dev_index,
            XINO_SHIFT,
            policy.xino_mode(),
        )?;

        // The cache fields start empty; entries are inserted/updated under the
        // caller's parent `DIR` transaction by the `projection` module lookup
        // flow.
        let bindings = BindingCache::new();
        let inodes = InodeCache::new();

        // Step 10b (root carrier) + Step 11 (publication) — the root is
        // materialized through the projection seam
        // `OverlayInode::new_root(Weak<OverlayFs>)`, and the `Arc<OverlayFs>`
        // is published once.
        //
        // Self-referential construction: the root carrier consumes the
        // published mount (`fs.layer_stack()` / `fs.identity()`), so it
        // cannot be built inside the `Arc::new_cyclic` closure —
        // `Weak::upgrade()` is documented-`None` during construction (the
        // strong count stays 0 until the closure returns; verified in the
        // pinned toolchain `alloc/src/sync.rs`, `new_cyclic_in`).
        // `Arc::new_cyclic` establishes the canonical `OverlayFs::self_weak`
        // reference (ramfs `Arc::new_cyclic` + `Weak<RamFs>` precedent), the
        // struct is built with an empty root publication slot, and the slot
        // is filled immediately after the strong reference exists via
        // `OverlayInode::new_root(Arc::downgrade(&overlay_fs))`. The seam
        // accepts the `Weak<OverlayFs>` (the upgrade is guaranteed at this
        // call site). The inode stores the weak (`Arc::downgrade` inside
        // `new_root`), so there is no `fs -> inode -> fs` strong cycle.
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
            // The `AnonDeviceId` RAII guard is retained for the mount lifetime
            // so the overlay `st_dev` (copied into
            // `IdentityPolicy::overlay_dev_id`) is never recycled under a live
            // mount. The substrate-idiomatic owner (every Asterinas pseudo-fs
            // and the legacy overlayfs hold `AnonDeviceId` on the fs struct)
            // is this `_anon_device_id: AnonDeviceId` field on `OverlayFs`.
            _anon_device_id: anon_device_id,
            // Cross-module carriers for copy-up, metadata security, and
            // namespace mutation. The three fields were added to
            // `mount/superblock.rs` and are initialized here in declaration
            // order; all three have trivial drops, so the RAII release order
            // above is undisturbed.
            //
            // `workdir_temp_serial` — the workdir unique-naming context
            // (`generate_workdir_temp_name`); a saturating `AtomicU64`
            // starting at 0, never gates I/O.
            workdir_temp_serial: AtomicU64::new(0),
            // `xattr_policy` — the immutable xattr public/private/escaped
            // classification policy; unit-struct default construction
            // (stateless).
            xattr_policy: OverlayXattrPolicy,
            // `whiteout_cache` — the mount-scoped reusable whiteout cache;
            // constructed through `WhiteoutCache::new`.
            whiteout_cache: Mutex::new(WhiteoutCache::new()),
        });
        let root_inode = OverlayInode::new_root(Arc::downgrade(&overlay_fs));
        *overlay_fs.root_inode.lock() = Some(root_inode);
        Ok(overlay_fs)
    }
}
