<!-- SPDX-License-Identifier: MPL-2.0 -->

# Creator Log

## Metadata

- Component ID: `EXR-BITMAP-21`
- Title: `ExfatFs` allocation-bitmap owner boundary
- Status: `SerialImplemented`
- Author: Codex
- Date: `2026-04-10`
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-BITMAP-21/20260410-1245-creator-serial-packet.md`
- Implemented spec: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BITMAP-21/01_designer_core.md`
- Pass kind: `serial implementation`

## Planned File Ownership

- Files edited:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BITMAP-21/10_creator_serial.md`
- Files intentionally left untouched:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/dentry.rs`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/io.rs`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/super_block.rs`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-20/` sibling artifacts

## Implementation Notes

Introduced an owner-local immutable `AllocationBitmap` snapshot in `bitmap.rs`.
The snapshot stores the validated bitmap bytes plus cached used/free accounting and exposes only the canonical read-only occupancy query.

Added owner-side publication and query wiring on `ExfatFs`:

- `load_allocation_bitmap()` validates one prepared bitmap-chain snapshot and publishes it behind the existing filesystem-owner mutex boundary.
- `cluster_is_allocated()` rejects out-of-range cluster ids and maps cluster `2` to bit `0`.
- `used_cluster_count()` and `free_cluster_count()` report derived counts from the same published snapshot.

The bitmap loader validates the advertised byte length against the superblock geometry, reads the bitmap payload cluster-by-cluster through `read_metadata_bytes`, and rejects publication unless every cluster in the bitmap file is itself marked allocated in the loaded image.

## Approved Deviations

None.

## Optional Self-Checks

- Commands run, if any: None.
- Compile checks run, if any: None.
- Manual reasoning checks:
  - Confirmed the new state stays owner-local to `bitmap.rs` and `fs.rs`.
  - Confirmed publication is atomic behind the existing mutex boundary.
  - Confirmed no directory rescanning, allocation mutation, or FAT write path was introduced.

## Remaining Risks

- The code was not compile-verified in this lane by design.
- Checker-owned regressions still need to prove bitmap rejection, occupancy, and accounting behavior against synthetic fixtures.
