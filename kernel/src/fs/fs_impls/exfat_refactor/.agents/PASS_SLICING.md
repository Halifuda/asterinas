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

**Decision:** Architect and Designer artifacts are accepted, but no Creator pass has been sliced.

| Candidate Pass Area | Current Decision | Rationale |
| :--- | :--- | :--- |
| Directory lookup and identity implementation | Not yet sliced. | Hold until the main agent selects a covered-micro subset from the accepted meso Designer contract. |
| Meso integration validation | Not yet sliced. | Integration Checker work waits for accepted implementation passes covering the target micro-features. |

### `meso_04_directory_entry_mutation`

**Decision:** Architect and Designer artifacts are accepted, but no Creator pass has been sliced.

| Candidate Pass Area | Current Decision | Rationale |
| :--- | :--- | :--- |
| Directory entry mutation implementation | Not yet sliced. | Hold until the main agent selects a covered-micro subset from the accepted meso Designer contract. |
| Meso integration validation | Not yet sliced. | Integration Checker work waits for accepted implementation passes covering the target micro-features. |

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
