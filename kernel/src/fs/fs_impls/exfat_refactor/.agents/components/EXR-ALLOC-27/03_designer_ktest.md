<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Ktest

## Metadata

- Component ID: `EXR-ALLOC-27`
- Title: `ExfatFs` Allocation Search, Reservation, And Commit Coverage
- Status: `Specified`
- Author: designer
- Date: `2026-04-12`
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-ALLOC-27/20260412-1202-designer-packet.md`
- Based on architect artifact: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-ALLOC-27/00_architect.md`

## Purpose

Define the minimum checker-owned regression coverage needed to prove that `ExfatFs` owns allocation search, reservation intent, and bitmap/FAT commit coordination.

## Test Ownership

- Owner: checker
- Test style: `#[ktest]`
- Placement: local to `allocator.rs`, or `fs.rs` if the owner-internal helper is wired there
- Helper touch: owner-private test helpers may be added only if needed to construct bitmap fixtures, FAT fixtures, and committed allocation outcomes

## Required Coverage

### Scenario 1: Free-space search finds a contiguous run

- Test intent:
  - Confirm the allocator prefers a contiguous free run when one is available.
- Suggested test shape:
  - Build a filesystem fixture with a bitmap snapshot that contains one sufficiently long free extent.
  - Request an allocation that fits entirely in that extent.
- Assertions:
  - The returned stable result reports `ChainMode::Contiguous`.
  - The reported start cluster and cluster count match the free extent.
  - No fragmented fallback is needed.

### Scenario 2: Fragmented allocation is chosen only when contiguous space is insufficient

- Test intent:
  - Confirm the allocator falls back to a FAT-backed run only when it cannot satisfy the request contiguously.
- Suggested test shape:
  - Build a bitmap fixture with free clusters split into smaller gaps that cannot satisfy the request as one run.
  - Request a cluster count larger than any single free run but still satisfiable in aggregate.
- Assertions:
  - The returned stable result reports `ChainMode::FatBacked`.
  - The allocator records the chosen fragmented run as one committed result.
  - The search does not pretend the fragmented request was contiguous.

### Scenario 3: Reservation intent does not escape before commit

- Test intent:
  - Confirm the temporary reservation state stays owner-private until commit succeeds.
- Suggested test shape:
  - Exercise the allocator with a request that can be forced to fail during the commit phase.
- Assertions:
  - No committed allocation result becomes visible after the failure.
  - The published bitmap state is unchanged.
  - Any temporary reservation is discarded.

### Scenario 4: Bitmap and FAT remain coherent after commit

- Test intent:
  - Confirm the allocator's commit handshake updates the bitmap and FAT consistently.
- Suggested test shape:
  - Run one contiguous-allocation case and one fragmented-allocation case.
  - Inspect the committed bitmap state and the resulting chain facts through the allocator-owned API.
- Assertions:
  - Allocated clusters are marked in the bitmap after commit.
  - The committed chain mode matches the mutation strategy.
  - Fragmented allocation leaves a FAT-backed chain that later owners can consume.
  - Contiguous allocation does not produce a FAT-backed result.

## Observability

- These tests should inspect only allocator-owned behavior on `ExfatFs`.
- They should exercise search and commit through the allocator result, not by re-testing bitmap scanning or FAT decoding in isolation.
- They should not introduce directory, inode-growth, truncate, or sync coverage.
- No dedicated concurrency tests are required beyond the serialization and visibility checks above.

## Minimal Checker Obligation

The checker must include regressions proving that:

- the allocator prefers contiguous free space when available,
- fragmented allocation is used only when necessary,
- reservations do not become visible before commit,
- and bitmap/FAT state remains coherent after commit.

## Exit Condition

The ktest plan is complete when a future checker can implement the coverage entirely in local allocator tests and can verify that `ExfatFs` owns search, reservation, and commit without promoting allocation into a public manager, inode helper, or sync service.

