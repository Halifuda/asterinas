<!-- SPDX-License-Identifier: MPL-2.0 -->

# Architect Handoff

## Metadata

- Component ID: `EXR-FILE-MAP-24`
- Title: `ExfatInode` read-path logical-to-physical file mapping
- Status: `Architected`
- Author: architect
- Date: `2026-04-10`
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-FILE-MAP-24/20260410-1620-architect-packet.md`

## Functional Unit Definition

- Functional goal: implement the smallest coherent `ExfatInode` unit that translates regular-file logical offsets into physical on-disk positions for later read-side consumers.
- Final architectural owner: `ExfatInode`
- Owner class:
  - VFS trait carrier
- Expected landing form:
  - owner-private helpers
- Boundary kind:
  - stable architectural boundary
- Why this boundary is architecturally real: logical-to-physical mapping is per-inode read-path behavior derived from inode-owned chain facts, file size, valid size, and cluster geometry. It is not a filesystem-global mapping service and it is not raw data I/O. The stable owner is therefore the inode carrier that already holds the file’s chain identity and metadata snapshot, with `ExfatChain` consumed as a validated traversal boundary rather than promoted into a new user-facing service.

## Purpose

This unit is the read-path address-translation layer that later buffered read code will call before issuing any actual data I/O.
It should consume the inode’s copied chain facts and validated chain-walking support to locate the cluster and in-cluster offset for a requested logical file position, while stopping before copying data, zero-filling, page-cache interaction, growth, or write-side mutation.

## Why This Comes Now

`EXR-INODE-CORE-17` already established `ExfatInode` as the stable owner of file metadata and copied chain facts, and `EXR-CHAIN-03B` already established the read-only traversal boundary needed to walk contiguous and FAT-backed cluster chains.
That makes file mapping the next smallest coherent read-side step: it can land now without reopening directory traversal, mount/open sequencing, or the later buffered-read contract in `EXR-READ-OPS-25`.

## Owner And Integration Convergence

- Interfaces, traits, services, or higher-level functions this unit ultimately serves:
  - future `EXR-READ-OPS-25`
  - later page-cache read integration in `EXR-PGCACHE-26`
  - the `ExfatInode` read-side helper surface under `inode.rs`
- If the unit is internal-only, why that internal ownership is still stable in the finished system:
  - it is internal to `ExfatInode`, but it is stable because every later read-side file path for that inode must translate logical offsets through the same inode-owned chain facts before any data transfer begins.
- Known non-goals or nearby logic that must remain in the parent owner:
  - actual block or page data I/O
  - buffered read semantics and zero-fill policy
  - page-cache ownership and caching policy
  - write-side growth, truncate, and allocation mutation
  - sync ordering and dirty-state traversal

Boundary consumption rules:

- `ExfatChain` remains the accepted read-only traversal boundary. This unit may reconstruct or consume an `ExfatChain` from inode-owned facts, but it must not turn chain walking into a separate mapping owner.
- The inode’s copied state such as start cluster, cluster count, traversal mode, file size, valid size, and allocated size remain the trusted mapping inputs under `ExfatInode`.
- Logical EOF, valid-size zero-fill, and short-read policy remain for `EXR-READ-OPS-25`; this unit should stop at address translation and span derivation.

## Dependency Contract

- Depends on:
  - `EXR-INODE-CORE-17`
  - `EXR-CHAIN-03B`
  - the VFS `Inode` carrier context for regular-file owners
- Blocks:
  - `EXR-READ-OPS-25`
  - later regular-file page-cache read integration in `EXR-PGCACHE-26`
  - any read-side path that needs stable offset-to-cluster translation
- Can run in parallel with:
  - `EXR-DIR-OPS-23` architect/design work, because both are read-only `ExfatInode` behaviors with distinct functional targets
  - sibling planning work that does not widen `inode.rs` into data I/O ownership
- Recommended parallel wave:
  - Wave C, with regular-file mapping kept separate from directory behavior and from the later buffered-read unit
- Stable pre-existing interfaces used:
  - `ExfatInode`
  - `ExfatChain`
  - `ChainMode`
  - `ClusterId`
  - inode-owned size and allocation facts
  - `ExfatSuperBlock` cluster geometry through the owning filesystem
- Prior sources or prior slices that materially shaped the split:
  - `WORKSPACE-ARCH-RESET/00_architect.md`
  - `EXR-INODE-CORE-17/00_architect.md`
  - `EXR-CHAIN-03B/00_architect.md`
  - `EXR-DIR-OPS-23/00_architect.md`
  - `EXR-DIR-OPS-23/01_designer_core.md`
  - `COMPONENT_INDEX.md`
  - `kernel/src/fs/vfs/fs_apis/inode.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/fat.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/fs.rs`

## Recommended Work Slices

These are candidate slices for scheduler consideration, not the globally active plan.

| Slice ID | Parent Unit Scope | Goal | Likely Write Set | Depends On | May Overlap With | Lane Class | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `WS-FILE-MAP-24-CHAIN` | `EXR-FILE-MAP-24` | Add owner-private helpers in `ExfatInode` that reconstruct or consume the inode’s chain facts and translate a logical byte offset to the corresponding chain position and in-cluster offset. | `kernel/src/fs/fs_impls/exfat_refactor/inode.rs` | `EXR-INODE-CORE-17`, `EXR-CHAIN-03B` | `WS-FILE-MAP-24-SPAN` if both land in the same helper region in `inode.rs` | creator | Keep the slice read-side only. Do not perform data reads and do not introduce a separate mapping owner. |
| `WS-FILE-MAP-24-SPAN` | `EXR-FILE-MAP-24` | Add owner-private range helpers that report the physically mappable span for a logical request using file-size, valid-size, and allocated-size facts, while leaving EOF and zero-fill policy to later read owners. | `kernel/src/fs/fs_impls/exfat_refactor/inode.rs` | `WS-FILE-MAP-24-CHAIN` | `WS-FILE-MAP-24-CHAIN` because both are expected to share `inode.rs` helper landings | creator | This slice must stop at address translation metadata. It must not start copying bytes, zero-filling, or integrating page cache. |

## exFAT Concepts Covered

- Regular-file cluster-chain translation.
- Contiguous versus FAT-backed file chains.
- Logical byte offset to cluster position translation.
- In-cluster byte offsets and physically mappable span derivation.
- File size, valid size, and allocated size as read-path bounds facts.
- Read-only mapping only; no allocation growth or write-side mutation.

## Boundary Rejections

- Splits considered but rejected:
  - a standalone mapping service separate from `ExfatInode`
  - folding actual data reads into the mapping layer
  - folding page-cache ownership into the mapping layer
  - folding growth, truncate, allocation, or dirty-state behavior into read-path mapping
- Why those rejected splits would be packet convenience, not real architecture:
  - they would hide the stable inode owner boundary that already owns the necessary chain and size facts
  - they would blur address translation with the later buffered-read and cache owners
  - they would recreate helper-first drift by turning a subordinate inode helper into a false service boundary

## Target Files

- Existing files likely to change:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- New files expected:
  - none

## Code Budget

- Target creator work-slice size: `160-240` lines
- Expected number of creator slices: `2`
- Reason if any single slice might exceed 500 lines: it should not. If it does, buffered-read policy, page-cache behavior, or write-side growth has likely leaked into this unit and the work should be re-sliced instead of expanded.

## Exit Condition

Design work may start once `EXR-FILE-MAP-24` is understood as exactly the `ExfatInode`-private read-path mapping layer: logical offset to chain position and physically mappable span using inode-owned chain and size facts, with no actual data I/O, zero-fill policy, cache behavior, growth, or allocation mutation folded in.

## Risks

- The mapping helpers can drift into a fake read service if byte-copying or read-loop ownership is added too early.
- `valid_size` versus logical `size` can be misused if this unit starts deciding zero-fill or EOF semantics instead of leaving that to `EXR-READ-OPS-25`.
- `inode.rs` is the likely shared landing zone for both helper slices, so fake parallelism should be avoided if the helper region collides.
- Empty-chain or zero-length-file handling must remain explicit so later read code can distinguish “no mapped clusters” from “I/O failure” without changing the owner boundary.
