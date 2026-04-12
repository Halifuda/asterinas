<!-- SPDX-License-Identifier: MPL-2.0 -->

# Architect Handoff

## Metadata

- Component ID: `EXR-ALLOC-27`
- Title: `ExfatFs` Cluster Allocation Service Boundary
- Status: `Architected`
- Author: architect
- Date: 2026-04-12
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-ALLOC-27/20260412-1148-architect-packet.md`

## Functional Unit Definition

- Functional goal: provide the smallest stable `ExfatFs`-owned allocation service that can search free space, reserve cluster runs, and coordinate bitmap plus FAT mutation without absorbing directory-entry writes, inode growth policy, truncate semantics, or filesystem-wide sync ordering.
- Final architectural owner: `ExfatFs`
- Owner class: structure owner
- Expected landing form: owner-internal service (`Allocator`) plus owner methods
- Boundary kind: stable architectural boundary
- Why this boundary is architecturally real: allocation is filesystem-wide mutable state with one shared lifecycle. It consumes already-owned bitmap facts and FAT decode/geometry helpers, then publishes a reservation or commit result that later namespace, write, and sync owners consume. That makes it a stable `ExfatFs` concern, but only as an internal service under the filesystem owner.

## Purpose

Describe the smallest functionally coherent unit that can choose, reserve, and commit free clusters under `ExfatFs` without becoming a standalone free-space manager.

This unit should own the search-and-reserve policy for free clusters and the mutation handshake that keeps the allocation bitmap and FAT coherent. It should not own directory-entry publication, file-size policy, or persistence ordering.

## Why This Comes Now

This boundary is dependency-safe now because `EXR-BITMAP-21` already owns the read-only allocation bitmap snapshot, while `EXR-FATVAL-03A` and `EXR-IO-02` already own the FAT decoding and geometry helpers the allocator must consult.

That means `EXR-ALLOC-27` can be the first write-side owner without reopening bitmap ownership, FAT value ownership, or low-level I/O ownership. The allocator can therefore sit directly under `ExfatFs` and serve later directory, namespace, write, and sync rows without duplicating their policy.

## Owner And Integration Convergence

- Interfaces, traits, services, or higher-level functions this unit ultimately serves: later directory-entry writes, namespace mutations, inode growth, buffered writes, and filesystem-wide sync on `ExfatFs`.
- If the unit is internal-only, why that internal ownership is still stable in the finished system: allocation search and reservation are shared filesystem runtime state. They must see the same bitmap snapshot and FAT helpers as every later mutator, so they belong to the same filesystem owner rather than a detached helper layer.
- Known non-goals or nearby logic that must remain in the parent owner: directory-entry mutation, file-size growth policy, truncate semantics, writeback ordering, page-cache integration, and sync ordering.

## Dependency Contract

- Depends on: `EXR-BITMAP-21`, `EXR-FATVAL-03A`, `EXR-IO-02`, and the normalized `ExfatSuperBlock` geometry from the boot foundation.
- Blocks: `EXR-DENTRY-WRITE-28`, `EXR-NAMESPACE-29`, `EXR-WRITE-30`, and `EXR-SYNC-31`.
- Can run in parallel with: later consumer planning for directory writes, namespace mutations, and writeback, provided those rows do not redefine allocation policy.
- Recommended parallel wave: first write-side allocator wave, after the read-side owner state has settled.
- Stable pre-existing interfaces used: `AllocationBitmap` read-only occupancy and accounting queries, `read_next_fat_value`, `FatValue`, `read_metadata_bytes`, and `ExfatSuperBlock`.
- Prior sources or prior slices that materially shaped the split: `WORKSPACE-ARCH-RESET/00_architect.md`, `EXR-BITMAP-21/00_architect.md`, `EXR-BITMAP-21/01_designer_core.md`, `EXR-FATVAL-03A/00_architect.md`, `EXR-CHAIN-03B/00_architect.md`, `EXR-IO-02/00_architect.md`, `linux-exFAT-implementation-summary.md`, and the current `fs.rs` / `bitmap.rs` owner shape.

## Recommended Work Slices

These are candidate creator slices for scheduler consideration, not the active global plan.

| Slice ID | Parent Unit Scope | Goal | Likely Write Set | Depends On | May Overlap With | Lane Class | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `WS-ALLOC-27-A` | `EXR-ALLOC-27` | Define the `Allocator` owner-internal state and the free-space search/reservation intent that selects candidate cluster runs from the published bitmap snapshot. | `kernel/src/fs/fs_impls/exfat_refactor/allocator.rs`, `kernel/src/fs/fs_impls/exfat_refactor/fs.rs` | `EXR-BITMAP-21`, `EXR-FATVAL-03A`, `EXR-IO-02` | none if the slice stays allocator-only | creator | Keep this slice on search and reservation intent only. It may choose contiguous or fragmented candidate runs, but it must not publish directory entries, change file size, or define sync ordering. |
| `WS-ALLOC-27-B` | `EXR-ALLOC-27` | Add the bitmap/FAT mutation handshake that commits a reserved allocation and exposes a stable result for later consumers. | `kernel/src/fs/fs_impls/exfat_refactor/allocator.rs`, `kernel/src/fs/fs_impls/exfat_refactor/fs.rs` | `WS-ALLOC-27-A` | none if writeback and namespace stay out of scope | creator | This slice is still allocator-owned. It may flip allocation bits, initialize or extend FAT links, and record the allocation result, but it must not touch directory-entry writes, inode growth policy, truncate ordering, or filesystem sync. |

## exFAT Concepts Covered

- Free-space search over the filesystem-wide allocation bitmap.
- Reservation of a cluster run for later filesystem writes.
- Coordinating allocation bitmap updates with FAT mutation.
- Selecting contiguous runs when available and falling back to FAT-linked allocation when fragmentation requires it.
- Returning a stable allocation result that later namespace and write owners can consume.
- Allocation state lifecycle as `ExfatFs` runtime state.

## Boundary Rejections

- Splitting allocation into a standalone free-space manager was rejected. That would be packet convenience, not a stable owner boundary.
- Folding directory-entry publication into this unit was rejected. That belongs to later directory-write and namespace owners.
- Folding inode growth, `valid_size`, or truncate policy into this unit was rejected. Those are later file-data ownership concerns.
- Folding filesystem-wide sync ordering into this unit was rejected. That belongs to `EXR-SYNC-31`.
- Reopening bitmap ownership or FAT value ownership inside this unit was rejected. This component must consume those accepted owners, not replace them.

## Target Files

- Existing files likely to change: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- New files expected: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/allocator.rs`

## Code Budget

- Target creator work-slice size: `220-320` lines
- Expected number of creator slices: `2`
- Reason if any single slice might exceed 500 lines: it should not. If it does, the slice has probably absorbed directory-entry writes, inode growth policy, or sync ordering, which means the boundary has drifted.

## Exit Condition

Design work may start once this component is understood as an `ExfatFs`-internal `Allocator` service that can search free space, reserve cluster runs, and coordinate bitmap plus FAT mutation through owner methods, while leaving directory-entry writes, inode growth, truncate semantics, and sync ordering to later owners.

## Risks

- The allocator can accidentally become a file-growth helper if it starts choosing file size policy instead of just cluster reservation.
- The allocator can accidentally become a directory-write helper if it starts publishing dentry changes or namespace outcomes.
- The bitmap/FAT mutation handshake needs explicit ownership discipline later so the allocator does not become a hidden sync layer.
- Lock ordering between the bitmap snapshot and FAT updates must be recorded during design; this architect pass should not guess it.
- Later write-side work must not reuse this artifact as justification for a standalone allocator crate or inode-local allocator surface.
