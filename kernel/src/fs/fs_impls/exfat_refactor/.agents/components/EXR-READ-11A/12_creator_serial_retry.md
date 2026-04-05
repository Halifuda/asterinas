<!-- SPDX-License-Identifier: MPL-2.0 -->

# Creator Log

## Metadata

- Component ID: `EXR-READ-11A`
- Title: `Logical-To-Physical Mapping For Existing Regular-File Reads`
- Status: `SerialImplementing`
- Author: `creator`
- Date: `2026-04-05`
- Task packet: `EXR-READ-11A-CREATE-20260405-1148`
- Implemented spec: `01_designer_core.md`, `03_designer_ktest.md`
- Pass kind: `serial repair`

## Planned File Ownership

- Files to edit: `kernel/src/fs/fs_impls/exfat_refactor/read.rs`, `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-READ-11A/12_creator_serial_retry.md`
- Files intentionally left untouched: `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`, `kernel/src/fs/fs_impls/exfat_refactor/fat.rs`, `kernel/src/fs/fs_impls/exfat_refactor/mod.rs`

## Implementation Notes

The existing code already matched the mapping-only behavior closely, so the redo stayed narrow.
The only production refinement was in `read.rs`: `map_logical_read_offset(...)` now consumes `ExfatInodeReadView` directly instead of taking the full `ExfatInodeMeta` shell and reopening it internally. That aligns the canonical mapper surface with the designer contract that the mapper should operate on the narrow immutable read-view boundary. The read-side regressions in `read.rs` were updated accordingly, including keeping directory rejection at the `read_view()` boundary so non-regular-file shells still cannot cross into placement mapping.

No helper expansion was needed in `inode.rs`, and no changes were needed in `fat.rs` or `mod.rs`.

## Approved Deviations

None

## Optional Self-Checks

- Commands run, if any: read-only file inspection commands only; no build, test, or QEMU commands were run.
- Compile checks run, if any: none
- Manual reasoning checks: verified the mapper still returns `None` for offsets at or beyond `valid_data_length`; verified contiguous and FAT-backed placement still reuse the existing chain-walk boundary without adding buffered-read behavior; verified the only cross-module read surface remains `ExfatInodeMeta::read_view()`.

## Remaining Risks

- This pass did not run compile or ktest verification, per packet constraints.
- `EXR-READ-11A` still has no downstream caller yet, so full integration of the narrowed mapper surface remains for later components.
