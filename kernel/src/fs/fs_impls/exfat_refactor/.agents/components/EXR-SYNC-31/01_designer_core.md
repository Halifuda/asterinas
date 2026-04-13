<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Core

## Metadata

- Component ID: `EXR-SYNC-31`
- Title: `ExfatFs` sync and flush-ordering owner boundary
- Status: `Specified`
- Author: designer
- Date: `2026-04-13`
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-SYNC-31/20260413-1304-designer-packet.md`
- Based on architect artifact: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-SYNC-31/00_architect.md`

## Scope

- In scope:
  - Make `ExfatFs` the only filesystem-wide sync and flush-ordering owner.
  - Define the narrow sync surface that serves `FileSystem::sync()`, `Inode::sync_all()`, `Inode::sync_data()`, and the writeback side of `PageCacheBackend::write_page_async()`.
  - Order already-published dirty state from `EXR-WRITE-30` and `EXR-NAMESPACE-29` without widening into a public writeback manager.
  - Leave room for later dirty producers such as `EXR-VOLLABEL-35`, `EXR-INODE-META-36`, and `EXR-BOOT-34` to feed the same owner-private drain path later.
  - Keep the sync boundary strictly about persistence ordering, not control-path policy.
- Out of scope:
  - Direct I/O.
  - Name conversion or charset policy.
  - Boot fallback decisions.
  - Volume-label user control.
  - FAT-attribute ioctls.
  - Trim/discard.
  - Forced shutdown.
  - A filesystem-global cache manager.
  - An inode-local sync owner separate from `ExfatFs`.

## Module Specification

- Dependencies:
  - `EXR-FS-CORE-16` for the filesystem owner carrier.
  - `EXR-WRITE-30` for buffered file dirty production.
  - `EXR-NAMESPACE-29` for namespace dirty production.
  - `EXR-PGCACHE-26` for inode-local page-cache integration.
  - The VFS `FileSystem`, `Inode`, and `PageCacheBackend` contracts.
- Interfaces provided:
  - `FileSystem::sync()`.
  - `Inode::sync_all()`.
  - `Inode::sync_data()`.
  - `PageCacheBackend::write_page_async()`.
  - A private `ExfatFs` sync root and owner-private dirty-state helper(s).
