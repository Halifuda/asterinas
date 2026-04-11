<!-- SPDX-License-Identifier: MPL-2.0 -->

# Creator Log

## Metadata

- Component ID: EXR-INODE-CACHE-18
- Title: ExfatFs Opened-Inode Table And Validated InodeKey
- Status: `SerialImplementing`
- Author: Codex
- Date: 2026-04-10
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-INODE-CACHE-18/20260410-1130-creator-repair-packet.md`
- Implemented spec: `EXR-INODE-CACHE-18` checker repair for the opened-inode cache `fs.rs` test helper
- Pass kind: `serial repair`

## Planned File Ownership

- Files to edit: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- Files intentionally left untouched: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`, `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fileset.rs`, `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs`

## Implementation Notes

Fixed the checker-blocking borrow-of-moved-value in the `test_inode()` helper by capturing `container_dev_id` from `disk.id()` before moving `disk` into `ExfatFs::new`. This keeps the checker-owned cache tests intact while restoring local buildability for the helper.

The three checker-added cache regressions and the root temporary seam were preserved unchanged.

## Approved Deviations

None.

## Optional Self-Checks

- Commands run, if any: None.
- Compile checks run, if any: None.
- Manual reasoning checks: Confirmed the fix stays within `fs.rs`, does not alter the cache contract, and avoids touching unrelated exFAT files.

## Remaining Risks

- No executable verification was run in this command-free repair lane.
- The unrelated `directory.rs` / `fileset.rs` checker-blocking build issues remain outside this repair slice.
