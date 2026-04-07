<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Ktest

## Metadata

- Component ID: EXR-FS-CORE-16
- Title: ExfatFs Filesystem Owner Boundary
- Status: `Specified`
- Author: designer
- Date: 2026-04-07
- Based on architect artifact: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-CORE-16/00_architect.md`

## Purpose

Define the minimal checker-owned regression coverage needed to prove that `ExfatFs` is the stable filesystem owner and that the temporary `root_inode()` seam and placeholder `sync()` stay explicit rather than turning into hidden mount or flush logic.

## Test Ownership

- Owner: checker
- Test style: `#[ktest]`
- Placement: local to `fs.rs`
- Helper touch: none expected

## Required Coverage

### Scenario 1: Filesystem identity and superblock snapshot are stable

- Test intent:
  - Confirm the owner exposes the canonical exFAT identity through the VFS `FileSystem` surface.
  - Confirm `sb()` reads back the same normalized snapshot from the same owner instance.
- Suggested test shape:
  - Build one `ExfatFs` instance from the known-good exFAT bootstrap path used by the refactor.
  - Call the `FileSystem` methods through the trait surface.
- Assertions:
  - `name()` returns the canonical exFAT filesystem identifier.
  - `sb()` returns a stable snapshot for the owner instance.
  - Repeated `sb()` calls are equivalent while the owner state is unchanged.

### Scenario 2: Subscriber stats stay attached to the same owner

- Test intent:
  - Confirm `fs_event_subscriber_stats()` returns the same owner-owned stats object each time.
  - Confirm the placeholder `sync()` path does not disturb that object.
- Suggested test shape:
  - Read the returned stats reference more than once and compare it by identity.
  - Call `sync()` between reads.
- Assertions:
  - The returned stats reference is stable.
  - Subscriber accounting does not move to a fresh wrapper per call.
  - `sync()` returns success and leaves the stable owner snapshot unchanged.

### Scenario 3: The temporary root seam remains explicit

- Test intent:
  - Confirm the root inode path is still exposed as the explicit `ExfatFs` owner seam.
  - Confirm the seam is not hidden behind a separate root-shell helper or an alternate owner object.
- Suggested test shape:
  - Exercise `root_inode()` directly on the filesystem owner once the creator wires the temporary seam.
  - Keep the test at the owner boundary and do not force mount/open sequencing into the case.
- Assertions:
  - The root inode path is reachable only through the explicit filesystem-owner seam.
  - The test does not need to validate real open sequencing or inode-cache behavior.
  - The future `EXR-FS-OPEN-22` handoff can replace the seam without changing the checker shape.

## Observability

- These tests should only inspect the owner boundary, the stable superblock snapshot, the subscriber stats reference, and the temporary root seam.
- They should not require inode cache, directory, bitmap, allocation, or page-cache coverage.
- They should not introduce a separate helper module unless the `fs.rs` test block becomes cluttered, which is not expected for this component.
- No dedicated concurrency tests are required because the component does not introduce a new lock hierarchy or async protocol.

## Minimal Checker Obligation

The checker must include a regression that proves the owner skeleton is still the thing implementing the VFS `FileSystem` surface, and not a split shell around `name()`, `sb()`, `fs_event_subscriber_stats()`, or `root_inode()`. The same regression set should show that `sync()` remains a placeholder and does not absorb real flush ordering.

## Exit Condition

The ktest plan is complete when a future checker can validate the owner skeleton entirely from `fs.rs`, see the same stable filesystem snapshot across repeated calls, and confirm that `root_inode()` is still the explicit temporary seam awaiting `EXR-FS-OPEN-22`.
