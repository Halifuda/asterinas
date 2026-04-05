<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-WRITE-13A-ARCH-20260405-1220`
- Packet file: `kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-WRITE-13A/20260405-1220-architect-packet.md`
- Supersedes: none
- Role: architect
- Component: `EXR-WRITE-13A`
- Phase: architect
- Authorizing main agent: main-agent
- Date: 2026-04-05 12:20 CST

## Goal

- Write `EXR-WRITE-13A`'s `00_architect.md`. The component should own allocation growth for writable regular files on top of accepted mount, bitmap, inode, and mapping boundaries, while keeping buffered page-cache writes in `EXR-WRITE-13B`, namespace behavior elsewhere, and truncate/shrink behavior in `EXR-WRITE-13C`.

## Read Set

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/README.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/ARCHITECT_TEMPLATE.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/ARCHITECT.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BITMAP-08A/00_architect.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BITMAP-08A/01_designer_core.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-MOUNT-09/01_designer_core.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-05B/01_designer_core.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-READ-11A/01_designer_core.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-11B/00_architect.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-READ-11B/00_architect.md`
- `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/fat.rs`
- `kernel/src/fs/vfs/page_cache.rs`
- `kernel/src/fs/fs_impls/exfat/inode.rs`
- `kernel/src/fs/fs_impls/exfat/fs.rs`
- `/home/halifuda/linux/fs/exfat/file.c`
- `/home/halifuda/linux/fs/exfat/inode.c`

## Write Set

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-13A/00_architect.md`

## Forbidden Files

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/*`
- any file under `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-13A/` other than `00_architect.md`
- all production Rust code

## Required Inputs

- Role protocol files accompanying this packet:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/ARCHITECT.md`
- Required artifact/style inputs:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/ARCHITECT_TEMPLATE.md`
  - accepted mount, inode, bitmap, and read-mapping boundaries
  - current `EXR-PGCACHE-11B` and `EXR-READ-11B` architect boundaries as downstream/non-owned context

## Semantic Prior Inputs

- Use:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/Microsoft-exFAT-spec.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/linux-exFAT-implementation-summary.md`
  - Linux `file.c` and `inode.c` as needed for cluster allocation growth and size-extension semantics
- Precedence:
  - Microsoft exFAT rules first
  - Linux exFAT implementation guidance second
- Semantic focus:
  - regular-file allocation growth
  - cluster-chain extension and allocation publication
  - separation between allocation growth, buffered writes, and truncate/shrink behavior

## Local Architectural Prior Inputs

- Use selected integration constraints from:
  - `EXR-MOUNT-09`
  - `EXR-INODE-05B`
  - `EXR-READ-11A`
  - `EXR-BITMAP-08A`
  - `EXR-PGCACHE-11B`
  - `EXR-READ-11B`
- Local focus:
  - `EXR-WRITE-13A` should own allocation growth only
  - page-cache backend ownership stays in `EXR-PGCACHE-11B`
  - buffered data copy/write policy stays in `EXR-WRITE-13B`
  - shrink/truncate stays in `EXR-WRITE-13C`

## Quality Prior Inputs

- Use `Q-ARCH` from `kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_CODE_QUALITY_PRIORS.md`
- In scope:
  - narrow ownership around growth semantics
  - explicit dependency placement against bitmap, backend, and buffered-write follow-ons
  - avoiding drift into namespace or truncate policy
- Out of scope:
  - creator-local implementation detail

## Prior Delivery Notes

- Keep the split narrow enough that it covers only making a writable file larger at the allocation and metadata boundary.
- Be explicit about what must still be left for `EXR-WRITE-13B` and `EXR-WRITE-13C`.
- If legacy exFAT couples allocation growth and buffered writes, use that only as contrast, not as the target shape.

## Temporary Interfaces And Exit Plan

- No temporary interface is authorized.

## Helper Justification

- No helper API is pre-authorized by this packet beyond the eventual allocation-growth boundary itself.

## Allowed Commands

- read-only shell inspection commands under `/home/halifuda/asterinas`

## Parallelism Classification

- Lane class:
  - command-free
- May overlap with:
  - the active `EXR-PGCACHE-11B` creator/checker/reviewer/final-checker chain
  - other command-free planning lanes with disjoint write sets
- Known conflicts:
  - any lane writing `EXR-WRITE-13A` architect artifacts

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

- Stop after writing `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-13A/00_architect.md`.
- Do not write designer artifacts, update the task board, or touch production code.

## Escalation Rule

- If the split appears to require buffered page writes, truncate/shrink behavior, or namespace mutation in the same component, stop and report the exact pressure instead of widening scope.
