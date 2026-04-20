<!-- SPDX-License-Identifier: MPL-2.0 -->

# Meso 08: `filesystem_sync_and_volume_state` Architecture

## 1. Meso-Component Definition
- **Component**: `filesystem_sync_and_volume_state`
- **Macro-Owner**: `ExfatFs`
- **Responsibility**: Owns the mount-scoped persistence and quiesce boundary after bootstrap, including whole-filesystem `sync`, durable dirty/clean posture, anomaly-state interpretation for `VolumeDirty`, `MediaFailure`, and `ClearToZero`, and the mount-wide consequences of administrative forced shutdown once that transition is admitted, while leaving root publication to `meso_01_mount_volume_state` and ordinary-file `fsync` to `meso_07_file_sync_and_persistence`.

## 2. Micro-Feature Traceability Matrix
<!-- List ALL micro-features from the inventory mapped to this component. NO OWNER GAPS ALLOWED. -->
<!-- Keep each micro-feature as an explicit row. The main agent will later group rows into Creator/Checker passes. -->
| Micro-Feature Name | Prior Reference | Description / Requisite |
|---|---|---|
| `VolumeDirty tracks in-flight versus quiesced filesystem state` | `INV-PHY-003`; `.agents/priors/linux-exFAT-implementation-summary.md` -> `2.1 Initialization & Global Status`; `.agents/priors/Microsoft-exFAT-spec-index.md` -> `#### 3.1.13 VolumeFlags Field`, `##### 3.1.13.2 VolumeDirty Field` | `filesystem_sync_and_volume_state` owns the mount-wide dirty/clean transition boundary, ensuring the durable dirty marker reflects filesystem-wide quiesce rather than one inode-local flush event. |
| `VolumeFlags also carry media-failure and clear-before-modify posture` | `INV-PHY-004`; `.agents/priors/Microsoft-exFAT-spec-index.md` -> `#### 3.1.13 VolumeFlags Field`, `##### 3.1.13.3 MediaFailure Field`, `##### 3.1.13.4 ClearToZero Field` | This meso owns the ongoing volume-state meaning of `MediaFailure` and `ClearToZero` after mount, including the rule that `ClearToZero` remains a pre-modification obligation before later filesystem, directory, or file mutation. |
| `Recommended write ordering uses VolumeDirty as the on-disk consistency bracket` | `INV-PHY-010`; `.agents/priors/Microsoft-exFAT-spec-index.md` -> `##### 3.1.13.2 VolumeDirty Field`, `### 8.1 Recommended Write Ordering` | This meso explicitly carries the static `VolumeDirty` overlay obligation for filesystem-wide persistence: mutator meso components still own their local ordered update domains, but whole-filesystem quiesce and clean completion must preserve `VolumeDirty` as the durability bracket around those consistency-sensitive sequences. |
| `Asterinas mount lifecycle must eagerly expose root inode and global sync state` | `INV-VFS-004`; `.agents/priors/ASTERINAS_INTEGRATION_PRIORS.md` -> `4. Exhaustive FileSystem Trait (VFS Mount & Superblock)` | `meso_01_mount_volume_state` keeps root publication and initial mount lifecycle ownership, while `filesystem_sync_and_volume_state` owns the continuing `FileSystem::sync()` side of that Asterinas contract for a live mounted instance and its whole-filesystem persistence state. |
| `Forced shutdown is a first-class volume-state transition` | `INV-VFS-036`; `.agents/priors/linux-exFAT-implementation-summary.md` -> `2.7 Administrative & Maintenance ABI`, `2.6 Runtime File I/O Surface` | Once an administrative forced-shutdown request is admitted, this meso owns the resulting mount-wide state transition: later ordinary I/O must fail fast, and runtime discard suppression becomes part of the filesystem's post-shutdown posture. |
| `Dirty, media-failure, clear-to-zero, and forced-shutdown remain anomaly surfaces` | `INV-VFS-043`; `.agents/priors/linux-exFAT-implementation-summary.md` -> `2.1 Initialization & Global Status`, `2.6 Runtime File I/O Surface`, `2.7 Administrative & Maintenance ABI`; `.agents/priors/Microsoft-exFAT-spec-index.md` -> `##### 3.1.13.2 VolumeDirty Field`, `##### 3.1.13.3 MediaFailure Field`, `##### 3.1.13.4 ClearToZero Field` | This meso keeps volume anomaly states out of ordinary steady-state semantics: they shape quiesce, sync, and later fast-fail posture instead of being treated as harmless flags with no architectural consequence. |

## 3. Static Lock Boundaries
- **Expected Inlet State**:
  - Whole-filesystem `sync`, clean unmount/quiesce, and later volume-state transitions must enter from the top of the hierarchy through `ExfatFs state rwlock`; callers must not start inside file-scoped or directory-scoped locks.
  - This meso may assume mount-time import of anomaly bits already happened in `meso_01_mount_volume_state`, but later persistent state transitions and quiesce boundaries are owned here rather than re-opened in the mount/bootstrap meso.
  - Callers must not arrive holding `ExfatFs allocator rwlock`, `ExfatInode rwlock`, or `ExfatStream extent rwlock`; if lower-level state must be coordinated, it must remain subordinate to the existing `ExfatFs`-first topology.
- **Topology Placement**:
  - Highest lock level permitted to acquire internally: `Level 4` (`ExfatFs allocator rwlock`) when filesystem-wide quiesce or state persistence must coordinate lower-level allocator or inode state in the frozen order.
  - Prohibited dependencies: `Cannot begin from file-scoped sync or mutation contexts`; `cannot treat one inode's persistence contract as equivalent to whole-filesystem quiesce`; `cannot invert the frozen ExfatFs state -> ExfatInode -> ExfatStream extent -> ExfatFs allocator hierarchy`.

## 4. External structural interactions
<!-- Static, strict interactions with other Macro components. 
DO NOT write dynamic execution paths. 
DO NOT advise on private helper function architectures (leave to Creator). -->
- Consumes the mount-time anomaly posture imported by `meso_01_mount_volume_state`, then owns later mount-wide transitions such as durable dirty/clean changes and forced-shutdown consequences.
- Preserves the Asterinas `FileSystem::sync()` contract from `INV-VFS-004` without re-absorbing root publication or initial mount lifecycle work from `meso_01_mount_volume_state`.
- Coordinates with `meso_07_file_sync_and_persistence` without absorbing its per-file contract: file-scoped `fsync` remains a sibling meso, while this component owns the volume-wide quiesce and clean-state boundary above it.
- Supplies post-transition volume-state inputs to `meso_02_free_space_accounting_and_discard`, `meso_04_directory_entry_mutation`, and `meso_06_file_content_mutation`, which must observe anomaly or shutdown posture but may not redefine it.
- Preserves `INV-PHY-010` only as a global persistence bracket / overlay obligation: `meso_04_directory_entry_mutation` and `meso_06_file_content_mutation` still own their local ordered mutation domains.
- Shares the control-surface boundary with `meso_11_volume_admin_identity`: administrative carriers may admit forced-shutdown commands there, but the resulting filesystem-wide state machine lives here.
