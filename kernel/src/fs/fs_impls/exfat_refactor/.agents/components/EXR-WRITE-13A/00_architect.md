<!-- SPDX-License-Identifier: MPL-2.0 -->

# Architect Handoff

## Metadata

- Component ID: `EXR-WRITE-13A`
- Title: Writable Regular-File Allocation Growth And Metadata Publication
- Status: `Architected`
- Author: `architect`
- Date: `2026-04-05`
- Task packet: [`EXR-WRITE-13A-ARCH-20260405-1220`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-WRITE-13A/20260405-1220-architect-packet.md)

## Purpose

This handoff covers the smallest useful exFAT write-side growth slice: make a writable regular file larger by allocating additional clusters, extending or publishing the cluster chain, and updating the inode and mount-visible size metadata that records the new allocation boundary.

It owns allocation growth only. It does not own buffered page-cache write policy, read-side zero-fill, namespace mutation, or truncate/shrink behavior.

## Why This Comes Now

The upstream read-only boundaries already exist. `EXR-MOUNT-09` owns mount bootstrap and shared filesystem state, `EXR-INODE-05B` owns the validated inode metadata shell, `EXR-READ-11A` owns accepted logical-to-physical placement for existing data, and `EXR-BITMAP-08A` already established the read-only allocation occupancy surface.

The remaining pressure is the physical-capacity side of regular-file writes: when a file must grow, who allocates the new clusters and publishes that larger allocation without also deciding how buffered bytes are copied or how truncation later frees space? That is this component.

Linux splits the same concerns. `file.c` handles `exfat_cont_expand()` and buffered write execution separately, while `inode.c` keeps mapping, allocation, and truncate/shrink behavior distinct. The refactor should keep that split instead of rebuilding a single monolithic write path.

## Dependency Contract

- Depends on:
  - `EXR-MOUNT-09`
  - `EXR-INODE-05B`
  - `EXR-READ-11A`
  - `EXR-CHAIN-03B`
  - the bitmap mutation and free-space boundary that will be supplied by `EXR-BITMAP-08B`
- Blocks:
  - `EXR-WRITE-13B`
  - `EXR-WRITE-13C`
  - `EXR-SYNC-13E`
  - any later writeback path that needs a finalized allocation-growth contract
- Can run in parallel with:
  - command-free planning or review work that only needs the already-accepted mount, inode, and placement contracts
- Recommended parallel wave:
  - keep the growth boundary isolated now;
  - let buffered writes wait for this contract;
  - keep shrink/truncate and sync work in their own later lanes instead of widening this slice.
- Stable pre-existing interfaces used:
  - mount-owned shared filesystem state from `EXR-MOUNT-09`
  - the validated inode metadata shell and read-view split from `EXR-INODE-05B`
  - the accepted logical-to-physical placement boundary from `EXR-READ-11A`
  - chain walking and chain-length helpers from `EXR-CHAIN-03B`
  - the read-only occupancy surface from `EXR-BITMAP-08A`, with mutation left for `EXR-BITMAP-08B`
  - `ExfatSuperBlock` geometry helpers for cluster sizing and byte-length translation
- Prior sources or prior slices that materially shaped the split:
  - `Microsoft-exFAT-spec.md` for allocation-length vs valid-data-length semantics and cluster-chain rules
  - Linux [`fs/exfat/file.c`](/home/halifuda/linux/fs/exfat/file.c) and [`fs/exfat/inode.c`](/home/halifuda/linux/fs/exfat/inode.c) for the separation between growth, buffered writes, and truncate/shrink
  - `EXR-MOUNT-09`, `EXR-INODE-05B`, `EXR-READ-11A`, `EXR-BITMAP-08A`, `EXR-PGCACHE-11B`, and `EXR-READ-11B` for the accepted upstream and downstream boundaries
  - `ASTERINAS_ARCHITECT_PRIORS.md` and `ASTERINAS_CODE_QUALITY_PRIORS.md` for the local rules on ownership boundaries, top-down decomposition, and keeping later live behavior separate from early value layers

## exFAT Concepts Covered

- Regular-file allocation growth by adding clusters to an existing file.
- Cluster-chain extension and publication of the new allocation boundary.
- Bitmap accounting for newly allocated clusters.
- Distinction between allocated length and buffered data completion.
- Preservation of the accepted mapping boundary instead of re-deriving placement here.
- Exclusion of page-cache write policy, namespace mutation, and truncate/shrink behavior.

## Target Files

- Existing files likely to change:
  - `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- New files expected:
  - none

## Code Budget

- Target new or heavily rewritten code size:
  - `200-300` lines
- Reason if the budget might exceed 500 lines:
  - It should stay within budget if the component remains a growth-only boundary. If it starts absorbing buffered byte-copy policy, read-side zero-fill, or truncate/shrink bookkeeping, the split is too wide and should be cut again.

## Exit Condition

Design work may start once the implementation plan expresses exactly one growth path that:

1. accepts validated mount state, inode metadata, and accepted placement facts,
2. allocates and links additional clusters through the bitmap and chain boundary,
3. publishes the larger allocation boundary and metadata consistently,
4. leaves buffered write-copy policy to `EXR-WRITE-13B`,
5. leaves truncate and shrink to `EXR-WRITE-13C`.

## Risks

- The growth path can accidentally absorb buffered write initialization if it starts deciding how newly allocated bytes are populated instead of only allocating space.
- Chain extension can drift back into read-side mapping if it starts re-deriving placement instead of consuming the accepted boundary.
- Bitmap publication order needs to stay explicit so allocation is not exposed before the bitmap and chain state agree.
- The component must not widen into namespace mutation or truncate/shrink policy just because both eventually touch the same inode metadata.
- Lock order between mount-owned state, bitmap mutation, and inode publication must be documented so growth does not create deadlocks later.
