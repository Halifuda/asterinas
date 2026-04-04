<!-- SPDX-License-Identifier: MPL-2.0 -->

# Checker Final Evidence

## Scope

- Component: `EXR-UPCASE-07A`
- Role: `checker`
- Execution stage: `final-checker`

## Locked Command

- Lock acquired with:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/tools/checker_lock.sh acquire --component EXR-UPCASE-07A --phase final-checker --command "docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test upcase_table::tests'" --retry-seconds 60 --wait-budget-seconds 1800`
- Verification command run under the lock:
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test upcase_table::tests'`

## Evidence

- The command built the kernel test base crate successfully.
- QEMU booted in TCG mode, as shown by the `qemu-system-x86_64` warnings about unsupported CPU features.
- The focused run completed with exit code `0`.
- No new in-scope blocker appeared in `upcase_table.rs`.

## Outcome

- Final checker pass succeeded.
