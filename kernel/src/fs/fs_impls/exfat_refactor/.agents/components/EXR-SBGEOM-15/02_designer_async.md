<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Async

## Metadata

- Component ID: EXR-SBGEOM-15
- Title: Explicit Data-Cluster Geometry Bounds
- Status: `Specified`
- Author: designer
- Date: 2026-04-01

## Purpose

Keep this repair synchronous and side-effect free beyond the existing superblock geometry construction.

The component is not an async workflow, not a background task, and not a concurrency policy change.

## Concurrency And Async Scope

- Shared state:
  - None introduced by this repair.
- Locks:
  - None introduced by this repair.
- Async operations:
  - None.
- Block-device or I/O waiting:
  - None inside the new geometry helpers.
- Mutation policy:
  - Read-only geometry helpers only; no writeback, no dirty-state tracking, and no background flush behavior.

## Required Behavior

- Keep the repair limited to pure geometry interpretation and cluster-bound validation.
- Do not introduce futures, tasks, channels, mutexes, atomics, or callbacks.
- Do not move boot-region validation or mount sequencing into this component.
- Do not make cluster-bound checks depend on runtime ownership of the filesystem object.

## Implications For Creator And Checker

- Creator work should remain confined to `super_block.rs`.
- Checker work should stay synchronous and should not need any async-specific harness.
- Any tests for this component should be ordinary `#[ktest]` checks against pure geometry helpers.

## Non-Goals

- No async read path.
- No async write path.
- No concurrent mutation policy.
- No lock-order discussion.
- No background retries or deferred recovery.

## Exit Condition

The component is correctly designed when a reader can confirm that the entire repair is a synchronous geometry cleanup and nothing more.
