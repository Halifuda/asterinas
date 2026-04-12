<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-PGCACHE-26-20260412-2159-CHECK-SERIAL-REFRESH`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-PGCACHE-26/20260412-2159-checker-serial-refresh-packet.md`
- Supersedes:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-PGCACHE-26/20260412-2151-checker-serial-recheck-packet.md`
- Role: `checker`
- Component: `EXR-PGCACHE-26`
- Phase: `serial checker refresh`
- Authorizing main agent: `main-agent`
- Date: `2026-04-12 21:59 CST`

## Goal

- Refresh the foreign-source view for `bitmap.rs` and `fat.rs`, then retry the exact `inode_page_cache_*` proofs under the checker lock and record whether the remaining blocker is a stale workspace view or a still-real foreign compile failure.

## Architectural Unit Context

- Functional goal: `ExfatInode` inode-local page-cache attachment and backend behavior
- Final architectural owner: `ExfatInode`
- Expected landing form: owner-internal state plus trait impl in `inode.rs`
- Prior checker artifacts:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-26/11_checker_serial.md`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-26/12_checker_serial_retry.md`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-26/13_checker_serial_recheck.md`

## Required Resolution Questions

- Confirm from the current host workspace whether `bitmap.rs` and `fat.rs` now contain `use ostd::mm::VmIo;` before retrying the exact page-cache proofs.
- Re-run the exact `inode_page_cache_*` filtered proofs if the refreshed source view shows the expected imports.
- If the build still fails in foreign files, record enough exact evidence to distinguish a stale workspace mismatch from a still-real foreign compile failure.
- If a compile/test failure is now strictly local to `inode.rs`, make the smallest in-scope fix and record it.

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/CHECKER_REPORT_TEMPLATE.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-26/10_creator_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-26/11_checker_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-26/12_checker_serial_retry.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-26/13_checker_serial_recheck.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-ALLOC-27/10_creator_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fat.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/test_support.rs`

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-26/14_checker_serial_refresh.md`

## Forbidden Files

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fat.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/`

## Required Inputs

- Skill to use: `$exfat-subagent-workflow`
- Required role reference: `references/checker.md`
- Also read: `references/testing-guide.md`

## Semantic Prior Inputs

- This is still a page-cache checker continuation for `inode.rs`, not an allocator or bitmap checker lane.
- The main agent can currently see `use ostd::mm::VmIo;` in the host workspace at:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fat.rs`
- If your refreshed source view disagrees with that, record the mismatch explicitly.

## Integration Prior Inputs

- Keep verification local to `inode.rs` and the existing checker-owned tests.
- Use read-only host inspection to confirm the current foreign-source state before rerunning cargo.
- If cargo still reports the same foreign compile error after the source refresh, capture the exact diagnostic details and stop.

## Workflow Prior Inputs

- Runtime/test-producing checker lane.
- Command-producing verification must hold the checker execution lock.
- Prefer the exact page-cache test names directly.

## Allowed Commands

- Read-only shell inspection commands under `/home/halifuda/asterinas`, including:
  - `sed -n '1,24p' /home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs`
  - `sed -n '1,20p' /home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fat.rs`
- Lock-guarded Docker commands in `codex-asterinas-dev`:
  - `docker exec codex-asterinas-dev bash -lc 'test -e /dev/kvm && ls -l /dev/kvm || echo no-kvm'`
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test inode_page_cache_attachment_stays_inode_local_for_regular_file_snapshot'`
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test inode_page_cache_backend_fills_backed_bytes_through_inode_owner'`
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test inode_page_cache_preserves_valid_size_gap_and_eof_zero_fill'`
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test inode_page_cache_repeated_commit_reads_are_stable_on_one_snapshot'`

## Parallelism Classification

- Lane class: `runtime/test-producing`
- May overlap with command-free lanes only

## Execution Environment

- Host and Docker

## Execution Lock

- Acquire with:
  - `bash ./kernel/src/fs/fs_impls/exfat_refactor/.agents/tools/checker_lock.sh acquire --component EXR-PGCACHE-26 --phase serial-refresh --command "<exact docker commands actually used>" --retry-seconds 60 --wait-budget-seconds 1800`
- Release with:
  - `bash ./kernel/src/fs/fs_impls/exfat_refactor/.agents/tools/checker_lock.sh release`

## Stop Condition

- Stop after writing `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-26/14_checker_serial_refresh.md`.

## Escalation Rule

- If the refreshed source view and the cargo diagnostics still disagree, record that exact mismatch and stop instead of guessing.
