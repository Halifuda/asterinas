<!-- SPDX-License-Identifier: MPL-2.0 -->

# Creator Log

## Metadata

- Component ID: EXR-INODE-CACHE-18
- Title: ExfatFs Opened-Inode Table And Validated InodeKey
- Status: `SerialImplementing`
- Author: Codex
- Date: 2026-04-10
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-INODE-CACHE-18/20260410-1058-creator-serial-packet.md`
- Implemented spec: `EXR-INODE-CACHE-18` designer core/async boundary for the owner-private inode cache and root slot
- Pass kind: `serial implementation`

## Planned File Ownership

- Files to edit: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- Files intentionally left untouched: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`, `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fileset.rs`

## Implementation Notes

Added an owner-private `InodeKey` value type in `fs.rs` with a constructor that accepts only trusted location facts. Added an `OpenedInodeState` guard under `ExfatFs` to own the ordinary opened-inode table plus a separate root slot, with reuse-first publication, exact-key removal, and root publication kept outside the keyed table.

The existing `root_inode()` seam now consults the dedicated root slot first and still preserves the packet-approved temporary panic when the later handoff has not published a root inode yet.

## Approved Deviations

None.

## Optional Self-Checks

- Commands run, if any: None.
- Compile checks run, if any: None.
- Manual reasoning checks: Confirmed the new cache boundary stays inside `fs.rs`, keeps the root slot separate from the ordinary keyspace, and avoids edits to forbidden sibling files.

## Remaining Risks

- The new cache API is owner-private and not yet wired into the later open/sequencing pass.
- No compile or ktest verification was run in this command-free creator lane.
