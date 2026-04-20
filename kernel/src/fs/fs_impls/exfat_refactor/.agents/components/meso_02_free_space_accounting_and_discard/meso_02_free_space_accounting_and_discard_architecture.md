<!-- SPDX-License-Identifier: MPL-2.0 -->

# Meso 02: `free_space_accounting_and_discard` Architecture

## 1. Meso-Component Definition
- **Component**: `free_space_accounting_and_discard`
- **Macro-Owner**: `ExfatFs`
- **Responsibility**: Owns the mount-global free-space truth after bootstrap, including steady-state `used_clusters` accounting, `statfs`-visible reporting, opportunistic runtime discard policy, administrative `FITRIM` scanning over free ranges, and recount fallback when cached accounting can no longer be trusted.

## 2. Micro-Feature Traceability Matrix
<!-- List ALL micro-features from the inventory mapped to this component. NO OWNER GAPS ALLOWED. -->
<!-- Keep each micro-feature as an explicit row. The main agent will later group rows into Creator/Checker passes. -->
| Micro-Feature Name | Prior Reference | Description / Requisite |
|---|---|---|
| `Allocation bitmap remains the durable free-space truth source` | `INV-PHY-002`; `.agents/priors/linux-exFAT-implementation-summary.md` -> `2.1 Initialization & Global Status`; `.agents/priors/Microsoft-exFAT-spec-index.md` -> `### 7.1 Allocation Bitmap Directory Entry`, `#### 7.1.5 Allocation Bitmap` | `free_space_accounting_and_discard` must continue to treat the Allocation Bitmap as the authoritative free-space source after mount seeding, so later allocation/free reporting never drifts onto unrelated metadata or ad hoc estimators. |
| `Superblock counters and statfs expose cached cluster accounting` | `INV-VFS-002`; `.agents/priors/linux-exFAT-implementation-summary.md` -> `2.1 Initialization & Global Status`; `.agents/priors/ASTERINAS_INTEGRATION_PRIORS.md` -> `4. Exhaustive FileSystem Trait (VFS Mount & Superblock)` | This meso owns the steady-state reporting contract that `statfs` and other superblock counters derive from accepted geometry plus incrementally maintained `used_clusters`, not from a fresh full bitmap scan on each query. |
| `Online discard is opportunistic and may downgrade at runtime` | `INV-VFS-003`; `.agents/priors/linux-exFAT-implementation-summary.md` -> `2.1 Initialization & Global Status`, `2.7 Administrative & Maintenance ABI` | Runtime discard remains a hint layered on correct cluster release: unsupported media, mount-time capability mismatch, or later `-EOPNOTSUPP` responses may disable future discards without invalidating ordinary free-space correctness. |
| `FITRIM is distinct from online discard` | `INV-VFS-035`; `.agents/priors/linux-exFAT-implementation-summary.md` -> `2.7 Administrative & Maintenance ABI`, `2.1 Initialization & Global Status` | This meso must preserve the distinction between an administrative bulk-trim scan over currently free ranges and opportunistic discard on future frees, including their separate gating checks, capability tests, and failure behavior. |
| `Accounting may fall back to recount under corruption-sensitive conditions` | `INV-VFS-042`; `.agents/priors/linux-exFAT-implementation-summary.md` -> `2.1 Initialization & Global Status` | When incremental `used_clusters` state is no longer trustworthy, this meso admits a recount from the Allocation Bitmap rather than continuing to expose stale free-space counters or allocator assumptions. |

## 3. Static Lock Boundaries
- **Expected Inlet State**:
  - `statfs`, allocator-accounting maintenance, online discard handling, and `FITRIM` admission must enter through `ExfatFs`-owned state; callers must not arrive holding any `ExfatInode rwlock` or `ExfatStream extent rwlock`.
  - Read-side reporting may begin at `ExfatFs state rwlock(Read)`, while allocator mutations, recount admission, and discard-policy transitions require `ExfatFs state rwlock(Write/intend-mutate)` before descending into allocator-owned state.
  - This meso may assume mount/bootstrap already seeded geometry, root publication, and initial bitmap acceptance through `meso_01_mount_volume_state`; it must not reopen boot validation inside the allocator lane.
- **Topology Placement**:
  - Highest lock level permitted to acquire internally: `Level 4` (`ExfatFs allocator rwlock`).
  - Prohibited dependencies: `Cannot enter from any inode-locked or stream-locked context`; `cannot acquire inode or stream-extent locks after entering allocator state`; `cannot revise the frozen macro hierarchy or absorb per-file / per-directory mutation semantics that belong to later meso components`.

## 4. External structural interactions
<!-- Static, strict interactions with other Macro components. 
DO NOT write dynamic execution paths. 
DO NOT advise on private helper function architectures (leave to Creator). -->
- Consumes the mount-established bitmap seed, geometry, and recount-admission posture from `meso_01_mount_volume_state`; this meso owns the later steady-state accounting and reporting contract.
- Supplies free-space and allocation-truth boundaries to `meso_04_directory_entry_mutation` and `meso_06_file_content_mutation`, which may request cluster allocation/free but must not redefine allocator authority.
- Shares the administrative trim boundary with `meso_11_volume_admin_identity`: this meso owns the free-range semantics of `FITRIM`, while carrier-specific privilege/refusal routing may remain outside it.
- Imports volume-state overlays from `meso_08_filesystem_sync_and_volume_state` when anomaly or shutdown posture suppresses later discard behavior, but does not itself own the global state transition that created that posture.
