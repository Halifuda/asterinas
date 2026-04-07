<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-INODE-CORE-17-20260407-1120-CHECK-DIAGNOSTIC`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-INODE-CORE-17/20260407-1120-checker-diagnostic-packet.md`
- Supersedes: `EXR-INODE-CORE-17-20260407-1100-CHECK-SERIAL-RETRY`
- Role: `checker`
- Component: `EXR-INODE-CORE-17`
- Phase: `serial checker diagnostic`
- Authorizing main agent: `main-agent`
- Date: `2026-04-07 11:20 CST`

## Goal

- Diagnose the nonzero `cargo osdk test inode_carrier_snapshots_metadata_and_rejects_temporary_seams` result from `12_checker_serial_retry.md`, capture enough evidence to classify the failure, and make only minimal in-scope `inode.rs` fixes if the cause is local to the assigned component or its checker-owned ktest.

## Architectural Unit Context

- Functional goal: `ExfatInode` VFS metadata carrier.
- Final architectural owner: `ExfatInode`.
- Expected landing form: owner type and `Inode` / `InodeIo` impls in `inode.rs`.
- Parent unit: `EXR-FS-CORE-16`.

## Required Resolution Questions

- Rerun the exact filtered ktest and capture enough tail/log detail to classify the failure as environment, build, harness/runtime, or test/assertion failure.
- If the failure is local to the checker-owned ktest or `inode.rs` implementation, make the smallest `inode.rs` fix and rerun the same filter.
- If the failure requires edits outside `inode.rs`, report the blocker and stop.
- Preserve the existing `11_checker_serial.md` and `12_checker_serial_retry.md` records; write a new diagnostic report.

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/CHECKER.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/TESTING_GUIDE.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/CHECKER_REPORT_TEMPLATE.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CORE-17/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CORE-17/03_designer_ktest.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CORE-17/10_creator_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CORE-17/11_checker_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CORE-17/12_checker_serial_retry.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- Read-only inspection commands inside `/home/halifuda/asterinas` are allowed.

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CORE-17/13_checker_serial_diagnostic.md`

## Forbidden Files

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/`
- Sibling component artifacts.

## Required Inputs

- Role-scoped protocol files: `COMMON_SUBAGENT.md`, `CHECKER.md`.
- Testing guide and checker report template.
- Prior checker reports listed above.

## Semantic Prior Inputs

- Use designer-derived constraints only. Do not widen the inode carrier.

## Integration Prior Inputs

- Use the local `Inode` / `InodeIo` / `FileSystem` trait surfaces only as needed.

## Workflow Prior Inputs

- Runtime/test-producing diagnostic lane.
- Command execution must hold `.agents/tools/checker_lock.sh`.
- Production or test fixes are authorized only inside `inode.rs` and only if required to resolve the diagnostic result.

## Quality Prior Inputs

- Use `ASTERINAS_CODE_QUALITY_PRIORS.md` profile `Q-CHECK`.

## Temporary Interfaces And Exit Plan

- Preserve explicit data-path and mutation temporary seam behavior.

## Helper Justification

- No new helpers unless required for a local test fix and clearly test-only.

## Allowed Commands

- Read-only inspection commands.
- Lock-guarded Docker commands:
  - `docker exec codex-asterinas-dev bash -lc 'test -e /dev/kvm && ls -l /dev/kvm || echo no-kvm'`
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test inode_carrier_snapshots_metadata_and_rejects_temporary_seams'`
- If the first diagnostic rerun still exits nonzero without a useful tail, inspect recent kernel test artifacts and logs under `/root/asterinas/kernel/target` or the OSDK output directory using read-only commands inside the same container, and record the exact commands used.
- If a minimal `inode.rs` fix is made, rerun:
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test inode_carrier_snapshots_metadata_and_rejects_temporary_seams'`

## Parallelism Classification

- Lane class: runtime/test-producing.
- May overlap with command-free reviewer/architect/designer lanes only.
- Must serialize command execution with other checker lanes through the checker lock.

## Execution Environment

- Docker container: `codex-asterinas-dev`.
- Working directory in container: `/root/asterinas/kernel`.

## Execution Lock

- Acquire:
  - `.agents/tools/checker_lock.sh acquire --component EXR-INODE-CORE-17 --phase serial-diagnostic --command "docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test inode_carrier_snapshots_metadata_and_rejects_temporary_seams'" --retry-seconds 60 --wait-budget-seconds 1800`
- Release:
  - `.agents/tools/checker_lock.sh release`
- Stale-lock review is main-agent-only.

## Stop Condition

- Stop after writing `EXR-INODE-CORE-17/13_checker_serial_diagnostic.md`.

## Escalation Rule

- If failures require edits outside `inode.rs`, report them and stop.
