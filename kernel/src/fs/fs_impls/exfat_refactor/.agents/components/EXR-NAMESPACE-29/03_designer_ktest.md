<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Ktest

## Metadata

- Component ID: `EXR-NAMESPACE-29`
- Title: `ExfatInode` Namespace Mutation Coverage
- Status: `Specified`
- Author: designer
- Date: 2026-04-12
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-NAMESPACE-29/20260412-2134-designer-packet.md`
- Based on architect artifact: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-NAMESPACE-29/00_architect.md`

## Purpose

Define the minimum checker-owned regression coverage needed to prove that namespace mutation stays on `ExfatInode`, consumes the consumed directory-write and allocation owners, and does not become a standalone namespace manager.

## Test Ownership

- Owner: checker
- Test style: `#[ktest]`
- Placement: local to `inode.rs`
- Helper touch: owner-private test helpers may be added only if needed to build directory fixtures, validated names, and committed allocation results

## Required Coverage

### Scenario 1: Create or mkdir publishes one canonical child handle

- Test intent:
  - Confirm that a namespace-creation path goes through `ExfatInode`, uses the installed canonicalization service, and publishes the resulting child through the existing `ExfatFs` reuse boundary.
- Suggested test shape:
  - Create a directory fixture, issue `create` or `mkdir`, then resolve the new child again through the inode-owned namespace surface.
- Assertions:
  - The new child is visible through the inode-owned namespace path.
  - Repeated resolution returns the same canonical child handle.
  - No separate namespace manager is required by the observed behavior.

### Scenario 2: Unlink or rmdir removes the live namespace entry

- Test intent:
  - Confirm that a namespace-removal path goes through `ExfatInode` and clears the directory entry through the consumed write boundary.
- Suggested test shape:
  - Populate a directory fixture with a child entry, remove it with `unlink` or `rmdir`, and then try to resolve it again.
- Assertions:
  - The entry is no longer visible as a live namespace child.
  - The removal path does not expose a second owner or a background coordinator.

### Scenario 3: Rename stays inside the inode owner boundary

- Test intent:
  - Confirm that `rename` coordinates source removal and destination publication without inventing a standalone namespace service.
- Suggested test shape:
  - Create a source child, rename it into a destination name, and observe the resulting lookup or readdir state.
- Assertions:
  - The source name no longer resolves as the live entry.
  - The destination name resolves to the renamed child.
  - The observed behavior is limited to inode-owned namespace mutation.

### Scenario 4: Growth consumes committed allocation results only when needed

- Test intent:
  - Confirm that namespace mutation uses committed allocation results as the only growth handoff.
- Suggested test shape:
  - Force a directory write that cannot stay in place and supply the committed allocation facts from the allocation owner.
- Assertions:
  - The mutation succeeds only through the supplied committed allocation result.
  - The path does not run allocation search or reservation logic.
  - The mutation remains namespace-only, not allocator-owned.

## Observability

- These tests should inspect namespace visibility, repeated resolution, and rename outcomes.
- They should not require sync-order coverage, background coordination, or allocator internals.
- They should not add a separate helper module unless the local `inode.rs` test block becomes unexpectedly cluttered, which is not expected for this component.
- No dedicated concurrency tests are required.

## Minimal Checker Obligation

The checker must include regressions proving that:

- create or mkdir publishes a canonical child handle through `ExfatFs`,
- unlink or rmdir removes the live namespace entry,
- rename stays inside the inode owner boundary,
- and committed allocation results are the only growth input.

## Exit Condition

The ktest plan is complete when a future checker can implement the coverage using only `inode.rs` tests and can verify that namespace mutation stays on `ExfatInode`, consumes the directory-write and allocation owners, and does not introduce a separate namespace manager.
