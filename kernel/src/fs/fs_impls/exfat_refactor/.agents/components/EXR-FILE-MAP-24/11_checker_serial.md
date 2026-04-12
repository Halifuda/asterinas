<!-- SPDX-License-Identifier: MPL-2.0 -->

# Checker Serial Report

## Metadata

- Component ID: `EXR-FILE-MAP-24`
- Title: `ExfatInode` Read-Path Logical-To-Physical File Mapping
- Status: `SerialChecked`
- Author: checker
- Date: `2026-04-12`
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-FILE-MAP-24/20260412-1105-checker-serial-packet.md`
- Checked implementation:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FILE-MAP-24/10_creator_serial.md`
- Pass kind: `serial`

## Scope of Review

- Checked the owner-private mapping helper set added to `ExfatInode` in `inode.rs` against the architect and designer constraints for translation-only ownership.
- Added local checker-owned `#[ktest]` coverage in `inode.rs` for logical-offset translation, span bounding, repeated-call stability, and explicit empty/unbacked results.
- Executed the packet-authorized filtered `cargo osdk test` commands under the checker lock in `codex-asterinas-dev`.

## Test Changes

- Added `prepared_mapping_context()` and `mapping_test_inode()` as local test-only helpers in `inode.rs` to build regular-file mapping fixtures without widening into `fs.rs`.
- Added these checker-owned `#[ktest]` cases in `inode.rs`:
  - `file_mapping_translates_logical_offsets_to_expected_physical_ranges`
  - `file_mapping_mappable_span_respects_size_facts_and_cluster_geometry`
  - `file_mapping_repeated_calls_are_stable_on_one_snapshot`
  - `file_mapping_empty_or_unbacked_requests_stay_explicit`
- Each new test has a short scenario comment immediately above it.

## Findings

- No findings.

## Verified Properties

- `map_physical_file_range()` stays owner-private to `ExfatInode` and remains a translation-only helper boundary; it does not widen into byte-copying, zero-fill, EOF policy, or page-cache behavior.
- Logical offsets map to the expected cluster id, physical byte offset, and in-cluster byte offset for a regular-file inode snapshot.
- The reported mappable span is capped by file-size facts, valid-size facts, and single-cluster geometry.
- Zero-length requests and requests that begin at the first unbacked offset remain explicit `Ok(None)` results instead of inventing a read policy.
- Repeated calls on the same inode snapshot return the same translation result.
- The explicit `&dyn BlockDevice` and `&ExfatSuperBlock` arguments remain acceptable as a temporary surface for this row because the packet forbids widening into `fs.rs`, and the creator artifact already records `EXR-READ-OPS-25` as the later owner that can collapse this temporary shape.

## Unverified Properties

- The first packet-authorized prefix filter, `cargo osdk test file_mapping_`, did not hit the new tests. Host-side `qemu-serial.log` showed `0 passed; 0 failed; 138 filtered out`, so that run was not used as final evidence.
- The checker did not widen into `fs.rs`, so it did not validate a future narrower traversal-context accessor beyond the already recorded temporary-surface justification.

## Recommendation

- Next owner: reviewer
- Reason: serial checker coverage is now present and the checked implementation matched the current designer contract without requiring a local production fix.
- Blocking or non-blocking: non-blocking
- Final-check note: not a post-review final checker; this is the required serial checker pass.

## Executable Evidence

- Lock acquisition:
  - `bash ./kernel/src/fs/fs_impls/exfat_refactor/.agents/tools/checker_lock.sh acquire --component EXR-FILE-MAP-24 --phase serial --command "docker exec codex-asterinas-dev bash -lc 'test -e /dev/kvm && ls -l /dev/kvm || echo no-kvm' && docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test file_mapping_'" --retry-seconds 60 --wait-budget-seconds 1800`
- KVM visibility check:
  - `docker exec codex-asterinas-dev bash -lc 'test -e /dev/kvm && ls -l /dev/kvm || echo no-kvm'`
  - Result: `/dev/kvm` was visible in the container.
- Initial filtered run:
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test file_mapping_'`
  - Result: command exited `0`, but `/home/halifuda/asterinas/qemu-serial.log` showed `0 passed; 0 failed; 138 filtered out`, so the prefix filter was too short for trustworthy hit proof.
- Debug reruns under the same checker lock:
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test file_mapping_translates_logical_offsets_to_expected_physical_ranges'`
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test file_mapping_mappable_span_respects_size_facts_and_cluster_geometry'`
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test file_mapping_repeated_calls_are_stable_on_one_snapshot'`
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test file_mapping_empty_or_unbacked_requests_stay_explicit'`
- Filter-hit proof:
  - Source-backed exact suffixes live at `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs:1074`, `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs:1143`, `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs:1208`, and `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs:1234`.
  - Host-side `qemu-serial.log` explicitly listed the exact test name for the first and last debug reruns and recorded `1 passed; 0 failed; 137 filtered out`.
- Runtime mode:
  - Although `/dev/kvm` was visible, QEMU printed repeated `TCG doesn't support requested feature` warnings during the executable runs, so the observed guest execution mode was treated as TCG rather than confirmed KVM acceleration.
- Lock release:
  - `bash ./kernel/src/fs/fs_impls/exfat_refactor/.agents/tools/checker_lock.sh release`
