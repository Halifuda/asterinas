<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Core

## Metadata

- Component ID: `EXR-PGCACHE-26`
- Title: `ExfatInode` Inode-Local Page-Cache Boundary
- Status: `Specified`
- Author: designer
- Date: `2026-04-12`
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-PGCACHE-26/20260412-1202-designer-packet.md`
- Based on architect artifact: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-26/00_architect.md`

## Scope

- In scope:
  - Define the inode-local `PageCache` attachment boundary under `ExfatInode`.
  - Define the inode-private `PageCacheBackend` surface that later creator work will land in `inode.rs`.
  - State how cache population reuses `EXR-READ-OPS-25` rather than re-owning EOF, short-read, or valid-size zero-fill policy.
  - State the temporary boundary for `write_page_async` while `EXR-WRITE-30` and `EXR-SYNC-31` remain future owners.
  - Keep the cache boundary read-only, inode-local, and subordinate to the existing inode snapshot.
- Out of scope:
  - A filesystem-global cache manager or cache service.
  - A second buffered-read implementation or any cache-specific read-policy shell.
  - Dirty eviction policy, writeback ordering, truncate policy, growth policy, or sync policy.
  - Directory, namespace, or allocator ownership.

## Module Specification

- Dependencies:
  - `EXR-INODE-CORE-17` for the stable `ExfatInode` carrier.
  - `EXR-FILE-MAP-24` for logical-to-physical translation consumed by the buffered read owner.
  - `EXR-READ-OPS-25` for buffered byte transfer, EOF handling, short-read handling, and valid-size zero-fill policy.
  - `kernel/src/fs/vfs/page_cache.rs` for `PageCache`, `PageCacheBackend`, and the generic cache container contract.
- Interfaces provided:
  - An inode-private `PageCache` attachment on `ExfatInode`.
  - An inode-private `PageCacheBackend` implementation in `inode.rs`.
  - Small owner-private glue for cache sizing and page-count accounting.
- Files or modules touched:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/mod.rs` only if imports or visibility need a narrow adjustment.
- Hidden implementation details:
  - Whether the cache field is attached directly on `ExfatInode` or through a narrow owner-private wrapper.
  - Whether the backend impl and the cache state land in one creator slice or two slices.
  - Whether the inode-private cache helper uses one or two small owner-private methods for sizing and backend construction.

## Functional Specification

- Precondition:
  - The inode snapshot already identifies a regular file with stable size facts and the mapping layer from `EXR-FILE-MAP-24` is available.
- Action:
  - `ExfatInode` owns a single inode-local `PageCache` object for the file snapshot.
  - The backend surface is implemented on the inode carrier itself, not on a filesystem-global service.
  - Cache misses for readable pages are populated by delegating through the buffered-read owner from `EXR-READ-OPS-25`.
  - The page-cache boundary does not reinterpret EOF, short-read, or valid-size zero-fill policy; it consumes that policy as already owned read behavior.
- Postcondition:
  - Cache-backed page access lives under `ExfatInode` and remains an inode-local integration detail.
  - Page-count accounting and cache sizing stay owner-local and derive from the inode snapshot.
  - No public cache manager or shared read cursor is introduced.

## Invariants

- The final architectural owner remains `ExfatInode`.
- The cache object is inode-local and must not be promoted into a filesystem-global service.
- `EXR-READ-OPS-25` remains the only owner of buffered byte-stream policy.
- `EXR-FILE-MAP-24` remains translation-only.
- `write_page_async` is architecturally real because the trait requires it, but dirty persistence is not owned here.
- Cache sizing may be derived from the inode snapshot, but growth and shrink policy belong to later write-side owners.

## Concurrency Specification

- Shared state:
  - The inode snapshot on `ExfatInode`.
  - The inode-local `PageCache` object.
  - The generic `PageCacheBackend` trait contract.
- Lock ordering:
  - Do not add an inode-level lock hierarchy above the cache's own synchronization.
  - Do not hold a filesystem or inode guard across blocking page-fill I/O.
- Atomicity requirements:
  - A page becomes visible to cache readers only after its fill operation has completed.
  - A page fill must be completed as one logical cache-publication step from the caller's point of view.
- Forbidden interleavings:
  - Do not publish a page-cache object before its backend can be reached from the inode carrier.
  - Do not let dirty-page persistence interleave with this row's read-only cache fill path.
  - Do not let cache sizing mutate write-side growth or truncate state.
- Allowed simplifications such as a temporary big lock:
  - No new big lock is required for this row.
  - The generic page-cache implementation may continue to provide its own internal synchronization.

## Pass Split

### Serial Creator Pass

- Required implementation obligations:
  - Add inode-local page-cache ownership to `ExfatInode`.
  - Land the inode-private `PageCacheBackend` impl in `inode.rs`.
  - Wire page-cache construction to the inode snapshot without creating a cache manager service.
  - Keep backend state and helper placement owner-private.
  - Preserve `EXR-READ-OPS-25` as the only owner of byte-stream policy.
- Explicit non-goals:
  - No dirty writeback logic.
  - No cache resize policy beyond snapshot-derived sizing.
  - No filesystem-global cache service.
  - No duplicate buffered-read implementation.

### Serial Checker Pass

- Required checker-owned tests:
  - A regression that confirms a regular-file inode can attach an inode-local page cache without changing buffered-read ownership.
  - A regression that confirms cache-backed page fill is serviced through the inode owner and not through a standalone cache service.
  - A regression that confirms the cache-visible page data still obeys the read-owner EOF and valid-size zero-fill policy.
  - A regression that confirms repeated cache-backed reads on one snapshot are stable.
- Observable properties that must pass before leaving the serial loop:
  - Cache ownership lives on `ExfatInode`.
  - The backend surface remains inode-private.
  - Read policy still lives on `EXR-READ-OPS-25`.
  - No dirty writeback or sync ownership appears in the row.

### Concurrency Creator Pass

- Required implementation obligations:
  - Keep the backend's per-page fill path consistent with the inode-local cache owner.
  - Preserve the generic page-cache synchronization model without adding a new inode-wide lock layer.
  - Leave `write_page_async` as a temporary unsupported or future-owned surface rather than inventing writeback policy.
- Explicit non-goals:
  - No dirty eviction owner.
  - No truncate or growth owner.
  - No sync-ordering owner.
  - No background cache service.

### Concurrency Checker Pass

- Required checker-owned concurrency tests:
  - No dedicated concurrency tests required.
- Observable properties that must pass before leaving the concurrency loop:
  - The cache boundary remains inode-local.
  - Page publication does not race ahead of page fill.
  - No new writeback or sync interleaving is introduced.

## Acceptance Notes

- Reviewers should confirm that the cache boundary stays subordinate to `ExfatInode` and does not become a service layer.
- Reviewers should confirm that cache population reuses `EXR-READ-OPS-25` instead of rebuilding EOF or valid-size policy.
- Reviewers should confirm that `write_page_async` is named as a future-owner surface, not as a hidden dirty-writeback owner in this row.
- Reviewers should confirm that `inode.rs` remains the primary landing zone and that `mod.rs` only changes if a narrow import or visibility adjustment is unavoidable.

