<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-PGCACHE-26-20260412-2126-CHECK-SERIAL`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-PGCACHE-26/20260412-2126-checker-serial-packet.md`
- Supersedes: None
- Role: `checker`
- Component: `EXR-PGCACHE-26`
- Phase: `serial checker`
- Authorizing main agent: `main-agent`
- Date: `2026-04-12 21:26 CST`

## Goal

- Validate the new inode-local page-cache landing in `inode.rs`, add the required local page-cache ktests, run filtered executable verification under the checker lock, and write the checker report.

## Architectural Unit Context

- Functional goal: `ExfatInode` inode-local page-cache attachment and backend behavior
- Final architectural owner: `ExfatInode`
- Expected landing form: owner-internal state plus trait impl in `inode.rs`
- Parent units:
  - `EXR-INODE-CORE-17`
  - `EXR-READ-OPS-25`

## Required Resolution Questions

- Verify a regular-file inode exposes an inode-local page-cache attachment without promoting cache ownership into `ExfatFs`.
- Verify page-cache fills reuse the accepted buffered-read owner and do not introduce a second byte-transfer or mapping-policy shell.
- Verify cache-visible data preserves the buffered-read EOF and valid-size zero-fill rules when a page crosses from backed bytes into the valid-size gap or logical EOF.
- Verify repeated cache-backed reads on one inode snapshot stay stable.
- Evaluate the temporary `write_page_async()` rejection as an explicitly future-owned surface; keep it only if the artifact and code still point to `EXR-WRITE-30` / `EXR-SYNC-31`.
- If a compile/test failure is strictly local to `inode.rs`, make the smallest in-scope fix and record it.

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/CHECKER_REPORT_TEMPLATE.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-26/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-26/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-26/02_designer_async.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-26/03_designer_ktest.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-26/10_creator_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-READ-OPS-25/10_creator_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-READ-OPS-25/11_checker_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-READ-OPS-25/30_reviewer_report.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/test_support.rs`
- `/home/halifuda/asterinas/kernel/src/fs/vfs/page_cache.rs`
- `/home/halifuda/asterinas/kernel/src/fs/vfs/fs_apis/inode.rs`

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-26/11_checker_serial.md`

## Forbidden Files

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
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
- `EXR-READ-OPS-25` remains the sole owner of buffered byte-stream policy. Page-cache verification must prove that the new backend consumes `read_at()` rather than cloning EOF or valid-size logic.
- `EXR-FILE-MAP-24` remains translation-only and out of scope except through the accepted buffered-read owner.

## Integration Prior Inputs

- Tests must stay local to `inode.rs` and exercise page-cache behavior through `ExfatInode`.
- Keep the cache attachment inode-local; do not widen `fs.rs`, add a cache manager, or reopen filesystem-global cache ownership.
- `write_page_async()` may remain a temporary rejection only if the code and checker artifact keep its future owner/removal condition explicit.

## Workflow Prior Inputs

- Runtime/test-producing checker lane.
- Command-producing verification must hold the checker execution lock.
- Add local `#[ktest]` coverage in `inode.rs` with a stable source-backed prefix. Prefer new test names beginning with `inode_page_cache_`.
- Prefer exact test names for final proof. If a broad prefix run does not prove the filter hit, rerun the intended tests with exact names derived from the source and record that evidence.
- If a nonzero exit does not clearly show the guest-side failure in terminal output, inspect `/home/halifuda/asterinas/qemu-serial.log` before classifying the result.

## Quality Prior Inputs

- Use the checker-role quality slice from `$exfat-subagent-workflow` and the local designer ktest obligations.
- Prefer test-only edits unless a strictly local production fix in `inode.rs` is necessary to satisfy the designer contract.

## Temporary Interfaces And Exit Plan

- Keep page-cache helpers owner-private to `ExfatInode`.
- Do not add a public cache service, duplicate buffered-read shell, or filesystem-global cache attachment.
- If `write_page_async()` remains after checker, record why it is acceptable for now and which later owner must absorb it.

## Helper Justification

- New helper changes are justified only when they keep page-cache verification local to `ExfatInode` or keep checker-owned local tests readable.

## Allowed Commands

- Read-only shell inspection commands under `/home/halifuda/asterinas`
- Lock-guarded Docker commands in `codex-asterinas-dev`:
  - `docker exec codex-asterinas-dev bash -lc 'test -e /dev/kvm && ls -l /dev/kvm || echo no-kvm'`
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test inode_page_cache_'`
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test <exact inode_page_cache test name>'`
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
  - `bash ./kernel/src/fs/fs_impls/exfat_refactor/.agents/tools/checker_lock.sh acquire --component EXR-PGCACHE-26 --phase serial --command "<exact docker commands actually used>" --retry-seconds 60 --wait-budget-seconds 1800`
- Release with:
  - `bash ./kernel/src/fs/fs_impls/exfat_refactor/.agents/tools/checker_lock.sh release`
- Stale-lock review is main-agent-only.

## Stop Condition

- Stop after writing `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-26/11_checker_serial.md`.

## Escalation Rule

- If the checker needs edits outside `inode.rs`, cannot keep page-cache verification local to the inode owner, or cannot get trustworthy filtered-test evidence from the allowed commands, report that and stop.
