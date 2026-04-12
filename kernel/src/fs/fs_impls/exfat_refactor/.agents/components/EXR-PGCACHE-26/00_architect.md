<!-- SPDX-License-Identifier: MPL-2.0 -->

# Architect Handoff

## Metadata

- Component ID: `EXR-PGCACHE-26`
- Title: `ExfatInode` page-cache integration boundary
- Status: `Architected`
- Author: architect
- Date: `2026-04-12`
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-PGCACHE-26/20260412-1112-architect-packet.md`

## Functional Unit Definition

- Functional goal: integrate a stable page-cache backend under `ExfatInode` so regular-file caching, cache sizing, and cache-backed page fill live on the inode carrier without absorbing buffered read semantics, write-side growth, truncate policy, or filesystem-global cache ownership.
- Final architectural owner: `ExfatInode`
- Owner class:
  - VFS trait carrier
- Expected landing form:
  - owner-internal state plus trait impl
- Boundary kind:
  - stable architectural boundary
- Why this boundary is architecturally real: page cache is the inode-local cache for one file snapshot. The inode already owns the file identity, size facts, and read-path translation boundary, while `page_cache.rs` defines the backend protocol that the inode must satisfy. The finished system therefore needs an inode-owned cache object and backend implementation, not a standalone cache manager service.

## Purpose

This unit gives `ExfatInode` the stable page-cache integration seam needed for later cached read/write work. The row should capture the owner-local cache object, the backend trait surface, and the inode-private glue that connects cache population to inode-owned read-path behavior.

The unit must stay narrower than “file I/O.” It may own cache attachment, cache sizing, page fill routing, and inode-local cache state, but it does not own the buffered read contract itself, and it does not own dirty writeback, growth, truncate, or filesystem-wide sync ordering.

## Why This Comes Now

`EXR-READ-OPS-25` already owns the buffered regular-file byte-transfer contract on `ExfatInode`, and `EXR-FILE-MAP-24` already owns the inode-private logical-to-physical translation used by the read path. That means page-cache work can now converge on a real inode-local cache boundary instead of inventing a separate buffering service.

This timing also keeps the cached path from re-owning read policy. `EXR-PGCACHE-26` should consume the buffered-read owner by delegating page population to the existing inode read-path behavior, rather than redefining EOF handling, zero-fill policy, or request splitting.

## Owner And Integration Convergence

- Interfaces, traits, services, or higher-level functions this unit ultimately serves:
  - `PageCacheBackend`
  - inode-local cached page management
  - later regular-file cached read/write behavior on `ExfatInode`
- If the unit is internal-only, why that internal ownership is still stable in the finished system:
  - it is not internal-only; the cache backend is a stable VFS integration detail for the inode carrier, and the page cache must live with the inode that owns the file snapshot
- Known non-goals or nearby logic that must remain in the parent owner:
  - buffered read semantics, EOF handling, and valid-size zero-fill policy
  - write-side growth and truncate policy
  - allocator mutation and dirty persistence policy
  - filesystem-global cache management
  - directory behavior and namespace mutation

Boundary consumption rules:

- `EXR-READ-OPS-25` remains the owner of buffered byte transfer. `EXR-PGCACHE-26` may call through that owner boundary to fill cache pages, but it must not create a second buffered-read implementation or a cache-specific read policy shell.
- `page_cache.rs` provides the trait contract and generic cache container. This row consumes that interface as a stable external protocol, not as a hint to create a filesystem-global cache service.
- Cache state that is architecturally real now is inode-local state: the `PageCache` object, the `PageCacheBackend` implementation, and the inode-private wiring needed to expose page-cache-backed access.
- Page-cache writeback, resize after growth, dirty eviction ordering, and final sync policy belong to later write-side and sync owners, not this row.

## Dependency Contract

- Depends on:
  - `EXR-INODE-CORE-17`
  - `EXR-FILE-MAP-24`
  - `EXR-READ-OPS-25`
  - `kernel/src/fs/vfs/page_cache.rs`
- Blocks:
  - later cached read/write work on `ExfatInode`
  - any future page-cache-backed read path that expects inode-owned cache state
  - later write-side and sync rows that need a stable inode cache owner to extend
- Can run in parallel with:
  - command-free architect or designer work for later rows that do not widen `inode.rs` into a cache service
  - read-only planning on future write-side and sync owners
- Recommended parallel wave:
  - Wave C-to-D bridge: cache integration can be specified after read-only ownership is stable, while later write-side planning remains separate
- Stable pre-existing interfaces used:
  - `PageCache`
  - `PageCacheBackend`
  - `ExfatInode`
  - `InodeIo::read_at`
  - `VmWriter`/`VmReader` as the surrounding VFS I/O context
- Prior sources or prior slices that materially shaped the split:
  - `WORKSPACE-ARCH-RESET/00_architect.md`
  - `EXR-INODE-CORE-17/00_architect.md`
  - `EXR-FILE-MAP-24/00_architect.md`
  - `EXR-READ-OPS-25/00_architect.md`
  - `kernel/src/fs/vfs/page_cache.rs`
  - `kernel/src/fs/vfs/fs_apis/inode.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
  - `kernel/src/fs/fs_impls/exfat/inode.rs`

