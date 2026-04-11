<!-- SPDX-License-Identifier: MPL-2.0 -->

# Creator Log

## Metadata

- Component ID: `EXR-FS-OPEN-22`
- Title: `ExfatFs` mount/open sequencing and root publication
- Status: `SerialImplemented`
- Author: Codex
- Date: `2026-04-10`
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-FS-OPEN-22/20260410-1510-creator-serial-packet.md`
- Implemented spec: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-OPEN-22/01_designer_core.md`
- Pass kind: `serial implementation`

## Planned File Ownership

- Files edited:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-OPEN-22/10_creator_serial.md`
- Files intentionally left untouched:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-OPEN-22/` sibling artifacts

## Implementation Notes

Replaced the indefinite `root_inode()` seam with explicit owner-side root publication behavior in `fs.rs`.
The `FileSystem::root_inode()` path now returns the canonical published root handle from `OpenedInodeState` and fails loudly if the root has not been published yet.

Kept the root special case distinct from ordinary opened-inode entries by using the existing owner-private root slot and the existing `publish_root_inode()` handoff.
The new root-publication regression in `fs.rs` publishes a root inode through the owner boundary and confirms repeated access returns the same canonical handle.

## Approved Deviations

None.

## Optional Self-Checks

- Commands run, if any: None.
- Compile checks run, if any: None.
- Manual reasoning checks:
  - Confirmed the root special case remains separate from the ordinary `InodeKey` cache.
  - Confirmed the owner boundary stays in `fs.rs`.
  - Confirmed the temporary `todo!` seam was removed from `root_inode()`.

## Remaining Risks

- The pass was not compile-verified in this lane by design.
- The deeper mount-time discovery path still depends on later integration work to feed the published root handle into `publish_root_inode()`.
