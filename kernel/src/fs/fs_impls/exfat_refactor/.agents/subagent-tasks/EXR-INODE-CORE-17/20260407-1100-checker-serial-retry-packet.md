<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-INODE-CORE-17-20260407-1100-CHECK-SERIAL-RETRY`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-INODE-CORE-17/20260407-1100-checker-serial-retry-packet.md`
- Supersedes: `EXR-INODE-CORE-17-20260407-1052-CHECK-SERIAL`
- Role: `checker`
- Component: `EXR-INODE-CORE-17`
- Phase: `serial checker retry`
- Authorizing main agent: `main-agent`
- Date: `2026-04-07 11:00 CST`

## Goal

- Rerun executable verification for `EXR-INODE-CORE-17` now that `codex-asterinas-dev` is running. Preserve the original `11_checker_serial.md` environment-failure record and write a new retry report.

## Architectural Unit Context

- Functional goal: `ExfatInode` VFS metadata carrier.
- Final architectural owner: `ExfatInode`.
- Expected landing form: owner type and `Inode` / `InodeIo` impls in `inode.rs`.

## Required Resolution Questions

- Run the exact source-backed test suffix from `11_checker_serial.md`.
- If compile/test failure is local to `inode.rs`, make a minimal fix and record it. Do not edit `fs.rs` or `mod.rs`.
- Record Docker/KVM observations and exact command evidence.

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/CHECKER.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/TESTING_GUIDE.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/CHECKER_REPORT_TEMPLATE.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CORE-17/11_checker_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CORE-17/12_checker_serial_retry.md`

## Forbidden Files

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- `COMPONENT_INDEX.md`
- main-agent handoffs
- sibling component artifacts.

## Required Inputs

- Prior checker report: `EXR-INODE-CORE-17/11_checker_serial.md`.
- Role protocol: `COMMON_SUBAGENT.md`, `CHECKER.md`.
- Testing guide.

## Semantic Prior Inputs

- Use prior checker source-backed suffix and designer-derived constraints only.

## Integration Prior Inputs

- Use local `Inode` / `InodeIo` / `FileSystem` trait surfaces only as needed.

## Workflow Prior Inputs

- Runtime/test-producing retry checker lane.
- Command execution must hold checker lock.

## Quality Prior Inputs

- Use `Q-CHECK`.

## Temporary Interfaces And Exit Plan

- Preserve explicit data-path and mutation temporary seam behavior.

## Helper Justification

- No new helpers unless required for local tests and clearly test-only.

## Allowed Commands

- Lock-guarded Docker commands:
  - `docker exec codex-asterinas-dev bash -lc 'test -e /dev/kvm && ls -l /dev/kvm || echo no-kvm'`
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test inode_carrier_snapshots_metadata_and_rejects_temporary_seams'`

## Parallelism Classification

- Lane class: runtime/test-producing.
- May overlap with command-free lanes only. Must serialize with other checker execution by lock.

## Execution Environment

- Docker container: `codex-asterinas-dev`.
- Working directory in container: `/root/asterinas/kernel`.

## Execution Lock

- Acquire:
  - `.agents/tools/checker_lock.sh acquire --component EXR-INODE-CORE-17 --phase serial-retry --command "docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test inode_carrier_snapshots_metadata_and_rejects_temporary_seams'" --retry-seconds 60 --wait-budget-seconds 1800`
- Release:
  - `.agents/tools/checker_lock.sh release`
- Stale-lock review is main-agent-only.

## Stop Condition

- Stop after writing `EXR-INODE-CORE-17/12_checker_serial_retry.md`.

## Escalation Rule

- If failures require edits outside `inode.rs`, report them and stop.
