<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Ktest

## Metadata

- Component ID: `EXR-FILE-MAP-24`
- Title: `ExfatInode` Read-Path Mapping Checker Coverage
- Status: `Specified`
- Author: designer
- Date: `2026-04-11`
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-FILE-MAP-24/20260411-1613-designer-packet.md`
- Based on architect artifact: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FILE-MAP-24/00_architect.md`

## Purpose

Define the minimal checker-owned regression coverage needed to prove that `ExfatInode` owns read-path logical-to-physical mapping without turning into a data-copy owner, a zero-fill owner, or a separate mapping service.

## Test Ownership

- Owner: checker
- Test style: `#[ktest]`
- Placement: local to `inode.rs`
- Helper touch: owner-private test helpers inside `inode.rs` only if needed to keep the tests readable

## Required Coverage

### Scenario 1: Logical offset resolves to the correct cluster position

- Test intent:
  - Confirm the helper consumes the inode-owned chain snapshot and cluster geometry to identify the cluster containing a logical file offset.
- Suggested test shape:
  - Build a regular-file inode snapshot with a known start cluster and chain length.
  - Exercise the helper at a cluster boundary and at a mid-cluster offset.
- Assertions:
  - The returned cluster position matches the expected cluster for the chosen logical offset.
  - The returned in-cluster byte offset matches the requested offset modulo cluster size.
  - The helper stays read-only and does not mutate inode state.

### Scenario 2: Physically mappable span respects inode size facts

- Test intent:
  - Confirm the helper derives a backed span from file-size, valid-size, and allocated-size facts instead of guessing about read policy.
- Suggested test shape:
  - Build an inode snapshot where the logical file size, valid size, and allocated size differ in a meaningful way.
  - Ask for a span that would cross one of those boundaries.
- Assertions:
  - The returned span stops at the first relevant backing boundary.
  - The helper does not claim ownership of EOF policy or zero-fill behavior.
  - The helper does not report a span longer than the cluster geometry permits.

### Scenario 3: Repeated calls are stable on the same snapshot

- Test intent:
  - Confirm the mapping helpers are deterministic read-side translations on one inode snapshot.
- Suggested test shape:
  - Call the offset translator twice with the same logical offset.
  - Call the span helper twice with the same request.
- Assertions:
  - Both calls return identical results.
  - No hidden cursor state causes the second call to drift.
  - The inode snapshot remains unchanged after the calls.

### Scenario 4: Empty or fully unbacked input remains explicit

- Test intent:
  - Confirm the helper returns an explicit empty or out-of-range result rather than silently inventing a read policy.
- Suggested test shape:
  - Use a zero-length file snapshot or a request that begins beyond the physically backed region.
- Assertions:
  - The helper returns an empty backed span or an explicit range error, depending on the chosen helper shape.
  - The result stays read-only and does not trigger byte-copying behavior.

## Observability

- These tests should inspect only read-path mapping behavior on `ExfatInode`.
- They should consume `ExfatChain`, inode-owned size facts, and filesystem geometry indirectly through the inode helper surface rather than testing those owners independently again.
- They should not introduce directory, mount/open, page-cache, or sync coverage.
- No dedicated concurrency tests are required beyond the repeated-call stability coverage above.

## Minimal Checker Obligation

The checker must include a regression that proves:

- a logical offset maps to the expected cluster and in-cluster byte offset,
- the physically mappable span is bounded by inode size facts and cluster geometry,
- repeated calls on the same snapshot are stable,
- and the helper does not claim zero-fill, EOF, or byte-copy ownership.

## Exit Condition

The ktest plan is complete when a future checker can implement the coverage entirely in `inode.rs` tests and can verify that `ExfatInode` owns read-path mapping as a pure translation layer without reopening mount/open, directory logic, or a separate mapping-service boundary.
