<!-- SPDX-License-Identifier: MPL-2.0 -->

# Main-Agent Pass Slicing Ledger

This file is the durable main-agent-owned record of how meso-level Architect / Designer contracts are split into pass-level Creator, Checker, and Reviewer work.

`SYSTEM_BLUEPRINT.md` remains the active status board. This ledger records the scheduling decision, covered-micro boundary, and rationale so later main agents do not rediscover or accidentally widen previous pass slices.

## Rules

- Only the main agent updates this file.
- Record a decision before or at the same time a Creator, Checker, or Reviewer packet is dispatched.
- Keep Designer artifacts meso-scoped; do not ask Designers to pre-slice implementation passes.
- Every Creator-synced Checker pass mirrors its Creator pass exactly.
- Keep meso integration passes separate from Creator-synced Checker passes.
- When a structural cleanup pass exists, list each cleanup objective separately and record whether it is fully closed or intentionally deferred.

## Current Pass Slicing Decisions

### `meso_01_mount_volume_state`

**Decision:** Slice the full mount-volume-state meso contract into one broad implementation pass, follow-up structural cleanup passes, and one independent meso integration Checker pass.

| Pass ID | Kind | Covered Boundary | Slicing Rationale | Decision State |
| :--- | :--- | :--- | :--- | :--- |
| `pass_01_mount_volume_state` | Creator-Synced Pass | Mount-time boot validation, allocation bitmap truth source, volume flags, up-case table load, mount options, statfs counters, root inode exposure, and recount fallback. | The initial bootstrap needed the coherent mount state surface to typecheck and expose a usable VFS mount point. | Accepted. |
| `pass_01_mount_volume_state_cleanup_01` | Creator Cleanup Pass | Structural cleanup over the pass01 production surface. | Reviewer found `ondisk.rs` and owner-placement debt after the first implementation pass. | Rejected by Reviewer; superseded by later cleanup passes. |
| `pass_01_mount_volume_state_cleanup_02` | Creator Cleanup Pass | Continued owner-placement cleanup for pass01. | Cleanup stayed scoped to the already-implemented mount-volume-state slice, not new features. | Checker found compile failures; superseded by `cleanup_03`. |
| `pass_01_mount_volume_state_cleanup_03` | Creator Cleanup Pass | Continued owner-placement cleanup for pass01. | Follow-up cleanup preserved the same covered-micro boundary while closing residual structure issues. | Checker passed; later structural cleanup still required. |
| `pass_01_mount_volume_state_cleanup_04` | Reviewer / Creator Cleanup Pass | Final removal of `ondisk.rs` production-path debt and test-support surface confirmation. | Structural debt was still not fully closed, so Reviewer findings reopened Creator cleanup instead of direct Reviewer topology edits. | Final Checker passed; accepted. |
| `pass_01_mount_volume_state_user_named_structural_cleanup_05` | Creator Cleanup Pass | User-named surviving mount surface: `test_support/boot_region.rs::read_device_bytes`, stale notes `ValidatedMount` / `PublishedMountState`, surviving `MountedVolumeState`, `MountVolumeStateError`, removed `MountVolumeStateTarget` / `MountVolumeStateOperation` / `MountVolumeStateOutcome`, removed `mount_volume_state`, surviving owner-local `mount_candidate` / `remount_published`, and mount-related `#[cfg(ktest)] mod tests` / test-only support placement. | The April 21 problem note names mount-state carriers and helpers that cannot be repaired inside a `meso_02` free-space-only pass. The parent meso boundary must stay `meso_01`; the pass must use default-reject proof for carriers, helper families, thin helpers, and packeted test surfaces. | Reviewer approved with line-level-only seam documentation; final Checker skippable. This pass is accepted through Reviewer. |
| `pass_02_mount_volume_state_integration` | Meso Integration Checker Pass | Integration tests declared by the meso Designer for mounted state, statfs, bitmap/upcase loading, and recount fallback. | Integration obligations are Checker-owned and intentionally separate from Creator-synced pass validation. | PASS. |

**Deferred / Exit Notes:**

- No further mount-volume-state pass is currently sliced.
- Any future mount-volume-state reopening must explicitly state whether it is new feature work, cleanup over surviving entities, or integration-only validation.

### `meso_02_free_space_accounting_and_discard`

**Decision:** Slice the meso contract into an initial accounting-only core pass, pass01-only cleanup passes, one online-discard policy pass, one administrative `FITRIM` boundary pass, and one independent meso integration Checker pass.

