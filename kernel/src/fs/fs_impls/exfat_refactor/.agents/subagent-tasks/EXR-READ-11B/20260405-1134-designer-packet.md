<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-READ-11B-DESIGN-20260405-1134`
- Packet file: `kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-READ-11B/20260405-1134-designer-packet.md`
- Supersedes: none
- Role: designer
- Component: `EXR-READ-11B`
- Phase: designer
- Authorizing main agent: main-agent
- Date: 2026-04-05 11:34 CST

## Goal

- Produce the bounded designer artifact set for `EXR-READ-11B`: `01_designer_core.md` and `03_designer_ktest.md`, plus `02_designer_async.md` only if the component has concurrency or serialization obligations that later roles cannot safely infer from the core spec.

## Read Set

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/README.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/DESIGNER_SPEC_TEMPLATE.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/DESIGNER.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-READ-11B/00_architect.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-MOUNT-09/01_designer_core.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-READ-11A/01_designer_core.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-11B/00_architect.md`
- `kernel/src/fs/fs_impls/exfat_refactor/read.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `kernel/src/fs/vfs/fs_apis/inode.rs`
- `kernel/src/fs/vfs/page_cache.rs`
- `kernel/src/fs/fs_impls/exfat/inode.rs`
- `/home/halifuda/linux/fs/exfat/file.c`
- `/home/halifuda/linux/fs/exfat/inode.c`

## Write Set

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-READ-11B/01_designer_core.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-READ-11B/02_designer_async.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-READ-11B/03_designer_ktest.md`

## Forbidden Files

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/*`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-READ-11B/00_architect.md`
- all production Rust code

## Required Inputs

- Role protocol files accompanying this packet:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/DESIGNER.md`
- Required artifact/style inputs:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/DESIGNER_SPEC_TEMPLATE.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-READ-11B/00_architect.md`

## Semantic Prior Inputs

- Use:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/Microsoft-exFAT-spec.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/linux-exFAT-implementation-summary.md`
  - Linux `file.c` and `inode.c` as needed for buffered read execution and zero-fill behavior
- Precedence:
  - Microsoft exFAT rules first
  - Linux exFAT implementation guidance second
- Semantic focus:
  - buffered regular-file reads
  - read-side zero-fill and EOF behavior
  - keeping buffered read execution separate from mapping and cache-backend ownership

## Local Architectural Prior Inputs

- Use integration constraints derived from:
  - `EXR-READ-11B/00_architect.md`
  - `EXR-MOUNT-09/01_designer_core.md`
  - `EXR-READ-11A/01_designer_core.md`
  - `EXR-PGCACHE-11B/00_architect.md`
  - `kernel/src/fs/vfs/fs_apis/inode.rs`
  - `kernel/src/fs/vfs/page_cache.rs`
- Local focus:
  - one buffered-read entry point
  - zero-fill behavior driven by the accepted valid-data boundary
  - no page-cache backend ownership here
  - no write-side growth or truncate behavior

## Quality Prior Inputs

- Use `Q-DESIGN` from `kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_CODE_QUALITY_PRIORS.md`
- In scope:
  - one canonical buffered-read surface
  - narrowly justified helper boundaries only when a named downstream caller needs them
  - checker-owned local ktest obligations
- Out of scope:
  - creator-local naming or formatting choices

## Prior Delivery Notes

- Keep the component above mapping and page-cache backend ownership, not mixed with them.
- If `02_designer_async.md` is omitted, say explicitly why buffered read execution does not need a separate async artifact yet.
- Do not specify write-side growth, allocation, or namespace behavior here.

## Temporary Interfaces And Exit Plan

- No temporary interface is authorized.

## Helper Justification

- Any helper or accessor must name the downstream buffered-read caller that needs it now and explain why the boundary cannot stay narrower.

## Allowed Commands

- read-only shell inspection commands only

## Parallelism Classification

- Lane class:
  - command-free
- May overlap with:
  - the local `EXR-READ-11A` creator/checker/reviewer flow
  - `EXR-PGCACHE-11B` designer work
  - other command-free lanes with disjoint write sets
- Known conflicts:
  - any lane writing `EXR-READ-11B` designer artifacts

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

- Stop after writing the required designer artifacts for `EXR-READ-11B`.
- Do not write creator artifacts, update the board, or touch production code.

## Escalation Rule

- If the design cannot stay confined to buffered regular-file reads and read-side zero-fill, stop and report exactly what pressure is trying to pull mapping, backend ownership, or write-side behavior into this component.
