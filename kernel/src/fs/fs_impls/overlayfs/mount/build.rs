// SPDX-License-Identifier: MPL-2.0

//! Construction orchestration for the overlay filesystem (`OverlayFs::new`).
//!
//! This module implements the frozen 11-step construction sequence of the
//! `mount_resource_policy` meso (spec §3.2) inside the single constructor
//! [`OverlayFs::new`]. The steps of spec §3.2 are tagged below (wave-1 review
//! step-label fix, item 8); each statement carries a unique tag. The list
//! follows the *spec* order, not the textual statement order: the actual
//! execution order is 1 → 4a → 2 → 4b → 4c → 3 → 5 → 6 → 7 → 8 → 9 → 4d →
//! 10a → 10b → 11, because tags 4a-4d execute around steps 2-3 for drop-order
//! reasons (the credential snapshot is declared before the layer stack so it
//! drops last), and steps 7-9 execute only on the writable branch (the
//! post-guard order, wave-1 review round 2, item 5). Step 10 is the meso-02
//! extension of the frozen step list (cross-meso owner rule, spec §3.5 item
//! 1): tag 10a is the meso-02 construction wiring (`AnonDeviceId` +
//! `IdentityPolicy` + the `bindings`/`inodes` caches, all required before the
//! root seam runs because `OverlayInode::new_root` reads `fs.identity()`),
//! and tag 10b is the frozen step-10 root-carrier seam
//! (`OverlayInode::new_root`; reconciled per wave-2 review item 1 — the seam
//! accepts the `Weak<OverlayFs>` and fills the late-bound `OnceLock` root
//! slot right after the `Arc` is published via `Arc::new_cyclic`).
//!
//! 1. parse the mount options (`OverlayMountOptions::parse`, P0-01);
//! 2. assemble the layer stack (`OverlayLayerStack::assemble`, P0-02);
//! 3. validate the upper/workdir pair structurally and probe instance
//!    stability (`UpperWorkdirClaim::validate_pair` +
//!    `verify_inode_instance_stability`, P0-03/P1-35);
//! 4. compute the policy draft — the creator-credential snapshot (P1-19),
//!    the effective read-only state (P0-18), and the write-access accounting
//!    (P1-20, writable mounts only) — split across tags 4a-4d because the
//!    credential snapshot must be declared before the layer stack so it drops
//!    last (spec §2 release-order invariant);
//! 5. determine the unified identity (fresh token for effective read-only
//!    overlays; `UpperWorkdirClaim::determine_identity` reuse-or-generate for
//!    writable overlays, P2-11);
//! 6. claim the upper/workdir slots (`UpperWorkdirClaim::claim`, P1-35);
//! 7. probe the upper capabilities and apply the d_type/whiteout gates
//!    (P0-02/P2-11/P1-25/P1-36; writable mounts only);
//! 8. prepare the workdir (`UpperWorkdirClaim::prepare_workdir`, P0-03;
//!    writable mounts only);
//! 9. persist the UUID when effective (`UpperWorkdirClaim::persist_identity`,
//!    P2-11; writable mounts only);
//! 10a. meso-02 wiring: acquire the overlay `AnonDeviceId` (fallible) and
//!      construct `IdentityPolicy` (`overlay_dev_id` set here; `layer_devs`
//!      from the published layer snapshot) plus the empty
//!      `bindings`/`inodes` caches (spec §3.5 item 1 / §3.4);
//! 10b. construct the root carrier via the real meso-02 seam
//!      (`OverlayInode::new_root`, spec §1 item 1 / §3.0.5 item 8; see the
//!      RECONCILIATION record at the call site);
//! 11. publish the `Arc<OverlayFs>` (the single publication point).
//!
//! On failure the locals drop in reverse declaration order, so the runtime
//! resources release in the spec's frozen order (BC-1 §14): root carrier /
//! workdir state / workdir claim / upper claim / layer pins / credential
//! snapshot. The meso-02 step-10a locals (overlay `AnonDeviceId`, the
//! `IdentityPolicy`, and the `bindings`/`inodes` caches) are declared after
//! the policy snapshot, so on rollback they release before the Wave-1
//! resources — the frozen Wave-1 release order is undisturbed.

use core::cell::OnceLock;
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
            file_system::{FileSystem, FsEventSubscriberStats, FsFlags},
            registry::FsCreationCtx,
        },
    },
    prelude::*,
};

