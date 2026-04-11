<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Async

## Metadata

- Component ID: `EXR-UPCASE-20`
- Title: `ExfatFs` Upcase Table Ownership And Canonicalization Services
- Status: `Specified`
- Author: designer
- Date: 2026-04-10

## Purpose

No dedicated async artifact is required for this component. The upcase-table work does not introduce a background task, a lock-free publication protocol, or a new asynchronous boundary.

This file records that negative decision explicitly so later creator work keeps the table as owner-private runtime state under `ExfatFs` instead of inventing a separate async shell.

## Concurrency And Async Scope

- Shared state:
  - The owner-private validated upcase table stored by `ExfatFs`.
- Locks:
  - The existing filesystem-owner serialization boundary is sufficient.
  - No additional lock hierarchy is introduced by this component.
- Async operations:
  - None.
- Block-device or I/O waiting:
  - Any I/O used to discover or validate the raw candidate completes before the table is published.
  - Folding and name-hash calls themselves are read-only owner services.
- Mutation policy:
  - Validate first.
  - Publish the table atomically once validation succeeds.
  - Keep the installed table immutable after publication.

## Required Behavior

- Linearize table installation so later callers see either the pre-install state or the fully validated table.
- Keep folding and hashing as direct owner reads from the installed table.
- Ensure the publication path does not grow into mount sequencing, directory traversal, or namespace mutation.

## Implications For Creator And Checker

- Creator work should keep validation and publication together inside `fs.rs`.
- Creator work should not add a separate background refresh path or a staging owner.
- Checker work does not need a dedicated async harness; it can observe the stable owner methods directly.

## Non-Goals

- No async open protocol.
- No background table reload.
- No lock-free cache.
- No separate helper owner for canonicalization.

## Exit Condition

The async decision is complete when the reader can see that `ExfatFs` owns one validated upcase table, that publication is linearized by the existing owner boundary, and that no extra async machinery was introduced.
