<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Async

## Metadata

- Component ID: `EXR-SYNC-31`
- Title: `ExfatFs` sync serialization and repeated-call behavior
- Status: `Specified`
- Author: designer
- Date: `2026-04-13`
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-SYNC-31/20260413-1304-designer-packet.md`
- Based on architect artifact: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-SYNC-31/00_architect.md`

## Scope

- In scope:
  - Define the serialization boundary for the filesystem-owned sync root.
  - State how repeated sync calls behave when no new dirty state is published.
  - State how inode sync hooks and page-cache writeback remain serialized through the same owner-private root.
  - Make explicit that the component does not own a background flush worker.
- Out of scope:
  - Deferred publish queues.
  - Background dirty-page workers.
  - A public writeback manager.
  - Direct I/O sequencing.
  - Control-path policy.

## Serialization Contract

- Shared boundaries involved:
  - The filesystem-private dirty-state snapshot.
  - The filesystem-private sync gate.
  - The inode-local page-cache writeback seam.
  - The owner-private helper used by `ExfatFs` to drain dirty producers.
- Rule 1:
  - One sync call owns one serialized drain of already-published dirty state for one filesystem instance.
  - Inode sync calls and page-cache writeback calls must enter that same serialization boundary rather than inventing their own.
- Rule 2:
  - The same dirty producer must not be observed as independently owned by `sync()`, `sync_all()`, `sync_data()`, and `write_page_async()`.
  - These entry points are different caller surfaces, not different persistence owners.
- Rule 3:
  - Repeated clean calls are allowed and should be behaviorally idempotent.
  - Once a dirty producer has been drained, the next identical call must see a clean or already-ordered state, not a widened owner boundary.
- Rule 4:
  - Any later dirty producer, including volume-label or boot-flag state, must still enter the same owner-private sync root when it is eventually added.
  - Those later producers do not justify a new sync manager.

## Repeated-Call Expectations

- `sync()`:
  - A second call after a successful clean drain should return success without altering the stable filesystem snapshot.
- `sync_all()`:
  - Repeating it after the same dirty state is drained should not expose a different owner or a different ordering root.
- `sync_data()`:
  - Repeating it after the same data has already been ordered should remain a thin delegate into the same root.
- `write_page_async()`:
  - Repeated dirty-page writeback requests should remain serialized through the same owner-private path, not split into a cache-specific manager and a sync-specific manager.

## Forbidden Interleavings

- Do not let `sync_all()` and `sync_data()` race as if they were different persistence services.
- Do not let `write_page_async()` bypass the filesystem-owned sync root.
- Do not create a deferred flush queue that outlives the sync call without an explicit future owner.
- Do not allow later control-path mutations to reorder ahead of already-published inode dirty state.

## Allowed Simplifications

- One private sync gate is sufficient.
- One private dirty-state snapshot is sufficient.
- Synchronous serialization is sufficient.
- No background flush worker is required.

## Why No Dedicated Async Creator Patch Is Needed

- The component is about ordering already-published dirty state, not about introducing a second asynchronous publication engine.
- The inode-side page-cache seam already exists as a VFS entry point, so the work is to delegate it into the same owner-private root, not to invent a new daemon.
- Later dirty producers can reuse the same root without widening the boundary.

## Reviewer And Checker Expectations

- Reviewers should confirm that the design remains a serialization boundary only.
- Reviewers should confirm that `sync_all()` and `sync_data()` do not become separate owners.
- Reviewers should confirm that `write_page_async()` is still downstream to `ExfatFs` instead of becoming a cache manager.
- Checkers should treat any background flush worker or deferred queue as boundary drift.
