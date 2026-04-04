<!-- SPDX-License-Identifier: MPL-2.0 -->

# Architect Handoff

## Metadata

- Component ID: `EXR-MOUNT-09`
- Title: Mount Bootstrap And Shared Filesystem State
- Status: `Architected`
- Author: `architect`
- Date: `2026-04-04`
- Task packet: [`EXR-MOUNT-09-ARCH-20260404-1501`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-MOUNT-09/20260404-1501-architect-packet.md)

## Purpose

This handoff covers the smallest useful mount component for the refactor: the filesystem open/bootstrap sequence and the shared filesystem state object that it seeds.

The component owns the mount-time ordering that turns validated superblock facts and validated root-discovery facts into a live filesystem instance with root-seeded shared state. That includes opening the root path in the correct order, loading the discovered upcase and allocation-bitmap tables into mount-owned state, and anchoring the reserved root inode in the shared filesystem object.

It does not own inode metadata shaping, page-cache backend behavior, directory lookup policy, free-space search, allocation mutation, or namespace mutation.

## Why This Comes Now

This split is safe now because the prerequisite boundaries already exist:

- `EXR-BOOT-01` and `EXR-IO-02` provide validated boot-sector parsing and read-side metadata I/O.
- `EXR-CHAIN-03B` provides trusted root-chain traversal facts.
- `EXR-INODE-05B` provides the read-only inode metadata shell, including the synthetic root special case.
- `EXR-SYSROOT-06` provides root-directory discovery facts for the upcase and bitmap entries.
- `EXR-UPCASE-07B` provides the canonical table-backed case-fold and hash service.
- `EXR-BITMAP-08A` provides the loaded read-only allocation bitmap surface.

The remaining pressure is no longer "how do we discover the root tables?" but "who owns the mount-wide object that consumes those already-discovered facts and wires the live filesystem together?" That is the mount component.

Linux `super.c` shows the same pressure explicitly: mount bootstrap validates the volume, constructs the root inode, loads the table-backed helpers, and then publishes the filesystem. `namei.c` shows the lookup policy is a separate concern and must not be pulled into mount.

## Dependency Contract

- Depends on:
  - `EXR-BOOT-01`
  - `EXR-IO-02`
  - `EXR-CHAIN-03B`
  - `EXR-INODE-05B`
  - `EXR-SYSROOT-06`
  - `EXR-UPCASE-07B`
  - `EXR-BITMAP-08A`
- Blocks:
  - `EXR-DIR-10`
  - `EXR-READ-11A`
  - `EXR-PGCACHE-11B`
  - `EXR-READ-11B`
  - `EXR-CREATE-12A`
  - `EXR-CREATE-12B`
  - `EXR-WRITE-13A`
  - `EXR-WRITE-13B`
  - `EXR-WRITE-13C`
  - `EXR-RENAME-13D`
  - `EXR-SYNC-13E`
- Can run in parallel with:
  - command-free design prep for `EXR-DIR-10` and `EXR-READ-11A` once this mount contract is accepted
  - implementation work for later data-path components that only need the finalized shared-state API shape, not mount bootstrap details
- Recommended parallel wave:
  - finish the mount shared-state contract first;
  - then let directory lookup planning and read-path planning proceed in parallel while write-side components remain blocked behind mount and bitmap evolution.
- Stable pre-existing interfaces used:
  - `ExfatSuperBlock` geometry and root-cluster translation helpers
  - `scan_root_system_entries()` from `sysroot.rs`
  - `ExfatUpcaseTable::load()` from `upcase_table.rs`
  - `ExfatAllocationBitmap::load()` from `bitmap.rs`
  - `ExfatInodeMeta::new_root()` from `inode.rs`
  - the root-chain construction and validated read-side metadata flow already established by earlier components
- Prior sources or prior slices that materially shaped the split:
  - `Microsoft-exFAT-spec.md` for root-directory system-entry semantics, volume-level bitmap ownership, and the fact that upcase and bitmap are mount-visible system resources.
  - `linux-exFAT-implementation-summary.md` plus Linux `super.c` and `namei.c` for the mount/bootstrap ordering and the separation between mount policy and lookup policy.
  - `EXR-SYSROOT-06` for root-discovery facts that must be consumed, not rediscovered.
  - `EXR-INODE-05B` for the synthetic root metadata shell that mount should seed, not recreate.
  - `EXR-UPCASE-07B` and `EXR-BITMAP-08A` for the consumer-side loaded-table surfaces that mount should own at the filesystem level.
  - `ASTERINAS_ARCHITECT_PRIORS.md` and `ASTERINAS_CODE_QUALITY_PRIORS.md` for the local rule that mount/open sequencing, shared-state ownership, and later lookup or page-cache behavior must stay separated.

## exFAT Concepts Covered

- Mount-time superblock consumption and open sequencing.
- Root-seeded shared filesystem state.
- Root inode publication as a synthetic filesystem anchor.
- Loading the accepted upcase-table surface and allocation-bitmap surface into mount-owned state.
- Volume-wide shared state needed by later inode, directory, and write paths.
- Rejection of any drift into directory lookup policy, inode metadata shaping, page-cache backend behavior, allocation search, or mutation.

## Target Files

- Existing files likely to change:
  - `kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/sysroot.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/upcase_table.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/super_block.rs`
- New files expected:
  - `kernel/src/fs/fs_impls/exfat_refactor/fs.rs`

## Code Budget

- Target new or heavily rewritten code size:
  - `220-320` lines
- Reason if the budget might exceed 500 lines:
  - It should not if the component remains a mount/bootstrap layer plus shared-state assembly. If it starts absorbing lookup policy, page-cache backend wiring, allocation search, or inode metadata shaping, the boundary is wrong and the work should be split instead of widened.

## Exit Condition

Design work may start when there is exactly one mount-owned entry point that:

1. consumes validated superblock facts and validated root-discovery facts,
2. loads the accepted upcase-table and allocation-bitmap surfaces into shared filesystem state,
3. seeds and publishes the root inode through the synthetic root metadata shell,
4. records the mount-wide shared state needed by later inode, directory, and write components,
5. does not implement directory lookup policy, page-cache backend behavior, allocation search, or namespace mutation.

## Risks

- The shared filesystem object could become a catch-all if mount starts owning inode metadata shaping or page-cache state instead of only bootstrap and shared runtime state.
- The open sequence could quietly absorb directory lookup policy if root publication and path resolution are mixed together.
- Root-discovered table loading could be reimplemented inside mount instead of consuming the accepted loader surfaces from `EXR-UPCASE-07B` and `EXR-BITMAP-08A`.
- The root inode path could drift back into generic inode construction instead of using the explicit synthetic root constructor from `EXR-INODE-05B`.
- Lock order and shared-state ownership need to stay explicit early, because later write-side and lookup components will depend on the mount object without being allowed to redefine it.
- If this component begins to manage allocation hints, free-space search, or bitmap mutation, it has crossed into `EXR-BITMAP-08B`.
