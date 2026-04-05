<!-- SPDX-License-Identifier: MPL-2.0 -->

# Creator Log

## Metadata

- Component ID: `EXR-PGCACHE-11B`
- Title: `Page-Cache Backend Integration For Regular Files`
- Status: `SerialImplementing`
- Author: `creator`
- Date: `2026-04-05`
- Task packet: `EXR-PGCACHE-11B-CREATE-20260405-1216`
- Implemented spec: `00_architect.md`, `01_designer_core.md`, `03_designer_ktest.md`
- Pass kind: `serial implementation`

## Planned File Ownership

- Files to edit:
  - `kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-11B/10_creator_serial.md`
- Files intentionally left untouched:
  - `kernel/src/fs/fs_impls/exfat_refactor/read.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/mod.rs`

## Implementation Notes

Implemented one canonical regular-file backend surface with narrow ownership:

- `inode.rs` now provides:
  - `ExfatRegularFileRuntime`, which owns `PageCache` plus a strong backend reference so backend lifetime is tied to runtime ownership.
  - regular-file-only sizing helpers:
    - `regular_file_valid_data_length()`
    - `regular_file_page_count()`
    - `regular_file_cache_capacity()`
  - `read_view()` now reuses the regular-file boundary check path.

- `fs.rs` now provides:
  - private `ExfatRegularFileBackend` that implements `PageCacheBackend`.
  - backend page count derived from `valid_data_length` via `ExfatInodeMeta::regular_file_page_count()`.
  - `attach_regular_file_runtime(self: &Arc<Self>, inode_meta: ExfatInodeMeta)` that constructs the backend and `PageCache::with_capacity(...)`, with capacity derived from visible length.
  - page-level read/write hooks route placement through `map_logical_read_offset(...)` from `read.rs`, then issue block-device I/O from the mapped physical byte offset.

This keeps the scope within backend attachment and page-level backend I/O only; no buffered `read_at`, growth, truncate, or second mapping path was introduced.

## Approved Deviations

None

## Optional Self-Checks

- Commands run, if any: read-only inspection commands only (`sed`, `rg`, `ls`, `git status`).
- Compile checks run, if any: none (not authorized by packet).
- Manual reasoning checks:
  - Verified backend `npages` source is `valid_data_length`, not `data_length`.
  - Verified page I/O mapping calls into `EXR-READ-11A` boundary (`map_logical_read_offset`) instead of re-deriving mapping.
  - Verified out-of-range page indices are rejected by backend-local range checks.

## Remaining Risks

- No compile or ktest execution was performed in this pass per packet constraints.
- Partial-page read/write behavior remains bounded by current page-level backend contract; buffered read-side copy and zero-fill policy remain out of scope for this component and belong to downstream `EXR-READ-11B`.
