<!-- SPDX-License-Identifier: MPL-2.0 -->

# System Blueprint & Dispatch Ledger

This file is the dynamic central blackboard and tracker for the multi-agent exFAT refactor. It tracks the progress of the Top-Down Strict Protocol, ensuring all artifacts are generated in the correct sequence and no concurrency invariants (locks/owner gaps) are violated. Managers and Agents must continuously update this ledger as work progresses.

## 1. Macro Topology & Global Status
<!-- Tracks the foundational Phase 1 architecture. This must be completed and frozen before downstream Meso-Components are processed. -->

- [x] **Phase 1: Global Backbone** (`macro_00_global_topology.md`)
  - **Status**: Accepted / Frozen
  - **Dispatch**: `.agents/subagent-tasks/macro_00_global_topology/macro_00_global_topology_architect_dispatch.md`
  - **Repair Dispatch**: `.agents/subagent-tasks/macro_00_global_topology/macro_00_global_topology_architect_repair_01_dispatch.md`
  - **Accepted Artifact**: `.agents/components/macro_00_global_topology/macro_00_global_topology.md`

## 2. Meso-Component Pipeline Index
<!-- Tracks the high-level end-to-end lifecycle of each Meso-Component. 
This tracks the macro-to-meso transition and architectural/design sign-off for the components as a whole.
Creator/Checker slicing happens later and is decided by the main agent. -->

| Meso-Component | 1. Architect Map | 2. Designer Contract | 3. Creator Passes | 4. Checker Passes | 5. Integration Pass | 6. Reviewer | Overall Status |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: | :--- |
| `meso_01_mount_volume_state` | [x] | [x] | [x] | [x] | [ ] | [x] | Creator/Checker/Reviewer pass 01 accepted; meso integration pending |
| `meso_02_free_space_accounting_and_discard` | [x] | [ ] | [ ] | [ ] | [ ] | [ ] | Architect Map accepted; ready for Designer dispatch |
| `meso_03_directory_lookup_and_identity` | [x] | [ ] | [ ] | [ ] | [ ] | [ ] | Architect Map accepted; ready for Designer dispatch |
| `meso_04_directory_entry_mutation` | [x] | [ ] | [ ] | [ ] | [ ] | [ ] | Architect Map accepted; ready for Designer dispatch |
| `meso_05_file_content_mapping_and_cached_io` | [ ] | [ ] | [ ] | [ ] | [ ] | [ ] | Planned from accepted macro topology |
| `meso_06_file_content_mutation` | [x] | [ ] | [ ] | [ ] | [ ] | [ ] | Architect Map accepted; ready for Designer dispatch |
| `meso_07_file_sync_and_persistence` | [ ] | [ ] | [ ] | [ ] | [ ] | [ ] | Planned from accepted macro topology |
| `meso_08_filesystem_sync_and_volume_state` | [x] | [ ] | [ ] | [ ] | [ ] | [ ] | Architect Map accepted; ready for Designer dispatch |
| `meso_09_file_metadata_projection_and_update` | [ ] | [ ] | [ ] | [ ] | [ ] | [ ] | Planned from accepted macro topology |
| `meso_10_directory_metadata_projection_and_update` | [ ] | [ ] | [ ] | [ ] | [ ] | [ ] | Planned from accepted macro topology |
| `meso_11_volume_admin_identity` | [ ] | [ ] | [ ] | [ ] | [ ] | [ ] | Planned from accepted macro topology |

## 3. Pass Tracking & Dispatch (Information Funnel)
<!-- Granular tracking of Creator/Checker/Reviewer passes linked to their parent Meso-Components.
The main agent decides which Micro-Features travel together in each pass. This queue must show the parent meso scope and the covered-micro set explicitly. -->

| Pass ID | Pass Kind | Parent Meso-Component | Covered Micro-Features | Assigned Role | Artifact / Code | Status |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| `pass_01_mount_volume_state` | Creator-Synced Pass | `meso_01_mount_volume_state` | `Boot region validation and parameter load at mount`; `Allocation bitmap is the free-space truth source`; `VolumeDirty marks in-flight versus quiesced global state`; `VolumeFlags also carries media-failure and clear-before-modify state`; `Up-case Table is the durable case-folding truth source`; `Mount option defaults and remount mutability boundary`; `Superblock counters and statfs reflect cached cluster accounting`; `Asterinas mount lifecycle must eagerly expose root inode and global sync state`; `Mount-time accounting may fall back to recount under corruption-recovery conditions` | Reviewer | Creator report: `.agents/components/meso_01_mount_volume_state/pass_01_mount_volume_state_creator.md`; Checker report: `.agents/components/meso_01_mount_volume_state/pass_01_mount_volume_state_checker.md`; Reviewer report: `.agents/components/meso_01_mount_volume_state/pass_01_mount_volume_state_reviewer.md`; Code: `kernel/src/fs/fs_impls/exfat_refactor/mod.rs`, `kernel/src/fs/fs_impls/exfat_refactor/fs.rs`, `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`, `kernel/src/fs/fs_impls/exfat_refactor/ondisk.rs`, `kernel/src/fs/fs_impls/mod.rs`, `osdk/src/base_crate/mod.rs` | Accepted; integration pending |
