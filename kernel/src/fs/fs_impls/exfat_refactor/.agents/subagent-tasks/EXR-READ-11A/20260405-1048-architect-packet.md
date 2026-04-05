<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-READ-11A-ARCH-20260405-1048`
- Packet file: `kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-READ-11A/20260405-1048-architect-packet.md`
- Supersedes: none
- Role: architect
- Component: `EXR-READ-11A`
- Phase: architect
- Authorizing main agent: main-agent
- Date: 2026-04-05 10:48 CST

## Goal

- Write the architect artifact for `EXR-READ-11A` in `00_architect.md`. The component must own logical-to-physical mapping for existing regular-file reads over mounted shared state and accepted chain facts, while staying out of directory lookup policy, page-cache backend ownership, and write-side allocation growth.

## Read Set

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/README.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/ARCHITECT_TEMPLATE.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/ARCHITECT.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-MOUNT-09/00_architect.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-MOUNT-09/01_designer_core.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-05B/01_designer_core.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-CHAIN-03B/00_architect.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-CHAIN-03B/01_designer_spec.md`
- `kernel/src/fs/fs_impls/exfat_refactor/fat.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/io.rs`
- `/home/halifuda/linux/fs/exfat/file.c`
- `/home/halifuda/linux/fs/exfat/inode.c`

## Write Set

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-READ-11A/00_architect.md`

## Forbidden Files

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/*`
- any file under `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-READ-11A/` other than `00_architect.md`
- all production Rust code

## Required Inputs

- Role protocol files accompanying this packet:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/ARCHITECT.md`
- Required artifact/style inputs:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/ARCHITECT_TEMPLATE.md`
  - accepted `EXR-MOUNT-09` architect artifact and current designer boundary
  - accepted `EXR-INODE-05B` and `EXR-CHAIN-03B` boundaries

## Semantic Prior Inputs

- Use:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/Microsoft-exFAT-spec.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/linux-exFAT-implementation-summary.md`
  - Linux `file.c` and `inode.c` as needed for read mapping ownership
- Precedence:
  - Microsoft exFAT rules first
  - Linux exFAT implementation guidance second
- Semantic focus:
  - logical offset to data-cluster mapping for existing files
  - read-side treatment of contiguous versus FAT-backed chains
  - keeping buffered read entry points and page-cache backend policy out of this first read slice

## Local Architectural Prior Inputs

- Use selected integration constraints from:
  - `EXR-MOUNT-09` architect and designer artifacts
  - `EXR-INODE-05B` designer core
  - `EXR-CHAIN-03B` architect and designer boundaries
- Local focus:
  - the read component consumes mount-owned state rather than becoming a second mount path
  - page-cache backend ownership stays in `EXR-PGCACHE-11B`
  - user-visible buffered `read_at` stays in `EXR-READ-11B`

## Quality Prior Inputs

- Use `Q-ARCH` from `kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_CODE_QUALITY_PRIORS.md`
- In scope:
  - small mapping-only split
  - explicit separation from page cache and buffered I/O
  - clear dependency relation with `EXR-PGCACHE-11B` and `EXR-READ-11B`
- Out of scope:
  - creator-local implementation detail

## Prior Delivery Notes

- Keep the component centered on mapping existing file contents only.
- Make sure the artifact explains why buffered reads and page-cache hooks remain separate later passes.
- Expose whether `EXR-READ-11A` and `EXR-DIR-10` can progress in parallel after mount acceptance.

## Temporary Interfaces And Exit Plan

- No temporary interface is authorized.

## Helper Justification

- No helper API is pre-authorized by this packet beyond the eventual mapping boundary itself.

## Allowed Commands

- read-only shell inspection commands only

## Parallelism Classification

- Lane class:
  - command-free
- May overlap with:
  - `EXR-DIR-10` architect work
  - `EXR-MOUNT-09` creator/checker/reviewer flow
  - other command-free lanes with disjoint write sets
- Known conflicts:
  - any lane writing `EXR-READ-11A` architect artifacts

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

- Stop after writing `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-READ-11A/00_architect.md`.
- Do not write designer artifacts, update the task board, or touch production code.

## Escalation Rule

- If the split seems to require buffered `read_at`, page-cache backend ownership, directory lookup policy, or write-side allocation growth in the same component, stop and report the exact pressure instead of widening scope.
