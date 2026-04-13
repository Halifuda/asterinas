<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Ktest

## Metadata

- Component ID: `EXR-WRITE-30`
- Title: `ExfatInode` buffered write and resize coverage
- Status: `Specified`
- Author: designer
- Date: `2026-04-13`
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-WRITE-30/20260413-0650-designer-repair-packet.md`
- Based on architect artifact: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/00_architect.md`

## Purpose

Define the minimum checker-owned regression coverage needed to prove that buffered write, growth, truncate, and resize stay on `ExfatInode`, consume the page-cache and allocation owners, and do not widen into direct-I/O or sync ownership.

## Test Ownership

- Owner: checker
- Test style: `#[ktest]`
- Placement: local to `inode.rs`
- Helper touch: owner-private test helpers may be added only if needed to build regular-file fixtures, committed allocation outcomes, and visible read-after-write checks

## Required Coverage

### Scenario 1: Buffered write updates the visible byte stream inside existing allocation

- Test intent:
  - Confirm that a regular-file buffered write stays on `ExfatInode` and updates the visible byte stream without consuming extra allocation when the request already fits the current allocation coverage.
- Suggested test shape:
  - Build a regular-file inode fixture with one allocated region and known bytes.
  - Issue a buffered write fully inside the existing allocation range, then read the written span back through the inode-visible data path.
- Assertions:
  - The returned byte count matches the written input.
  - The later read observes the new bytes at the expected offset.
  - The inode snapshot keeps the same committed allocation facts and page-cache size.
  - The write does not widen into a new allocator or sync owner.

### Scenario 2: Write growth zero-fills the skipped valid-size gap before publishing it

- Test intent:
  - Confirm that a write beginning beyond `valid_size` preserves the accepted zero-fill contract before it advances the initialized range.
- Suggested test shape:
  - Build a regular-file inode fixture with `valid_size < size` or with a later write that skips forward.
  - Issue a buffered write whose starting offset is greater than the original `valid_size`.
  - Read back the span from the old `valid_size` through the end of the new write.
- Assertions:
  - Bytes before the write offset are zero-filled.
  - Bytes at and after the write offset match the caller input.
  - The published `size` and `valid_size` reflect one coherent post-write state.
  - The page-cache size matches the newly visible EOF.

### Scenario 3: Resize shrink truncates visible EOF and cache sizing

- Test intent:
  - Confirm that `resize` shrink remains inode-owned and clamps both the visible EOF and inode-local cache sizing.
- Suggested test shape:
  - Build a regular-file inode fixture with at least one page of data.
  - Shrink the file to a smaller size, then attempt reads at the new EOF and inspect the inode-local cache size.
- Assertions:
  - Reads at or beyond the new EOF return `0`.
  - The inode-local page cache has been resized to the new logical size.
  - The truncated tail is no longer visible as live file data.

### Scenario 4: Growth beyond current allocation coverage consumes committed allocation results only when needed

- Test intent:
  - Confirm that file growth uses `EXR-ALLOC-27` only as a committed-growth handoff and does not re-open allocation search inside the write owner.
- Suggested test shape:
  - Build a regular-file inode fixture whose current allocated coverage is smaller than the requested grown size.
  - Exercise either `write_at` or `resize` so the inode must grow beyond its current allocation coverage.
  - Read back any unwritten grown suffix through the inode-visible path.
- Assertions:
  - The inode publishes a larger size and updated allocation facts only after the growth call succeeds.
  - The unwritten grown suffix is zero-visible.
  - The observed behavior stays on `ExfatInode` and consumes committed allocation facts rather than exposing allocator internals.
  - The new `start_cluster`, `cluster_count`, `chain_mode`, and `allocated_size` values are visible on the inode snapshot only after commit.

## Observability

- These tests should inspect write-visible bytes, `size`, `valid_size`, `start_cluster`, `cluster_count`, `chain_mode`, `allocated_size`, inode-local page-cache sizing, and allocation facts on the inode snapshot.
- They should not introduce sync-order coverage, background writeback behavior, or direct-I/O coverage.
- They should not retest allocator search or mapping translation in isolation; those remain covered by their own components.
- No dedicated concurrency tests are required because `02_designer_async.md` documents a synchronous publication boundary only.

## Minimal Checker Obligation

The checker must include regressions proving that:

- buffered writes update the visible byte stream on `ExfatInode`,
- skipped valid-size gaps become zero-visible before later bytes are published,
- resize shrink truncates EOF and cache sizing coherently,
- and growth beyond current allocation coverage consumes committed allocation results only when needed.

## Exit Condition

The ktest plan is complete when a future checker can implement the coverage entirely in `inode.rs` tests and can verify that buffered write and size mutation stay on `ExfatInode`, consume page-cache and committed allocation owners, and do not introduce a direct-I/O path or sync shell.
