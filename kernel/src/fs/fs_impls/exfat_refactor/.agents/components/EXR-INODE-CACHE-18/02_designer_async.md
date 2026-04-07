<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Async

## Metadata

- Component ID: EXR-INODE-CACHE-18
- Title: ExfatFs Opened-Inode Table And Validated InodeKey
- Status: `Specified`
- Author: designer
- Date: 2026-04-07
- Based on architect artifact: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CACHE-18/00_architect.md`

## Purpose

Record the serialization contract for opened-inode publication under `ExfatFs`.

This component is not an async workflow and it does not introduce background tasks, but it does need a precise owner-side linearization point so later creator work does not accidentally publish duplicate inode handles or smuggle the root special case into the ordinary keyspace.

## Concurrency And Async Scope

- Shared state:
  - The owner-private opened-inode table.
  - The dedicated root special-case slot.
  - Any small bookkeeping needed to keep one canonical handle per validated key.
- Locks:
  - One filesystem-owner serialization boundary is sufficient for this component.
  - The same boundary guards ordinary table mutation and root-slot publication.
- Async operations:
  - None.
- Block-device or I/O waiting:
  - None while the serialization boundary is held.
  - Any inode construction, directory validation, or disk access must complete before publication enters the critical section.
- Mutation policy:
  - Construct or validate first, then publish canonically under the owner boundary.
  - Removal uses the same linearization point as publication.
  - The root special case stays disjoint from the keyed table even though it uses the same owner-side serialization rule.

## Required Behavior

- Linearize ordinary lookup, reuse, insert, and remove through the owner boundary.
- Ensure that two racing publishers for the same validated `InodeKey` converge on one canonical `Arc<ExfatInode>`.
- Ensure that a racing lookup sees either the old canonical handle or the fully published new one, never a partially initialized placeholder.
- Keep root publication separate from keyed publication so a root handle never masquerades as an ordinary cache entry.
- Keep the publication critical section short and free of blocking work.

## Implications For Creator And Checker

- Creator work should build the inode snapshot and validate the key before entering the owner serialization boundary.
- Creator work should publish the canonical handle only at the end of the critical section, after it is safe to reuse or discard a competing publication.
- Creator work should not add per-inode locks, atomics, or background maintenance just to satisfy the table contract.
- Checker work does not need a dedicated async harness; ordinary ktests can observe that the table reuses handles, that root stays separate, and that the component did not grow a hidden concurrency protocol.

## Non-Goals

- No background refresh or cleanup task.
- No lock-free map.
- No per-inode lock hierarchy.
- No async open protocol.
- No blocking I/O inside the owner serialization boundary.
- No separate root-shell helper.

## Exit Condition

The component is correctly designed when a reader can see one explicit filesystem-owner serialization boundary that protects opened-inode publication and the root special case, with all heavy work done before the critical section and no extra async machinery added.
