<!-- SPDX-License-Identifier: MPL-2.0 -->

# Checker Serial Evidence

## Metadata

- Component ID: `EXR-UPCASE-07B`
- Title: Canonical Upcase-Backed Case Folding And Name Hashing
- Status: `SerialChecked`
- Author: `checker`
- Date: `2026-04-04`
- Task packet: `EXR-UPCASE-07B-CHECK-20260404-1517`
- Checked implementation:
  - [`upcase_table.rs`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/upcase_table.rs)
  - [`fileset.rs`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fileset.rs)
- Pass kind: `serial`

## Scope Of Review

- Checked the canonical upcase-backed fold-and-hash surface in [`upcase_table.rs`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/upcase_table.rs).
- Checked the consumer-side `fileset.rs` constructor path that was added for table-backed hashing.
- Verified only the packet-authorized ktest surface and the exact focused command under the checker lock.

## Test Changes

- Added [`name_hash_uses_folded_units_from_full_table_surface`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/upcase_table.rs#L374) in `upcase_table.rs` to prove a later table entry folds before hashing and that raw UTF-16 bytes produce a different hash.
- Added [`fileset_canonical_metadata_construction_uses_upcase_table_hash`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fileset.rs#L518) in `fileset.rs` to prove the canonical constructor uses the loaded upcase table hash instead of the raw checksum path.
- Each checker-owned `#[ktest]` added here has a short scenario comment immediately above it.

## Locked Command

- Lock acquired with:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/tools/checker_lock.sh acquire --component EXR-UPCASE-07B --phase checker-serial --command "docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test upcase_table::tests'" --retry-seconds 60 --wait-budget-seconds 1800`
- Verification command run under the lock:
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test upcase_table::tests'`

## Evidence

- The command built the kernel test crate successfully inside `codex-asterinas-dev`.
- QEMU fell back to TCG in this container, which is expected here, and the test run completed successfully with exit code `0`.
- No compile or runtime failure remained in the checked component after the new regressions were added.

## Findings

### Blocking defect

- Severity: `blocking`
- Location: [`fileset.rs`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fileset.rs#L182)
- Description: The production validation path in `ExfatDentrySet::validate()` still compares `stream_dentry.name_hash` against `checksum_utf16(&raw_name_units)`. That keeps the canonical consumer path on the provisional raw-UTF-16 checksum rather than the table-backed `NameHash` service.
- Violated spec clause or expected behavior: The component spec requires `fileset.rs` to consume the canonical table-backed service and not retain an independent raw-UTF-16 name-hash path.
- Reproduction or reasoning: The new ktest proves the table-backed constructor works, but the validation path still uses the raw checksum comparison at line `182`, so the consumer-side redirection is not complete.

## Verified Properties

- The loaded upcase-table surface still preserves the full table bytes, including entries beyond the legacy 128-entry prefix.
- The canonical hash service folds a code unit through the table before hashing.
- The new fileset constructor path can populate `name_hash` from the loaded table-backed service.
- The locked focused command completed successfully in the containerized QEMU flow.

## Unverified Properties

- The production consumer validation path in `fileset.rs` now uses the canonical table-backed hash service.
- Any later call site outside this packet's write set is redirected away from the raw checksum path.

## Recommendation

- Next owner: `creator` or `main-agent`
- Reason: the consumer path still carries the raw checksum comparison and needs a production fix or explicit re-scope.
- Blocking or non-blocking: `blocking`
