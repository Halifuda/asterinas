<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-DIR-OPS-23-20260410-1545-DESIGN`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-DIR-OPS-23/20260410-1545-designer-packet.md`
- Supersedes: None
- Role: `designer`
- Component: `EXR-DIR-OPS-23`
- Phase: `designer`
- Authorizing main agent: `main-agent`
- Date: `2026-04-10 15:45 CST`

## Goal

- Produce the split designer artifact set for `EXR-DIR-OPS-23` so later creator work can implement `ExfatInode::lookup` and `ExfatInode::readdir_at` as read-only directory methods without guessing about owner boundaries, helper shape, or test obligations.

## Architectural Unit Context

- Functional goal: `ExfatInode` read-only directory operations
- Final architectural owner: `ExfatInode`
- Expected landing form: owner methods in `inode.rs`
- Architect artifact:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-OPS-23/00_architect.md`

## Required Resolution Questions

- Specify the read-only `lookup` and `readdir_at` behavior on `ExfatInode`.
- Make clear how the row consumes `DirectoryEngine`, `UpcaseTable`, and `ExfatFs`-owned opened-inode reuse without absorbing those owners.
- Keep root publication and mount/open sequencing out of scope except as prerequisites already handled by `EXR-FS-OPEN-22`.
- Define creator and checker obligations narrowly enough that later work does not guess where lookup ends and mutation begins.
- State the serialization and local invariant expectations for repeated lookup / readdir behavior.

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/DESIGNER_CORE_TEMPLATE.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/DESIGNER_ASYNC_TEMPLATE.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/DESIGNER_KTEST_TEMPLATE.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-OPS-23/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CORE-17/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-ENGINE-19/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-ENGINE-19/03_designer_ktest.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-20/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-OPEN-22/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/vfs/fs_apis/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-OPS-23/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-OPS-23/02_designer_async.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-OPS-23/03_designer_ktest.md`

## Forbidden Files

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- production code
- creator, checker, advisor, and reviewer artifacts for any component

## Required Inputs

- Skill to use: `$exfat-subagent-workflow`
- Required role reference: `references/designer.md`

## Semantic Prior Inputs

- Use the accepted architect boundary only.
- Keep `lookup` and `readdir_at` read-only.
- Keep name folding and hashing dependent on the accepted `UpcaseTable` owner.
- Keep opened-inode reuse dependent on the accepted `ExfatFs` owner.

## Integration Prior Inputs

- `EXR-FS-OPEN-22` owns mount/open sequencing and ready-root publication; this row begins after that input exists.
- `DirectoryEngine` remains an `ExfatFs`-owned service, not a new VFS-facing owner.
- `EXR-NAMESPACE-29` will own mutation later, so this row must not smuggle in create/unlink/rename semantics.

## Workflow Prior Inputs

- Command-free designer lane.
- Stay designer-only: do not drift into creator details beyond creator-ready obligations.
- Produce the standard split artifact set: core, async, and ktest.

## Quality Prior Inputs

- Use the designer-role quality slice from `$exfat-subagent-workflow`.

## Temporary Interfaces And Exit Plan

- Do not authorize a separate lookup service, scanner owner, or mutation shell.
- If a helper is needed, keep it explicitly subordinate to `ExfatInode` or to consumed `DirectoryEngine` behavior.

## Allowed Commands

- Read-only shell inspection commands under `/home/halifuda/asterinas`

## Parallelism Classification

- Lane class: `command-free planning`
- May overlap with the single creator round because the write set is artifact-only

## Execution Environment

- Host workspace only

## Execution Lock

- None

## Stop Condition

- Stop after writing:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-OPS-23/01_designer_core.md`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-OPS-23/02_designer_async.md`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-OPS-23/03_designer_ktest.md`

## Escalation Rule

- If the row cannot be specified cleanly without reopening mount sequencing, mutation, or a separate lookup-service boundary, report that exact boundary problem and stop.
