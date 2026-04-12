<!-- SPDX-License-Identifier: MPL-2.0 -->

# Creator Serial Repair

## Metadata

- Packet ID: `EXR-DIR-OPS-23-20260412-0853-CREATOR-READDIR-FS-LIFETIME-REPAIR`
- Role: `creator`
- Component: `EXR-DIR-OPS-23`
- Date: `2026-04-12`
- Status: `completed`

## Repair Summary

- Repaired the two `readdir_*` ktests in `kernel/src/fs/fs_impls/exfat_refactor/inode.rs` so they keep the returned `Arc<ExfatFs>` alive while invoking `root.readdir_at(...)`.
- The repair is test-only and preserves the existing production implementation.
- No fixture, visitor, or production `readdir_at` behavior was changed in this packet.

## Tests Repaired

- `readdir_emits_visible_entries_in_stable_order`
- `readdir_continuation_remains_stable_across_repeated_calls`

## Lifetime Preservation

- Both tests now bind the tuple as `(fs, root)` and retain the filesystem owner through a local `_fs` binding for the duration of the test.
- This prevents the `Arc<ExfatFs>` owner from being dropped before `root.readdir_at(...)` upgrades the inode's weak filesystem reference.

## Boundary

- Production code remained untouched.
- The repair stops at the packet boundary and does not reopen directory semantics or `.` / `..` behavior.
