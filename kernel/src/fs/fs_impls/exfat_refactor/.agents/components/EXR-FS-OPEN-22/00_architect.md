<!-- SPDX-License-Identifier: MPL-2.0 -->

# Architect Handoff

## Metadata

- Component ID: `EXR-FS-OPEN-22`
- Title: `ExfatFs` mount/open sequencing and root publication
- Status: `Architected`
- Author: architect
- Date: `2026-04-10`
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-FS-OPEN-22/20260410-1245-architect-packet.md`

## Functional Unit Definition

- Functional goal: implement the smallest coherent `ExfatFs::open(...)` sequencing unit that turns trusted boot facts into a mounted filesystem with a published root inode and the root-directory system-entry handoff completed under `ExfatFs`.
- Final architectural owner: `ExfatFs`
- Owner class:
  - structure owner
- Expected landing form:
  - owner methods plus sequencing invariants
- Boundary kind:
  - stable architectural boundary
- Why this boundary is architecturally real: VFS mount/open needs a single filesystem owner that sequences boot facts, root inode publication, and root-directory system-entry discovery without inserting a separate mount object or root-scanner owner. That sequencing is a durable `ExfatFs` responsibility because it coordinates the accepted internal owners, publishes the root handle, and preserves the filesystem-wide trust boundary between validated on-disk facts and VFS-visible handles.

## Purpose

This unit is the mount/open bridge that makes `ExfatFs` usable as a live filesystem instance rather than only as a trait carrier with a temporary `root_inode()` seam.
It absorbs the current root-publication seam by making root construction and publication part of `ExfatFs` owner behavior, while still leaving later directory mutation, allocator mutation policy, and sync ordering outside this unit.

## Why This Comes Now

`EXR-FS-CORE-16` already established `ExfatFs` as the filesystem owner, `EXR-INODE-CORE-17` already established the inode carrier, `EXR-INODE-CACHE-18` already established the opened-inode table and the root-as-distinguished-slot rule, and `EXR-DIR-ENGINE-19` / `EXR-UPCASE-20` provide the internal services needed for mount-time discovery.
The missing coherent step is the owner method that ties those pieces together into a ready root, so this unit can land now without depending on later read/write or namespace work.

## Owner And Integration Convergence

- Interfaces, traits, services, or higher-level functions this unit ultimately serves: VFS `FileSystem::root_inode()`, future filesystem open/mount entry points, root publication, and root-directory system-entry discovery under `ExfatFs`.
- If the unit is internal-only, why that internal ownership is still stable in the finished system: it is not internal-only. The finished system still needs `ExfatFs` to own the mount/open choreography because VFS talks to the filesystem owner directly and the root inode must be published from that owner, not from a separate staging object.
- Known non-goals or nearby logic that must remain in the parent owner: later directory operations, namespace mutation, allocator mutation policy, buffered read/write, page-cache ownership, and filesystem-wide sync ordering stay out of this boundary.

The unit consumes accepted internal owners as follows:

- `ExfatFs`: final runtime owner and coordination root for the sequencing.
- `ExfatInode`: the root inode carrier and later non-root inode carrier, published by `ExfatFs`.
- opened-inode cache: the filesystem-wide identity table used to publish and reuse the root slot and later inode handles.
- `DirectoryEngine`: the read-only directory stream used during mount-time root-directory system-entry discovery.
- `UpcaseTable`: the name-folding and hash service installed before directory-name work can be trusted.
- `AllocationBitmap`: the filesystem-wide occupancy state owned by `ExfatFs`; this unit should consume its read-only mount-time presence once the bitmap owner lands, but it must not define bitmap mutation policy.

Root-publication handoff:

- The temporary `root_inode()` seam in `EXR-FS-CORE-16` is absorbed by this unit.
- `EXR-FS-OPEN-22` is the named owner of the transition from boot facts to a ready root inode.
- The root handle should be published through `ExfatFs`-owned state, not through a fake mount shell or a synthetic root carrier.
- The ordinary opened-inode keyspace remains for non-root entries; the root remains a distinguished owner slot.

## Dependency Contract

- Depends on:
  - `EXR-FS-CORE-16`
  - `EXR-INODE-CORE-17`
  - `EXR-INODE-CACHE-18`
  - `EXR-DIR-ENGINE-19`
  - `EXR-UPCASE-20`
  - `EXR-BITMAP-21`
  - `EXR-BOOT-01`
  - `EXR-SBGEOM-15`
  - the VFS `FileSystem` contract
- Blocks:
  - the real `root_inode()` implementation path
  - VFS mount/open readiness for exFAT
  - later directory operations that assume a published root inode
- Can run in parallel with:
  - `EXR-DIR-OPS-23` architect work only after root publication is fully named
  - implementation work in sibling lanes that touch disjoint files, provided no one reopens mount sequencing as a separate owner
- Recommended parallel wave:
  - Wave B: finish mount-critical `ExfatFs` sequencing after the trait-carrier and cache owners are in place, then let later inode directory work consume the published root
- Stable pre-existing interfaces used:
  - `FileSystem`
  - `Inode`
  - `ExfatFs`
  - `ExfatInode`
  - `DirectoryEngine`
  - `UpcaseTable`
  - `OpenedInodeState`
  - the validated boot and superblock foundations
- Prior sources or prior slices that materially shaped the split:
  - `WORKSPACE-ARCH-RESET/00_architect.md`
  - `EXR-FS-CORE-16/00_architect.md`
  - `EXR-INODE-CORE-17/00_architect.md`
  - `EXR-INODE-CACHE-18/00_architect.md`
  - `EXR-DIR-ENGINE-19/00_architect.md`
  - `EXR-UPCASE-20/00_architect.md`
  - `COMPONENT_INDEX.md`
  - `kernel/src/fs/vfs/fs_apis/file_system.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/directory.rs`

## Recommended Work Slices

These are candidate creator slices for scheduler consideration, not the active global plan.

| Slice ID | Parent Unit Scope | Goal | Likely Write Set | Depends On | May Overlap With | Lane Class | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `WS-FS-OPEN-22-ROOT` | `EXR-FS-OPEN-22` | Replace the temporary `root_inode()` seam with real `ExfatFs` root publication backed by the opened-inode owner state. | `kernel/src/fs/fs_impls/exfat_refactor/fs.rs` | `EXR-FS-CORE-16`, `EXR-INODE-CORE-17`, `EXR-INODE-CACHE-18` | `EXR-DIR-ENGINE-19` only at the architect/scheduler level; not file-parallel if the same `fs.rs` region is still moving | creator | Keep the slice root-specific. Do not add namespace mutation, cache policy generalization, or new owner shells. |
| `WS-FS-OPEN-22-MOUNT` | `EXR-FS-OPEN-22` | Sequence boot facts, upcase installation, bitmap availability, and root-directory discovery into the ready-root path. | `kernel/src/fs/fs_impls/exfat_refactor/fs.rs` | `EXR-DIR-ENGINE-19`, `EXR-UPCASE-20`, `EXR-BITMAP-21`, `EXR-BOOT-01`, `EXR-SBGEOM-15` | `WS-FS-OPEN-22-ROOT` if both touch the same `fs.rs` sequencing region | creator | Keep the slice mount/open only. It may call internal owner services, but it must not invent a separate mount object or directory-scanner owner. |

## exFAT Concepts Covered

- Mount/open sequencing.
- Root inode publication.
- Root-directory system-entry discovery.
- Boot facts and normalized superblock reuse.
- Opened-inode reuse with a distinguished root slot.
- Upcase-table and allocation-bitmap readiness as prerequisites for open.
- VFS `FileSystem::root_inode()` convergence on a real owner method.

## Boundary Rejections

- Splits considered but rejected:
  - a standalone mount object between VFS and `ExfatFs`
  - a separate root scanner or system-root owner
  - a synthetic root carrier that pretends to be the root handoff
  - folding later directory mutation, allocator mutation, or sync ordering into open sequencing
- Why those rejected splits would be packet convenience, not real architecture:
  - they would recreate the old staging-layer drift instead of converging on the stable `ExfatFs` owner
  - they would hide root publication behind a helper owner instead of naming the actual integration boundary
  - they would blur the line between mount-time discovery and later filesystem mutation policies

## Target Files

- Existing files likely to change:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- New files expected:
  - none

## Code Budget

- Target creator work-slice size: `220-320` lines
- Expected number of creator slices: `2`
- Reason if any single slice might exceed 500 lines: it should not. If it does, mount sequencing has leaked into a broader owner redesign and the unit should be re-sliced rather than expanded.

## Exit Condition

Design work may start once `ExfatFs` has a named owner-method path for mount/open sequencing, the root-publication handoff is explicit, and the only remaining filesystem-root seam is a real `ExfatFs` implementation detail rather than a temporary placeholder.

## Risks

- The root special case can drift back into an anonymous staging helper if the publication handoff is not named clearly.
- `AllocationBitmap` must remain read-only at this stage; otherwise open sequencing will swallow allocator policy too early.
- `DirectoryEngine` should stay a read-only discovery service here, not a general directory mutation surface.
- The `root_inode()` seam must not survive as a permanent placeholder after this unit lands.
