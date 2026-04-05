<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-PGCACHE-11B-ARCH-20260405-1128`
- Packet file: `kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-PGCACHE-11B/20260405-1128-architect-packet.md`
- Supersedes: none
- Role: architect
- Component: `EXR-PGCACHE-11B`
- Phase: architect
- Authorizing main agent: main-agent
- Date: 2026-04-05 11:28 CST

## Goal

- Write `EXR-PGCACHE-11B`'s `00_architect.md`. The component should own only page-cache backend integration for the refactored exFAT regular-file path, building on the accepted mount and inode boundaries plus the in-flight `EXR-READ-11A` mapping boundary. Keep buffered `read_at`, write-side growth, and namespace work out of scope.

## Read Set

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/README.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/ARCHITECT_TEMPLATE.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/ARCHITECT.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-MOUNT-09/01_designer_core.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-05B/01_designer_core.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-READ-11A/00_architect.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-READ-11A/01_designer_core.md`
- `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `kernel/src/fs/vfs/page_cache.rs`
- `kernel/src/fs/fs_impls/exfat/inode.rs`
- `/home/halifuda/linux/fs/exfat/inode.c`

## Write Set

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-11B/00_architect.md`

## Forbidden Files

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/*`
- any file under `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-11B/` other than `00_architect.md`
- all production Rust code

## Required Inputs

- Role protocol files accompanying this packet:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/ARCHITECT.md`
- Required artifact/style inputs:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/ARCHITECT_TEMPLATE.md`
  - accepted `EXR-MOUNT-09` and `EXR-INODE-05B` boundaries
  - current `EXR-READ-11A` architect and designer boundaries

## Semantic Prior Inputs

- Use:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/Microsoft-exFAT-spec.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/linux-exFAT-implementation-summary.md`
  - Linux `inode.c` as needed for page-cache and block-mapping separation
- Precedence:
  - Microsoft exFAT rules first
  - Linux exFAT implementation guidance second
- Semantic focus:
  - regular-file cached read integration
  - separation between physical mapping, cache backend ownership, and user-visible buffered reads

## Local Architectural Prior Inputs

- Use selected integration constraints from:
  - `EXR-MOUNT-09`
  - `EXR-INODE-05B`
  - `EXR-READ-11A`
  - `kernel/src/fs/vfs/page_cache.rs`
- Local focus:
  - keep this component on `PageCacheBackend` ownership only
  - make `EXR-READ-11B` the later owner of buffered `read_at`
  - avoid collapsing backend wiring and read policy into one component

## Quality Prior Inputs

- Use `Q-ARCH` from `kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_CODE_QUALITY_PRIORS.md`
- In scope:
  - narrow ownership around cache backend responsibilities
  - explicit dependency placement against `EXR-READ-11A` and `EXR-READ-11B`
  - avoiding helper or API drift that would pre-commit buffered read semantics here
- Out of scope:
  - creator-local implementation detail

## Prior Delivery Notes

- Keep the split narrow enough that it covers page-cache backend hooks and cache-size coordination only.
- Explicitly say what `EXR-READ-11B` should still own after this component lands.
- If legacy exFAT couples backend and read paths, use that only as a contrast, not as a target to copy.

## Temporary Interfaces And Exit Plan

- No temporary interface is authorized.

## Helper Justification

- No helper API is pre-authorized by this packet beyond the eventual page-cache-backend boundary itself.

## Allowed Commands

- read-only shell inspection commands only

## Parallelism Classification

- Lane class:
  - command-free
- May overlap with:
  - the local `EXR-READ-11A` creator/checker/reviewer flow
  - `EXR-READ-11B` architect work
  - other command-free lanes with disjoint write sets
- Known conflicts:
  - any lane writing `EXR-PGCACHE-11B` architect artifacts

## Execution Environment

- Host or Docker:
  - host workspace only
- Required command prefix:
  - read-only shell commands under `/home/halifuda/asterinas`
- Required working directory:
  - `/home/halifuda/asterinas`
- Isolation notes:
  - shared worktree; do not edit outside the write set
- This task is command-free and must not add compile or runtime commands.

## Execution Lock

- Lock script:
  - not applicable
- Lock path:
  - not applicable
- Lock metadata file:
  - not applicable

## Stop Condition

- Stop after writing `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-11B/00_architect.md`.
- Do not write designer artifacts, update the task board, or touch production code.

## Escalation Rule

- If the split appears to require buffered `read_at`, write-side growth, or namespace behavior in the same component, stop and report the exact pressure instead of widening scope.
