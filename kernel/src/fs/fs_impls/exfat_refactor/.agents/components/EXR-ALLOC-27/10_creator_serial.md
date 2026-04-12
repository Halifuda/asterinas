<!-- SPDX-License-Identifier: MPL-2.0 -->

# Creator Log

## Metadata

- Component ID: `EXR-ALLOC-27`
- Title: `ExfatFs` Cluster Allocation Service Boundary
- Status: `SerialImplemented`
- Author: creator
- Date: `2026-04-12`
- Implemented spec: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-ALLOC-27/01_designer_core.md`

## Planned File Ownership

- Files edited:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/allocator.rs`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fat.rs`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-ALLOC-27/10_creator_serial.md`
- Files intentionally left untouched:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/`

## Implementation Notes

Implemented the filesystem-owned allocator boundary as an `ExfatFs`-internal service.

- Added `allocator.rs` with:
  - owner-local `Allocator` state,
  - a small committed `AllocationResult`,
  - a temporary reservation record that stays private to the allocator call,
  - contiguous-first search, fragmented fallback, bitmap/FAT commit orchestration, and search-cursor advancement.
- Wired `ExfatFs` to own the allocator service in `fs.rs` and exposed an owner-private `allocate_clusters()` wrapper for later consumers.
- Extended `AllocationBitmap` so the allocator can search and persist the published bitmap snapshot without turning bitmap ownership into a standalone manager.
- Added a narrow FAT write helper so the allocator can materialize a FAT-backed chain when fragmentation requires it.

## New Helper Surfaces

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs`
  - Added `AllocationBitmap::find_contiguous_free_run()`, `collect_free_clusters()`, `reserve_clusters()`, and `write_to_disk()`.
  - Added private bitmap mutation/search helpers `normalize_search_start()`, `set_cluster_bit()`, and reservation scanning helpers.
  - Added `bitmap_chain` storage and `Clone` on `AllocationBitmap` so the allocator can stage a committed snapshot locally.
  - Final owner: `ExfatFs` allocator boundary.
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fat.rs`
  - Added `write_next_fat_value()` for allocator-owned FAT commit and rollback support.
  - Final owner: `ExfatFs` allocator boundary.
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
  - Added the allocator field, `allocation_bitmap()` guard accessor, and `allocate_clusters()` wrapper.
  - Final owner: `ExfatFs`.
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
  - Added `mod allocator;` to expose the new owner-internal module.
  - Final owner: `exfat_refactor` module wiring.

## Repair Note

- Fixed the host-side compile break reported by checker feedback in `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs`.
- The bitmap test import block now correctly brings `read_primary_super_block` into scope, so the local `bitmap_accounting_ignores_padding_bits_beyond_valid_range` regression compiles again.
- Fixed the follow-up host-side compile blockers in `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs` and `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fat.rs` by importing `ostd::mm::VmIo`, which is required for the allocator-owned `write_bytes()` calls.
- Fixed the remaining host-side type mismatch in `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fat.rs` by converting the mirror-write rollback error with `error.into()`.

## Verification

- No compile, test, format, Docker, or QEMU commands were run in the creator lane.
- The repair above was made from source inspection only, within the authorized write set.

## Residual Risks

- Checker-owned regressions are still needed for contiguous search, fragmented fallback, reservation visibility, and commit coherence.
- The allocator commit path now stages rollback for bitmap/FAT divergence, but only checker execution can prove the full behavior on the target image.
