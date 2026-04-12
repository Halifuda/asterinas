<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Ktest

## Metadata

- Component ID: `EXR-PGCACHE-26`
- Title: `ExfatInode` Page-Cache Checker Coverage
- Status: `Specified`
- Author: designer
- Date: `2026-04-12`
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-PGCACHE-26/20260412-1202-designer-packet.md`
- Based on architect artifact: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-26/00_architect.md`

## Purpose

Define the minimum checker-owned regression coverage needed to prove that `ExfatInode` owns inode-local page-cache attachment and backend behavior while still consuming `EXR-READ-OPS-25` as the only buffered byte-stream owner.

## Test Ownership

- Owner: checker
- Test style: `#[ktest]`
- Placement: local to `inode.rs`
- Helper touch: owner-private test helpers inside `inode.rs` only if needed to build regular-file fixtures with distinct physical bytes, logical file size, valid size, allocation facts, and page-cache attachment facts

## Required Coverage

### Scenario 1: Inode-local cache attachment is visible on a regular-file snapshot

- Test intent:
  - Confirm that a regular-file inode can own a `PageCache` attachment without promoting the cache into a filesystem-global service.
- Suggested test shape:
  - Build a regular-file inode fixture with a stable file snapshot.
  - Inspect the inode-local cache attachment and its size facts through the public behavior available to the test.
- Assertions:
  - The inode has a cache attachment.
  - The cache is attached to `ExfatInode`, not to a standalone service object.
  - The initial cache sizing is derived from the inode snapshot rather than from write-side policy.

### Scenario 2: Backend page fill reuses the read owner

- Test intent:
  - Confirm that cache misses are populated through the inode-owned page-cache backend and the existing buffered-read contract.
- Suggested test shape:
  - Build a regular-file inode fixture with known bytes that cover at least one page.
  - Trigger a cache-backed page read for a range fully inside the physically backed region.
- Assertions:
  - The filled page matches the expected backing bytes.
  - The test observes inode-owned page fill behavior rather than a separate cache manager service.
  - The cache fill does not re-implement mapping or byte-transfer policy.

### Scenario 3: Cache-backed reads preserve the read-owner EOF and zero-fill rules

- Test intent:
  - Confirm that page-cache population does not duplicate read policy and still honors the valid-size gap and logical EOF rules from `EXR-READ-OPS-25`.
- Suggested test shape:
  - Build a regular-file inode fixture where `valid_size < size`.
  - Request a cache-backed page or read that crosses from backed data into the valid-size gap, and a separate request at or beyond EOF.
- Assertions:
  - The backed prefix matches the file bytes.
  - The gap before logical EOF is zero-filled.
  - Reads at or beyond logical EOF return `0`.

### Scenario 4: Repeated cache-backed reads stay stable on one snapshot

- Test intent:
  - Confirm that the inode-local cache path does not keep hidden progress state.
- Suggested test shape:
  - Invoke the same cache-backed read or page-fill path twice on the same inode snapshot.
  - Include at least one case that crosses a page boundary or valid-size gap.
- Assertions:
  - Both calls return the same byte count.
  - Both calls produce the same byte stream.
  - No hidden cache or read cursor state drifts between calls.

## Observability

- These tests should inspect only inode-local page-cache behavior on `ExfatInode`.
- They should consume the buffered-read contract indirectly through the page-cache path rather than retesting `read_at()` or `map_physical_file_range()` in isolation.
- They should not introduce page-cache-manager, directory, write-side, allocator, or sync coverage.
- No dedicated concurrency tests are required beyond repeated-call stability on one snapshot.

## Minimal Checker Obligation

The checker must include regressions proving that:

- the inode owns the page-cache attachment,
- cache-backed page fill is served through the inode owner and not through a filesystem-global cache service,
- page fill preserves the read-owner EOF, short-read, and valid-size zero-fill policy,
- and repeated cache-backed reads on the same snapshot are stable.

## Exit Condition

The ktest plan is complete when a future checker can implement the coverage entirely in `inode.rs` tests and can verify that `ExfatInode` owns inode-local page-cache behavior without promoting page cache, write-side policy, or mapping translation into this row.
