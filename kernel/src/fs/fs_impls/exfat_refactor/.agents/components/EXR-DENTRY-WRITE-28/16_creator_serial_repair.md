<!-- SPDX-License-Identifier: MPL-2.0 -->

# Creator Repair Log

## Metadata

- Component ID: `EXR-DENTRY-WRITE-28`
- Title: `DirectoryEngine` unused-tail repair
- Status: `SerialRepairing`
- Author: Codex
- Date: `2026-04-13`
- Repair packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-DENTRY-WRITE-28/20260413-0806-creator-repair-unused-tail-packet.md`
- Prior creator artifacts:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DENTRY-WRITE-28/10_creator_serial.md`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DENTRY-WRITE-28/12_creator_serial_repair.md`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DENTRY-WRITE-28/14_creator_serial_repair.md`

## Planned File Ownership

- Files edited:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DENTRY-WRITE-28/16_creator_serial_repair.md`
- Files intentionally left untouched:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/allocator.rs`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fat.rs`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/`
  - sibling component artifacts

## Repair Notes

- Added an explicit owner-private placement discriminator:
  - `DirectorySlotSearch`
  - final owner: `DirectoryEngine`
  - removal condition: none; it remains the local result shape for distinguishing reusable fits, tail-short growth cases, and the absence of a reusable tail.
- Added an owner-private trailing-tail inspection helper:
  - `trailing_reusable_tail_state`
  - final owner: `DirectoryEngine`
  - removal condition: none; it exists to decide whether an in-place expansion consumes an existing `Unused` terminator.
- Updated `place_dentry_set()` and relocation handling so fits that consume an existing `Unused` terminator publish a replacement terminator unless the record ends exactly at directory EOF.
- Updated in-place expansion so expansion beyond `existing_slots` only republishes `Unused` when the existing terminator was actually consumed.
- Kept growth-path publication scan-safe while treating exact EOF as a valid no-op:
  - `publish_unused_terminator` now returns `Ok(())` when the new record ends exactly at the logical directory EOF.
- Preserved the narrow `DirectoryEngine` boundary and did not widen into namespace policy, allocator ownership, or sync ordering.

## Verification

- Per task instructions, I did not run compile, test, format, Docker, or QEMU commands.

## Residual Risks

- Checker-owned regressions are still needed to prove the unused-tail replacement behavior on in-place expansion, relocated fits, and growth placements that end exactly at EOF.
- The repair remains intentionally narrow and owner-private to `DirectoryEngine`.