| Pass ID | Kind | Covered Boundary | Slicing Rationale | Decision State |
| :--- | :--- | :--- | :--- | :--- |
| `pass_01_free_space_accounting_and_discard_core` | Creator-Synced Pass | Allocation bitmap as durable free-space truth source; cached cluster accounting through superblock/statfs; recount fallback under corruption-sensitive conditions. | The accounting lane is required before later discard policy can safely consume free-space state, while discard / `FITRIM` policy remains separable. | Accepted through Reviewer. |
| `pass_01_free_space_accounting_and_discard_core_cleanup_01` | Creator Cleanup Pass | Same pass01 accounting-only boundary; cleanup objective Line A was owner/helper placement. | Reviewer/user feedback identified structural debt in the pass01 implementation, but the first cleanup packet bundled multiple objectives too coarsely. | Partial closure only; do not treat as Checker-ready. |
| `pass_01_free_space_accounting_and_discard_core_cleanup_02` | Creator Cleanup Pass | Same pass01 accounting-only boundary; cleanup objective Line B was return-carrier consistency plus temporary error-seam closure. | This continuation existed because `cleanup_01` visibly closed owner/helper work but left return-carrier and error-boundary decisions open. | Accepted through Reviewer; final checker skipped after comment-only review edit. |
| `pass_02_free_space_accounting_and_discard_online_discard_policy` | Creator-Synced Pass | `Online discard is opportunistic and can downgrade at runtime`; the post-free path remains subordinate to `Allocation bitmap is the free-space truth source` and `Superblock counters and statfs reflect cached cluster accounting`. | Online discard is advisory and depends on correct free-space commit semantics from pass01, but it must not wait for administrative `FITRIM` because post-free downgrade policy is a distinct runtime path. | Creator artifact present; a user-directed full-surface Reviewer structural audit now runs before Checker. |
| `pass_02_free_space_accounting_and_discard_online_discard_policy_structural_audit_01` | Reviewer Pass | Same pass02 covered-micro boundary, plus a full-surface audit of every production `struct`, `enum`, carrier, and non-trait helper in `fs.rs` / `bitmap.rs`. | The user explicitly called out repeated helper / `struct` omissions in `fs.rs`, so this audit forces symbol-by-symbol disposition of surviving entities before the pass can continue. | Dispatched; does not replace the ordinary post-Checker Reviewer gate if pass02 later survives Checker unchanged. |
| `pass_02_free_space_accounting_and_discard_online_discard_policy_cleanup_01` | Creator Cleanup Pass | Same pass02 covered-micro boundary; fix the rejected free-space helper-family placement, dormant trim carriers, and registration panic surface. | The structural audit rejected the pass on three individuated objectives, so the next route must be a Creator cleanup pass rather than Checker. | Creator artifact present; Reviewer follow-up dispatched to verify structural closure before Checker. |
| `pass_02_free_space_accounting_and_discard_online_discard_policy_user_named_structural_cleanup_02` | Creator Cleanup Pass | User-named surviving free-space surface: moved `ClusterRange`, renamed `FreeSpaceAllocatorState`, `FreeSpaceSnapshot`, surviving `FreeSpaceAccountingOutcome`, `FreeSpaceAccountingOperation`, `FreeSpaceAccountingError`, `free_space_accounting_and_discard`, free-space helper family placement, `AllocationBitmap` / absent `BitmapInner` naming and owner boundary, `bitmap.rs` dependence on `MountVolumeStateError`, and free-space-related tests / test-only support placement. | The April 21 problem note shows `cleanup_01` was too narrow: it closed the previous three objectives but did not force default-reject proof for the surviving carrier family and owner naming. This pass must not absorb mount-state carrier cleanup; that belongs to `meso_01`. | Reviewer rejected for structural cleanup: surviving `FreeSpaceAccountingOperation / Outcome`, `free_space_accounting_and_discard`, the `ClusterRange` re-export seam, the `bitmap.rs` `MountVolumeStateError` seam, and the inline `fs.rs` ktest module remain open. Route back to Creator cleanup. |
| `pass_02_free_space_accounting_and_discard_online_discard_policy_user_named_structural_cleanup_03` | Creator Cleanup Pass | Reviewer-rerouted residual free-space surface: surviving `FreeSpaceAccountingOperation / Outcome`, surviving `free_space_accounting_and_discard`, the `ClusterRange` re-export seam, the `bitmap.rs` `MountVolumeStateError` seam, and the inline `fs.rs` `#[cfg(ktest)] mod tests` topology. | The tightened Reviewer protocol converted the broad “cleanup looks better” wave into explicit residual blockers. This follow-up pass should close only those named residuals instead of reopening already accepted owner placement work. | Reviewer approved with no code edits; final Checker skippable. This pass is accepted through Reviewer. |
| `pass_03_free_space_accounting_and_discard_admin_trim_boundary` | Creator-Synced Pass | `FITRIM and online discard are distinct administrative free-space paths`; current Asterinas-local `EOPNOTSUPP` fast-fail boundary; no allocator/counter mutation on trim rejection. | The Designer records that Asterinas currently lacks a dedicated VFS `FITRIM` hook and lower trim primitive, so this pass owns the meso-local operation/outcome boundary and explicit unsupported result without smuggling ioctl routing or privilege checks from `meso_11`. | Accepted through Reviewer on April 22, 2026; final Checker skippable because Reviewer made no edits. |
| `pass_04_free_space_accounting_and_discard_integration` | Meso Integration Checker Pass | Cross-pass validation for allocation/free/cached reporting, online-discard downgrade, `FITRIM` fast-fail distinction, recount failure preservation, repeated snapshots, and lock-linearized allocator observations. | Integration obligations in the Designer ktest must stay Checker-owned and run only after the relevant implementation passes exist. | Initial Checker run failed before the intended assertions because `bitmap.rs` wrote a non-sector-aligned `1984`-byte slice and returned `DeviceIo`; the lane is intentionally superseded by `pass_01_free_space_accounting_and_discard_core_cleanup_03` plus `pass_04_free_space_accounting_and_discard_integration_rerun_01`. |
| `pass_01_free_space_accounting_and_discard_core_cleanup_03` | Creator Cleanup Pass | Allocation/free bitmap writeback repair for the accepted pass01 core boundary after integration exposed `DeviceIo` on non-sector-aligned bitmap writes. | `pass_04` integration failed before reaching its real assertions because `bitmap.rs` wrote `1984` bytes through a sector-alignment-requiring block-device path. Route the Checker repair batch back to the narrowest existing owner boundary: pass01 core allocation/free accounting. | Creator landed the aligned writeback repair and the synchronized Checker pass then cleared the blocker; use this pass only as the repair lane, not as a substitute for the dedicated integration acceptance lane. |
| `pass_01_free_space_accounting_and_discard_core_cleanup_03_checker` | Creator-Synced Checker Pass | Regression proof that the aligned bitmap writeback repair removes the allocate/free `DeviceIo` blocker on the pass01 accounting path. | Use the exact failing `pass_04` reproduce commands as regression receipts without collapsing the dedicated integration lane into this synchronized pass. | Checker PASS; the allocate/free `DeviceIo` blocker is cleared and the acceptance proof continues under `pass_04_free_space_accounting_and_discard_integration_rerun_01`. |
| `pass_04_free_space_accounting_and_discard_integration_rerun_01` | Meso Integration Checker Pass | Full rerun of the four `pass_04` integration obligations after `cleanup_03` cleared the bitmap writeback `DeviceIo` blocker. | The first two integration tests passed under the synchronized cleanup checker, but the evidence had to be re-collected under the dedicated integration lane and include the two previously blocked obligations. | Checker PASS. |
| `pass_04_free_space_accounting_and_discard_integration_reviewer` | Reviewer Pass | Narrow final test-surface quality gate for the already-passing `pass_04` integration ktests and their helper support. | The integration lane became runtime-complete after the rerun; Reviewer then approved the touched test-only surface with line-level-only formatting/import edits. | Reviewer APPROVED; final Checker skippable; `pass_04` accepted. |

