<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Async

## Metadata

- Component ID: `EXR-MOUNT-09`
- Title: Mount Bootstrap And Shared Filesystem State
- Status: `Specified`
- Author: `designer`
- Date: `2026-04-04`
- Task packet: `EXR-MOUNT-09-DESIGN-20260404-1511`
- Based on architect artifact: `00_architect.md`

## Purpose

Record the mount-local publication and ordering contract for the shared filesystem object.

The component does not introduce awaitable work, background tasks, or a long-lived asynchronous protocol. It does, however, publish one filesystem object that later components will share, so the construction boundary must stay atomic and free of partial visibility.

## Concurrency And Async Scope

- Shared state:
  - One mount-owned filesystem object containing the validated superblock facts, loaded upcase table, loaded allocation bitmap, and synthetic root inode shell.
  - The object is private during bootstrap and becomes shareable only after the constructor succeeds.
- Locks:
  - No lock is required by the mount contract itself.
  - If the implementation uses a temporary private assembly lock, that lock is only for making publication atomic and must not escape the constructor.
- Async operations:
  - None.
- I/O waiting:
  - Mount bootstrap may perform synchronous metadata reads through the existing loaders, but it does not expose an awaitable interface.
- Mutation policy:
  - All mutation is confined to local construction of the mount object.
  - After publication, the mount object is treated as read-only from this component's point of view.

## Required Behavior

- Keep mount bootstrap synchronous.
- Keep the shared-state handoff all-or-nothing.
- Keep the accepted root discovery aggregate consumed exactly once and not redistributed through a second helper layer.
- Do not add futures, tasks, channels, atomics, condition variables, or background invalidation.
- Do not hold any mount-local lock across unrelated directory, read, or write work.
- Do not let lookup policy, allocation policy, or page-cache behavior leak into the publication boundary.

## Publication Ordering

- The mount constructor must finish the dependent loader calls before the filesystem object becomes visible to later code.
- The synthetic root inode must not be published before the upcase table and allocation bitmap are available.
- The object must not be partially initialized and then completed later through a second public step.
- If any bootstrap step fails, no partially built filesystem object should remain visible to callers.

## Ownership Boundaries

- `EXR-SYSROOT-06` remains the owner of root-entry discovery.
- `EXR-UPCASE-07B` remains the owner of table-backed case folding and name hashing.
- `EXR-BITMAP-08A` remains the owner of read-only occupancy queries.
- `EXR-INODE-05B` remains the owner of the synthetic root metadata shell.
- `EXR-MOUNT-09` owns only the mount-local assembly and publication of the combined shared filesystem object.

## Implications For Creator And Checker

- Creator work should stay inside a single synchronous mount constructor.
- Checker work should prove atomic publication with failure injection rather than with a dedicated async harness.
- Any future coordination between readers of the published filesystem object belongs to later components, not to the mount constructor itself.

## Non-Goals

- No awaitable mount API.
- No background load or deferred publication.
- No lock-order design for unrelated later mutable subsystems.
- No shared mutable cache ownership beyond the one published mount object.
- No directory namespace policy.

## Exit Condition

The component is correctly designed when a reader can see that the entire mount path is synchronous, one-shot, and publication-safe, with no partial shared state and no async machinery.