impl OverlayFs {
    /// Constructs and publishes a fully prepared overlay filesystem.
    ///
    /// The 11 ordered steps of spec §3.2 are local statements. The
    /// construction resources are declared in creation order (creator
    /// credential policy first, then the layer stack, then the claims, then
    /// the policy snapshot and the root carrier), so a failure at any point
    /// rolls back in reverse declaration order: root carrier / workdir state /
    /// workdir claim / upper claim / layer pins / credential snapshot (spec §2
    /// release-order invariant, BC-1 §14).
    pub(super) fn new(fs_creation_ctx: &FsCreationCtx) -> Result<Arc<Self>> {
        // Step 1 — parse the mount options (P0-01). The parsed fields are
        // consumed here as `pub(super)`-visible construction inputs within the
        // `mount` tree (visibility reconciled with the sibling `options.rs`;
        // wave-1 review `information-hiding` fix, item 7).
        let options = OverlayMountOptions::parse(fs_creation_ctx.args(), fs_creation_ctx.flags())?;

        // The reported mount source (P0-05 show-options surface); the fs type
        // name is the default when the mount(2) call supplies no source string
        // (single representation via `OVERLAY_FS_NAME`, wave-1 review `dry`
        // fix, item 8).
        let mount_source = fs_creation_ctx
            .source()
            .unwrap_or(OVERLAY_FS_NAME)
            .to_string();

        // Step 4a (policy draft, P1-19) — the creator credential snapshot is
        // taken once, at construction, and is declared first so it is dropped
        // last (spec §2 release-order invariant: the credential snapshot is
        // the final release).
        let credential_policy =
            CreatorCredentialPolicy::new(fs_creation_ctx.task_ctx().posix_thread.credentials_dup());

        // Step 2 — assemble the layer stack (P0-02). The parsed
        // `is_forced_read_only` flag is passed in (wave-1 review `dry` fix,
        // item 10) instead of being re-derived from `fs_creation_ctx.flags()`
        // inside `assemble`.
        let layer_stack = OverlayLayerStack::assemble(
            fs_creation_ctx,
            options.upper_dir.clone(),
            options.lower_dirs.clone(),
            options.is_forced_read_only,
        )?;

        // Step 4b (policy draft, P0-18) — effective read-only state, computed
        // before any claim is taken: no upper, forced read-only, or a
        // read-only upper backend (spec §2 Case 6 / §3.2 step 4).
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
        // writable overlays (consolidated read-only guard, wave-1 review items
        // 2/3): a read-only overlay never probes, never checks the workdir for
        // emptiness, and never persists, so `write_access`/`upper_capabilities`/
        // `uuid` all stay `None`.
        let mut write_access = None;
        let mut claims = None;
        let mut upper_capabilities = None;
        let mut uuid = None;
        if let Some(upper) = &layer_stack.upper {
            // Step 4c (policy draft, P1-20) — write-access accounting for
            // genuinely writable mounts (spec §4 invariant: `write_access` is
            // `Some` iff `is_effective_read_only` is false).
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

            // Step 3 — structural upper/workdir validation (P0-03) and the
            // instance-stability probe for both roots (P1-35 pre-claim
            // evidence; heuristic, spec §3.0.2). The probe now compares the
            // layer-pinned inodes (`upper.root_inode` and the resolved workdir
            // inode) against fresh resolutions, so the checked objects are
            // exactly the objects claimed in step 6 (wave-1 review item 6,
            // TOCTOU check/use alignment). Both paths go through the shared
            // `resolve_root_path` helper (wave-1 review `dry` fix, item 8).
            let upper_path = resolve_root_path(fs_creation_ctx, upper_dir)?;
            let workdir_path = resolve_root_path(fs_creation_ctx, work_dir)?;
            UpperWorkdirClaim::validate_pair(&upper_path, &workdir_path)?;
            verify_inode_instance_stability(fs_creation_ctx, upper_dir, &upper.root_inode)?;
            verify_inode_instance_stability(fs_creation_ctx, work_dir, workdir_path.inode())?;

            // Step 5 — determine the unified identity before the claim step
            // (P2-11; the token must be known at claim time, spec §3.0.4).
            //
            // Effective read-only overlays never persist (steps 7-9 are
            // skipped, so `uuid` stays `None`): a fresh non-zero claim token
            // is generated directly, so `UuidMode::On` cannot fail closed on
            // an xattr read that would only matter for persistence (wave-1
            // review round 2, item 2). Writable overlays go through the full
            // `determine_identity` (reuse-or-generate, per `uuid_mode`).
            let identity = if is_effective_read_only {
                OverlayUuid::generate()
            } else {
                UpperWorkdirClaim::determine_identity(&upper.root_inode, options.uuid_mode)?
            };

            // Step 6 — claim the upper slot first, then the workdir slot
            // (P1-35, Scheme A); a workdir conflict rolls back the upper claim.
            let claimed_pair = UpperWorkdirClaim::claim(
                upper.root_inode.clone(),
                workdir_path.inode().clone(),
                upper.fs.clone(),
                identity,
            )?;

            if !is_effective_read_only {
                // Step 7 — probe the upper capabilities post-claim and apply
                // the d_type/whiteout gates (P0-02 / P2-11 / P1-25 / P1-36;
                // writable overlays only — the whiteout gate is irrelevant to
                // read-only overlays and the char-device probe performs a
                // write).
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
                // Whiteout-capability gate (revision 05; spec §2 Case 11): a
                // writable overlay needs at least one whiteout form to delete
                // lower-backed names.
                if !capabilities.can_mknod_char() && !capabilities.can_store_private_xattr() {
                    return_errno_with_message!(
                        Errno::EOPNOTSUPP,
                        "the upper filesystem supports no whiteout form"
                    );
                }
                // P2-11 effectiveness (spec §2 Case 10): `On` fails closed
                // without xattr persistence; `Auto` degrades; `Off`/`Null`
                // never persist.
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

                // Step 8 — prepare the workdir (P0-03; `ENOTEMPTY` on residue;
                // skipped for read-only overlays, wave-1 review item 2).
                claimed_pair.prepare_workdir()?;

                // Step 9 — persist the UUID when effective (P2-11, BC-1 step
                // 9). `On` persist failure fails closed; `Auto` degrades to
                // not-effective. A successful persist is a durable identity
                // record and is never rolled back (spec §3.3 Hazard 7).
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
        // all constituents exist (identity from step 5, capabilities from step
        // 7; spec §3.2 step 4 + §4).
        let policy = MountPolicy::assemble(
            is_effective_read_only,
            credential_policy,
            options.uuid_mode,
            uuid,
            upper_capabilities,
            write_access,
            options.is_default_permissions,
        );

        // Step 10 — meso-02 construction wiring (cross-meso owner rule, spec
        // §3.5 item 1): the extended `OverlayFs::new` acquires the overlay
        // `AnonDeviceId` (fallible) and constructs the `IdentityPolicy`
        // (`overlay_dev_id` set here; `layer_devs` from the published layer
        // snapshot). The `bindings`/`inodes` caches are initialized empty
        // here too — they are the meso-02 per-mount state fields added to the
        // Macro-Owner carrier (spec §1 item 2 / §3.4), and the `projection`
        // module publishes them through `OverlayFs::bindings()`/`inodes()`/
        // `identity()` (sibling `projection/mod.rs`).
        //
        // The overlay `AnonDeviceId` is the mount's own `st_dev` (major-0
        // pseudo device, `pseudofs::AnonDeviceId`). Acquisition is fallible —
        // the minor-number pool can be exhausted — and maps to `ENOSPC`; no
        // `.expect()`/`.unwrap()` in production paths (meso-02 spec §4 "no
        // unwrap" rule; `AnonDeviceId` acquisition "happens in the extended
        // (fallible) `OverlayFs::new`, not a seam `.expect()`").
        let anon_device_id = AnonDeviceId::acquire().ok_or_else(|| {
            Error::with_message(
                Errno::ENOSPC,
                "no anonymous device ID is available for the overlay mount",
            )
        })?;
        let overlay_dev_id = anon_device_id.id();

        // `layer_devs` is the immutable `fsid -> container_dev_id` table of
        // the published layer snapshot (upper first when present, then the
        // lowers topmost-first) — the same single source that feeds
        // `IdentityPolicy::is_all_layers_same_fs` (meso-02 spec §4
        // `IdentityPolicy`; revision-07 ledger item 7: "built at construction
        // from the published layer snapshot"). The table is fsid-sorted and
        // frozen inside `IdentityPolicy::new`.
        let layer_capacity =
            layer_stack.lowers.len() + if layer_stack.upper.is_some() { 1 } else { 0 };
        let mut layer_devs: Vec<(u64, DeviceId)> = Vec::with_capacity(layer_capacity);
        if let Some(upper) = &layer_stack.upper {
            layer_devs.push((upper.fsid, upper.container_dev_id));
        }
        for lower in &layer_stack.lowers {
            layer_devs.push((lower.fsid, lower.container_dev_id));
        }

        // The frozen xino mask width ("e.g. 64 - 16 = 48-bit payload", spec
        // §4); the frozen spec assigns no value, so the build packet owns the
        // policy value (identity.rs creator record §5 deviation 2). `new` is
        // fallible only to enforce the frozen `xino_shift <= 63` invariant.
        const XINO_SHIFT: u32 = 16;
        let identity =
            IdentityPolicy::new(overlay_dev_id, layer_devs.into_boxed_slice(), XINO_SHIFT)?;

        // The meso-02 cache fields start empty; entries are inserted/updated
        // under the caller's parent `DIR` transaction by the `projection`
        // module lookup flow (spec §4 lock-carrier table).
        let bindings = BindingCache::new();
        let inodes = InodeCache::new();

        // Step 10 (root carrier) + Step 11 (publication) — the root is
        // materialized through the real meso-02 seam
        // `OverlayInode::new_root(Weak<OverlayFs>)` (the Wave-2-landed symbol,
        // reconciled per wave-2 review item 1), and the `Arc<OverlayFs>` is
        // published once.
        //
        // SELF-REFERENTIAL CONSTRUCTION RECONCILIATION (spec §3.0.5 item 8,
        // ramfs baseline; wave-2 review item 1 — critical fix): the root
        // carrier consumes the published mount (`fs.layer_stack()` /
        // `fs.identity()`), so it cannot be built inside the
        // `Arc::new_cyclic` closure — `Weak::upgrade()` is documented-`None`
        // during construction (the strong count stays 0 until the closure
        // returns; verified in the pinned toolchain `alloc/src/sync.rs`,
        // `new_cyclic_in`). The reconciliation: `Arc::new_cyclic` establishes
        // the canonical `OverlayFs::self_weak` reference (ramfs
        // `Arc::new_cyclic` + `Weak<RamFs>` precedent), the struct is built
        // with an empty `OnceLock` root slot, and the slot is filled
        // immediately after the strong reference exists via
        // `OverlayInode::new_root(Arc::downgrade(&overlay_fs))`. The seam now
        // accepts the `Weak<OverlayFs>` — the recorded deviation from the
        // provisional `new_root(fs: Arc<OverlayFs>)` signature (the upgrade is
        // guaranteed at this call site). The inode stores the weak
        // (`Arc::downgrade` inside `new_root`), so there is no
        // `fs -> inode -> fs` strong cycle (B/C-2 lifetime rule).
        let overlay_fs = Arc::new_cyclic(move |weak| OverlayFs {
            layer_stack,
            claims,
            policy,
            mount_source,
            root_inode: OnceLock::new(),
            lifecycle: Mutex::new(MountLifecycle {
                phase: MountPhase::Ready,
            }),
            fs_event_stats: FsEventSubscriberStats::new(),
            bindings,
            inodes,
            identity,
            self_weak: weak.clone(),
            // The `AnonDeviceId` RAII guard is retained for the mount
            // lifetime so the overlay `st_dev` (copied into
            // `IdentityPolicy::overlay_dev_id`) is never recycled under a
            // live mount. The substrate-idiomatic owner (every Asterinas
            // pseudo-fs and the legacy overlayfs hold `AnonDeviceId` on the
            // fs struct) is this `_anon_device_id: AnonDeviceId` field on
            // `OverlayFs` — the recorded one-field widening of
            // `mount/superblock.rs` (see the Creator report §5).
            _anon_device_id: anon_device_id,
            // Wave-3 shared-carrier seams (cross-meso owner rule; parent N/A —
            // seam placement only, the owning meso passes claim the features in
            // Wave 4). The three fields were added by the sibling
            // `mount/superblock.rs` seam packet and are initialized here in the
            // frozen declaration order; all three have trivial drops, so the
            // frozen RAII release order above is undisturbed.
            //
            // `workdir_temp_serial` (meso-04 P1-34) — the workdir unique-naming
            // context (`generate_workdir_temp_name`); a saturating `AtomicU64`
            // starting at 0, never gates I/O.
            workdir_temp_serial: AtomicU64::new(0),
            // `xattr_policy` (meso-05 P1-33) — the immutable xattr
            // public/private/escaped classification policy; unit-struct default
            // construction per its frozen shape (stateless this wave; the type
            // lands in Wave 4 `metadata_security/xattr.rs`).
            xattr_policy: OverlayXattrPolicy,
            // `whiteout_cache` (meso-06 P1-36) — the mount-scoped `WL`-domain
            // reusable whiteout cache; constructed through the frozen
            // `WhiteoutCache::new` name (constructor lands in Wave 4
            // `dir/whiteout.rs`; forward reference by frozen name).
            whiteout_cache: Mutex::new(WhiteoutCache::new()),
        });
        let _ = overlay_fs
            .root_inode
            .set(OverlayInode::new_root(Arc::downgrade(&overlay_fs)));
        Ok(overlay_fs)
    }
}
