<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-PGCACHE-26-20260412-2208-CHECK-SERIAL-FINAL-RECHECK`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-PGCACHE-26/20260412-2208-checker-serial-final-recheck-packet.md`
- Supersedes:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-PGCACHE-26/20260412-2159-checker-serial-refresh-packet.md`
- Role: `checker`
- Component: `EXR-PGCACHE-26`
- Phase: `serial checker final recheck`
- Authorizing main agent: `main-agent`
- Date: `2026-04-12 22:08 CST`

## Goal

- Retry the exact `inode_page_cache_*` proofs after the latest foreign `fat.rs:342` `error.into()` repair and record whether `EXR-PGCACHE-26` now has complete executable evidence.

## Required Resolution Questions

- Re-run the exact page-cache tests under the checker lock.
- If the tests now execute, record the exact-name proof and any runtime mode observations.
- If a new foreign blocker still appears, record it exactly and stop.

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-26/11_checker_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-26/12_checker_serial_retry.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-26/13_checker_serial_recheck.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-26/14_checker_serial_refresh.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-ALLOC-27/10_creator_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fat.rs`

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-26/15_checker_serial_final_recheck.md`

## Required Inputs

- Skill to use: `$exfat-subagent-workflow`
- Required role reference: `references/checker.md`
- Also read: `references/testing-guide.md`

## Workflow Prior Inputs

- Runtime/test-producing checker lane.
- Command-producing verification must hold the checker execution lock.
- Re-run the exact page-cache test names directly.

## Allowed Commands

- Read-only shell inspection commands under `/home/halifuda/asterinas`
- Lock-guarded Docker commands in `codex-asterinas-dev`:
  - `docker exec codex-asterinas-dev bash -lc 'test -e /dev/kvm && ls -l /dev/kvm || echo no-kvm'`
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test inode_page_cache_attachment_stays_inode_local_for_regular_file_snapshot'`
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test inode_page_cache_backend_fills_backed_bytes_through_inode_owner'`
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test inode_page_cache_preserves_valid_size_gap_and_eof_zero_fill'`
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test inode_page_cache_repeated_commit_reads_are_stable_on_one_snapshot'`

## Execution Lock

- Acquire with:
  - `bash ./kernel/src/fs/fs_impls/exfat_refactor/.agents/tools/checker_lock.sh acquire --component EXR-PGCACHE-26 --phase serial-final-recheck --command "<exact docker commands actually used>" --retry-seconds 60 --wait-budget-seconds 1800`
- Release with:
  - `bash ./kernel/src/fs/fs_impls/exfat_refactor/.agents/tools/checker_lock.sh release`

## Stop Condition

- Stop after writing `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-26/15_checker_serial_final_recheck.md`.

## Escalation Rule

- If the recheck still hits a foreign compile/test blocker, record it exactly and stop.
