<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Async

## Metadata

- Component ID: `EXR-WRITE-13A`
- Title: Writable Regular-File Allocation Growth And Metadata Publication
- Status: `Specified`
- Author: `designer`
- Date: `2026-04-05`
- Task packet: `EXR-WRITE-13A-DESIGN-20260405-1224`
- Based on architect artifact: `00_architect.md`

## Purpose

Record the synchronization and publication contract for allocation growth.

The component does not introduce awaitable work or background tasks. It does, however, need a clearly ordered growth transaction so bitmap reservation, chain publication, and inode metadata publication do not drift apart.

## Concurrency And Async Scope

- Shared state:
  - Mount-owned filesystem state.
  - Allocation bitmap mutation state.
  - The writable inode metadata being expanded.
  - Any chain publication scratch needed to keep the enlarged allocation coherent.
- Locks:
  - No long-lived async lock is introduced by this component.
  - A short private growth lock is acceptable if the implementation needs one to keep allocation and publication atomic.
  - If the implementation uses more than one lock, the required order is bitmap mutation first, then chain publication, then inode metadata publication.
- Async operations:
  - None.
- I/O waiting:
  - Allocation bookkeeping may touch the block device synchronously, but this component does not expose an async interface or background retry mechanism.
- Mutation policy:
  - The growth transaction is synchronous and one-shot.
  - No partial growth state may be published to later callers.
  - Buffered write initialization and truncate/shrink remain outside this component.

## Required Behavior

- Keep growth synchronous.
- Keep allocation reservation and metadata publication coupled.
- Keep the valid-data boundary separate from the allocation boundary.
- Do not add futures, tasks, channels, atomics, condition variables, or deferred invalidation.
- Do not let page-cache writeback, buffered write policy, or truncate policy share the growth critical section.
- Do not expose a second public growth path just to split allocation from publication.

## Publication Ordering

- The bitmap must reflect the reserved clusters before the inode is published with the enlarged allocation boundary.
- The chain must be linked or extended before the inode metadata claims the new allocation as visible.
- The inode metadata must not advance the valid-data boundary as part of allocation growth alone.
- If any step fails, no partially grown inode should remain visible to callers.

## Ownership Boundaries

- `EXR-BITMAP-08B` remains the owner of bitmap mutation mechanics.
- `EXR-CHAIN-03B` remains the owner of chain walking and chain facts.
- `EXR-INODE-05B` remains the owner of the read-only inode shell that growth updates.
- `EXR-WRITE-13A` owns only the synchronous allocation-growth and metadata-publication sequence.

## Implications For Creator And Checker

- Creator work should stay confined to one synchronous growth path and the smallest possible publication helper surface.
- Checker work should prove atomic publication and the preserved gap between allocation and valid-data length with ordinary `#[ktest]` regressions.
- Any future coordination between buffered writes and growth belongs to later write-side components, not to this component.

## Non-Goals

- No async growth queue.
- No background allocator.
- No deferred chain linking.
- No page-cache writeback protocol.
- No truncate or shrink protocol.

## Exit Condition

The component is correctly designed when a reader can see that allocation growth is synchronous, ordered, and atomic, with no partial publication and no shared async machinery.
