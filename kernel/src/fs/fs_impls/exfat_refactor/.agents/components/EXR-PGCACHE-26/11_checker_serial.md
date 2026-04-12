<!-- SPDX-License-Identifier: MPL-2.0 -->

# Checker Report

## Metadata

- Component ID: `EXR-PGCACHE-26`
- Title: `ExfatInode` page-cache integration boundary
- Status: `SerialChecked`
- Author: checker
- Date: `2026-04-12`
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-PGCACHE-26/20260412-2126-checker-serial-packet.md`
- Checked implementation: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- Pass kind: `serial`

## Scope of Review

- Checked the inode-local page-cache landing in `inode.rs` at the owner field, constructor wiring, backend impl, and public `page_cache()` exposure points.
- Checked the new checker-owned `#[ktest]` coverage in `inode.rs` for the inode-local attachment, backend fill, valid-size/EOF zero-fill, and repeated-read stability scenarios.
- Ran the packet-authorized lock-guarded Docker verification commands in `codex-asterinas-dev`.

## Test Changes

- Added the checker-owned `#[ktest]` cases in `inode.rs`:
  - `inode_page_cache_attachment_stays_inode_local_for_regular_file_snapshot`
  - `inode_page_cache_backend_fills_backed_bytes_through_inode_owner`
  - `inode_page_cache_preserves_valid_size_gap_and_eof_zero_fill`
  - `inode_page_cache_repeated_commit_reads_are_stable_on_one_snapshot`
- Added the local test helper `committed_page_bytes()` in `inode.rs` to commit one cached page and read the page bytes back for assertions.
- Each checker-owned `#[ktest]` has a short scenario comment immediately above it.

## Findings

No findings in `inode.rs`.

## Verified Properties

- `ExfatInode` now carries inode-local page-cache state and exposes it through `page_cache()`.
- The regular-file snapshot test for inode-local page-cache attachment passed under QEMU/TCG.
- The cache backend remains inode-owned in `inode.rs`, and `write_page_async()` is still explicitly deferred to `EXR-WRITE-30` and `EXR-SYNC-31`.

## Unverified Properties

- The exact filtered run for `inode_page_cache_backend_fills_backed_bytes_through_inode_owner` did not complete because the workspace hit an unrelated host-side compile error in `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs:650` (`read_primary_super_block` not found in scope).
- Because that compile failure is outside the packet write set, the remaining exact filtered page-cache regressions were not attempted.
- I therefore did not get full executable proof for cache-backed fill, valid-size/EOF zero-fill, or repeated cache-backed stability in this checker pass.

## Recommendation

- Next owner: the lane that owns the unrelated `bitmap.rs` compile break, then this checker pass should be rerun.
- Reason: the remaining exact `inode_page_cache_` evidence is blocked by the foreign compile error, not by `inode.rs`.
- Blocking or non-blocking: blocking.
- If this is the last checker pass before acceptance, state whether it was a required final checker or a previously recorded skip case: required final checker, but incomplete because of the unrelated compile failure.

## Executable Evidence

- Lock acquisition:
  - `bash ./kernel/src/fs/fs_impls/exfat_refactor/.agents/tools/checker_lock.sh acquire --component EXR-PGCACHE-26 --phase serial --command "docker exec codex-asterinas-dev bash -lc 'test -e /dev/kvm && ls -l /dev/kvm || echo no-kvm' && docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test inode_page_cache_attachment_stays_inode_local_for_regular_file_snapshot' && docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test inode_page_cache_backend_fills_backed_bytes_through_inode_owner' && docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test inode_page_cache_preserves_valid_size_gap_and_eof_zero_fill' && docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test inode_page_cache_repeated_commit_reads_are_stable_on_one_snapshot'" --retry-seconds 60 --wait-budget-seconds 1800`
- KVM visibility check:
  - `docker exec codex-asterinas-dev bash -lc 'test -e /dev/kvm && ls -l /dev/kvm || echo no-kvm'`
  - Result: `/dev/kvm` was visible in the container.
- Exact filtered run that passed:
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test inode_page_cache_attachment_stays_inode_local_for_regular_file_snapshot'`
  - Result: exited `0`.
  - Runtime mode: QEMU printed `TCG doesn't support requested feature` warnings, so the observed execution mode was TCG.
- Exact filtered run that blocked the rest:
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test inode_page_cache_backend_fills_backed_bytes_through_inode_owner'`
  - Result: exited `6` with a host-side compile error in `bitmap.rs:650`.
  - Error: `E0425`, `read_primary_super_block` not found in scope.
  - Because the failure was host-side, `/home/halifuda/asterinas/qemu-serial.log` did not need to be consulted.
- Remaining exact filtered runs were not attempted after the unrelated compile break:
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test inode_page_cache_preserves_valid_size_gap_and_eof_zero_fill'`
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test inode_page_cache_repeated_commit_reads_are_stable_on_one_snapshot'`
- Lock release:
  - `bash ./kernel/src/fs/fs_impls/exfat_refactor/.agents/tools/checker_lock.sh release`
