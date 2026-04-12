<!-- SPDX-License-Identifier: MPL-2.0 -->

# Checker Report

## Metadata

- Component ID: `EXR-ALLOC-27`
- Title: `ExfatFs` Cluster Allocation Service Boundary
- Status: `SerialChecked`
- Author: `checker`
- Date: `2026-04-12`
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-ALLOC-27/20260412-2201-checker-serial-packet.md`
- Checked implementation: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/allocator.rs`, `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs`, `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fat.rs`, `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- Pass kind: `serial`

## Scope of Review

Validated the filesystem-owned allocator boundary under `ExfatFs`, including free-space search, reservation intent, bitmap/FAT commit coordination, and the new checker-owned allocator regressions. The review stayed inside the packet write set and used exact-name filtered reruns for proof.

## Test Changes

- Added three checker-owned `#[ktest]` regressions in `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/allocator.rs` at lines `496`, `532`, and `583`.
- Each regression has a short scenario comment describing the allocator behavior under test.
- The tests cover contiguous preference, fragmented fallback, and failed-commit visibility.
- No tests were added in `fs.rs`.
- Exact filtered verification was rerun with `CARGO_NET_OFFLINE=true` after crates.io TLS fetches failed in the default online attempt.

## Findings

### [P1] Sector-aligned writeback was required for allocator commit

- Severity: high
- Location: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs:233` and `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fat.rs:312`
- Description: The bitmap writer and FAT entry writer were sending short buffers directly through `dyn BlockDevice`. `VmIo` for `dyn BlockDevice` only accepts sector-aligned writes, so allocator commit reruns failed with `EINVAL` when the writeback path tried to publish bitmap or FAT updates.
- Violated spec clause or expected behavior: bitmap and FAT state must remain coherent after commit, and the commit path must work through the block-device owner boundary instead of relying on short unaligned writes.
- Reproduction or reasoning: the first exact contiguous allocator rerun failed in `allocator.rs` during commit before the fix. After switching both writers to sector-aligned read-modify-write helpers, the exact reruns passed under QEMU/TCG.

## Verified Properties

- The allocator prefers a contiguous run when one is available.
- Fragmented allocation is chosen only when contiguous space is insufficient.
- Reservation intent does not become visible when commit fails.
- Bitmap and FAT state remain coherent after commit.
- Exact-name filtered reruns passed for:
  - `allocator_prefers_contiguous_free_run_when_available`
  - `allocator_falls_back_to_fragmented_free_clusters_only_when_needed`
  - `allocator_keeps_reservation_private_until_commit_succeeds`
- `/dev/kvm` was visible on the host, but the guest runs used QEMU TCG, as confirmed by the serial warnings.

## Unverified Properties

- None.

## Recommendation

- Next owner: `main-agent`
- Reason: the checker pass is complete and the local production fix is in scope.
- Blocking or non-blocking: `non-blocking`
- If this is the last checker pass before acceptance, state whether it was a required final checker or a previously recorded skip case: required final checker.