**Cleanup Objective Closure:**

- **Line A:** Owner/helper structural cleanup closed by promoting `AllocationBitmapRecord` to `AllocationBitmap`, moving bitmap helper behavior under that owner boundary, and removing helper-only `LogicalClusterRange`.
- **Line B:** Return-carrier and temporary error-seam cleanup closed by using a pass-local `FreeSpaceAccountingError` across the accounting/reporting/recount lane and making the mount/bootstrap conversion seam explicit.

**Deferred / Exit Notes:**

- Pass02 must commit free-space correctness before any discard hint and must downgrade / disable future discard attempts without rolling back bitmap truth or cached counters.
- Pass02 is also the latest acceptable point to confirm whether `FreeSpaceAccountingError` is promoted from pass-local boundary to the stable meso-owned error surface for widened discard behavior.
- Pass03 must keep administrative `FITRIM` distinct from online discard. Under current Asterinas interfaces, an explicit `EOPNOTSUPP` boundary is a valid implementation result; it must not silently pretend real trim I/O exists.
- Pass03 may define meso-local operation/outcome carriers for administrative trim admission, but carrier-specific ioctl routing, privilege checks, and user ABI remain outside this pass unless the main agent later schedules `meso_11` coordination.
- Neither pass02 nor pass03 may reuse `MountVolumeStateError` as a broad shared enum. Any temporary seam needs an explicit exit plan before the pass can reach Reviewer.
- Structural cleanup after pass02 or pass03 is not pre-sliced. If Reviewer finds helper-family, owner-placement, return-carrier, or error-seam debt, the main agent must open a named cleanup pass with individuated objectives before continuing toward integration.
- User-directed full-surface structural audits are separate from ordinary cleanup packets: they exist to force explicit disposition of every surviving helper / `struct` in the named write-set when generic cleanup wording proved too weak.

