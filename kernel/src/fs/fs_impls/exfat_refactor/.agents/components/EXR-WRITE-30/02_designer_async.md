<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Async

## Metadata

- Component ID: `EXR-WRITE-30`
- Title: `ExfatInode` write-side serialization and call-local publication order
- Status: `Specified`
- Author: designer
- Date: `2026-04-13`
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-WRITE-30/20260413-0650-designer-repair-packet.md`
- Based on architect artifact: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/00_architect.md`

## Scope

- In scope:
  - Define the per-call serialization boundary for buffered writes, growth, and truncate on `ExfatInode`.
  - State what later readers may observe before and after a successful write or resize call.
  - State how the call-local `ExfatInodeWriteState`, committed allocation results, and inode-local page-cache state are sequenced without inventing a background writer.
  - Make explicit that this row does not reserve a separate post-serial concurrency creator/checker patch sequence.
  - Explain why this component still does not own durable flush ordering.
- Out of scope:
  - A background dirty-page worker.
  - Deferred publish queues for file size or allocation facts.
  - Page-cache writeback ordering or sync policy.
  - Direct-I/O sequencing.

## Serialization Contract

- Shared boundaries involved:
  - The call-local `ExfatInodeWriteState`.
  - The inode-local `PageCache`.
  - The filesystem-owned committed-allocation service under `ExfatFs`.
  - The downstream `write_page_async()` / sync seam.
- Rule 1:
  - One write or resize request owns its own allocation consumption, gap handling, cache sizing, byte publication, and inode-state publication.
  - Temporary helper state must remain call-local and must not outlive the write or resize call.
- Rule 2:
  - A larger logical size must not become visible until the inode has either consumed the required committed allocation result or proved that no extra allocation coverage is needed.
  - A larger initialized range must not become visible until any skipped valid-size gap has become zero-visible and the written bytes for that call are visible.
- Rule 3:
  - Later readers should observe either the old inode-visible byte stream or the fully applied new byte stream for one call.
  - No caller should observe a half-grown file, a half-truncated file, or a partially published allocation result as the visible inode state.
- Rule 4:
  - Dirty byte publication is allowed to remain inode-local, but durable flush ordering is still downstream.
  - `write_page_async()` must remain a future-owner seam for `EXR-SYNC-31` rather than becoming a hidden writeback protocol in this row.

## Repeated-Call Expectations

- Write stability:
  - Repeating the same buffered write against the same starting inode snapshot and reader bytes should produce the same visible byte stream and the same published size facts.
- Resize stability:
  - Repeating the same successful grow or shrink request against the same starting snapshot should produce the same EOF and valid-size outcome.
- Allocation visibility:
  - Later reads or growth calls must not observe a committed allocation result until the inode has fully folded it into its owned file-state snapshot.
- Locality of state:
  - Any mutation holder or temporary write context must remain inode-local; it must not become a filesystem-global coordinator.

## Forbidden Interleavings

- Do not let a write publish a larger `valid_size` before its skipped gap has become zero-visible.
- Do not let `resize` publish a larger EOF before page-cache sizing and allocation-state updates for that call are complete.
- Do not expose a committed allocation result as a reusable handle outside the inode call that consumed it.
- Do not add a background flush queue, writeback worker, or deferred size-publication task.
- Do not reserve a second `EXR-WRITE-30` async creator/checker patch after the serial loop closes; the serial loop is the only follow-up lane this row needs.

## Allowed Simplifications

- One inode-local mutation holder is sufficient.
- Synchronous consumption of `ExfatFs::allocate_clusters()` is sufficient.
- Dirty data may remain inode-local and unsynced after the call as long as the visible byte stream is coherent and the later sync owner is still named explicitly.

## Why No Dedicated Async Artifact Is Needed

- Buffered write and size mutation are synchronous inode-owner concerns.
- The component does not introduce background publication, deferred cleanup, or cross-thread reservation visibility.
- The only cross-owner handoffs are the committed allocation result from `EXR-ALLOC-27`, the already attached inode-local cache from `EXR-PGCACHE-26`, and the downstream sync seam already reserved for `EXR-SYNC-31`.
- That means the core design already covers the required serialization discipline, and a separate async protocol would add surface without creating a new architectural owner.

## Reviewer And Checker Expectations

- Reviewers should confirm that write visibility and size publication stay inode-local and synchronous.
- Reviewers should confirm that `write_page_async()` is still a future-owner seam rather than a silently absorbed writeback protocol.
- Reviewers should confirm that `02_designer_async.md` documents serialization only and does not reserve a later concurrency patch sequence.
- Checkers should treat any attempt to add a background writer, deferred publish queue, or filesystem-global mutation manager as boundary drift.
