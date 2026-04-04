<!-- SPDX-License-Identifier: MPL-2.0 -->

# Creator Serial Retry

## Metadata

- Component ID: `EXR-UPCASE-07B`
- Title: Canonical Upcase-Backed Case Folding And Name Hashing
- Status: `SerialImplementingRetry`
- Author: `creator`
- Date: `2026-04-04`
- Task packet: `EXR-UPCASE-07B-CREATE-20260404-1559`

## Repair Summary

- Split `ExfatDentrySet` validation into a canonical `new(dentries, &ExfatUpcaseTable)` boundary plus a ktest-only `new_structure_only()` helper.
- The canonical constructor now validates `stream_dentry.name_hash` against the loaded upcase-table hash service instead of a raw UTF-16 checksum.
- Kept the temporary ktest-only raw builder for staged file-record synthesis, while the structure-only helper preserves the local test coverage that only cares about ordering and checksum shape.

## Remaining Limitation

- No remaining blocker is known in the owned files.
- I did not widen scope into caller files or `upcase_table.rs`, per packet restrictions.

## Edited Files

- `kernel/src/fs/fs_impls/exfat_refactor/fileset.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07B/12_creator_serial_retry.md`
