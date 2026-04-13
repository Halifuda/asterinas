<!-- SPDX-License-Identifier: MPL-2.0 -->

# Architect Handoff

## Metadata

- Component ID: `EXR-SYNC-31`
- Title: `ExfatFs` sync and flush-ordering owner boundary
- Status: `Architected`
- Author: architect
- Date: `2026-04-13`
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-SYNC-31/20260413-1301-architect-packet.md`

## Functional Unit Definition

- Functional goal: implement the smallest coherent `ExfatFs` unit that owns filesystem-wide sync and flush ordering across already-published dirty state, while consuming inode-side and filesystem-side dirty producers without absorbing boot fallback, direct I/O, volume-label control, inode metadata policy, or admin/control ioctls.
- Final architectural owner: `ExfatFs`
- Owner class:
  - VFS filesystem carrier
- Expected landing form:
  - owner methods plus owner-private dirty-state helpers
- Boundary kind:
  - stable architectural boundary
- Why this boundary is architecturally real:
  - `sync()` is the filesystem-wide VFS contract, but the actual persistent ordering is broader than any one inode. `ExfatFs` is the only stable owner that can coordinate inode dirty state, filesystem-owned metadata, and the persistence sequence for already-published changes without turning sync into a catch-all control bucket.

## Purpose

This unit makes `ExfatFs` the owner of flush ordering, not the owner of every filesystem control path.
It should define how already-dirty exFAT state is drained to disk, in what sequence filesystem-wide and inode-local state is persisted, and how `FileSystem::sync()`, inode `sync_all()`, inode `sync_data()`, and the writeback side of `write_page_async()` relate to each other.

The owner boundary should stay narrow:

1. `ExfatFs` owns the filesystem-wide sync contract and the final persistence ordering.
2. `ExfatInode` remains the owner of buffered mutation, namespace mutation, and inode-visible dirty production.
3. `EXR-WRITE-30` remains the producer of buffered file dirtiness.
4. `EXR-NAMESPACE-29` remains the producer of namespace dirtiness.
5. `EXR-VOLLABEL-35` and `EXR-INODE-META-36` may produce filesystem-visible dirty state, but they do not become sync owners.
6. `EXR-BOOT-34` may contribute a dirty boot-flag output only after mount policy has already decided it belongs on the dirty-state side of the boundary.

## Why This Comes Now

The boundary is stable now because the sync seam is already explicit, even if it is still a placeholder:

- `fs.rs` already carries a placeholder `FileSystem::sync()`.
- `inode.rs` already inherits default `sync_all()` / `sync_data()` behavior.
- `inode.rs` already has a placeholder `write_page_async()`.
- `EXR-WRITE-30` and `EXR-NAMESPACE-29` already establish the main inode-side dirty producers that `ExfatFs` must consume later.

That means the next coherent step is not a new control service. It is the filesystem-owned ordering boundary that drains dirty state from those producers in a defined sequence.

## Owner And Integration Convergence

- Interfaces, traits, services, or higher-level functions this unit ultimately serves:
  - VFS `FileSystem::sync()`
  - VFS `Inode::sync_all()`
  - VFS `Inode::sync_data()`
  - `PageCacheBackend::write_page_async()` as the writeback seam for already-dirty data
  - the existing `ExfatFs` owner state that coordinates filesystem-wide persistence
- If the unit is internal-only, why that internal ownership is still stable in the finished system:
  - it is not internal-only; sync is a VFS-visible filesystem contract, and the filesystem owner is the correct stable root for ordering already-published dirty state.
- Known non-goals or nearby logic that must remain in the parent owner:
  - direct I/O contract
  - name conversion
  - boot fallback decision-making
  - volume-label mutation and user-facing label control
  - FAT-attribute ioctls
  - trim/discard
  - forced shutdown
  - allocation search and reservation
  - namespace mutation policy
  - inode metadata policy as a user-control surface

Boundary consumption rules:

- `ExfatFs` may decide the order in which already-dirty filesystem state is flushed, but it must not become the owner of how that state was produced.
- `sync_all()` and `sync_data()` should funnel into the same filesystem-owned persistence root unless a later creator slice proves a real semantic split is needed.
- `write_page_async()` should be treated as the writeback edge for already-owned dirty pages, not as a new page-cache manager.
- `ExfatFs` may host owner-private dirty-state bookkeeping helpers if they are necessary to remember what must be flushed next, but those helpers must stay internal to the filesystem owner and must not become a separate public writeback manager.

## Dependency Contract

- Depends on:
  - `EXR-FS-CORE-16`
  - `EXR-WRITE-30`
  - `EXR-NAMESPACE-29`
  - the VFS `FileSystem`, `Inode`, and `PageCacheBackend` contracts
- Blocks:
  - final filesystem sync behavior on `ExfatFs`
  - inode sync hooks that should delegate to filesystem-owned ordering
  - later dirty-state writeback work that must consume this boundary
- Can run in parallel with:
  - inode write-path work that only produces dirty state
  - namespace work that only produces dirty state
  - later admin/control rows, provided they do not redefine sync ownership
- Recommended parallel wave:
  - Wave E after buffered write and namespace ownership have established the dirty producers, with sync kept on `ExfatFs`
- Stable pre-existing interfaces used:
  - `ExfatFs`
  - `FileSystem`
  - `Inode`
  - `PageCacheBackend`
  - current placeholder `sync()` and `write_page_async()` seams
- Prior sources or prior slices that materially shaped the split:
  - `EXR-FS-CORE-16/00_architect.md`
  - `EXR-WRITE-30/00_architect.md`
  - `EXR-NAMESPACE-29/00_architect.md`
  - `WORKSPACE-ARCH-POST28/00_architect.md`
  - `ASTERINAS_ARCHITECT_PRIORS.md`
  - `linux-exFAT-implementation-summary.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
  - `kernel/src/fs/vfs/fs_apis/file_system.rs`
  - `kernel/src/fs/vfs/fs_apis/inode.rs`

