<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-DENTRY-WRITE-28-20260412-1217-ARCH`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-DENTRY-WRITE-28/20260412-1217-architect-packet.md`
- Supersedes: None
- Role: `architect`
- Component: `EXR-DENTRY-WRITE-28`
- Phase: `architect`
- Authorizing main agent: `main-agent`
- Date: `2026-04-12 12:17 CST`

## Goal

- Architect the stable directory-entry write boundary so later work can update on-disk file-record sets through `DirectoryEngine` while consuming allocation results and validated fileset state without folding in namespace policy, inode publication, or sync ordering.

## Architectural Unit Context

- Functional goal: directory-entry update primitives for later create/delete/rename work
- Final architectural owner: `ExfatFs` internal `DirectoryEngine` write methods, consumed later by `ExfatInode` namespace owners
- Expected landing form: `DirectoryEngine` write methods plus narrow owner wiring
- Boundary expectation from board reset:
  - `EXR-DIR-ENGINE-19` already owns directory record streaming under `ExfatFs`
  - `EXR-FILESET-04B` already owns validated file-record shape and serialization
  - `EXR-ALLOC-27` now owns allocation search, reservation, and committed allocation results
  - later `EXR-NAMESPACE-29` consumes this row rather than re-owning raw directory mutation

## Required Resolution Questions

- Define the smallest stable write-side directory-record unit under the existing `DirectoryEngine` owner boundary.
- State how `EXR-DENTRY-WRITE-28` consumes validated `ExfatDentrySet` data and committed allocation results without reopening fileset ownership or allocation ownership.
- State which operations belong here now: slot discovery, record placement/removal/update, tombstoning/overwrite policy, and on-disk serialization boundaries.
- Keep name policy, lookup semantics, inode publication, high-level namespace decisions, file-size policy, and sync ordering out of scope.
- Recommend dependency-safe creator slices without inventing a separate directory-write manager detached from `ExfatFs`.

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/ARCHITECT_TEMPLATE.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/WORKSPACE-ARCH-RESET/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-ENGINE-19/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FILESET-04B/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-OPS-23/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-ALLOC-27/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/linux-exFAT-implementation-summary.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fileset.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/dentry.rs`

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DENTRY-WRITE-28/00_architect.md`

## Forbidden Files

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- production code
- designer, creator, checker, advisor, and reviewer artifacts for any component

## Required Inputs

- Skill to use: `$exfat-subagent-workflow`
- Required role reference: `references/architect.md`

## Semantic Prior Inputs

- Use owner-first board-reset reasoning as authoritative for this split.
- `EXR-DIR-ENGINE-19` remains the stable `ExfatFs`-owned directory engine. This row should extend that owner with write-side primitives rather than inventing a new manager.
- `EXR-FILESET-04B` remains the validated file-record boundary.
- `EXR-ALLOC-27` remains the owner of allocation search, reservation, and committed allocation results.

## Integration Prior Inputs

- Consume the current `directory.rs` owner shape as the read-side foundation for later write methods; do not redesign directory streaming inside this architect pass.
- Keep later consumers such as `EXR-NAMESPACE-29`, `EXR-WRITE-30`, and `EXR-SYNC-31` out of scope except as downstream dependencies the boundary must serve.
- Use the local Linux summary only as orientation for directory-update shape. It does not override the refactor boundary.

## Workflow Prior Inputs

- Command-free architect lane.
- This lane may overlap with the active `EXR-READ-OPS-25` checker lane because the write set is artifact-only and disjoint.
- Stay architect-only; do not drift into designer detail or implementation planning beyond boundary-safe work-slice recommendations.

## Quality Prior Inputs

- Use architect-level boundary guidance only.

## Temporary Interfaces And Exit Plan

- No production temporary interface is authorized in this architect pass unless the artifact names the future owner or removal condition.
- Do not authorize a standalone directory-write manager, namespace helper service, or sync/writeback layer here.

## Helper Justification

- Any helper-like surface proposed here must remain subordinate to `DirectoryEngine` write ownership and justified by stable on-disk directory mutation boundaries rather than packet convenience.

## Allowed Commands

- Read-only shell inspection commands under `/home/halifuda/asterinas`

## Parallelism Classification

- Lane class: `command-free planning`
- May overlap with:
  - `EXR-READ-OPS-25` checker
  - later artifact-only planning for `EXR-NAMESPACE-29` or `EXR-WRITE-30`

## Execution Environment

- Host workspace only

## Execution Lock

- None

## Stop Condition

- Stop after writing `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DENTRY-WRITE-28/00_architect.md`

## Escalation Rule

- If the directory-write boundary cannot be defined without absorbing namespace policy, inode publication, allocator ownership, or sync ordering, report the exact missing dependency and stop instead of inventing a staging owner.
