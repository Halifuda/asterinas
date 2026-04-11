<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-FS-OPEN-22-20260410-1245-ARCH`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-FS-OPEN-22/20260410-1245-architect-packet.md`
- Supersedes: None
- Role: `architect`
- Component: `EXR-FS-OPEN-22`
- Phase: `architect`
- Authorizing main agent: `main-agent`
- Date: `2026-04-10 12:45 CST`

## Goal

- Architect the stable `ExfatFs` mount/open sequencing unit that absorbs the current `root_inode()` seam and wires boot facts, root publication, and root-directory system-entry discovery through the already accepted internal owners.

## Architectural Unit Context

- Functional goal: implement `ExfatFs::open(...)` or equivalent mount/open sequencing behavior from boot facts to ready root
- Final architectural owner: `ExfatFs`
- Expected landing form: owner methods plus sequencing invariants
- Boundary expectation from board reset: mount/open sequencing is real behavior, but not a separate long-lived owner

## Required Resolution Questions

- Define the smallest functionally coherent open-sequencing unit under `ExfatFs`.
- State how the unit consumes accepted owners: `ExfatFs`, `ExfatInode`, opened-inode cache, `DirectoryEngine`, `UpcaseTable`, and `AllocationBitmap`.
- State what still belongs outside this unit, especially later directory ops, namespace mutation, allocator mutation policy, and sync ordering.
- Make the root-publication handoff explicit so the current `root_inode()` seam has a named absorbing owner.
- Recommend dependency-safe work slices without inventing a standalone mount object or system-root scanner owner.

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/ARCHITECT_TEMPLATE.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/WORKSPACE-ARCH-RESET/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-CORE-16/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CORE-17/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CACHE-18/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-ENGINE-19/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-20/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BITMAP-21/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-CORE-16/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CACHE-18/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `/home/halifuda/asterinas/kernel/src/fs/vfs/fs_apis/file_system.rs`
- `/home/halifuda/asterinas/kernel/src/fs/vfs/fs_apis/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs`

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-OPEN-22/00_architect.md`

## Forbidden Files

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- production code
- designer, creator, checker, advisor, and reviewer artifacts for other components

## Required Inputs

- Skill to use: `$exfat-subagent-workflow`
- Required role reference: `references/architect.md`

## Semantic Prior Inputs

- Use owner-first board-reset reasoning as authoritative for this split.
- Exact Microsoft/Linux behavior is not the main question here; the main question is stable Asterinas owner convergence for mount/open sequencing.

## Integration Prior Inputs

- `EXR-FS-OPEN-22` must absorb the current `root_inode()` temporary seam.
- Do not interpose a separate mount object, system-root scanner owner, or fake root carrier between VFS and `ExfatFs`.

## Workflow Prior Inputs

- Command-free architect lane.
- This is parallel pre-research and must remain architect-only; do not drift into designer or creator detail.

## Quality Prior Inputs

- Use architect-level boundary guidance only.

## Temporary Interfaces And Exit Plan

- Temporary seams may be referenced only to explain how `EXR-FS-OPEN-22` absorbs them.
- Do not authorize a new long-lived staging owner in this architect pass.

## Helper Justification

- Any helper-like surface proposed here must be justified as owner-internal to `ExfatFs` and stable after mount/open lands.

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

- Stop after writing `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-OPEN-22/00_architect.md`

## Escalation Rule

- If accepted prior artifacts are still insufficient to name a stable owner-first mount/open unit, report the exact missing handshake and stop instead of inventing a staging owner.
