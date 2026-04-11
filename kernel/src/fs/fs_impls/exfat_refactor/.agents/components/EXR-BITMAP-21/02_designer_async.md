<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Async

## Metadata

- Component ID: `EXR-BITMAP-21`
- Title: `ExfatFs` Allocation Bitmap Owner State And Read-Only Accounting
- Status: `Specified`
- Author: designer
- Date: 2026-04-10
- Based on architect artifact: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BITMAP-21/00_architect.md`

## Purpose

Record the publication and serialization rule for the validated bitmap snapshot.

This component does not introduce an async workflow or a background task. It does need one explicit owner-side linearization point so later creator work does not expose a partially loaded bitmap or split the immutable snapshot across multiple helper shells.

## Concurrency And Async Scope

- Shared state:
  - The owner-owned validated bitmap image.
  - Any derived used/free counts that are cached alongside that image.
- Locks:
  - One filesystem-owner serialization boundary is sufficient for bitmap publication.
  - The same boundary protects any state transition from "not yet loaded" to "loaded and immutable".
- Async operations:
  - None.
- Block-device or I/O waiting:
  - Bitmap payload reads happen before publication enters the owner critical section.
  - No blocking I/O should be introduced while the immutable snapshot is already published.
- Mutation policy:
  - Validate first, then publish the whole snapshot.
  - After publication, the bitmap is read-only until a later write-side owner exists.
  - Do not split publication into separate helper objects for occupancy and accounting.

## Required Behavior

- Linearize bitmap load, validation, and owner publication through the same filesystem-owner boundary.
- Ensure that a reader sees either no bitmap or one fully validated bitmap snapshot.
- Ensure that occupancy queries and derived accounting operate on the same immutable image.
- Keep the publication critical section short and free of blocking work.

## Implications For Creator And Checker

- Creator work should complete all I/O and validation before entering the publication boundary.
- Creator work should publish one canonical immutable snapshot rather than a partially built helper chain.
- Creator work should not add per-query atomics or lock-free machinery just to serve read-only occupancy.
- Checker work does not need a dedicated async harness; the important regression is that the snapshot stays coherent after publication.

## Non-Goals

- No background refresh task.
- No lock-free publication.
- No per-cluster atomic mutation protocol.
- No asynchronous bitmap loading API.
- No helper shell that only wraps occupancy or accounting results.

## Exit Condition

The async aspect is sufficiently specified when a reader can see one explicit filesystem-owner publication boundary for the bitmap snapshot, with validation completed before publication and no extra async machinery added.
