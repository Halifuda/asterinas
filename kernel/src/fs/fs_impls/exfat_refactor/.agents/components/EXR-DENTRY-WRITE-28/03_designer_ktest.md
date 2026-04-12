<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Ktest

## Metadata

- Component ID: `EXR-DENTRY-WRITE-28`
- Title: `DirectoryEngine` Write-Side Directory Entry Mutation Coverage
- Status: `Specified`
- Author: designer
- Date: 2026-04-12
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-DENTRY-WRITE-28/20260412-2049-designer-packet.md`
- Based on architect artifact: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DENTRY-WRITE-28/00_architect.md`

## Purpose

Define the minimum checker-owned regression coverage needed to prove that `DirectoryEngine` owns write-side directory-entry mutation without becoming namespace policy, allocation search, or a standalone manager.

## Test Ownership

- Owner: checker
- Test style: `#[ktest]`
- Placement: local to `directory.rs`
- Helper touch: owner-private test helpers may be added only if needed to construct directory fixtures, validated `ExfatDentrySet` values, and committed allocation results

## Required Coverage

### Scenario 1: Tombstoned slot ranges are reused

- Test intent:
  - Confirm the write path reuses tombstoned directory space before extending the directory chain.
- Suggested test shape:
  - Build a directory fixture with a tombstoned run large enough for one validated file record.
  - Write a validated `ExfatDentrySet` into that run.
- Assertions:
  - The new record occupies the reused slot range.
  - The tombstoned entries are replaced or consumed instead of left live.
  - No extra directory growth is needed.

### Scenario 2: In-place rewrite preserves the existing location

- Test intent:
  - Confirm a validated record that still fits can be rewritten in place.
- Suggested test shape:
  - Start from an existing record location and rewrite it with a new validated set of the same placement footprint.
- Assertions:
  - The record stays at the same directory location.
  - The serialized bytes reflect the new validated set.
  - The write path does not force relocation when in-place rewrite is sufficient.

### Scenario 3: Directory growth uses a committed allocation result

- Test intent:
  - Confirm that the write path consumes a committed allocation result instead of searching for free space itself.
- Suggested test shape:
  - Provide a validated set that cannot fit in the current directory window.
  - Supply a committed allocation result from `EXR-ALLOC-27`.
- Assertions:
  - The directory grows only through the supplied committed allocation facts.
  - The write path does not run allocation search or reservation logic.
  - The validated record is written into the newly available space.

### Scenario 4: Namespace policy stays outside the write primitive

- Test intent:
  - Confirm the write path consumes trusted validated records instead of re-deriving namespace policy.
- Suggested test shape:
  - Use a prevalidated `ExfatDentrySet` and write it through the directory helper.
- Assertions:
  - The helper accepts the validated set directly.
  - No name normalization, inode publication, or lookup policy is required by the write primitive.
  - The observable behavior is limited to directory mutation.

## Observability

- These tests should only inspect directory mutation, slot reuse, in-place rewrite, and growth behavior.
- They should not require inode-cache, VFS namespace, or sync-order coverage.
- They should not add a separate helper module unless the local `directory.rs` test block becomes unexpectedly cluttered, which is not expected for this component.
- No dedicated concurrency tests are required.

## Minimal Checker Obligation

The checker must include regressions proving that:

- tombstoned slots are reused before directory growth,
- in-place rewrite preserves location when the validated set still fits,
- committed allocation results are the only growth input,
- and namespace policy remains outside the write primitive.

## Exit Condition

The ktest plan is complete when a future checker can implement the coverage using only `directory.rs` tests and can verify that write-side directory mutation stays inside `DirectoryEngine`, consumes validated `ExfatDentrySet` values, and treats committed allocation results as the only growth handoff.