### `meso_03_directory_lookup_and_identity`

**Decision:** Slice the meso contract into one durable-structure-backed lookup Creator pass, one durable-structure-backed directory enumeration Creator pass, and one independent meso integration Checker pass. The slicing vocabulary may cite inherited Asterinas VFS directory surfaces, but it must not create or freeze new Rust signatures, request / outcome carrier names, or dispatcher families.

| Pass ID | Kind | Covered Boundary | Slicing Rationale | Decision State |
| :--- | :--- | :--- | :--- | :--- |
| `pass_01_directory_lookup_and_identity_lookup_resolution` | Creator-Synced Pass | Lookup resolution grounded in two durable inputs: the validated Up-case Table On-disk Structure Owner published by `meso_01_mount_volume_state`, and this directory's checksum-validated `DirectoryEntrySet` material. Covers mounted codec selection, Up-case folding, `NameHash` as prefilter only, trailing-dot-sensitive folded-name comparison under `keep_last_dots`, contiguous primary / secondary entry-set validation, location-derived child identity from the matched entry-set location, alias-equivalent spelling coherence, create-oriented stale-negative revalidation, and separation among absence, invalid caller spelling, and integrity / I/O failure on the inherited lookup surface. | The first implementation pass should make ordinary lookup correct as one coherent durable-structure pipeline. Splitting Up-case Table consumption, DirectoryEntrySet validation, location-derived identity, alias coherence, and stale-negative revalidation into separate early passes would either freeze artificial helper seams or leave a Checker with only partial, non-user-meaningful lookup behavior. | Checker rerun 02 PASS confirmed. Reviewer found a structural `iocharset` lookup seam in `inode.rs`, but user explicitly decided to absorb that seam cleanup into `pass_02_directory_lookup_and_identity_readdir_visibility_boundary` and treat `pass_01` as accepted. |
| `pass_02_directory_lookup_and_identity_readdir_visibility_boundary` | Creator-Synced Pass | Directory enumeration grounded in the same checksum-validated `DirectoryEntrySet` material and the same location-derived identity rules from `pass_01`: scan-order traversal, traversal-progress semantics on the inherited enumeration surface, inherited visitor rejection / stop behavior, visible-entry filtering, dirent identity emission from durable entry-set location, and typed benign-versus-critical handling for unrecognized directory entries on the enumeration-facing surface. User-carried forward cleanup also absorbs the unresolved lookup-side `iocharset` seam from `pass_01`: either the published codec policy shapes a real lookup/enumeration codec boundary here, or the seam is narrowed / documented precisely enough that it no longer survives as an inert no-op publication consumer. | Enumeration has a distinct public behavior boundary: offset/progress handling, visitor failure propagation, visible-entry filtering, and unrecognized-entry presentation. Keeping it separate lets `pass_01` stabilize child resolution while `pass_02` explicitly proves enumeration reuses the same durable DirectoryEntrySet and Up-case Table / identity truths rather than inventing a parallel namespace path. The user-directed carry-forward avoids reopening accepted `pass_01` solely for the inert codec seam. | Planned; absorb the carried `iocharset` seam cleanup before or with `pass_02` dispatch. |
| `pass_03_directory_lookup_and_identity_integration` | Meso Integration Checker Pass | Cross-surface validation that inherited lookup and enumeration surfaces present one coherent namespace view over the same Up-case Table On-disk Structure Owner and DirectoryEntrySet truth: mixed-case and trailing-dot-sensitive lookup, alias-equivalent identity reuse, repeated-call stability, stale-negative revalidation, and integrity-failure separation for fractured / unrecognized entry sets. | The Designer ktest couples lookup identity, enumeration visibility, and anomaly classification across both inherited VFS surfaces, so the final acceptance proof must remain Checker-owned and separate from the two Creator-synced passes. | Planned; dispatch only after `pass_01` and `pass_02` are accepted. |

