<!-- SPDX-License-Identifier: MPL-2.0 -->

# Architect Handoff

## Metadata

- Component ID: `EXR-PGCACHE-11B`
- Title: Page-Cache Backend Integration For Regular Files
- Status: `Architected`
- Author: `architect`
- Date: `2026-04-05`
- Task packet: [`EXR-PGCACHE-11B-ARCH-20260405-1128`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-PGCACHE-11B/20260405-1128-architect-packet.md)

## Purpose

This handoff covers the smallest useful page-cache component for the refactor: the exFAT regular-file backend that satisfies `PageCacheBackend` and keeps cache sizing aligned with the file's visible length.

The component owns the backend bridge between mount-owned filesystem state, inode read facts, and the page-cache runtime. It is responsible for the page-level read/write hooks and the backend page-count contract that the cache uses to decide when a page is backed by disk and when it should read as zero.

It does not own buffered `read_at`, byte-copy policy, direct I/O, namespace behavior, allocation growth, or truncate policy.

## Why This Comes Now

This split is safe now because the upstream boundaries already exist:

- `EXR-MOUNT-09` owns mount bootstrap and shared filesystem state.
- `EXR-INODE-05B` owns the read-only inode metadata shell.
- `EXR-READ-11A` owns logical-to-physical placement for existing regular-file data.

Linux `inode.c` and `file.c` show the same separation: the mapping and page-cache backend hooks sit below buffered read execution, and buffered read policy sits above them. The refactor should preserve that split instead of reintroducing a monolithic inode object.

The remaining pressure is no longer "how do we identify the file or map its clusters?" It is "who owns the backend that the page cache calls into once those facts are already accepted?" That is this component.

## Dependency Contract

- Depends on:
  - `EXR-MOUNT-09`
  - `EXR-INODE-05B`
  - `EXR-READ-11A`
- Blocks:
  - `EXR-READ-11B`
  - later write-side data-path components that need a finalized exFAT page-cache backend
- Can run in parallel with:
  - `EXR-DIR-10` once the mount and inode contracts are accepted
  - other command-free planning work that only consumes the mount/shared-state shape
- Recommended parallel wave:
  - finish mount and inode acceptance first;
  - let `EXR-READ-11A` define placement;
  - then bring up `EXR-PGCACHE-11B` alongside unrelated lookup planning, while keeping buffered `read_at` blocked until the backend exists.
- Stable pre-existing interfaces used:
  - mount-owned shared filesystem state from `EXR-MOUNT-09`
  - validated inode read facts from `EXR-INODE-05B`
  - the physical-placement boundary from `EXR-READ-11A`
  - `PageCache`, `PageCacheBackend`, `CachePage`, `BioWaiter`, and `PageState` from [`kernel/src/fs/vfs/page_cache.rs`](/home/halifuda/asterinas/kernel/src/fs/vfs/page_cache.rs)
  - `ExfatSuperBlock` geometry and cluster-to-byte helpers
- Prior sources or prior slices that materially shaped the split:
  - `Microsoft-exFAT-spec.md` for `NoFatChain`, valid-data-length, and contiguous-versus-FAT-backed placement rules.
  - `linux-exFAT-implementation-summary.md` plus Linux [`fs/exfat/inode.c`](/home/halifuda/linux/fs/exfat/inode.c) and [`fs/exfat/file.c`](/home/halifuda/linux/fs/exfat/file.c) for the split between mapping, backend hooks, and buffered read execution.
  - `EXR-MOUNT-09`, `EXR-INODE-05B`, and `EXR-READ-11A` for the accepted mount, inode, and placement boundaries.
  - `ASTERINAS_ARCHITECT_PRIORS.md` and `ASTERINAS_CODE_QUALITY_PRIORS.md` for the local rules on ownership boundaries and top-down component shape.

## exFAT Concepts Covered

- `PageCacheBackend` ownership for regular files.
- Page-level fetch and eviction/writeback hooks for already-mounted files.
- Backend page-count coordination from the file's visible length.
- Page-cache zero behavior for pages beyond the backend-visible range.
- Use of the accepted placement boundary instead of re-deriving cluster mapping.
- Exclusion of buffered `read_at`, namespace mutation, allocation growth, and truncate policy.

## Target Files

- Existing files likely to change:
  - [`kernel/src/fs/fs_impls/exfat_refactor/inode.rs`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs)
  - [`kernel/src/fs/fs_impls/exfat_refactor/fs.rs`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs)
  - [`kernel/src/fs/fs_impls/exfat_refactor/mod.rs`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/mod.rs)
- New files expected:
  - none

## Code Budget

- Target new or heavily rewritten code size:
  - `180-280` lines
- Reason if the budget might exceed 500 lines:
  - It should not if the component stays at backend hooks and cache-size coordination. If it starts absorbing buffered read policy, write-side growth, or namespace behavior, the split is wrong and should be widened elsewhere instead.

## Exit Condition

Design work may start when there is exactly one page-cache backend surface that:

1. implements `PageCacheBackend` for exFAT regular files using accepted mount state and inode facts,
2. routes page reads and writes through the accepted placement boundary instead of re-deriving physical mapping,
3. reports backend page count from the file's visible length so cache sizing stays aligned with the readable range,
4. leaves buffered `read_at`, page-copy policy, write-side growth, and namespace behavior to later components.

## Risks

- The backend can become a second mount path if it starts reopening volume state instead of borrowing the accepted mount-owned object.
- The backend can drift into buffered `read_at` if it starts choosing how bytes are copied instead of only supplying page-level I/O.
- If backend page count is tied to allocated length instead of visible length, the page cache will expose stale or unwritten pages.
- If writeback, growth, and truncation are merged into the same component, the boundary has crossed into write-side data-path work.
- If helper accessors proliferate, the inode shell will drift from a read-only boundary into a live behavior layer.
