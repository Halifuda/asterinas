<!-- SPDX-License-Identifier: MPL-2.0 -->

# Creator Repair Log

## Metadata

- Component ID: `EXR-WRITE-30`
- Role: `creator`
- Date: `2026-04-13`
- Packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-WRITE-30/20260413-1807-creator-repair-packet.md`
- Prior creator artifact: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/10_creator_serial.md`

## Repair Scope

- Repaired extending writes on already non-empty files so committed growth becomes reachable from the preexisting file chain after `write_at` returns.
- Kept the repair owner-private to `ExfatInode` in `inode.rs`.
- Left `resize`, truncate, direct I/O, and sync ordering unchanged and out of scope.

## Implementation Notes

- Imported the allocator-owned `AllocationResult` and FAT write seam into `inode.rs` so the inode write owner can stitch committed growth without adding a new public filesystem helper.
- Reworked `fold_committed_allocation()` so it now decides between two owner-private outcomes:
  - preserve `ChainMode::Contiguous` only when the existing file chain is already contiguous and the new committed run is also contiguous starting exactly at the previous tail plus one;
  - otherwise collect the existing chain and appended chain clusters, materialize the full combined chain with `write_next_fat_value()`, and publish `ChainMode::FatBacked` only after that combined chain is coherent.
- Added owner-private helpers `can_preserve_contiguous_chain()`, `collect_chain_clusters()`, and `materialize_fat_chain()` in `inode.rs`.

## Test Touch

- Updated the empty-file growth ktest to install mount prerequisites through `ExfatFs::open_root_inode()` before it exercises allocation-backed growth.
- Added a local non-empty growth ktest that writes across the old EOF and confirms the grown bytes remain visible after the repaired chain stitch path.

## Ownership Record

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
  - `can_preserve_contiguous_chain()`
  - `collect_chain_clusters()`
  - `materialize_fat_chain()`
  - Final owner: `ExfatInode` buffered write growth.

## Verification

- No compile, test, format, Docker, KVM, or QEMU commands were run in this creator repair lane.

## Residual Risks

- The repair publishes coherent in-memory chain facts only after stitching is complete, but durable ordering and writeback remain downstream with `EXR-SYNC-31`.
- The local non-empty growth ktest is still a creator-lane source inspection addition; checker remains responsible for executable proof.
