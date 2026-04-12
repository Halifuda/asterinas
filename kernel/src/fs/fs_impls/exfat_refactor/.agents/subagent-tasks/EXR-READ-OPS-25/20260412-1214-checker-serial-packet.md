<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-READ-OPS-25-20260412-1214-CHECK-SERIAL`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-READ-OPS-25/20260412-1214-checker-serial-packet.md`
- Supersedes: None
- Role: `checker`
- Component: `EXR-READ-OPS-25`
- Phase: `serial checker`
- Authorizing main agent: `main-agent`
- Date: `2026-04-12 12:14 CST`

## Goal

- Validate the new buffered regular-file `read_at` path in `inode.rs`, add the required local buffered-read ktests, run filtered executable verification under the checker lock, and write the checker report.

## Architectural Unit Context

- Functional goal: `ExfatInode` buffered regular-file read path
- Final architectural owner: `ExfatInode`
- Expected landing form: owner methods and owner-private helpers in `inode.rs`
- Parent units:
  - `EXR-INODE-CORE-17`
  - `EXR-FILE-MAP-24`

## Required Resolution Questions

- Verify buffered `read_at` copies physically backed bytes and truncates at logical EOF.
- Verify reads that cross `valid_size` return copied bytes followed by zero-filled bytes only inside logical EOF.
- Verify reads at or beyond logical EOF return `0` without mutating writer-visible contents.
- Verify repeated reads on one inode snapshot return the same byte stream and byte count.
- Evaluate the thin `ExfatFs::file_read_context()` seam as a temporary surface: keep it only if the packet-scoped boundary justifies it and record the later owner/removal condition; otherwise report the issue instead of widening scope.
- If a compile/test failure is strictly local to `inode.rs`, make the smallest in-scope fix and record it.

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/CHECKER_REPORT_TEMPLATE.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-READ-OPS-25/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-READ-OPS-25/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-READ-OPS-25/02_designer_async.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-READ-OPS-25/03_designer_ktest.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-READ-OPS-25/10_creator_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FILE-MAP-24/10_creator_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FILE-MAP-24/11_checker_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FILE-MAP-24/30_reviewer_report.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/io.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/test_support.rs`
- `/home/halifuda/asterinas/kernel/src/fs/vfs/fs_apis/inode.rs`
- `/home/halifuda/asterinas/ostd/src/mm/io/mod.rs`

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-READ-OPS-25/11_checker_serial.md`

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

- Use the accepted `EXR-READ-OPS-25` designer constraints only.
- `EXR-FILE-MAP-24` remains translation-only. Do not turn the checker into a mapping or page-cache review lane.
- The creator-owned smoke test in `inode.rs` may remain, but checker coverage must add dedicated buffered-read regressions that directly satisfy the four designer scenarios.

## Integration Prior Inputs

- Tests must stay local to `inode.rs` and validate buffered read behavior through `ExfatInode`.
- The thin `file_read_context()` seam is acceptable only as the current packet-scoped traversal-context source. The checker must not widen it into a generic reader or cache helper.
- Keep directory behavior, page-cache integration, write-side mutation, allocator policy, and sync ordering out of scope.

## Workflow Prior Inputs

- Runtime/test-producing checker lane.
- Command-producing verification must hold the checker execution lock.
- Add local `#[ktest]` coverage in `inode.rs` with a stable source-backed prefix. Prefer new names beginning with `file_buffered_read_`.
- Prefer exact test names for final proof. If a broad prefix run does not prove the filter hit, rerun the intended tests with exact names derived from the source and record that evidence.
- If a nonzero exit does not clearly show the guest-side failure in terminal output, inspect `/home/halifuda/asterinas/qemu-serial.log` before classifying the result.

## Quality Prior Inputs

- Use the checker-role quality slice from `$exfat-subagent-workflow` and the local designer ktest obligations.
- Prefer test-only edits unless a strictly local production fix in `inode.rs` is necessary to satisfy the designer contract.

## Temporary Interfaces And Exit Plan

- Keep buffered-read helpers owner-private to `ExfatInode`.
- Do not add a public read service, page-cache shell, or filesystem-global traversal helper.
- If `file_read_context()` remains after checker, record why it is acceptable for now and what later owner should absorb it.

## Helper Justification

- New helper changes are justified only when they keep buffered-read verification local to `ExfatInode` or keep checker-owned local tests readable.

## Allowed Commands

- Read-only shell inspection commands under `/home/halifuda/asterinas`
- Lock-guarded Docker commands in `codex-asterinas-dev`:
  - `docker exec codex-asterinas-dev bash -lc 'test -e /dev/kvm && ls -l /dev/kvm || echo no-kvm'`
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test file_buffered_read_'`
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test <exact buffered-read test name>'`
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
  - `bash ./kernel/src/fs/fs_impls/exfat_refactor/.agents/tools/checker_lock.sh acquire --component EXR-READ-OPS-25 --phase serial --command "<exact docker commands actually used>" --retry-seconds 60 --wait-budget-seconds 1800`
- Release with:
  - `bash ./kernel/src/fs/fs_impls/exfat_refactor/.agents/tools/checker_lock.sh release`
- Stale-lock review is main-agent-only.

## Stop Condition

- Stop after writing `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-READ-OPS-25/11_checker_serial.md`.

## Escalation Rule

- If the checker needs edits outside `inode.rs` or cannot get trustworthy filtered-test evidence from the allowed commands, report that and stop.