## Recommended Work Slices

These are candidate creator slices for scheduler consideration, not the active global plan.

| Slice ID | Parent Unit Scope | Goal | Likely Write Set | Depends On | May Overlap With | Lane Class | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `WS-SYNC-31-OWNER` | `EXR-SYNC-31` | Implement the filesystem-owned sync entry point and the owner-private dirty-state handoff needed to order existing inode and filesystem dirty producers. | `kernel/src/fs/fs_impls/exfat_refactor/fs.rs`, `kernel/src/fs/fs_impls/exfat_refactor/inode.rs` | `EXR-FS-CORE-16`, `EXR-WRITE-30`, `EXR-NAMESPACE-29` | later inode sync slices in `inode.rs` because the sync delegation points and the inode-owned dirty producers will likely share the same owner file region | creator | Keep this slice focused on ordering and delegation. It may introduce owner-private tracking helpers if needed, but it must not absorb control-path policy, direct I/O, or admin ioctls. |
| `WS-SYNC-31-WRITEBACK` | `EXR-SYNC-31` | Implement the writeback side of `write_page_async()` so dirty page-cache state reaches the filesystem-owned persistence sequence. | `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`, `kernel/src/fs/fs_impls/exfat_refactor/fs.rs` | `EXR-WRITE-30`, `EXR-FS-CORE-16` | `WS-SYNC-31-OWNER` because writeback plumbing and sync delegation will likely share the same owner-private seam | creator | Treat page writeback as a downstream persistence seam. Do not use it to reopen buffered-write ownership or to invent a new cache manager. |
| `WS-SYNC-31-INODE-HOOKS` | `EXR-SYNC-31` | Replace the default inode `sync_all()` / `sync_data()` behavior with explicit delegation into the filesystem-owned sync boundary. | `kernel/src/fs/fs_impls/exfat_refactor/inode.rs` | `EXR-FS-CORE-16`, `EXR-WRITE-30`, `EXR-NAMESPACE-29` | `WS-SYNC-31-OWNER` because the hooks will likely be thin wrappers over the same owner-private flush path | creator | Keep the inode hooks thin. If they start owning dirty-state collection themselves, the boundary has drifted away from `ExfatFs`. |

## exFAT Concepts Covered

- Filesystem-wide sync ordering.
- Inode sync hooks that delegate to the filesystem owner.
- Page-cache writeback as a downstream persistence seam.
- Dirty-state consumption from buffered writes and namespace mutation.
- Filesystem-owned persistence sequencing for already-published changes.

## Boundary Rejections

- Splits considered but rejected:
  - a separate writeback manager distinct from `ExfatFs`
  - an inode-local sync owner separate from the filesystem owner
  - folding boot fallback into sync
  - folding volume-label user control into sync
  - folding direct I/O into sync
  - folding trim/discard or forced shutdown into sync
  - folding FAT-attribute ioctls into sync
- Why those rejected splits would be packet convenience, not real architecture:
  - `sync` is a filesystem-wide coordination boundary, so the stable owner is the filesystem object itself, not a helper service
  - the row must consume already-dirty producers rather than become the place where control-path decisions are made
  - control surfaces such as boot fallback, label mutation, and admin ioctls are user-visible policy paths, not persistence-ordering paths

## Target Files

- Existing files likely to change:
  - `kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- New files expected:
  - none

## Code Budget

- Target creator work-slice size: `120-240` lines
- Expected number of creator slices: `1-2`
- Reason if any single slice might exceed 500 lines:
  - it should not. If the work starts pulling in boot policy, label control, direct I/O, or administrative ioctl behavior, the boundary is wrong and the unit must be re-sliced instead of expanded.

## Exit Condition

Design work may start once `ExfatFs` is understood as the single filesystem-wide flush-ordering owner that consumes already-published dirty state from inode and filesystem producers, while keeping control-path policy and unrelated admin surfaces out of the unit.

## Risks

- `sync()` can drift into a catch-all if it starts absorbing policy decisions instead of ordering already-dirty state.
- `inode.rs` is a likely collision point because the inode sync hooks and the writeback seam both live there, so creator waves must stay intentionally thin.
- The boundary must not silently promote dirty-state tracking into a separate public writeback manager.
- Control-path features such as boot fallback, label mutation, or admin ioctls should remain outside this row even if they also end up persisting on disk.
