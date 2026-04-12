<!-- SPDX-License-Identifier: MPL-2.0 -->

# Architect Handoff

## Metadata

- Component ID: `EXR-WRITE-30`
- Title: `ExfatInode` write-side file mutation owner boundary
- Status: `Architected`
- Author: architect
- Date: 2026-04-12
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-WRITE-30/20260412-2211-architect-packet.md`

## Functional Unit Definition

- Functional goal: implement the smallest coherent `ExfatInode` write-side unit that owns buffered `write_at`, growth, truncate, and resize behavior for regular files while consuming inode-local page-cache ownership, inode-private mapping helpers, and filesystem-owned committed allocation results without absorbing sync ordering.
- Final architectural owner: `ExfatInode`
- Owner class:
  - VFS trait carrier
- Expected landing form:
  - owner methods plus owner-private helpers
- Boundary kind:
  - stable architectural boundary
- Why this boundary is architecturally real:
  - buffered file mutation is the stable VFS-visible write contract for a regular-file inode. The inode already owns the file snapshot, mapping helpers, and page-cache attachment, while `EXR-ALLOC-27` owns the filesystem-wide allocation search/commit handshake. The finished system therefore needs inode-owned write methods that consume those services, not a filesystem-global writer or sync shell.

## Purpose

This unit turns the temporary `InodeIo::write_at` and size-changing inode seams in `inode.rs` into the real write-side owner for exFAT regular files.
It should own the buffered write contract, file growth and shrink behavior, and inode-local size mutation policy, while stopping before durable flush ordering and filesystem-wide sync semantics.

The owner boundary should remain narrow:

1. `ExfatInode` owns buffered write behavior and the user-visible file-size mutation contract.
2. `EXR-PGCACHE-26` remains the inode-local cache owner that this row consumes, not re-homes.
3. `EXR-ALLOC-27` remains the owner of free-space search, reservation intent, and commit.
4. `EXR-FILE-MAP-24` remains the owner of logical-to-physical translation used by the write path.
5. `EXR-READ-OPS-25` remains the owner of buffered read policy, which write-side zero-fill logic may reference only as a user-visible byte-stream invariant.
6. `EXR-SYNC-31` remains the downstream owner of flush ordering and durable writeback semantics.

## Why This Comes Now

The boundary is stable now because its prerequisites already exist as real architecture:

- `EXR-INODE-CORE-17` established `ExfatInode` as the VFS carrier.
- `EXR-FILE-MAP-24` established the inode-private mapping layer.
- `EXR-READ-OPS-25` and `EXR-PGCACHE-26` established the read-side and cache-side file snapshot owners that the write path must not reopen.
- `EXR-ALLOC-27` established committed allocation results as the only growth handoff.

That makes write-side file mutation the next coherent inode-owned step, rather than another staging manager.

## Owner And Integration Convergence

- Interfaces, traits, services, or higher-level functions this unit ultimately serves:
  - VFS `InodeIo::write_at`
  - VFS `Inode::resize`
  - later truncate or shrink behavior on `ExfatInode`
  - later dirty producers consumed by `EXR-SYNC-31`
- If the unit is internal-only, why that internal ownership is still stable in the finished system:
  - it is not internal-only; buffered write and size mutation are stable VFS-visible inode behaviors, and the inode carrier is the final owner that VFS expects to call.
- Known non-goals or nearby logic that must remain in the parent owner:
  - allocation search and reservation
  - directory-entry publication
  - filesystem-wide sync ordering
  - namespace mutation
  - filesystem-global cache management

Boundary consumption rules:

- `ExfatInode` may decide how much data to write, when to grow, and when to shrink, but it must consume committed allocation results rather than reopening allocation ownership.
- `EXR-PGCACHE-26` provides the inode-local cache attachment; write-side work may interact with that cache but must not promote it into a write manager.
- `EXR-FILE-MAP-24` provides the translation helpers used by the write path; this row must not duplicate mapping logic as a new owner boundary.
- `EXR-SYNC-31` owns persistence ordering. This row may produce dirty inode state and cache-visible mutations, but it must not define the final flush protocol.

## Dependency Contract

- Depends on:
  - `EXR-INODE-CORE-17`
  - `EXR-FILE-MAP-24`
  - `EXR-READ-OPS-25`
  - `EXR-PGCACHE-26`
  - `EXR-ALLOC-27`
  - the VFS `InodeIo` and `Inode` contracts
- Blocks:
  - regular-file buffered write on `ExfatInode`
  - size-changing inode mutation on `ExfatInode`
  - later dirty writeback and sync work in `EXR-SYNC-31`
- Can run in parallel with:
  - read-only inode work that stays outside write mutation
  - allocator work only if it does not widen into inode-owned write policy
  - sync planning work only if it does not redefine durable ordering here
- Recommended parallel wave:
  - Wave D after read-side ownership and allocation commit are both specified, with write-side mutation kept on `ExfatInode`
- Stable pre-existing interfaces used:
  - `ExfatInode`
  - `PageCache`
  - `PageCacheBackend`
  - `EXR-ALLOC-27` committed allocation results
  - inode-owned mapping helpers
- Prior sources or prior slices that materially shaped the split:
  - `WORKSPACE-ARCH-RESET/00_architect.md`
  - `EXR-INODE-CORE-17/00_architect.md`
  - `EXR-FILE-MAP-24/00_architect.md`
  - `EXR-READ-OPS-25/00_architect.md`
  - `EXR-PGCACHE-26/00_architect.md`
  - `EXR-PGCACHE-26/30_reviewer_report.md`
  - `EXR-ALLOC-27/00_architect.md`
  - `EXR-ALLOC-27/01_designer_core.md`
  - `EXR-ALLOC-27/02_designer_async.md`
  - `linux-exFAT-implementation-summary.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/allocator.rs`

## Recommended Work Slices

These are candidate slices for scheduler consideration, not the active global plan.

| Slice ID | Parent Unit Scope | Goal | Likely Write Set | Depends On | May Overlap With | Lane Class | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `WS-WRITE-30-WRITEAT` | `EXR-WRITE-30` | Implement buffered `ExfatInode::write_at` using inode-owned mapping, page-cache attachment, and commit-safe writeback of user data without redefining allocation or sync ownership. | `kernel/src/fs/fs_impls/exfat_refactor/inode.rs` | `EXR-INODE-CORE-17`, `EXR-FILE-MAP-24`, `EXR-READ-OPS-25`, `EXR-PGCACHE-26` | later write-side helper slices in `inode.rs` because the core write path and helper glue will likely share the same owner file region | creator | Keep this slice focused on buffered file writes only. It may route through inode-private mapping and cache helpers, but it must not absorb truncate, resize, or durable flush policy. |
| `WS-WRITE-30-GROWTH` | `EXR-WRITE-30` | Implement size growth, zero-fill, and committed-allocation consumption for writes that extend the file. | `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`, `kernel/src/fs/fs_impls/exfat_refactor/fs.rs` | `EXR-ALLOC-27`, `EXR-PGCACHE-26`, `EXR-FILE-MAP-24` | `WS-WRITE-30-WRITEAT` because growth and buffered write are expected to collide in `inode.rs` | creator | Treat committed allocation results as a fixed growth input. Do not perform allocation search or reservation here. |
| `WS-WRITE-30-TRUNCATE` | `EXR-WRITE-30` | Implement truncate and resize-side size mutation on `ExfatInode`, including freeing or preserving file state as needed before later sync ownership lands. | `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`, `kernel/src/fs/fs_impls/exfat_refactor/fs.rs` | `EXR-ALLOC-27`, `EXR-PGCACHE-26` | `WS-WRITE-30-WRITEAT` if helper reuse in `inode.rs` overlaps | creator | Keep truncate and resize inside inode ownership. If the slice starts defining flush ordering, it has crossed into `EXR-SYNC-31`. |

## exFAT Concepts Covered

- Buffered regular-file writes on `ExfatInode`.
- File growth using committed allocation results.
- Size mutation, zero-fill, truncate, and resize behavior.
- Inode-local page-cache interaction on write paths.
- Logical-to-physical mapping consumed by writes.
- User-visible write-side data-path behavior only; no allocator search, no directory publication, and no sync ownership.

## Boundary Rejections

- Splits considered but rejected:
  - a filesystem-global write manager separate from `ExfatInode`
  - a cache-backed write service that would absorb `EXR-PGCACHE-26`
  - folding allocation search or reservation into the write row
  - folding durable flush ordering into this unit
  - folding directory mutation or namespace publication into this unit
- Why those rejected splits would be packet convenience, not real architecture:
  - they would hide the stable inode carrier boundary that already owns the file snapshot and buffered mutation contract
  - they would blur write-side file behavior with allocation and sync ownership instead of letting the inode carrier consume those accepted services directly
  - they would recreate helper-first drift by turning a subordinate inode write seam into a false service boundary

## Target Files

- Existing files likely to change:
  - `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- New files expected:
  - none

## Code Budget

- Target creator work-slice size: `200-350` lines
- Expected number of creator slices: `2-3`
- Reason if any single slice might exceed 500 lines:
  - it should not. If a slice grows that large, buffered write, truncate, or sync policy has leaked into the inode carrier and the unit must be re-sliced instead of expanded.

## Exit Condition

Design work may start once `ExfatInode` is understood as the single write-side owner that consumes inode-local page-cache ownership, inode-private file mapping, committed allocation results, and later sync ownership only as a downstream dependency, while keeping allocation search, directory publication, and flush ordering out of the unit.

## Risks

- Buffered write can drift into a fake sync shell if durability ordering is specified too early.
- File growth can drift into allocator ownership if the design tries to search or reserve clusters instead of consuming committed results.
- `inode.rs` is the likely shared landing zone for the write path and growth helpers, so fake parallelism should be avoided if helper placement collides.
- Truncate and resize must remain explicit inode-owned size mutation behaviors; if they become a generic file-control API, the boundary has widened too far.
- `write_page_async()` remains a downstream seam owned by future sync work; if this row tries to close it now, it has crossed into `EXR-SYNC-31`.
