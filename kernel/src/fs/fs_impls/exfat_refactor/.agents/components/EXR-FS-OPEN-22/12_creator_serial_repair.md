<!-- SPDX-License-Identifier: MPL-2.0 -->

# Creator Repair Log

## Metadata

- Component ID: `EXR-FS-OPEN-22`
- Title: `ExfatFs` mount/open sequencing and root publication
- Status: `SerialRepaired`
- Author: Codex
- Date: `2026-04-10`
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-FS-OPEN-22/20260410-1535-creator-repair-packet.md`
- Implemented spec: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-OPEN-22/01_designer_core.md`
- Pass kind: `serial repair`

## Planned File Ownership

- Files edited:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-OPEN-22/12_creator_serial_repair.md`
- Files intentionally left untouched:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-OPEN-22/` sibling artifacts

## Implementation Notes

Preserved the earlier root-publication work in `fs.rs` and added an owner-local mount/open entrypoint that actually sequences the prerequisites before publishing the root handle.

- Added a filesystem-owner serialization boundary with `mount_open_state` so root mount/open is linearized inside `ExfatFs`.
- Added `ExfatFs::open_root_inode(&Arc<Self>)` as the owner-side mount/open path.
- That path now:
  - builds the root directory chain,
  - scans the root directory through `DirectoryEngine`,
  - discovers the `Upcase` and `Bitmap` system entries,
  - installs the upcase table first,
  - loads the allocation bitmap second,
  - constructs the root inode from trusted root facts,
  - publishes the canonical root handle through the root-private slot.
- Kept the root special case distinct from the ordinary `InodeKey` cache.
- Added a regression that proves the mount path makes upcase folding and bitmap accounting available only after the open sequence publishes the root.

## Approved Deviations

None.

## Optional Self-Checks

- Commands run, if any: None.
- Compile checks run, if any: None.
- Manual reasoning checks:
  - Confirmed the mount/open sequence stays owner-local to `fs.rs`.
  - Confirmed the canonical root still flows through the dedicated root slot instead of the ordinary opened-inode keyspace.
  - Confirmed the new open path uses `DirectoryEngine`, `UpcaseTable`, and `AllocationBitmap` as prerequisites rather than as separate owners.

## Remaining Risks

- The repair was not compile-verified in this lane by design.
- Later integration still needs to call the new owner-side open path at the actual mount boundary.
