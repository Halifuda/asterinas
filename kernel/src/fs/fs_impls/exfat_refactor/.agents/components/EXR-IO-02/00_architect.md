<!-- SPDX-License-Identifier: MPL-2.0 -->

# Architect Handoff

## Metadata

- Component ID: EXR-IO-02
- Title: Metadata Byte I/O And Cluster Address Translation Helpers
- Status: `Architected`
- Author: architect
- Date: 2026-03-31

## Purpose

Provide the smallest reusable I/O and geometry helper layer that later exFAT components can depend on after `EXR-BOOT-01`.

This component should extract and normalize two families of behavior:

1. block-device-facing metadata byte reads for arbitrary volume offsets, including alignment handling needed by the current boot-region code path;
2. pure translation and validation helpers derived from `ExfatSuperBlock`, such as cluster validity checks and cluster-to-byte or cluster-to-sector mapping.

The goal is to give later read-side components a stable way to obtain metadata bytes and convert cluster identifiers into physical locations without forcing them to duplicate offset math or block-alignment quirks.

## Why This Comes Now

This ordering is dependency-safe because `EXR-BOOT-01` already establishes the trusted runtime geometry that these helpers need, while later components such as `EXR-CHAIN-03`, `EXR-SYSROOT-06`, `EXR-UPCASE-07`, `EXR-BITMAP-08`, and `EXR-MOUNT-09` all need the same low-level translation rules and metadata read path.

Putting this layer first avoids leaking ad hoc cluster-offset math and byte-read helpers into FAT, inode, bitmap, or mount code. It also lets `EXR-BOOT-01` reuse the same aligned metadata-read path instead of owning a private special case.

## Dependency Contract

- Depends on:
  - `EXR-BOOT-01`
- Blocks:
  - `EXR-CHAIN-03`
  - `EXR-INODE-05`
  - `EXR-SYSROOT-06`
  - `EXR-UPCASE-07`
  - `EXR-BITMAP-08`
  - `EXR-MOUNT-09`
- Stable pre-existing interfaces used:
  - `aster_block::BlockDevice`
  - `ostd::mm::VmIo`
  - the normalized `ExfatSuperBlock` accepted in `EXR-BOOT-01`
  - existing kernel error conventions under `kernel/`

## exFAT Concepts Covered

- Volume byte offsets versus sector offsets.
- Aligned metadata byte access over the block-device interface.
- Cluster identifier validity in normalized exFAT geometry.
- Cluster-to-volume-byte translation.
- Cluster-to-sector translation.
- Data-region offset calculations derived from the accepted boot geometry.

This component deliberately stays below higher-level exFAT semantics:

- no FAT entry interpretation,
- no cluster-chain walking,
- no inode mapping policy,
- no dentry parsing,
- no mount object construction.

## Target Files

- Existing files likely to change:
  - `kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/boot_sector.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/super_block.rs`
- New files expected:
  - `kernel/src/fs/fs_impls/exfat_refactor/io.rs`

## Code Budget

- Target new or heavily rewritten code size: `250-350` lines
- Reason if the budget might exceed 500 lines:
  - It should stay within budget if the component is limited to aligned metadata byte I/O, pure translation helpers, and cluster validity predicates.
  - If the scope expands into FAT semantics, page-cache ownership, writeback policy, or inode-specific sector mapping, the boundary is wrong and must be split instead of enlarged.

## Exit Condition

Design work may start once this component boundary is accepted as all of the following and nothing more:

1. A reusable aligned metadata byte-read helper exists outside `boot_sector.rs`.
2. The accepted `ExfatSuperBlock` exposes or supports pure helpers for:
   - sector size and cluster size access,
   - cluster validity checks,
   - cluster-range validity checks,
   - cluster-to-byte translation,
   - cluster-to-sector translation.
3. `EXR-BOOT-01` can depend on this shared metadata-read helper without pulling in mount or FAT policy.
4. No FAT walking, page-cache backend logic, inode logical-offset mapping, metadata writeback policy, or sync semantics are required to begin implementation.

Observable readiness means the designer can specify a narrow helper API that later components may call directly, while leaving higher-level ownership and mutation policy to later stages.

## Risks

- The designer must distinguish volume-byte offsets, sector indices, and block-device alignment units precisely. This component must not leave offset units implicit.
- The designer must decide which translations are pure `ExfatSuperBlock` methods and which remain standalone helper functions. The API surface should be minimal and consistent.
- Metadata write and sync helpers are tempting to include because the legacy `ExfatFs` owns `read_meta_at`, `write_meta_at`, and `sync_meta_at`, but pulling writeback into this component would entangle it with later mutation policy and page-cache ownership. The default should be read-side helpers only unless the main agent explicitly widens scope.
- Cluster-to-sector translation must stop at physical placement math. It must not absorb FAT-chain walking, file logical-offset mapping, or inode page-cache behavior from later components.
- Concurrency work should stay lightweight here. If helper sharing introduces non-trivial synchronization or cache ownership, that is a signal that a later component is being pulled in too early.
