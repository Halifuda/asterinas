<!-- SPDX-License-Identifier: MPL-2.0 -->

# Creator Log

## Metadata

- Component ID: `EXR-READ-11A`
- Title: Logical-To-Physical Mapping For Existing Regular-File Reads
- Status: `SerialImplementing`
- Author: `main-agent`
- Date: `2026-04-05`
- Task packet: locally executed by `main-agent` in the `sable-lattice` wave; no delegated creator packet
- Implemented spec:
  - `00_architect.md`
  - `01_designer_core.md`
  - `03_designer_ktest.md`
- Pass kind: `serial implementation`

## Planned File Ownership

- Files to edit:
  - `kernel/src/fs/fs_impls/exfat_refactor/read.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/fat.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- Files intentionally left untouched:
  - `kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/sysroot.rs`
  - all directory, bitmap, and upcase modules

## Implementation Notes

Implemented the canonical read-mapping boundary in `read.rs` as `map_logical_read_offset(...)`, which returns only physical placement facts and stops at the valid-data boundary instead of inventing buffered-read policy.

Added the narrow immutable inode bridge in `inode.rs` as `ExfatInodeMeta::read_view()`, which rejects directory shells before they cross the mapping boundary.

Added one chain helper in `fat.rs`, `current_cluster_id()`, so the mapper can publish the destination cluster without reopening chain internals or duplicating traversal logic.

Kept the checker-owned ktests local to `read.rs` and aligned them with the designer obligations:

- contiguous placement without FAT reads,
- FAT-backed placement through chain walking,
- EOF behavior at and beyond valid-data length,
- rejection of directory shells.

## Approved Deviations

None.

## Optional Self-Checks

- Commands run, if any:
  - none in the creator step
- Compile checks run, if any:
  - none in the creator step
- Manual reasoning checks:
  - confirmed the mapper stays read-only and reuses `ExfatChain::walk_to_cluster_at_offset(...)`
  - kept the inode helper to one read-view boundary instead of separate field accessors
  - kept `ExfatReadPlacement` to cluster-plus-intra-cluster offset only

## Remaining Risks

- The checker still needs to confirm the exact ktests compile and run with the local fixture shapes.
- Later buffered-read and page-cache components must consume this mapping boundary instead of re-deriving placement.
