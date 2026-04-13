<!-- SPDX-License-Identifier: MPL-2.0 -->

# Creator Log

## Metadata

- Component ID: `EXR-WRITE-30`
- Role: `creator`
- Date: `2026-04-13`
- Packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-WRITE-30/20260413-1755-creator-serial-packet.md`

## Changes

- Replaced the temporary buffered `write_at` rejection in `inode.rs` with inode-owned buffered write behavior for regular files.
- Added call-local `ExfatInodeWriteState` and inode-snapshot helpers so buffered writes and growth consume a single mutable write-state carrier per call.
- Implemented committed-growth consumption through `ExfatFs::allocate_clusters()` for write requests that extend beyond current allocation coverage.
- Added gap zero-filling for writes that begin beyond the current `valid_size`, then published the written bytes through the file’s disk-visible byte stream.
- Resized and invalidated the inode-local page-cache range touched by each write so the cache size stays coherent with the new inode snapshot.
- Added a growth regression for an empty file and refreshed the existing inode state regression to assert buffered writes now update visible bytes instead of returning `EOPNOTSUPP`.

## Boundary Notes

- `resize` remains deferred in this slice, consistent with the narrower write-only focus for this packet step.
- Direct I/O, truncate, and sync ordering were not absorbed into this pass.
- The inode-local write path now owns the visible byte publication step; allocation search and reservation still stay inside `EXR-ALLOC-27`.

## New Helper Surfaces

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
  - Added `ExfatInodeState` and `ExfatInodeWriteState`.
  - Added owner-private snapshot and commit helpers for write-side publication.
  - Added owner-private helpers for committed growth calculation, allocation folding, and disk-backed gap/data writes.
  - Final owner: `ExfatInode`.

## Verification

- No compile, test, format, Docker, KVM, or QEMU commands were run in this creator lane.

## Residual Risks

- The new write path still depends on later sync ownership for durable flush ordering.
- Growth-on-write currently stays within the inode-owned boundary and does not introduce a separate truncate or writeback manager.
