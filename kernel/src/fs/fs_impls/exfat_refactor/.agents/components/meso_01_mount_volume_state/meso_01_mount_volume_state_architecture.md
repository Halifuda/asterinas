<!-- SPDX-License-Identifier: MPL-2.0 -->

# Meso 01: `mount_volume_state` Architecture

## 1. Meso-Component Definition
- **Component**: `mount_volume_state`
- **Macro-Owner**: `ExfatFs`
- **Responsibility**: Establishes the mount-scoped exFAT runtime from validated boot and root anchors, fixes the remount policy boundary, seeds superblock-visible counters from accepted durable sources, and synchronously exposes the mounted root / superblock view required by Asterinas without taking ownership of later allocator-steady-state or lookup-steady-state behavior.

## 2. Micro-Feature Traceability Matrix
<!-- List ALL micro-features from the inventory mapped to this component. NO OWNER GAPS ALLOWED. -->
<!-- Keep each micro-feature as an explicit row. The main agent will later group rows into Creator/Checker passes. -->
| Micro-Feature Name | Prior Reference | Description / Requisite |
|---|---|---|
| `Boot region validation and parameter load at mount` | `INV-PHY-001`; `.agents/priors/linux-exFAT-implementation-summary.md` -> `2.1 Initialization & Global Status`; `.agents/priors/Microsoft-exFAT-spec-index.md` -> `## 3 Main and Backup Boot Regions` | `mount_volume_state` must accept geometry, root-directory anchor, and boot-region runtime state only after checksum and field-range validation, while treating Backup Boot `VolumeFlags` as stale recovery material rather than live authority. |
| `Allocation bitmap is the free-space truth source` | `INV-PHY-002`; `.agents/priors/linux-exFAT-implementation-summary.md` -> `2.1 Initialization & Global Status`; `.agents/priors/Microsoft-exFAT-spec-index.md` -> `### 7.1 Allocation Bitmap Directory Entry` | This meso seeds the mount-global free-space view from the Allocation Bitmap during mount/bootstrap so later `sb()` / `statfs` queries start from accepted durable truth instead of ad hoc scans. |
| `VolumeDirty marks in-flight versus quiesced global state` | `INV-PHY-003`; `.agents/priors/linux-exFAT-implementation-summary.md` -> `2.1 Initialization & Global Status`; `.agents/priors/Microsoft-exFAT-spec-index.md` -> `##### 3.1.13.2 VolumeDirty Field` | This meso interprets the persisted dirty marker into mount-visible runtime posture and preserves the fact that dirty/clean state is volume-scoped rather than inode-scoped. |
| `VolumeFlags also carries media-failure and clear-before-modify state` | `INV-PHY-004`; `.agents/priors/Microsoft-exFAT-spec-index.md` -> `##### 3.1.13.3 MediaFailure Field`, `##### 3.1.13.4 ClearToZero Field` | `mount_volume_state` must import `MediaFailure` / `ClearToZero` as accepted anomaly posture from the boot region so later mutators and recovery-aware paths inherit the correct preconditions. |
| `Up-case Table is the durable case-folding truth source` | `INV-PHY-005`; `.agents/priors/linux-exFAT-implementation-summary.md` -> `2.1 Initialization & Global Status`; `.agents/priors/Microsoft-exFAT-spec-index.md` -> `### 7.2 Up-case Table Directory Entry` | This meso loads and validates the Up-case Table at mount so later lookup-family meso components consume a mounted naming truth that is already checksum-vetted. |
| `Mount option defaults and remount mutability boundary` | `INV-VFS-001`; `.agents/priors/linux-exFAT-implementation-summary.md` -> `2.1 Initialization & Global Status` | This meso owns initial option parsing plus remount admissibility, including the rule that dentry/inode-semantic options stay fixed across remount while `discard` remains the only explicitly mutable option in the current priors. |
| `Superblock counters and statfs reflect cached cluster accounting` | `INV-VFS-002`; `.agents/priors/linux-exFAT-implementation-summary.md` -> `2.1 Initialization & Global Status`; `.agents/priors/ASTERINAS_INTEGRATION_PRIORS.md` -> `4. Exhaustive FileSystem Trait (VFS Mount & Superblock)` | `mount_volume_state` must expose `sb()` / `statfs` geometry from accepted mount parameters plus cached `used_clusters`, not from a fresh bitmap walk on every query. |
| `Asterinas mount lifecycle must eagerly expose root inode and global sync state` | `INV-VFS-004`; `.agents/priors/ASTERINAS_INTEGRATION_PRIORS.md` -> `4. Exhaustive FileSystem Trait (VFS Mount & Superblock)` | This meso must publish one coherent mounted instance that can synchronously return `root_inode()` exactly once, answer `sb()`, and advertise filesystem-wide state through the `FileSystem` trait surface. |
| `Mount-time accounting may fall back to recount under corruption-recovery conditions` | `INV-VFS-042`; `.agents/priors/linux-exFAT-implementation-summary.md` -> `2.1 Initialization & Global Status` | When the cached `used_clusters` seed cannot be trusted during mount/bootstrap, this meso admits the recovery boundary that a recount may be required before superblock-visible counters are considered coherent. |

## 3. Static Lock Boundaries
- **Expected Inlet State**:
  - `mount`, `remount`, `root_inode()`, and `sb()` must enter without any pre-held `ExfatInode rwlock`, `ExfatStream extent rwlock`, or `ExfatFs allocator rwlock`.
  - The externally visible mounted state linearizes through `ExfatFs state rwlock`; fresh mount/bootstrap may instantiate that state before publication, but no caller may enter this meso from below Level 1.
  - Root bootstrap may rely on the accepted root-directory anchor from the validated boot region, but it may not assume a pre-existing published inode object.
- **Topology Placement**:
  - Highest lock level permitted to acquire internally: `Level 4` during one-time allocator seeding or recount admission at mount; `Level 2` for root-directory publication / read-side bootstrap once `Level 1` is established.
  - Prohibited dependencies: `Cannot begin from any inode-locked or allocator-locked context`; `cannot acquire a higher lock after entering Level 2/4`; `cannot pull in directory-lookup, namespace-mutation, or steady-state allocator contracts that belong to later meso components`.

## 4. External structural interactions
<!-- Static, strict interactions with other Macro components. 
DO NOT write dynamic execution paths. 
DO NOT advise on private helper function architectures (leave to Creator). -->
- Seeds `ExfatFs` mount-global geometry, anomaly posture, and option state that later `ExfatFs` meso components must consume without redefining boot-region authority.
- Initializes the Allocation Bitmap / `used_clusters` reporting seed for `meso_02_free_space_accounting_and_discard`; later incremental accounting and runtime discard downgrades remain outside this meso.
- Publishes the validated Up-case Table and root-directory anchor for `meso_03_directory_lookup_and_identity`; later name-folding and identity reconstruction stay in that downstream meso.
- Exposes Asterinas `FileSystem` surfaces (`root_inode()`, `sb()`, mount flags view) needed for a coherent mounted instance, while later filesystem-wide dirty-state transitions and sync persistence remain owned by `meso_08_filesystem_sync_and_volume_state`.
- Carries mount-visible anomaly interpretation (`VolumeDirty`, `MediaFailure`, `ClearToZero`) as static mount posture only; later mutator obligations triggered by those flags attach to the mutation and sync meso components rather than being redefined here.
