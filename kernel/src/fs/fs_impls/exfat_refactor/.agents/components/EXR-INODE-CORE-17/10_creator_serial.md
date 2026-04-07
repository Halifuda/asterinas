<!-- SPDX-License-Identifier: MPL-2.0 -->

# Creator Log

## Metadata

- Component ID: EXR-INODE-CORE-17
- Title: Inode Carrier And Metadata Owner
- Status: `SerialImplementing`
- Author: creator
- Date: 2026-04-07
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-INODE-CORE-17/20260407-1043-creator-serial-packet.md`
- Implemented spec: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CORE-17/01_designer_core.md`
- Pass kind: `serial implementation`

## Planned File Ownership

- Files to edit:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CORE-17/10_creator_serial.md`
- Files intentionally left untouched:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
  - Sibling `EXR-FS-CORE-16` artifacts

## Implementation Notes

Implemented `ExfatInode` as a crate-local VFS `Inode` / `InodeIo` carrier in `inode.rs`. The inode keeps a weak `ExfatFs` back-reference, a copied `Metadata` snapshot, owner-private dentry-location state, file/stream scalar facts, and chain scalar facts copied from trusted `ExfatDentrySet` and `ExfatChain` inputs.

The constructor copies file size, valid size, file attributes, starting cluster, cluster count, chain mode, and allocation size into inode-owned state. It derives `Metadata::nr_sectors_allocated` from the copied allocation-size snapshot and keeps `metadata()` coherent with the dedicated metadata accessors by returning the same inode-owned snapshot.

`read_at()` and `write_at()` reject explicitly with the required temporary seam comment naming `EXR-READ-OPS-25`, `EXR-WRITE-30`, and `EXR-PGCACHE-26`. `resize()`, `set_mode()`, `set_owner()`, and `set_group()` reject instead of mutating hidden writeback state. Timestamp setters remain no-op temporary seams naming `EXR-WRITE-30` and `EXR-SYNC-31`, because the VFS trait requires them to return `()`.

No inode cache, `InodeKey`, page-cache backend, directory operations, namespace mutation, sync policy, or module wiring was added in this pass.

## Approved Deviations

None.

## Optional Self-Checks

- Commands run, if any: read-only inspection commands only (`sed`, `rg`).
- Compile checks run, if any: none; the packet required a command-free creator lane and disallowed build/test/format/QEMU commands.
- Manual reasoning checks:
  - Confirmed `metadata()` and the dedicated metadata accessors read from the same copied `Metadata` snapshot.
  - Confirmed the filesystem owner edge is `Weak<ExfatFs>` and `fs()` upgrades it on demand.
  - Confirmed the required data-path temporary seam comment is present on both `InodeIo` methods.
  - Confirmed the pass did not edit `mod.rs`, `fs.rs`, `COMPONENT_INDEX.md`, or sibling artifacts.

## Remaining Risks

- `inode.rs` imports `super::fs::ExfatFs`; this relies on sibling `EXR-FS-CORE-16` to land the filesystem owner and module wiring it owns.
- Checker-owned `#[ktest]` coverage remains for the next role as specified in `03_designer_ktest.md`.