**Deferred / Exit Notes:**

- `pass_01` intentionally carries the lookup-facing identity and stale-negative obligations together with Up-case Table consumption and DirectoryEntrySet validation; do not peel alias reuse or create-oriented revalidation into a third early Creator pass unless a future Reviewer rejection proves the current slice is too broad.
- `pass_02` must reuse the durable naming / identity rules already established by `pass_01`; it must not invent a separate enumeration-only identity path, weaker DirectoryEntrySet validation, or weaker unrecognized-entry policy.
- Mentions of inherited VFS names such as lookup, enumeration, or visitor behavior are routing anchors only. Creator packets must not treat this slicing ledger as authorization to invent new aggregate request / outcome carriers, new dispatcher enums, or exact function signatures beyond pre-existing stable kernel interfaces.
- Structural cleanup is not pre-sliced. If Reviewer later finds helper-family, identity-cache, return-carrier, or test-topology debt, open a named cleanup pass with explicit objectives instead of widening `pass_01` or `pass_02` after the fact.

### `meso_04_directory_entry_mutation`

**Decision:** Architect and Designer artifacts are accepted. This meso now starts with one prerequisite owner-local directory-entry skeleton pass before the user-visible create / remove / rename passes.

| Candidate Pass Area | Current Decision | Rationale |
| :--- | :--- | :--- |
| `pass_01_directory_entry_mutation_direntry_owner_local_skeleton` | Creator Cleanup Pass. Covered micro-features: `File and directory names live in consecutive directory-entry sets guarded by set checksums`; `directory removal and directory-target rename require an emptiness gate`; `Unrecognized directory entries impose typed invalidity and no-modify boundaries instead of generic ignore behavior` | `meso_03` established low-level read-side helpers, but it did not leave behind one shared owner-local directory-entry mutation substrate. Without an explicit first pass, `create`, `unlink` / `rmdir`, and `rename` would each re-open scan / classification / emptiness / slot-location logic inside long `inode.rs` methods. This pass may introduce only an owner-local internal seam for validated entry-set walking, slot-range / vacancy discovery, emptiness scanning, and typed invalidity classification under `ExfatInode(dir)`; it must not invent a new public facade, request / outcome carrier family, or Designer-frozen generic interface layer. |
| `pass_02_directory_entry_mutation_create_mkdir_publication` | Creator-Synced Pass. Covered micro-features: `File and directory names live in consecutive directory-entry sets guarded by set checksums`; `create and mkdir secure directory slots before committing new entry sets`; `zero_size_dir changes only the newborn directory's initial allocation shape`; `Asterinas path-tree surface is positional and still requires explicit refusal boundaries` | After `pass_01` establishes the shared owner-local direntry substrate, creation becomes the first coherent user-visible publication boundary: parent slot reservation, optional parent growth, contiguous checksum-valid entry-set publication, and newborn-directory shape rules can be implemented without entangling delete or rename ordering. This pass should also close the local refusal boundary for unsupported adjacent tree-mutation surfaces that are naturally exercised near the inherited create-side routing. |
| `pass_03_directory_entry_mutation_unlink_rmdir_ordering` | Creator-Synced Pass. Covered micro-features: `unlink and rmdir invalidate entry sets before freeing their cluster state`; `directory removal and directory-target rename require an emptiness gate`; `Unrecognized directory entries impose typed invalidity and no-modify boundaries instead of generic ignore behavior` | Delete-like paths share one distinct correctness contract: namespace-first invalidation, live emptiness gating for directory removal, typed refusal on invalid / unrecognized target material, and allocator handoff that never leaves a visible name pointing at reclaimed clusters. Keeping them together gives Checker one clear failure-ordering proof. |
| `pass_04_directory_entry_mutation_rename_ordering` | Creator-Synced Pass. Covered micro-features: `Cross-directory rename secures the new home before invalidating the old one`; `rename accepts ordinary semantics and RENAME_NOREPLACE only`; `Cross-directory rename crash windows may transiently duplicate reachability rather than orphan it`; `directory removal and directory-target rename require an emptiness gate`; `Asterinas path-tree surface is positional and still requires explicit refusal boundaries` | Rename is the highest-coupling mutation path: it spans same-directory and cross-directory rewriting, destination-first ordering, durable-identity lock order, the allowed rename semantic boundary, and directory-target emptiness. It should not be mixed with creation or delete-like cleanup because those would blur the crash-window and multi-directory admission proof. |
| `pass_05_directory_entry_mutation_integration` | Meso Integration Checker Pass | Integration remains Checker-owned and waits for accepted implementation passes. It should validate the full create → cross-directory rename → unlink / rmdir namespace sequence plus failure-maintenance scenarios after `pass_02` through `pass_04` are closed. |

