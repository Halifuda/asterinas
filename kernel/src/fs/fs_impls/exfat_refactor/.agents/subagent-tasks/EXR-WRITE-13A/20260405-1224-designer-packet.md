<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-WRITE-13A-DESIGN-20260405-1224`
- Packet file: `kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-WRITE-13A/20260405-1224-designer-packet.md`
- Supersedes: none
- Role: designer
- Component: `EXR-WRITE-13A`
- Phase: designer
- Authorizing main agent: main-agent
- Date: 2026-04-05 12:24 CST

## Goal

- Produce the bounded designer artifact set for `EXR-WRITE-13A`: `01_designer_core.md` and `03_designer_ktest.md`, plus `02_designer_async.md` only if the growth path introduces concurrency or serialization obligations that later roles cannot safely infer from the core spec.

## Read Set

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/README.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/DESIGNER_SPEC_TEMPLATE.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/DESIGNER.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-13A/00_architect.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-MOUNT-09/01_designer_core.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-05B/01_designer_core.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-READ-11A/01_designer_core.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-11B/00_architect.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-READ-11B/00_architect.md`
- `kernel/src/fs/fs_impls/exfat_refactor/fat.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `kernel/src/fs/fs_impls/exfat/inode.rs`
- `kernel/src/fs/fs_impls/exfat/fs.rs`
- `/home/halifuda/linux/fs/exfat/file.c`
- `/home/halifuda/linux/fs/exfat/inode.c`

## Write Set

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-13A/01_designer_core.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-13A/02_designer_async.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-13A/03_designer_ktest.md`

## Forbidden Files

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/*`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-13A/00_architect.md`
- all production Rust code

## Required Inputs

- Role protocol files accompanying this packet:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/DESIGNER.md`
- Required artifact/style inputs:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/DESIGNER_SPEC_TEMPLATE.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-13A/00_architect.md`

## Semantic Prior Inputs

- Use:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/Microsoft-exFAT-spec.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/linux-exFAT-implementation-summary.md`
  - Linux `file.c` and `inode.c` as needed for growth, allocation publication, and size semantics
- Precedence:
  - Microsoft exFAT rules first
  - Linux exFAT implementation guidance second
- Semantic focus:
  - allocation growth and chain publication
  - allocated length vs valid-data-length boundaries
  - keeping buffered writes and truncate work outside this component

## Local Architectural Prior Inputs

- Use integration constraints derived from:
  - `EXR-WRITE-13A/00_architect.md`
  - `EXR-MOUNT-09/01_designer_core.md`
  - `EXR-INODE-05B/01_designer_core.md`
  - `EXR-READ-11A/01_designer_core.md`
  - current downstream `EXR-PGCACHE-11B` and `EXR-READ-11B` architect boundaries
- Local focus:
  - growth-only ownership
  - later buffered-write ownership in `EXR-WRITE-13B`
  - later truncate/shrink ownership in `EXR-WRITE-13C`
  - no namespace or sync drift

## Quality Prior Inputs

- Use `Q-DESIGN` from `kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_CODE_QUALITY_PRIORS.md`
- In scope:
  - one canonical growth surface
  - narrowly justified helper boundaries only when a named downstream caller needs them
  - checker-owned local ktest obligations
- Out of scope:
  - creator-local naming or formatting choices

## Prior Delivery Notes

- Keep the component strictly at allocation growth and metadata publication.
- If `02_designer_async.md` is omitted, say explicitly why no separate async artifact is needed and where any lock-order or publication assumptions are recorded.
- Do not specify buffered page-cache write policy, truncate/shrink behavior, or namespace mutation here.

## Temporary Interfaces And Exit Plan

- No temporary interface is authorized.

## Helper Justification

- Any helper or accessor must name the downstream growth or publication caller that needs it now and explain why the existing boundaries are otherwise too wide or error-prone.

## Allowed Commands

- read-only shell inspection commands under `/home/halifuda/asterinas`

## Parallelism Classification

- Lane class:
  - command-free
- May overlap with:
  - the active `EXR-PGCACHE-11B` implementation loop
  - other command-free planning lanes with disjoint write sets
- Known conflicts:
  - any lane writing `EXR-WRITE-13A` designer artifacts

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

- Stop after writing the required designer artifacts for `EXR-WRITE-13A`.
- Do not write creator artifacts, update the board, or touch production code.

## Escalation Rule

- If the design cannot stay confined to allocation growth and publication, stop and report exactly what pressure is trying to pull buffered writes, truncate/shrink, or namespace work into this component.
