<!-- SPDX-License-Identifier: MPL-2.0 -->

# Creator Log

## Metadata

- Component ID: `EXR-DIR-OPS-23`
- Title: `ExfatInode` Read-Only Directory Operations
- Status: `SerialBlocked`
- Author: creator
- Date: `2026-04-11`
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-DIR-OPS-23/20260411-1613-creator-serial-packet.md`
- Implemented spec: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-OPS-23/01_designer_core.md`
- Pass kind: `serial implementation`

## Planned File Ownership

- Files edited:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-OPS-23/10_creator_serial.md`
- Files intentionally left untouched:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`

## Blocker Summary

Creator work stopped under the packet escalation rule because the accepted designer behavior cannot be implemented inside `inode.rs` alone with the currently exposed owner methods.

Required read-only directory lookup/readdir behavior depends on:

- constructing a `DirectoryEngine` for the current inode directory chain using filesystem-owned block-device and `ExfatSuperBlock` inputs,
- and resolving matched children through the filesystem-owned opened-inode reuse/publish boundary.

Both dependencies are currently owner-private to `fs.rs` with no callable `pub(super)` bridge for `inode.rs`:

- `DirectoryEngine::new(...)` needs `block_device` and `super_block` owned privately by `ExfatFs`.
- opened-inode reuse helpers in `ExfatFs` (`lookup_opened_inode`, `publish_opened_inode`, and `remove_opened_inode`) are private methods.
- `InodeKey` is defined in `fs.rs`, but `inode.rs` has no callable owner method that accepts lookup facts and returns/reuses a child inode handle.

Because `fs.rs` is outside this packet write set and no existing `inode.rs`-visible handoff exists, implementing `lookup` and `readdir_at` in-scope would require widening edits into forbidden files.

## Exact Missing Handshake

One of the following owner-facing handshakes is required before creator can land this packet in `inode.rs`:

1. `ExfatFs` owner methods callable from `inode.rs` that:
   - create/drive a `DirectoryEngine` for a provided directory-chain snapshot, and
   - resolve a matched file record into a canonical child inode via opened-inode reuse.
2. Equivalent `pub(super)` bridges in `fs.rs` that expose the same behavior without leaking private fields or moving ownership out of `ExfatFs`.

## Approved Deviations

- Packet escalation rule invoked: the pass is blocked due to missing cross-owner handshake and no legal in-lane implementation path.

## Optional Self-Checks

- Commands run, if any: read-only inspection (`sed`, `rg`) only.
- Compile checks run, if any: none (packet forbids compile/test commands in this lane).
- Manual reasoning checks:
  - Confirmed no in-scope `inode.rs` access path exists to `ExfatFs` private mount/runtime fields needed for `DirectoryEngine`.
  - Confirmed no in-scope callable `ExfatFs` API exists for opened-inode reuse from `inode.rs`.

## Remaining Risks

- `EXR-DIR-OPS-23` remains unimplemented until the missing `ExfatFs` handshake is provided or lane scope is expanded to include `fs.rs`.
