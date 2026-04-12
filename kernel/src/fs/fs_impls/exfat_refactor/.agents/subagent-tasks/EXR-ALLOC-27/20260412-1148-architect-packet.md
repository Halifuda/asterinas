<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-ALLOC-27-20260412-1148-ARCH`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-ALLOC-27/20260412-1148-architect-packet.md`
- Supersedes: None
- Role: `architect`
- Component: `EXR-ALLOC-27`
- Phase: `architect`
- Authorizing main agent: `main-agent`
- Date: `2026-04-12 11:48 CST`

## Goal

- Architect the stable `ExfatFs` allocation-service boundary so later work can search free space and coordinate bitmap plus FAT mutation without folding in directory-entry writes, inode growth policy, or sync ordering.

## Architectural Unit Context

- Functional goal: `ExfatFs`-owned cluster allocation service
- Final architectural owner: `ExfatFs`
- Expected landing form: owner-internal service (`Allocator`) plus owner methods
- Boundary expectation from board reset:
  - `EXR-BITMAP-21` already owns read-only allocation bitmap state
  - `EXR-FATVAL-03A` and `EXR-IO-02` already own the FAT decode and geometry inputs the allocator will consume
  - later namespace, write, and sync rows consume allocation results but do not replace the allocator owner

## Required Resolution Questions

- Define the smallest stable allocation-service unit under `ExfatFs`.
- State how `EXR-ALLOC-27` consumes accepted bitmap state and FAT helpers without reopening bitmap ownership, FAT value ownership, or low-level I/O ownership.
- State what belongs in allocator search and reservation policy now, and what remains for later directory-entry writes, inode growth, writeback, and sync owners.
- Keep directory-entry mutation, file-size policy, truncate semantics, and filesystem-global sync ordering out of scope.
- Recommend dependency-safe creator slices without inventing a standalone free-space manager outside `ExfatFs`.

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/ARCHITECT_TEMPLATE.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/WORKSPACE-ARCH-RESET/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-IO-02/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FATVAL-03A/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-CHAIN-03B/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BITMAP-21/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BITMAP-21/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BITMAP-21/10_creator_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BITMAP-21/30_reviewer_report.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/linux-exFAT-implementation-summary.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fat.rs`

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-ALLOC-27/00_architect.md`

## Forbidden Files

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- production code
- designer, creator, checker, advisor, and reviewer artifacts for any component

## Required Inputs

- Skill to use: `$exfat-subagent-workflow`
- Required role reference: `references/architect.md`

## Semantic Prior Inputs

- Use owner-first board-reset reasoning as authoritative for this split.
- `EXR-BITMAP-21` remains the owner of read-only allocation bitmap state and occupancy/accounting queries.
- `EXR-ALLOC-27` is the first owner of allocation search, reservation intent, and the bitmap/FAT mutation handshake under `ExfatFs`.

## Integration Prior Inputs

- Consume the accepted bitmap state and existing FAT helpers as prerequisites; do not redesign those rows inside this architect pass.
- Use the local Linux summary only as an orientation aid for allocator shape. It does not override the refactor boundary.
- Keep later consumers such as `EXR-DENTRY-WRITE-28`, `EXR-NAMESPACE-29`, and `EXR-WRITE-30` out of scope except as downstream dependencies.

## Workflow Prior Inputs

- Command-free architect lane.
- This lane may overlap with `EXR-FILE-MAP-24` checker/reviewer preparation and `EXR-READ-OPS-25` designer closure because the write set is artifact-only and disjoint.
- Stay architect-only; do not drift into designer details or implementation planning beyond boundary-safe work-slice recommendations.

## Quality Prior Inputs

- Use architect-level boundary guidance only.

## Temporary Interfaces And Exit Plan

- No production temporary interface is authorized in this architect pass unless the artifact names the future owner or removal condition.
- Do not authorize a standalone allocator crate/service, inode-local allocator, or sync/writeback manager here.

## Helper Justification

- Any helper-like surface proposed here must remain subordinate to `ExfatFs` allocator ownership and justified by stable mutation ownership rather than packet convenience.

## Allowed Commands

- Read-only shell inspection commands under `/home/halifuda/asterinas`

## Parallelism Classification

- Lane class: `command-free planning`
- May overlap with:
  - `EXR-FILE-MAP-24` checker or reviewer
  - `EXR-READ-OPS-25` closure work
  - `EXR-PGCACHE-26` later designer planning

## Execution Environment

- Host workspace only

## Execution Lock

- None

## Stop Condition

- Stop after writing `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-ALLOC-27/00_architect.md`

## Escalation Rule

- If the allocator boundary cannot be defined without absorbing directory-entry writes, inode growth/truncate policy, or sync ordering, report the exact missing dependency and stop instead of inventing a staging owner.
