<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-PGCACHE-26-20260412-1112-ARCH`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-PGCACHE-26/20260412-1112-architect-packet.md`
- Supersedes: None
- Role: `architect`
- Component: `EXR-PGCACHE-26`
- Phase: `architect`
- Authorizing main agent: `main-agent`
- Date: `2026-04-12 11:12 CST`

## Goal

- Architect the stable `ExfatInode` page-cache integration boundary so later work can add `PageCacheBackend` ownership without absorbing buffered read, write-side growth, or a filesystem-global cache service.

## Architectural Unit Context

- Functional goal: `PageCacheBackend` integration for exFAT inodes
- Final architectural owner: `ExfatInode`
- Expected landing form: owner-internal state plus trait impl
- Boundary expectation from board reset:
  - `EXR-READ-OPS-25` owns buffered read behavior first
  - `EXR-PGCACHE-26` adds cache integration after the buffered-read owner exists
  - later write-side rows still own growth, truncate, and dirty persistence policy

## Required Resolution Questions

- Define the smallest stable page-cache integration unit under `ExfatInode`.
- State how `EXR-PGCACHE-26` consumes the buffered-read owner from `EXR-READ-OPS-25` without re-owning buffered read semantics.
- State what cache state and trait surfaces are architecturally real now, and what still belongs to later write-side and sync owners.
- Keep allocator mutation, directory behavior, namespace mutation, and filesystem-global cache services out of scope.
- Recommend dependency-safe work slices without inventing a separate cache manager owner.

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/ARCHITECT_TEMPLATE.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/WORKSPACE-ARCH-RESET/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CORE-17/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CORE-17/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-READ-OPS-25/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FILE-MAP-24/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FILE-MAP-24/10_creator_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/vfs/fs_apis/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/vfs/page_cache.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat/inode.rs`

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-26/00_architect.md`

## Forbidden Files

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- production code
- designer, creator, checker, advisor, and reviewer artifacts for any component

## Required Inputs

- Skill to use: `$exfat-subagent-workflow`
- Required role reference: `references/architect.md`

## Semantic Prior Inputs

- Use owner-first board-reset reasoning as authoritative for this split.
- `EXR-READ-OPS-25` remains the first owner of buffered byte transfer. `EXR-PGCACHE-26` must consume that owner boundary rather than replace it.
- Page-cache integration is inode-owned, not filesystem-global.

## Integration Prior Inputs

- Use `kernel/src/fs/vfs/page_cache.rs` as the trait and cache-owner context for this row.
- Legacy `kernel/src/fs/fs_impls/exfat/inode.rs` is integration reference material for page-cache shape only; it does not override the owner-first refactor boundary.
- The current `EXR-FILE-MAP-24` creator artifact is relevant only as current read-path dependency context; do not redesign mapping inside this architect pass.

## Workflow Prior Inputs

- Command-free architect lane.
- This lane may overlap with `EXR-FILE-MAP-24` checker and `EXR-READ-OPS-25` designer work because the write set is artifact-only and disjoint.
- Stay architect-only; do not drift into designer details or production edit plans beyond boundary-safe work-slice recommendations.

## Quality Prior Inputs

- Use architect-level boundary guidance only.

## Temporary Interfaces And Exit Plan

- No production temporary interface is authorized in this architect pass unless the artifact names the future owner or removal condition.
- Do not authorize a filesystem-global cache service, writeback manager, or read-policy shell.

## Helper Justification

- Any helper-like surface proposed here must remain subordinate to `ExfatInode` page-cache ownership and justified by stable cache integration rather than packet convenience.

## Allowed Commands

- Read-only shell inspection commands under `/home/halifuda/asterinas`

## Parallelism Classification

- Lane class: `command-free planning`
- May overlap with:
  - `EXR-FILE-MAP-24` checker
  - `EXR-READ-OPS-25` designer

## Execution Environment

- Host workspace only

## Execution Lock

- None

## Stop Condition

- Stop after writing `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-26/00_architect.md`

## Escalation Rule

- If the page-cache boundary cannot be defined without absorbing buffered read semantics, write-side growth, or filesystem-global cache ownership, report the exact missing dependency and stop instead of inventing a staging owner.
