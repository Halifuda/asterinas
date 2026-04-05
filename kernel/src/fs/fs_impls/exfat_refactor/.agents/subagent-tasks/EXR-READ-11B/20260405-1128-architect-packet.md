<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-READ-11B-ARCH-20260405-1128`
- Packet file: `kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-READ-11B/20260405-1128-architect-packet.md`
- Supersedes: none
- Role: architect
- Component: `EXR-READ-11B`
- Phase: architect
- Authorizing main agent: main-agent
- Date: 2026-04-05 11:28 CST

## Goal

- Write `EXR-READ-11B`'s `00_architect.md`. The component should own buffered regular-file `read_at` and read-side zero-fill behavior on top of the accepted mount state and the in-flight `EXR-READ-11A` mapping boundary, while keeping page-cache backend ownership in `EXR-PGCACHE-11B` and keeping write-side growth out of scope.

## Read Set

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/README.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/ARCHITECT_TEMPLATE.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/ARCHITECT.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-MOUNT-09/01_designer_core.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-READ-11A/00_architect.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-READ-11A/01_designer_core.md`
- `kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `kernel/src/fs/vfs/fs_apis/inode.rs`
- `kernel/src/fs/vfs/page_cache.rs`
- `kernel/src/fs/fs_impls/exfat/inode.rs`
- `/home/halifuda/linux/fs/exfat/file.c`
- `/home/halifuda/linux/fs/exfat/inode.c`

## Write Set

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-READ-11B/00_architect.md`

## Forbidden Files

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/*`
- any file under `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-READ-11B/` other than `00_architect.md`
- all production Rust code

## Required Inputs

- Role protocol files accompanying this packet:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/ARCHITECT.md`
- Required artifact/style inputs:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/ARCHITECT_TEMPLATE.md`
  - accepted `EXR-MOUNT-09` boundary
  - current `EXR-READ-11A` architect and designer boundaries

## Semantic Prior Inputs

- Use:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/Microsoft-exFAT-spec.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/linux-exFAT-implementation-summary.md`
  - Linux `file.c` and `inode.c` as needed for buffered read behavior, mapping boundaries, and zero-fill semantics
- Precedence:
  - Microsoft exFAT rules first
  - Linux exFAT implementation guidance second
- Semantic focus:
  - regular-file buffered reads
  - EOF and zero-fill behavior
  - separation between buffered copy policy and lower mapping or cache-backend ownership

## Local Architectural Prior Inputs

- Use selected integration constraints from:
  - `EXR-MOUNT-09`
  - `EXR-READ-11A`
  - `kernel/src/fs/vfs/fs_apis/inode.rs`
  - `kernel/src/fs/vfs/page_cache.rs`
- Local focus:
  - buffered `read_at` should depend on the mapping layer and page cache backend rather than redoing either
  - page-cache backend ownership belongs in `EXR-PGCACHE-11B`
  - write-side growth and truncation stay out of scope

## Quality Prior Inputs

- Use `Q-ARCH` from `kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_CODE_QUALITY_PRIORS.md`
- In scope:
  - narrow buffered-read ownership
  - explicit dependency story against `EXR-READ-11A` and `EXR-PGCACHE-11B`
  - avoiding drift into write-side allocation or mount bootstrap
- Out of scope:
  - creator-local implementation detail

## Prior Delivery Notes

- Keep the slice narrow enough that it is about read execution only.
- Make the handoff explicit about which lower-layer placement and cache responsibilities stay elsewhere.
- If the legacy implementation bundles write-side work into the same file, separate that concern in the architect result instead of inheriting it.

## Temporary Interfaces And Exit Plan

- No temporary interface is authorized.

## Helper Justification

- No helper API is pre-authorized by this packet beyond the eventual buffered-read boundary itself.

## Allowed Commands

- read-only shell inspection commands only

## Parallelism Classification

- Lane class:
  - command-free
- May overlap with:
  - the local `EXR-READ-11A` creator/checker/reviewer flow
  - `EXR-PGCACHE-11B` architect work
  - other command-free lanes with disjoint write sets
- Known conflicts:
  - any lane writing `EXR-READ-11B` architect artifacts

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

- Stop after writing `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-READ-11B/00_architect.md`.
- Do not write designer artifacts, update the task board, or touch production code.

## Escalation Rule

- If the split appears to require owning page-cache backend hooks, mount bootstrap, or write-side allocation growth in the same component, stop and report the exact pressure instead of widening scope.