**Deferred / Exit Notes:**

- `pass_01` is not a license to invent a reusable public `direntry` API. The intended target is an owner-local internal substrate under `ExfatInode(dir)` and adjacent exFAT-local code only.
- `pass_01` may create a dedicated `direntry`-focused Rust file only if the Creator can justify that the seam is still owner-local / exFAT-local and materially reduces duplicated scan logic without introducing a new carrier family.
- Later passes must reuse the `pass_01` substrate instead of cloning directory-entry walking, emptiness scanning, or typed invalidity classification loops into separate long methods.
- `pass_02` through `pass_04` remain user-visible behavior passes. If `pass_01` grows beyond the internal direntry skeleton boundary, reject it and route a narrower repair instead of silently absorbing public mutation semantics early.

### `meso_05_file_content_mapping_and_cached_io`

**Decision:** Architect and Designer artifacts are accepted, but no Creator pass has been sliced.

| Candidate Pass Area | Current Decision | Rationale |
| :--- | :--- | :--- |
| File content mapping / cached I/O implementation | Not yet sliced. | Hold until the main agent selects a covered-micro subset from the accepted meso Designer contract. |
| Meso integration validation | Not yet sliced. | Integration Checker work waits for accepted implementation passes covering the target micro-features. |

### `meso_06_file_content_mutation`

**Decision:** Architect and Designer artifacts are accepted, but no Creator pass has been sliced.

| Candidate Pass Area | Current Decision | Rationale |
| :--- | :--- | :--- |
| File content mutation implementation | Not yet sliced. | Hold until the main agent selects a covered-micro subset from the accepted meso Designer contract. |
| Meso integration validation | Not yet sliced. | Integration Checker work waits for accepted implementation passes covering the target micro-features. |

### `meso_07_file_sync_and_persistence`

**Decision:** Architecture artifact is accepted, but Designer work is not yet complete, so no pass-level implementation slicing is allowed.

| Candidate Pass Area | Current Decision | Rationale |
| :--- | :--- | :--- |
| File sync and persistence implementation | Blocked. | Top-down protocol forbids Creator passes before the meso Designer spec and ktest contract exist. |
| Meso integration validation | Blocked. | Integration obligations must come from the future Designer ktest contract. |

### `meso_08_filesystem_sync_and_volume_state`

**Decision:** Architect and Designer artifacts are accepted, but no Creator pass has been sliced.

| Candidate Pass Area | Current Decision | Rationale |
| :--- | :--- | :--- |
| Filesystem sync and volume-state implementation | Not yet sliced. | Hold until the main agent selects a covered-micro subset from the accepted meso Designer contract. |
| Meso integration validation | Not yet sliced. | Integration Checker work waits for accepted implementation passes covering the target micro-features. |

### `meso_09_file_metadata_projection_and_update`

**Decision:** Planned from accepted macro topology only; no Architect, Designer, or pass-level slicing exists yet.

### `meso_10_directory_metadata_projection_and_update`

**Decision:** Planned from accepted macro topology only; no Architect, Designer, or pass-level slicing exists yet.

### `meso_11_volume_admin_identity`

**Decision:** Planned from accepted macro topology only; no Architect, Designer, or pass-level slicing exists yet.

## Process Notes From Cleanup Gaps

- `meso_02` cleanup split happened because the original cleanup packet grouped owner/helper placement and carrier/error-boundary work together; visible progress on Line A was briefly mistaken for full cleanup closure.
- Future cleanup packets must list structural objectives individually, and Creator / Reviewer artifacts must disposition each objective independently.
- Checker run archives now live under `.agents/checker-runs/<meso-component>/...` so evidence remains grouped by parent meso-component.
