<!-- SPDX-License-Identifier: MPL-2.0 -->

# Checker Serial Evidence

## Scope

- Component: `EXR-UPCASE-07A`
- Role: `checker`
- Execution stage: `checker-serial`

## Local Coverage Added

- Added `#[cfg(ktest)]` coverage in [`upcase_table.rs`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/upcase_table.rs#L129) for:
  - valid-load preservation of the full table surface,
  - checksum mismatch rejection,
  - malformed discovery rejection,
  - truncated-payload rejection.
- The tests stay local to `upcase_table.rs` and exercise only the loader result and error path.

## Locked Command

- Lock acquired with:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/tools/checker_lock.sh acquire --component EXR-UPCASE-07A --phase checker-serial --command "docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test upcase_table::tests'" --retry-seconds 60 --wait-budget-seconds 1800`
- Verification command run under the lock:
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test upcase_table::tests'`

## Evidence

- The command entered the containerized QEMU test flow and began building the kernel test crate.
- The run failed before any `upcase_table::tests` execution because the shared ktest crate already has compile failures in `kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs`:
  - missing `Errno` imports at lines `308`, `326`, `337`, `338`, and `341`,
  - `ExfatAllocationBitmap` lacking `Debug` for `unwrap_err()` at lines `306` and `324`.
- No `upcase_table.rs` compile errors remained in the second run output before the shared `bitmap.rs` failures stopped the build.

## Outcome

- Checker pass did not reach runtime test completion because of pre-existing shared compile blockers outside the write set.
