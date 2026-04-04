<!-- SPDX-License-Identifier: MPL-2.0 -->

# Checker Serial Retry Evidence

## Metadata

- Component ID: `EXR-UPCASE-07B`
- Title: Canonical Upcase-Backed Case Folding And Name Hashing
- Status: `SerialCheckedRetry`
- Author: `checker`
- Date: `2026-04-04`
- Task packet: `kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-UPCASE-07B/20260404-1605-checker-retry-packet.md`
- Checked implementation:
  - [`fileset.rs`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fileset.rs)

## Scope Of Review

- Re-checked the repaired consumer boundary in [`fileset.rs`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fileset.rs).
- Verified only the packet-authorized focused ktest command under the checker lock.
- Did not touch any file outside the write set.

## Locked Command

- Lock acquired with:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/tools/checker_lock.sh acquire --component EXR-UPCASE-07B --phase checker-serial-retry --command "docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test fileset::tests'" --retry-seconds 60 --wait-budget-seconds 1800`
- Verification command run under the lock:
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test fileset::tests'`

## Evidence

- The command completed successfully with exit code `0`.
- QEMU fell back to TCG in `codex-asterinas-dev`, which is expected in this container.
- The checked consumer path in `fileset.rs` now uses `ExfatUpcaseTable::name_hash()` for stream-entry validation.

## Findings

- None in the owned scope for this retry pass.

## Notes

- No production code edit was needed in this retry pass.
- The retry closes out the earlier raw-UTF-16 checksum blocker by confirming the focused checker command passes against the current `fileset.rs` implementation.
