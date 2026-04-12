<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Async

## Metadata

- Component ID: `EXR-PGCACHE-26`
- Title: `ExfatInode` Page-Cache Fill Sequencing And Writeback Boundary
- Status: `Specified`
- Author: designer
- Date: `2026-04-12`
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-PGCACHE-26/20260412-1202-designer-packet.md`
- Based on architect artifact: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-26/00_architect.md`

## Scope

- In scope:
  - Define per-page sequencing for inode-local page-cache population.
  - State how cache fills consume `EXR-READ-OPS-25` without duplicating read policy.
  - State the temporary treatment of `write_page_async` while dirty persistence remains unowned.
  - Keep the row free of background cache management or new shared async state.
- Out of scope:
  - A shared read cursor.
  - A new lock hierarchy across inode, cache, or filesystem objects.
  - Dirty eviction policy, writeback ordering, truncate policy, or sync policy.

## Async Sequencing Contract

- Shared boundaries involved:
  - The inode-local `PageCache` object.
  - The inode-private `PageCacheBackend` implementation.
  - The buffered-read owner from `EXR-READ-OPS-25`.
- Rule 1:
  - One page fill is one inode-local cache publication unit.
  - The backend must not mark a page ready before the underlying fill for that page is complete.
- Rule 2:
  - Cache fills consume the buffered-read owner rather than recreating EOF, short-read, or valid-size zero-fill policy.
  - If the page spans a valid-size gap, the page fill must preserve the exact byte stream already defined by `EXR-READ-OPS-25`.
- Rule 3:
  - Page population is per-page and call-local.
  - A page miss may be handled by a synchronous fill or by a backend waiter, but the row does not introduce any background worker or shared cursor.
- Rule 4:
  - `write_page_async` is a structurally required trait method, but dirty persistence is not owned in this row.
  - If the backend needs a temporary unsupported answer for writeback, it must be named as future-owned by `EXR-WRITE-30` and `EXR-SYNC-31`.

## Per-Call And Per-Page Rules

- For page reads:
  - The backend may fill a cache page by delegating through the inode owner and the buffered-read contract.
  - The backend must remain inode-local, and it must not publish a separate cache-service abstraction.
  - The page cache may treat a zero-request or immediate-fill result as a completed page without inventing extra state.
- For page counts:
  - `npages()` reflects the inode snapshot and bounds the cache-visible file range.
  - Pages beyond the current file-backed page count are not part of this row's cached read population.
- For writeback:
  - `write_page_async` stays outside the supported semantic core for this row.
  - No dirty-page eviction or persistence sequencing is defined here.

## Forbidden Interleavings

- Do not hold a future page-cache guard while performing raw physical reads.
- Do not let a page become cache-visible before the inode-owned fill has finished.
- Do not let a dirty-page flush race with the read-only cache-fill path in this row.
- Do not let one page's fill bookkeeping leak into another page's state.

## Allowed Simplifications

- A synchronous page fill that returns an already-completed waiter is acceptable.
- Reconstructing the inode read path per page is acceptable.
- A temporary unsupported writeback path is acceptable if it is explicitly handed off to later write-side owners.

## Reviewer/Checker Expectations

- Reviewers should confirm that page-fill sequencing reuses `EXR-READ-OPS-25` and does not re-own read policy.
- Reviewers should confirm that `write_page_async` is acknowledged as a future-owner surface, not silently absorbed into this row.
- Checkers should confirm that page-ready publication follows fill completion and that repeated fills on one snapshot remain stable.