- Files or modules touched:
  - `kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- Hidden implementation details:
  - Whether the owner keeps a private pending-dirty snapshot, a private sync gate, or both.
  - The exact internal shape of the order queue, so long as it remains private to `ExfatFs`.
  - The exact helper names used by `inode.rs` to delegate into the filesystem owner.

## Functional Specification

### Operation

- Name: `ExfatFs::sync`
- Inputs:
  - `&self`
- Preconditions:
  - The filesystem owner already contains any dirty state published by buffered writes, namespace mutation, or later metadata producers.
- Actions:
  - Acquire the filesystem-private sync gate.
  - Snapshot the already-published dirty producers that belong to this filesystem instance.
  - Drain dirty state in owner-defined persistence order.
  - Keep the ordering boundary focused on flush sequencing only.
  - Do not make policy choices about boot fallback, label control, or administrative ioctls.
- Outputs:
  - `Result<()>`
- Postconditions:
  - All dirty state that was already published to the filesystem owner before the call is either flushed in the chosen order or left in a clean, repeatable state for the next call.
  - Repeated clean calls remain success-only and do not widen the owner boundary.

### Operation

- Name: `Inode::sync_all`
- Inputs:
  - `&self`
- Preconditions:
  - The inode is an `ExfatInode`.
- Actions:
  - Delegate directly to the filesystem-owned sync root.
  - Use the full flush-ordering scope for inode-visible dirty state.
  - Keep the method thin enough that it does not become a second sync owner.
- Outputs:
  - `Result<()>`
- Postconditions:
  - The inode sync-all path shares the same owner-private ordering boundary as `FileSystem::sync()`.

### Operation

- Name: `Inode::sync_data`
- Inputs:
  - `&self`
- Preconditions:
  - The inode is an `ExfatInode`.
- Actions:
  - Delegate directly to the same filesystem-owned sync root used by `sync_all()`.
  - Keep the data-only surface thin and owner-local.
  - Do not introduce a separate public writeback manager to distinguish it from `sync_all()`.
- Outputs:
  - `Result<()>`
- Postconditions:
  - Data sync and full inode sync remain two entry points into the same owner-private persistence boundary unless a later row proves a real semantic split.

### Operation

- Name: `PageCacheBackend::write_page_async`
- Inputs:
  - `&self`
  - A cached page index and page frame.
- Preconditions:
  - The page belongs to an inode that already owns inode-local page-cache state.
- Actions:
  - Route dirty page writeback into the same filesystem-owned ordering path.
  - Keep the method downstream from the cache owner instead of turning it into a second cache manager.
  - Preserve the existing inode/page-cache ownership split.
- Outputs:
  - `Result<BioWaiter>` or the current backend equivalent.
- Postconditions:
  - Dirty page writeback remains a persistence seam, not a new page-cache owner.

## Invariants

- `ExfatFs` is the only filesystem-wide sync owner.
- `sync()` is a flush-ordering boundary, not a control-path bucket.
- `sync_all()` and `sync_data()` do not own independent persistence policy.
- `write_page_async()` is downstream to the same owner-private flush root.
- Dirty producers may expand later, but they do not justify a broader sync owner.
- Repeated sync calls on a clean owner are idempotent from the caller point of view.

## Concurrency Specification

- Shared state:
  - The filesystem-private dirty-state snapshot.
  - The filesystem-private sync gate.
  - Any inode/page-cache writeback intents already published to `ExfatFs`.
- Lock ordering:
  - Acquire the filesystem-private sync gate before inspecting dirty producers.
  - Do not hold inode-local page-cache guards while entering filesystem-wide flush ordering.
  - Do not hold a filesystem-global lock across unrelated policy decisions or control-path lookups.
- Atomicity requirements:
  - A single sync call linearizes the already-published dirty state for one filesystem instance.
  - `sync_all()` and `sync_data()` must serialize through the same owner-private root rather than racing as independent owners.
- Forbidden interleavings:
  - Do not let `write_page_async()` bypass the owner-private flush root and invent a second writeback manager.
  - Do not flush later control-path state through a separate code path that reorders the same dirty producer set.
  - Do not expose the sync gate as a public API.
- Allowed simplifications:
  - One private mutex or equivalent gate is sufficient.
  - One private dirty-state accumulator is sufficient.
  - No background worker is required.

## Pass Split

### Serial Creator Pass

- Required implementation obligations:
  - Implement the filesystem-owned sync root in `fs.rs`.
  - Add thin `sync_all()` and `sync_data()` delegation hooks on `ExfatInode` in `inode.rs`.
  - Route `write_page_async()` into the same owner-private flush path instead of creating a second cache owner.
  - Keep the implementation centered on already-published dirty-state ordering only.
- Explicit non-goals:
  - No direct I/O.
  - No name conversion or boot policy.
  - No volume-label control.
  - No trim/discard or forced shutdown.
  - No public writeback manager.
  - No filesystem-global cache service.

### Serial Checker Pass

- Required checker-owned tests:
  - A regression that proves `FileSystem::sync()` remains success-only on a clean filesystem and does not change stable owner-visible snapshot data.
  - A regression that proves `Inode::sync_all()` and `Inode::sync_data()` both delegate into the same filesystem-owned persistence boundary.
  - A regression that proves `write_page_async()` remains a downstream persistence seam rather than a second page-cache owner.
  - A regression that proves repeated sync calls remain idempotent after the same dirty state has been drained once.
- Observable properties that must pass before leaving the serial loop:
  - Sync stays a flush-ordering owner boundary, not a control bucket.
  - The inode hooks are thin delegates, not competing owners.
  - Dirty state is consumed from the accepted producer set without widening the boundary.

### Concurrency Creator Pass

- Required implementation obligations:
  - Provide only the private serialization needed by the filesystem sync root.
  - Keep the sync gate and dirty-state snapshot owner-private.
  - Preserve the same ordering root for all sync entry points.
- Explicit non-goals:
  - No public concurrency API.
  - No background flush thread.
  - No separate writeback service.

### Concurrency Checker Pass

- Required checker-owned concurrency tests:
  - A concurrency regression that exercises concurrent `sync()` and inode `sync_all()` or `sync_data()` calls against the same filesystem instance and confirms they serialize through one owner-private root.
- Observable properties that must pass before leaving the concurrency loop:
  - Concurrent entry does not create duplicate owners or duplicate writeback managers.
  - The dirty producer set is drained in one serialized order per filesystem instance.

## Acceptance Notes

- The reviewer should confirm that `ExfatFs` remains the only filesystem-wide persistence owner.
- The reviewer should reject any design that promotes `write_page_async()` into a second cache manager.
- The reviewer should confirm that `sync_all()` and `sync_data()` are thin delegates and do not introduce separate policy.
- The reviewer should treat `EXR-VOLLABEL-35`, `EXR-INODE-META-36`, and `EXR-BOOT-34` as future dirty producers, not as reasons to widen this row now.
- The reviewer should confirm that the sync boundary excludes control-path policy, direct I/O, and admin surfaces.
