<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-PGCACHE-26-20260412-2142-CHECK-SERIAL-RETRY`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-PGCACHE-26/20260412-2142-checker-serial-retry-packet.md`
- Supersedes:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-PGCACHE-26/20260412-2126-checker-serial-packet.md`
- Role: `checker`
- Component: `EXR-PGCACHE-26`
- Phase: `serial checker retry`
- Authorizing main agent: `main-agent`
- Date: `2026-04-12 21:42 CST`

## Goal

- Resume the blocked `EXR-PGCACHE-26` checker pass after the foreign `bitmap.rs` compile repair, rerun the exact `inode_page_cache_*` proofs under the checker lock, and record final executable evidence.

## Architectural Unit Context

- Functional goal: `ExfatInode` inode-local page-cache attachment and backend behavior
- Final architectural owner: `ExfatInode`
- Expected landing form: owner-internal state plus trait impl in `inode.rs`
- Prior checker artifact:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-26/11_checker_serial.md`

## Required Resolution Questions

- Re-run the exact `inode_page_cache_*` filtered proofs now that the foreign `bitmap.rs` compile break has been repaired.
- Confirm the checker-owned regressions still prove inode-local cache attachment, backend fill through the read owner, valid-size/EOF zero-fill preservation, and repeated-read stability.
- If a compile/test failure is now strictly local to `inode.rs`, make the smallest in-scope fix and record it.
- If a new failure is still foreign to the packet write set, record it exactly and stop instead of widening scope.

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/CHECKER_REPORT_TEMPLATE.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-26/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-26/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-26/02_designer_async.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-26/03_designer_ktest.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-26/10_creator_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-26/11_checker_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-ALLOC-27/10_creator_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/test_support.rs`
- `/home/halifuda/asterinas/kernel/src/fs/vfs/page_cache.rs`
- `/home/halifuda/asterinas/kernel/src/fs/vfs/fs_apis/inode.rs`

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-26/12_checker_serial_retry.md`

## Forbidden Files

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/`
- sibling component artifacts
- reviewer, advisor, and handoff artifacts

## Required Inputs

- Skill to use: `$exfat-subagent-workflow`
- Required role reference: `references/checker.md`
- Also read: `references/testing-guide.md`

## Semantic Prior Inputs

- Use the accepted `EXR-PGCACHE-26` designer constraints only.
- The previous checker artifact already established there were no findings in `inode.rs`; this retry is for blocked executable proof completion, not for widening the review scope.
- The foreign `bitmap.rs` compile repair came from the active allocator creator lane and stays outside this packet's ownership.

## Integration Prior Inputs

- Keep page-cache verification local to `inode.rs` and the existing checker-owned tests.
- Do not reopen allocator ownership, bitmap helper ownership, or filesystem-global cache ownership during this retry.
- If the retried commands surface another foreign compile failure, record the exact blocker and stop.

## Workflow Prior Inputs

- Runtime/test-producing checker lane.
- Command-producing verification must hold the checker execution lock.
- Re-run the exact page-cache test names directly; do not rely on a broad prefix proof for the retry artifact.
- If a nonzero exit does not clearly show the guest-side failure in terminal output, inspect `/home/halifuda/asterinas/qemu-serial.log` before classifying the result.

## Quality Prior Inputs

- Use the checker-role quality slice from `$exfat-subagent-workflow`.
- Prefer test-only edits unless a strictly local production fix in `inode.rs` is necessary to satisfy the designer contract.

## Temporary Interfaces And Exit Plan

- Keep page-cache helpers owner-private to `ExfatInode`.
- Do not add a public cache service, duplicate buffered-read shell, or filesystem-global cache attachment.
- If `write_page_async()` remains after the retry, keep its future owner/removal condition explicit in the checker artifact.

## Helper Justification

- New helper changes are justified only when they keep page-cache verification local to `ExfatInode` or keep checker-owned local tests readable.

## Allowed Commands

- Read-only shell inspection commands under `/home/halifuda/asterinas`
- Lock-guarded Docker commands in `codex-asterinas-dev`:
  - `docker exec codex-asterinas-dev bash -lc 'test -e /dev/kvm && ls -l /dev/kvm || echo no-kvm'`
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test inode_page_cache_attachment_stays_inode_local_for_regular_file_snapshot'`
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test inode_page_cache_backend_fills_backed_bytes_through_inode_owner'`
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test inode_page_cache_preserves_valid_size_gap_and_eof_zero_fill'`
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test inode_page_cache_repeated_commit_reads_are_stable_on_one_snapshot'`
- If the guest-side failure is unclear after a nonzero run, read-only inspection of `/home/halifuda/asterinas/qemu-serial.log` is allowed.

## Parallelism Classification

- Lane class: `runtime/test-producing`
- May overlap with command-free lanes only
- Known conflicts:
  - serialized checker command lane
  - `inode.rs`

## Execution Environment

- Host and Docker
- Required command prefix:
  - `docker exec codex-asterinas-dev bash -lc`
- Required working directory:
  - `/root/asterinas/kernel`

## Execution Lock

- Lock script:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/tools/checker_lock.sh`
- Lock path:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/locks/checker-execution.lock/`
- Lock metadata file:
  - `owner.toml`
- Acquire with:
  - `bash ./kernel/src/fs/fs_impls/exfat_refactor/.agents/tools/checker_lock.sh acquire --component EXR-PGCACHE-26 --phase serial-retry --command "<exact docker commands actually used>" --retry-seconds 60 --wait-budget-seconds 1800`
- Release with:
  - `bash ./kernel/src/fs/fs_impls/exfat_refactor/.agents/tools/checker_lock.sh release`
- Stale-lock review is main-agent-only.

## Stop Condition

- Stop after writing `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-26/12_checker_serial_retry.md`.

## Escalation Rule

- If the retry still needs edits outside `inode.rs`, cannot get trustworthy exact-name evidence from the allowed commands, or encounters a new foreign compile/test blocker, report that exactly and stop.