## Recommended Work Slices

These are candidate creator slices for scheduler consideration, not the active global plan.

| Slice ID | Parent Unit Scope | Goal | Likely Write Set | Depends On | May Overlap With | Lane Class | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `WS-PGCACHE-26-STRUCT` | `EXR-PGCACHE-26` | Add inode-local page-cache state and the narrow constructor/wiring needed to attach a `PageCache` to `ExfatInode` without changing buffered read policy. | `kernel/src/fs/fs_impls/exfat_refactor/inode.rs` | `EXR-INODE-CORE-17`, `EXR-READ-OPS-25` | `WS-PGCACHE-26-BACKEND` if both slices land in the same `inode.rs` region | creator | Keep this slice cache-state only. Do not add dirty writeback, resize policy, or a new cache manager owner. |
| `WS-PGCACHE-26-BACKEND` | `EXR-PGCACHE-26` | Implement the inode-owned `PageCacheBackend` surface so cache misses and page population are served through the inode carrier and its existing read-path boundary. | `kernel/src/fs/fs_impls/exfat_refactor/inode.rs` | `EXR-INODE-CORE-17`, `EXR-FILE-MAP-24`, `EXR-READ-OPS-25` | `WS-PGCACHE-26-STRUCT` if the trait impl and state land together in `inode.rs` | creator | Keep the backend inode-owned. It may use buffered-read behavior, but it must not duplicate read semantics or invent a page-cache service layer. |
| `WS-PGCACHE-26-READONLY-GLUE` | `EXR-PGCACHE-26` | Add the small glue methods that expose cache-backed inode access, page-count accounting, and read-only cache sizing until later write-side owners define growth and eviction policy. | `kernel/src/fs/fs_impls/exfat_refactor/inode.rs` | `EXR-READ-OPS-25` | `WS-PGCACHE-26-STRUCT` if helper placement collides | creator | This slice should stay read-only and owner-local. If it starts handling dirty eviction, truncate, or sync ordering, it has crossed into later rows. |

## exFAT Concepts Covered

- Inode-local page cache ownership.
- `PageCacheBackend` as the stable integration protocol.
- Cache-backed regular-file reads on `ExfatInode`.
- Page population through the inode-owned read path.
- Cache sizing and page-count accounting.
- Read-only cache attachment versus later writeback and growth policy.

## Boundary Rejections

- Splits considered but rejected:
  - a filesystem-global cache manager separate from `ExfatInode`
  - a new buffering service that re-implements `EXR-READ-OPS-25`
  - moving writeback, dirty eviction, or growth policy into the page-cache row
  - folding directory or namespace behavior into the cache boundary
- Why those rejected splits would be packet convenience, not real architecture:
  - they would hide the stable inode carrier that actually owns the file snapshot
  - they would blur cache integration with the already-owned buffered-read contract
  - they would recreate a generic service layer where the finished system needs inode-local ownership

## Target Files

- Existing files likely to change:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- New files expected:
  - none

## Code Budget

- Target creator work-slice size: `160-280` lines
- Expected number of creator slices: `2` to `3`
- Reason if any single slice might exceed 500 lines:
  - it should not. If the slice grows that large, cached read/write policy or sync behavior has leaked into the cache boundary and the unit should be re-sliced instead of expanded.

## Exit Condition

Design work may start once `ExfatInode` is understood as the inode-local owner of a stable `PageCache` attachment plus the `PageCacheBackend` surface, with buffered read semantics still owned by `EXR-READ-OPS-25` and write-side dirty policy still deferred to later rows.

## Risks

- `PageCacheBackend` may tempt a reimplementation of buffered reads. The design must keep byte-stream policy on `EXR-READ-OPS-25`.
- `inode.rs` is likely the shared landing zone for both cache-state and backend slices, so fake parallelism should be avoided if helper placement collides.
- Cache sizing can drift into growth policy if the design tries to solve truncate or file expansion early.
- Dirty eviction and sync ordering must stay outside this row until the later write-side and sync owners exist.
