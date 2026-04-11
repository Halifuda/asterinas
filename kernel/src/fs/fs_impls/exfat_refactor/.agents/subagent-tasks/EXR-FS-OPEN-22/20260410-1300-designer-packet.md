<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-FS-OPEN-22-20260410-1300-DESIGN`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-FS-OPEN-22/20260410-1300-designer-packet.md`
- Supersedes: None
- Role: `designer`
- Component: `EXR-FS-OPEN-22`
- Phase: `design`
- Authorizing main agent: `main-agent`
- Date: `2026-04-10 13:00 CST`

## Goal

- Turn the accepted `EXR-FS-OPEN-22` architect boundary into a bounded designer spec set that defines how `ExfatFs` mount/open sequencing absorbs the current `root_inode()` seam without widening into later directory mutation, allocator mutation, or sync behavior.

## Architectural Unit Context

- Functional goal: `ExfatFs::open(...)` or equivalent mount/open sequencing from boot facts to ready root
- Final architectural owner: `ExfatFs`
- Owner class: `structure owner`
- Expected landing form: owner methods plus sequencing invariants
- Boundary kind: stable architectural boundary

## Required Resolution Questions

- Specify the minimal owner-method sequencing from trusted boot facts to a published root inode.
- Specify how mount/open consumes the accepted internal owners: opened-inode cache, `DirectoryEngine`, `UpcaseTable`, and `AllocationBitmap`.
- Make the root-publication handoff explicit so the existing `root_inode()` seam has a named exit path.
- State serialization / sequencing obligations clearly enough that creator work does not have to guess lock/order expectations.
- Keep later directory ops, namespace mutation, allocator mutation, read/write data paths, and sync ordering out of this unit.

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-OPEN-22/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-CORE-16/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CACHE-18/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-ENGINE-19/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-20/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BITMAP-21/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/vfs/fs_apis/file_system.rs`
- `/home/halifuda/asterinas/kernel/src/fs/vfs/fs_apis/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs`

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-OPEN-22/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-OPEN-22/02_designer_async.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-OPEN-22/03_designer_ktest.md`

## Forbidden Files

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- production code
- sibling component artifacts

## Required Inputs

- Skill to use: `$exfat-subagent-workflow`
- Required role reference: `references/designer.md`

## Semantic Prior Inputs

- Use the accepted architect boundary only. Do not reopen mount/open as a separate owner and do not invent a new mount object, scanner owner, or fake root carrier.

## Integration Prior Inputs

- The root-publication seam from `EXR-FS-CORE-16` must be absorbed here.
- `DirectoryEngine`, `UpcaseTable`, and `AllocationBitmap` are already separate accepted or specified owners; consume them, do not redefine them.

## Workflow Prior Inputs

- Command-free designer lane.
- Produce the full designer artifact set because this unit has meaningful sequencing/serialization obligations.

## Quality Prior Inputs

- Use designer-role quality guidance only.

## Temporary Interfaces And Exit Plan

- Temporary seams may be specified only when they name a later absorbing owner or explicit stop condition.
- Do not leave `root_inode()` as an indefinite staging seam after this unit lands.

## Helper Justification

- Any helper or sequencing sub-step described here must remain owner-local to `ExfatFs` and justified by mount/open behavior, not by packet convenience.

## Allowed Commands

- Read-only shell inspection commands under `/home/halifuda/asterinas`

## Parallelism Classification

- Lane class: `command-free planning`
- May overlap with the active creator round because the write set is artifact-only

## Execution Environment

- Host workspace only

## Execution Lock

- None

## Stop Condition

- Stop after writing the three designer artifacts for `EXR-FS-OPEN-22`.

## Escalation Rule

- If the accepted architect result is still too coarse to produce narrow creator-ready specs, report that and stop instead of silently widening scope.
