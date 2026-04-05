<!-- SPDX-License-Identifier: MPL-2.0 -->

# Reviewer Report

## Metadata

- Component ID: `EXR-READ-11A`
- Title: `Logical-To-Physical Mapping For Existing Regular-File Reads`
- Status: `Reviewed`
- Author: `reviewer`
- Date: `2026-04-05`
- Task packet: `EXR-READ-11A-REVIEW-20260405-1148`
- Reviewed implementation:
  - `kernel/src/fs/fs_impls/exfat_refactor/read.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/fat.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/mod.rs`

## Review Scope

Reviewed the current `EXR-READ-11A` working-tree implementation against the architect and designer constraints, with emphasis on helper justification, mapping-boundary discipline, visibility shape, and whether the code still matches the mapping-only slice. No build, test, or QEMU commands were run in this pass.

## Findings

### Finding

- Severity: `Low`
- Location:
  - `kernel/src/fs/fs_impls/exfat_refactor/fat.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/read.rs`
- Description:
  `read.rs` only needed the destination cluster id from `walk_to_cluster_at_offset(...)`, but the helper returned an `ExfatChain` that the mapper immediately unwrapped again through a separate `current_cluster_id()` accessor. That accessor widened the chain surface without a caller-backed reason beyond this component's mapper.
- Guideline or style principle involved:
  Keep helpers purposeful; avoid field-exposing accessors and keep the mapper canonical and narrow.
- Action taken:
  Changed `walk_to_cluster_at_offset(...)` to return `(ClusterId, usize)` directly, removed `current_cluster_id()`, and updated `read.rs` plus the local `fat.rs` tests to consume the narrower return value.

## Direct Edits

- `kernel/src/fs/fs_impls/exfat_refactor/read.rs`
  - Simplified `map_logical_read_offset(...)` to consume the walked destination cluster id directly.
- `kernel/src/fs/fs_impls/exfat_refactor/fat.rs`
  - Narrowed `walk_to_cluster_at_offset(...)` to return the destination `ClusterId` and intra-cluster byte offset.
  - Removed the now-unjustified `current_cluster_id()` accessor.
  - Updated local `fat.rs` tests to assert on the returned cluster id directly.
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-READ-11A/32_reviewer_followup.md`
  - Recorded the reviewer redo findings and direct edits.

## Residual Concerns

- `ExfatInodeMeta::read_view()` remains the single justified cross-module read helper for this component and still enforces the directory boundary before mapping.
- `ExfatChain::byte_len(...)` was not changed in this pass because it has an existing caller in `fs.rs`; no additional unjustified helper surface was found inside the `EXR-READ-11A` mapping slice after the cleanup above.

## Recommendation

- Next owner: `checker`
- Reason:
  The mapping boundary now matches the intended slice more closely: one justified inode read-view helper, one canonical mapper, and no extra chain accessor retained solely for this component's use.
