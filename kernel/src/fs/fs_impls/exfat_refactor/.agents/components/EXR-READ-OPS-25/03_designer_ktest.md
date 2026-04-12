<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Ktest

## Metadata

- Component ID: `EXR-READ-OPS-25`
- Title: `ExfatInode` Buffered Read Checker Coverage
- Status: `Specified`
- Author: designer
- Date: `2026-04-12`
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-READ-OPS-25/20260412-1110-designer-packet.md`
- Based on architect artifact: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-READ-OPS-25/00_architect.md`

## Purpose

Define the minimum checker-owned regression coverage needed to prove that `ExfatInode` owns buffered regular-file `read_at` semantics while consuming `EXR-FILE-MAP-24` as a translation-only dependency.

## Test Ownership

- Owner: checker
- Test style: `#[ktest]`
- Placement: local to `inode.rs`
- Helper touch: owner-private test helpers inside `inode.rs` only if needed to build regular-file fixtures with distinct physical bytes, logical file size, valid size, and allocation facts

## Required Coverage

### Scenario 1: Buffered read copies physically backed bytes

- Test intent:
  - Confirm `read_at` consumes the mapping output and copies real file bytes into the caller writer without reopening mapping ownership.
- Suggested test shape:
  - Build a regular-file inode fixture with known on-disk bytes spanning at least one mapped range.
  - Read from an offset that stays entirely within the physically backed and valid region.
- Assertions:
  - The returned byte count matches the copied byte length.
  - The writer receives the expected backing bytes in order.
  - The result does not depend on a separate filesystem-global read service.

### Scenario 2: Read crossing `valid_size` zero-fills the logical gap

- Test intent:
  - Confirm buffered read owns valid-size zero-fill presentation.
- Suggested test shape:
  - Build a regular-file inode fixture where `valid_size < size` and the request begins in the physically backed region but extends into the valid-size gap.
- Assertions:
  - The prefix before `valid_size` matches the physical file bytes.
  - The suffix after `valid_size` and before logical EOF is zero-filled.
  - The returned byte count includes both the copied prefix and zero-filled suffix.

### Scenario 3: Read at or beyond logical EOF returns zero

- Test intent:
  - Confirm logical EOF handling lives in `read_at`.
- Suggested test shape:
  - Issue one read exactly at logical EOF and one beginning past EOF.
- Assertions:
  - Both reads return `0`.
  - The writer contents remain unchanged.
  - No zero-fill or physical copy is attempted beyond logical EOF.

### Scenario 4: Repeated reads are stable on one snapshot

- Test intent:
  - Confirm the buffered read path is deterministic and does not retain hidden progress state.
- Suggested test shape:
  - Invoke `read_at` twice with the same inode snapshot, offset, and request length.
  - Include at least one case that spans multiple mapped slices or crosses into the zero-fill region.
- Assertions:
  - Both calls return identical byte counts.
  - Both calls produce identical byte streams.
  - No hidden cursor state causes the second read to drift.

## Observability

- These tests should inspect only buffered regular-file read behavior on `ExfatInode`.
- They should consume mapping indirectly through `read_at` rather than retesting `map_physical_file_range()` in isolation.
- They should not introduce page-cache, directory, write-side, allocator, or sync coverage.
- No dedicated concurrency tests are required beyond repeated-call stability.

## Minimal Checker Obligation

The checker must include regressions proving that:

- buffered `read_at` copies physically backed bytes through the inode owner,
- reads that extend past `valid_size` zero-fill the logical gap up to EOF,
- reads at or beyond logical EOF return `0`,
- and repeated reads on the same snapshot are stable.

## Exit Condition

The ktest plan is complete when a future checker can implement the coverage entirely in `inode.rs` tests and can verify that `ExfatInode` owns buffered read semantics without promoting page cache, write-side policy, or mapping translation into this row.
