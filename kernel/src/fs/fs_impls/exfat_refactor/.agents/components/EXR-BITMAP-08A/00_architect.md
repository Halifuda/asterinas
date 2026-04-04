<!-- SPDX-License-Identifier: MPL-2.0 -->

# Architect Handoff

## Metadata

- Component ID: `EXR-BITMAP-08A`
- Title: Allocation Bitmap Loading And Read-Only Occupancy Queries
- Status: `Architected`
- Author: `architect`
- Date: `2026-04-04`
- Task packet: [`EXR-BITMAP-08A-ARCH-20260404-1410`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-BITMAP-08A/20260404-1410-architect-packet.md)

## Purpose

This handoff covers the smallest useful exFAT bitmap component. It consumes the validated root-discovery facts from `EXR-SYSROOT-06`, loads the on-disk allocation bitmap into a read-only in-memory bitmap surface, validates that the bitmap is large enough for the volume geometry, and answers occupancy queries over the loaded bits.

It does not own allocation policy, free-space search, hint advancement, dirty tracking, discard or trim policy, or any bit mutation. It also does not re-scan the root directory or rediscover the bitmap entry.

## Why This Comes Now

`EXR-IO-02` already supplies metadata reads and cluster translation. `EXR-CHAIN-03B` already supplies read-only chain walking for a bitmap file when the stream entry requires it. `EXR-SYSROOT-06` already isolates discovery of the bitmap root entry, so the loader can now consume a narrow, trusted input instead of duplicating root-directory scanning.

This split is dependency-safe because bitmap validation needs the geometry and discovery facts that already exist, while later allocation policy needs a loaded bitmap but not the loader's own validation logic. Linux still couples loading, free-space search, and mutation in `balloc.c`; this component breaks that bundle before `EXR-BITMAP-08B` adds any write-side behavior.

## Dependency Contract

- Depends on:
  - `EXR-IO-02`
  - `EXR-CHAIN-03B`
  - `EXR-FILESET-04B`
  - `EXR-SYSROOT-06`
- Blocks:
  - `EXR-BITMAP-08B`
  - `EXR-MOUNT-09`
  - later bitmap-consuming write-side components that need a loaded bitmap with mutation support
- Can run in parallel with:
  - `EXR-UPCASE-07A` loader work after `EXR-SYSROOT-06`
  - `EXR-UPCASE-07B` case-folding work once the upcase table loader has a compatible input surface
- Recommended parallel wave:
  - finish `EXR-SYSROOT-06`, then overlap bitmap loading work with upcase-table loading work rather than serializing them behind mount bootstrap
- Stable pre-existing interfaces used:
  - the root-directory discovery result from `EXR-SYSROOT-06`
  - `read_metadata_bytes` from `io.rs`
  - `ExfatSuperBlock` geometry helpers, especially data-cluster validation and cluster-to-byte translation
  - read-only chain traversal from `EXR-CHAIN-03B` when the bitmap file is not a single direct span
- Prior sources or prior slices that materially shaped the split:
  - `Microsoft-exFAT-spec.md` for bitmap bit semantics, root-entry identity, and minimum size against `ClusterCount`
  - Linux `balloc.c` and `super.c` for the load/validate order and the fact that allocation search and mutation are separate concerns
  - `ASTERINAS_ARCHITECT_PRIORS.md` for the local rule that bitmap management must stay separate from mount bootstrap and upcase loading
  - legacy Asterinas `bitmap.rs` only as a warning that loading, search policy, dirty tracking, and mutation were historically over-coupled

## exFAT Concepts Covered

- Allocation bitmap entry discovery as a root-directory fact, not a mount-owned scan.
- Bitmap file loading from the validated start cluster and byte size recorded by `EXR-SYSROOT-06`.
- Minimum bitmap size calculation from the number of data clusters, with the bitmap bits starting at cluster `2`.
- Validation that the bitmap is large enough to represent every data cluster on the volume.
- Validation that the bitmap file's own occupied clusters are marked allocated before the bitmap is exposed.
- In-memory bit lookup for read-only occupancy queries over cluster IDs and bounded ranges.
- Rejection of out-of-range cluster IDs instead of interpreting tail padding as real volume space.
- No free-space search, no first-free hinting, no used-cluster accounting, and no write path.

## Target Files

- Existing files likely to change:
  - `kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- New files expected:
  - `kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs`

## Code Budget

- Target new or heavily rewritten code size: `180-260` lines
- Reason if the budget might exceed 500 lines:
  - It should stay within budget if the component remains a loader plus read-only query surface. If free-space search, cursor policy, dirty tracking, or bitmap writes appear here, the boundary is wrong and the work should move to `EXR-BITMAP-08B`.

## Exit Condition

Design work may start once the implementation plan expresses the bitmap slice as exactly this and nothing more:

1. consume validated bitmap-entry facts from `EXR-SYSROOT-06`,
2. load the bitmap bytes through the existing read-side I/O and chain helpers,
3. validate the bitmap size against the volume geometry,
4. validate that the bitmap file's own clusters are marked allocated,
5. expose read-only occupancy queries only,
6. keep allocation policy, free-space hints, dirty tracking, and mutation out of scope.

## Risks

- The loader can accidentally become a free-space allocator if it starts caching search hints or first-free results.
- The loader can accidentally reintroduce root-directory scanning if it stops trusting the `EXR-SYSROOT-06` discovery result and rediscovering the bitmap entry itself.
- Size validation needs to be strict enough to reject undersized bitmaps, but not so strict that an oversized on-disk bitmap is treated as an error when the volume geometry only requires a smaller minimum.
- Bitmap state must remain read-only here; adding dirty-byte tracking or bit writes would collapse `EXR-BITMAP-08A` into `EXR-BITMAP-08B`.
- The component should not absorb mount-owned state or volume-dirty policy. Those belong with `EXR-MOUNT-09`.
