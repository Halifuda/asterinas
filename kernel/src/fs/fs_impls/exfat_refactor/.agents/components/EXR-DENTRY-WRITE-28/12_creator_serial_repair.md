<!-- SPDX-License-Identifier: MPL-2.0 -->

# Creator Repair Log

## Metadata

- Component ID: `EXR-DENTRY-WRITE-28`
- Title: `DirectoryEngine` write-side logical-offset repair
- Status: `SerialRepairing`
- Author: Codex
- Date: `2026-04-13`
- Repair packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-DENTRY-WRITE-28/20260413-0720-creator-repair-packet.md`
- Prior creator artifact: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DENTRY-WRITE-28/10_creator_serial.md`

## Planned File Ownership

- Files edited:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DENTRY-WRITE-28/12_creator_serial_repair.md`
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

- Repaired `DirectoryEngine` write helpers so `DirectoryRecordLocation::dentry_set_byte_offset` stays a logical directory-stream offset.
- Added owner-private logical-to-physical chunk mapping inside `DirectoryEngine`:
  - `read_directory_bytes`
  - `write_directory_bytes`
  - `logical_directory_chunk_at`
- Kept the low-level block-aligned helper private to `DirectoryEngine`:
  - `write_physical_metadata_bytes`
  - final owner: `DirectoryEngine`
  - removal condition: none; it remains the local physical write primitive beneath the logical directory-stream wrapper.
- Updated slot discovery, in-place rewrite, tombstoning, and committed-growth placement to use chain-aware logical I/O, including FAT-backed directory chains after growth.

## Verification

- Per task instructions, I did not run compile, test, format, Docker, or QEMU commands.

## Residual Risks

- Checker-owned regressions are still needed to prove the logical-offset repair on both contiguous and FAT-backed directory chains.
- The repair remains intentionally narrow and does not alter namespace policy, allocation ownership, or sync ordering.
