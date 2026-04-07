<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-INODE-CORE-17-20260407-1052-CHECK-SERIAL`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-INODE-CORE-17/20260407-1052-checker-serial-packet.md`
- Supersedes: None
- Role: `checker`
- Component: `EXR-INODE-CORE-17`
- Phase: `serial checker`
- Authorizing main agent: `main-agent`
- Date: `2026-04-07 10:52 CST`

## Goal

- Validate the `EXR-INODE-CORE-17` serial implementation against its designer spec, add focused checker-owned ktests if practical, run lock-guarded filtered verification, and record findings.

## Architectural Unit Context

- Functional goal: `ExfatInode` VFS metadata carrier.
- Final architectural owner: `ExfatInode`.
- Landing form: `inode.rs` owner type and `Inode` / `InodeIo` impls.
- Parent unit: none.

## Required Resolution Questions

- Check `inode.rs` only.
- Verify no inode cache, `InodeKey`, page cache, directory ops, or durable write-side mutation leaked in.
- If compile/test failures are local to this component, make minimal in-scope fixes and record them. Do not edit `mod.rs` or `fs.rs`.

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/CHECKER.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/TESTING_GUIDE.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/CHECKER_REPORT_TEMPLATE.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CORE-17/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CORE-17/03_designer_ktest.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CORE-17/10_creator_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/vfs/fs_apis/inode.rs`
- Read-only inspection commands inside `/home/halifuda/asterinas` are allowed.

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CORE-17/11_checker_serial.md`

## Forbidden Files

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- `COMPONENT_INDEX.md`
- main-agent handoffs
- sibling component artifacts.

## Required Inputs

- Role-scoped protocol files: `COMMON_SUBAGENT.md`, `CHECKER.md`.
- Testing guide and checker report template.
- Designer and creator artifacts listed above.

## Semantic Prior Inputs

- Use designer-derived constraints and accepted `fileset.rs` / `fat.rs` behavior only.

## Integration Prior Inputs

- Use local `Inode`, `InodeIo`, and `FileSystem` trait surfaces and testing guide.

## Workflow Prior Inputs

- Checker may prepare tests command-free before acquiring the execution lock.
- Command-producing verification must hold `.agents/tools/checker_lock.sh`.
- Production fixes are authorized only when they are minimal, local to `inode.rs`, and required for compile/spec compliance.

## Quality Prior Inputs

- Use `ASTERINAS_CODE_QUALITY_PRIORS.md` profile `Q-CHECK`.

## Temporary Interfaces And Exit Plan

- `read_at()` and `write_at()` must keep the explicit temporary seam to `EXR-READ-OPS-25`, `EXR-WRITE-30`, and `EXR-PGCACHE-26`.
- `resize()`, `set_mode()`, `set_owner()`, and `set_group()` must reject rather than mutating hidden writeback state.

## Helper Justification

- Fail the pass if `InodeKey`, inode cache helpers, page-cache helpers, or field-exposing accessors appear without designer-backed justification.

## Allowed Commands

- Read-only inspection commands.
- Lock-guarded verification commands in Docker only:
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test <exact-test-suffix>'`
- If the checker adds tests, name them with exact unique function names and use those exact suffixes for filtered execution.
- If adding tests is not practical before compile fixes, run the smallest compile/ktest command that exposes the issue and record why coverage is incomplete.

## Parallelism Classification

- Lane class: runtime/test-producing.
- May overlap with command-free lanes only. It must serialize command execution with any other checker by using the lock.
- Known conflicts: do not edit `fs.rs` or `mod.rs`.

## Execution Environment

- Host or Docker: Docker for command-producing verification.
- Required command prefix: `docker exec codex-asterinas-dev bash -lc`
- Required working directory in container: `/root/asterinas/kernel`
- Filtered tests must prove coverage via exact suffixes or output naming executed tests.

## Execution Lock

- Lock script: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/tools/checker_lock.sh`
- Lock path: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/locks/checker-execution.lock/`
- Acquire command shape:
  - `.agents/tools/checker_lock.sh acquire --component EXR-INODE-CORE-17 --phase serial --command "docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test <exact-test-suffix>'" --retry-seconds 60 --wait-budget-seconds 1800`
- Release with `.agents/tools/checker_lock.sh release` after command-producing verification.
- Stale-lock review is reserved to the main agent.

## Stop Condition

- Stop after writing `EXR-INODE-CORE-17/11_checker_serial.md`. Do not proceed to reviewer.

## Escalation Rule

- If failures require edits outside `inode.rs`, record them as findings for the main agent instead of editing around them.
