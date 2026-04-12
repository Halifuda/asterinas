<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-ALLOC-27-20260412-2201-CHECK-SERIAL`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-ALLOC-27/20260412-2201-checker-serial-packet.md`
- Supersedes: None
- Role: `checker`
- Component: `EXR-ALLOC-27`
- Phase: `serial checker`
- Authorizing main agent: `main-agent`
- Date: `2026-04-12 22:01 CST`

## Goal

- Validate the new filesystem-owned allocator boundary, add the required local allocator ktests, run filtered executable verification under the checker lock, and write the checker report.

## Architectural Unit Context

- Functional goal: `ExfatFs`-owned cluster allocation service
- Final architectural owner: `ExfatFs`
- Expected landing form: owner-internal service plus owner methods across `allocator.rs`, `fs.rs`, `bitmap.rs`, and `fat.rs`

## Required Resolution Questions

- Verify free-space search prefers a contiguous run when one exists.
- Verify fragmented allocation is chosen only when contiguous space is insufficient.
- Verify reservation intent does not escape before commit.
- Verify bitmap and FAT state remain coherent after commit.
- If a compile/test failure is strictly local to the component files, make the smallest in-scope fix and record it.

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/CHECKER_REPORT_TEMPLATE.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-ALLOC-27/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-ALLOC-27/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-ALLOC-27/02_designer_async.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-ALLOC-27/03_designer_ktest.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-ALLOC-27/10_creator_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/allocator.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fat.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/test_support.rs`

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/allocator.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fat.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-ALLOC-27/11_checker_serial.md`

## Forbidden Files

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/`
- sibling component artifacts

## Required Inputs

- Skill to use: `$exfat-subagent-workflow`
- Required role reference: `references/checker.md`
- Also read: `references/testing-guide.md`

## Semantic Prior Inputs

- Use the accepted `EXR-ALLOC-27` designer constraints only.
- `EXR-ALLOC-27` remains the owner of search, reservation intent, and commit under `ExfatFs`.
- Keep directory write policy, inode growth policy, truncate semantics, and sync ordering out of scope.

## Integration Prior Inputs

- Prefer placing new checker-owned regressions in `allocator.rs`; use `fs.rs` only if the owner-visible wrapper makes that materially clearer.
- Keep tests local to the allocator owner boundary.
- Do not turn checker work into a review of namespace mutation or directory-entry placement.

## Workflow Prior Inputs

- Runtime/test-producing checker lane.
- Command-producing verification must hold the checker execution lock.
- Add local `#[ktest]` coverage with a stable source-backed prefix. Prefer new names beginning with `allocator_`.
- Prefer exact test names for final proof. If a broad prefix run does not prove the filter hit, rerun the intended tests with exact names derived from the source and record that evidence.
- If a nonzero exit does not clearly show the guest-side failure in terminal output, inspect `/home/halifuda/asterinas/qemu-serial.log` before classifying the result.

## Quality Prior Inputs

- Use the checker-role quality slice from `$exfat-subagent-workflow`.
- Prefer test-only edits unless a strictly local production fix in the component files is necessary to satisfy the designer contract.

## Temporary Interfaces And Exit Plan

- Keep allocator-owned helper surfaces subordinate to `ExfatFs`.
- Do not add a public reservation lease, background allocator worker, or sync/writeback shell.
- If the checker leaves any temporary seam in place, record why it is acceptable for now and which owner should eventually absorb it.

## Helper Justification

- New helper changes are justified only when they keep allocator verification local to the component owner boundary or keep checker-owned local tests readable.

## Allowed Commands

- Read-only shell inspection commands under `/home/halifuda/asterinas`
- Lock-guarded Docker commands in `codex-asterinas-dev`:
  - `docker exec codex-asterinas-dev bash -lc 'test -e /dev/kvm && ls -l /dev/kvm || echo no-kvm'`
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test allocator_'`
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test <exact allocator test name>'`
- If the guest-side failure is unclear after a nonzero run, read-only inspection of `/home/halifuda/asterinas/qemu-serial.log` is allowed.

## Parallelism Classification

- Lane class: `runtime/test-producing`
- May overlap with command-free lanes only
- Known conflicts:
  - serialized checker command lane
  - `allocator.rs`
  - `fs.rs`
  - `bitmap.rs`
  - `fat.rs`

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
  - `bash ./kernel/src/fs/fs_impls/exfat_refactor/.agents/tools/checker_lock.sh acquire --component EXR-ALLOC-27 --phase serial --command "<exact docker commands actually used>" --retry-seconds 60 --wait-budget-seconds 1800`
- Release with:
  - `bash ./kernel/src/fs/fs_impls/exfat_refactor/.agents/tools/checker_lock.sh release`
- Stale-lock review is main-agent-only.

## Stop Condition

- Stop after writing `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-ALLOC-27/11_checker_serial.md`.

## Escalation Rule

- If the checker needs edits outside the authorized component files or cannot get trustworthy filtered-test evidence from the allowed commands, report that and stop.
