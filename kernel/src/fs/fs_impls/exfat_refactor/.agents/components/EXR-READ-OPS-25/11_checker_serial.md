<!-- SPDX-License-Identifier: MPL-2.0 -->

# Checker Serial Report

## Metadata

- Component ID: `EXR-READ-OPS-25`
- Title: `ExfatInode` Buffered Regular-File Read Path
- Status: `SerialChecked`
- Author: checker
- Date: `2026-04-12`
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-READ-OPS-25/20260412-1214-checker-serial-packet.md`
- Checked implementation:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-READ-OPS-25/10_creator_serial.md`
- Pass kind: `serial`

## Scope of Review

- Checked the buffered regular-file `read_at` path in `inode.rs` against the accepted `EXR-READ-OPS-25` architect and designer constraints, including EOF ownership, valid-size zero-fill ownership, and repeated-call determinism.
- Added local checker-owned `#[ktest]` coverage in `inode.rs` for the four required buffered-read scenarios.
- Ran the packet-authorized lock-guarded Docker verification commands in `codex-asterinas-dev`.

## Test Changes

- Added local checker-owned buffered-read fixture helpers in `inode.rs`:
  - `prepared_buffered_read_context()`
  - `buffered_read_test_inode()`
  - `patterned_file_bytes()`
  - `write_contiguous_file_bytes()`
- Added these checker-owned `#[ktest]` cases in `inode.rs`:
  - `file_buffered_read_copies_backed_bytes_and_truncates_at_eof`
  - `file_buffered_read_zero_fills_from_valid_size_to_logical_eof`
  - `file_buffered_read_at_or_beyond_eof_returns_zero_without_mutation`
  - `file_buffered_read_repeated_calls_are_stable_on_one_snapshot`
- Applied one strictly local production fix in `inode.rs` after the first filtered build failed: `zero_fill_valid_size_gap()` now wraps `VmWriter::fill_zeros()` with `Ok(...?)` so the OSTD error path coerces back into the kernel `Error` type.
- Each checker-owned `#[ktest]` has a short scenario comment immediately above it.

## Findings

- No findings.

## Verified Properties

- `ExfatInode::read_at` copies physically backed bytes into the caller writer and truncates the visible result at logical EOF.
- Reads that begin in backed data and cross `valid_size` return the copied byte prefix followed by zero-filled bytes only inside logical EOF.
- Reads that start at logical EOF or beyond return `0` and leave the caller-visible buffer unchanged.
- Repeated reads on the same inode snapshot return the same byte count and byte stream, including a request that spans physical data and the valid-size gap.
- Buffered read policy remains on `ExfatInode`; `map_physical_file_range()` remains a translation-only dependency rather than inheriting EOF, retry, zero-fill, or page-cache ownership.
- The thin `ExfatFs::file_read_context()` seam remains acceptable for this packet because it exposes only the current `&dyn BlockDevice` and `&ExfatSuperBlock` traversal context, and the creator artifact already records the later inode-owned cache/read consolidation row as the removal point instead of widening the accessor into a generic read service.

## Unverified Properties

- The broad filtered run, `cargo osdk test file_buffered_read_`, exited `0` after the local fix but did not print an executed-test line in terminal output, so it was not used as the final filter-hit proof.
- This checker pass did not widen into page-cache integration, directory behavior, write-side mutation, allocator policy, or sync ordering.

## Recommendation

- Next owner: reviewer
- Reason: the buffered-read row now has the required checker-owned regression coverage, the local compile break was repaired in-scope, and the final executable evidence matches the current designer contract.
- Blocking or non-blocking: non-blocking
- Final-check note: not a post-review final checker; this is the required serial checker pass.

## Executable Evidence

- Lock acquisition:
  - `bash ./kernel/src/fs/fs_impls/exfat_refactor/.agents/tools/checker_lock.sh acquire --component EXR-READ-OPS-25 --phase serial --command "docker exec codex-asterinas-dev bash -lc 'test -e /dev/kvm && ls -l /dev/kvm || echo no-kvm' && docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test file_buffered_read_' && docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test file_buffered_read_copies_backed_bytes_and_truncates_at_eof' && docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test file_buffered_read_zero_fills_from_valid_size_to_logical_eof' && docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test file_buffered_read_at_or_beyond_eof_returns_zero_without_mutation' && docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test file_buffered_read_repeated_calls_are_stable_on_one_snapshot'" --retry-seconds 60 --wait-budget-seconds 1800`
- KVM visibility check:
  - `docker exec codex-asterinas-dev bash -lc 'test -e /dev/kvm && ls -l /dev/kvm || echo no-kvm'`
  - Result: `/dev/kvm` was visible in the container.
- Initial filtered run before the local repair:
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test file_buffered_read_'`
  - Result: exited `6` with an explicit host-side compile error, `E0308`, at `zero_fill_valid_size_gap()` in `inode.rs`, so the failure was classified as a local build failure and `/home/halifuda/asterinas/qemu-serial.log` inspection was not needed.
- Broad filtered rerun after the local repair:
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test file_buffered_read_'`
  - Result: exited `0`, but terminal output still did not explicitly list the executed tests, so the broad suffix was not used as final filter-hit proof.
- Exact reruns used for final proof:
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test file_buffered_read_copies_backed_bytes_and_truncates_at_eof'`
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test file_buffered_read_zero_fills_from_valid_size_to_logical_eof'`
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test file_buffered_read_at_or_beyond_eof_returns_zero_without_mutation'`
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test file_buffered_read_repeated_calls_are_stable_on_one_snapshot'`
  - Result: each exact rerun exited `0` under the checker lock.
- Filter-hit proof:
  - Source-backed exact suffixes live at `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs:1462`, `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs:1487`, `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs:1513`, and `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs:1545`.
  - Because `cargo osdk test` uses suffix matching, these full function names provide the final trustworthy filter proof even though the broad filter did not echo executed-test lines in terminal output.
- Runtime mode:
  - Although `/dev/kvm` was visible, QEMU printed repeated `TCG doesn't support requested feature` warnings during the executable runs, so the observed guest execution mode was treated as TCG rather than confirmed KVM acceleration.
- Lock release:
  - `bash ./kernel/src/fs/fs_impls/exfat_refactor/.agents/tools/checker_lock.sh release`
