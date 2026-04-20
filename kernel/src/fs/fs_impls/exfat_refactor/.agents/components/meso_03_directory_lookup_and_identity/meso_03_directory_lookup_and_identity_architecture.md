<!-- SPDX-License-Identifier: MPL-2.0 -->

# Meso 03: `directory_lookup_and_identity` Architecture

## 1. Meso-Component Definition
- **Component**: `directory_lookup_and_identity`
- **Macro-Owner**: `ExfatInode(dir)`
- **Responsibility**: Owns read-side namespace resolution for exFAT directories, including durable naming truth consumption, encoding-aware name comparison, stable inode-identity reconstruction from directory-entry location, cache/alias reconciliation, and typed lookup-facing anomaly boundaries for unrecognized directory entries.

## 2. Micro-Feature Traceability Matrix
<!-- List ALL micro-features from the inventory mapped to this component. NO OWNER GAPS ALLOWED. -->
<!-- Keep each micro-feature as an explicit row. The main agent will later group rows into Creator/Checker passes. -->
| Micro-Feature Name | Prior Reference | Description / Requisite |
|---|---|---|
| `Up-case Table remains the durable naming truth` | `INV-PHY-005`; `.agents/priors/Microsoft-exFAT-spec-index.md` -> `### 7.2 Up-case Table Directory Entry`, `#### 7.2.2 TableChecksum Field`, `#### 7.2.5 Up-case Table` | `directory_lookup_and_identity` consumes the mount-validated Up-case Table as the architectural case-folding authority for non-UTF-8-specific lookup paths rather than replacing it with a transient cache policy. |
| `Directory-entry sets must be read as contiguous checksum-validated units` | `INV-PHY-007`; `.agents/priors/Microsoft-exFAT-spec-index.md` -> `#### 6.3.2 SecondaryCount Field`, `#### 6.3.3 SetChecksum Field`, `### 7.6 Stream Extension Directory Entry`, `#### 7.7.3 FileName Field` | Lookup and readdir must trust directory entries only as consecutive primary-plus-secondary sets guarded by `SecondaryCount` and `SetChecksum`, because fractured or invalid sets cannot safely yield stable identity or resolved names. |
| `iocharset selects the lookup codec and comparison pipeline` | `INV-VFS-005`; `.agents/priors/linux-exFAT-implementation-summary.md` -> `2.1 Initialization & Global Status`, `2.2 Name Resolution & Encoding` | This meso owns the rule that mount-selected `iocharset` determines whether name materialization and comparison use the UTF-8-specific path or an NLS-backed conversion path. |
| `Stable inode identity comes from directory-entry location` | `INV-VFS-006`; `.agents/priors/linux-exFAT-implementation-summary.md` -> `2.2 Name Resolution & Encoding`; `.agents/priors/ASTERINAS_INTEGRATION_PRIORS.md` -> `5. Exhaustive VFS Inode Interfaces` | Non-root identity must be reconstructed from durable directory-entry location rather than transient pathname spelling, and lookup/readdir paths must preserve that identity anchor when rediscovering an object. |
| `Create-oriented resolution cannot trust stale negative-cache state` | `INV-VFS-007`; `.agents/priors/linux-exFAT-implementation-summary.md` -> `2.2 Name Resolution & Encoding`; `.agents/priors/ASTERINAS_INTEGRATION_PRIORS.md` -> `5. Exhaustive VFS Inode Interfaces` | If a namespace carrier caches misses, this meso owns the lookup-side requirement that create-oriented resolution re-evaluate the exact user spelling instead of blindly trusting an old miss once aliases or parent-state changes may have materialized. |
| `Name matching is encoding-aware, case-folded, and trailing-dot-sensitive` | `INV-VFS-008`; `.agents/priors/linux-exFAT-implementation-summary.md` -> `2.2 Name Resolution & Encoding`; `.agents/priors/Microsoft-exFAT-spec-index.md` -> `### 7.2 Up-case Table Directory Entry`, `#### 7.6.4 NameHash Field`, `#### 7.7.3 FileName Field` | `NameHash` remains only a prefilter; candidate resolution still requires full folded-name comparison, with `iocharset` and `keep_last_dots` determining the normalization and comparison path. |
| `keep_last_dots changes lookup equivalence but not creation legality` | `INV-VFS-009`; `.agents/priors/linux-exFAT-implementation-summary.md` -> `2.2 Name Resolution & Encoding` | This meso owns the lookup-side equivalence rule for trailing dots while leaving the later create/rename refusal boundary to the namespace-mutation meso. |
| `Alias-bearing names require identity reuse and explicit coherence handling` | `INV-VFS-010`; `.agents/priors/linux-exFAT-implementation-summary.md` -> `2.2 Name Resolution & Encoding`; `.agents/priors/ASTERINAS_INTEGRATION_PRIORS.md` -> `5. Exhaustive VFS Inode Interfaces` | Distinct spellings that resolve to one on-disk object must reuse or reconcile one identity, and case-only rename fallout remains an explicit coherence boundary rather than an invisible implementation detail. |
| `Unrecognized directory entries impose typed lookup-facing boundaries` | `INV-VFS-044`; `.agents/priors/Microsoft-exFAT-spec-index.md` -> `### 8.2 Implications of Unrecognized Directory Entries` | Lookup and readdir must respect the spec's typed invalidity and limited-allowance rules for unrecognized primary and secondary entries instead of collapsing them into generic ignore-or-fail behavior. |

## 3. Static Lock Boundaries
- **Expected Inlet State**:
  - `lookup`, `readdir`, alias reconciliation, and negative-cache revalidation must enter with at most `ExfatFs state rwlock(Read)` and the target directory's `ExfatInode rwlock(Read)`.
  - Callers must not arrive holding `ExfatFs allocator rwlock` or any `ExfatStream extent rwlock`; this meso is a read-side namespace boundary, not a free-space or stream-mutation entry.
  - Root-directory lookup may consume the root identity and naming truth published by `meso_01_mount_volume_state`, but it may not republish mount-global state on its own.
- **Topology Placement**:
  - Highest lock level permitted to acquire internally: `Level 2` (`ExfatInode rwlock(Read)`).
  - Prohibited dependencies: `Cannot acquire stream-extent or allocator locks`; `cannot pull create/unlink/rmdir/rename mutation semantics into this meso except as downstream refusal or interaction notes`; `cannot revise the frozen ExfatFs -> ExfatInode hierarchy`.

## 4. External structural interactions
<!-- Static, strict interactions with other Macro components. 
DO NOT write dynamic execution paths. 
DO NOT advise on private helper function architectures (leave to Creator). -->
- Consumes the validated Up-case Table, mount-selected codec policy, and root-directory identity seed established by `meso_01_mount_volume_state`.
- Supplies stable identity and lookup resolution boundaries that `meso_04_directory_entry_mutation`, `meso_09_file_metadata_projection_and_update`, and `meso_10_directory_metadata_projection_and_update` must preserve when they later rewrite names or entry locations.
- Leaves fresh-name legality, namespace mutation ordering, and trailing-dot creation refusal to `meso_04_directory_entry_mutation`; this meso only defines the read-side lookup equivalence and cache-revalidation contract.
- Shares the typed anomaly boundary for unrecognized directory entries with `meso_04_directory_entry_mutation`: this meso owns scan/traverse/readdir-facing interpretation, while later delete-or-move allowances on such entries remain downstream mutation concerns.
