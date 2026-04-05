<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-READ-11A-DESIGN-20260405-1059`
- Packet file: `kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-READ-11A/20260405-1059-designer-packet.md`
- Supersedes: none
- Role: designer
- Component: `EXR-READ-11A`
- Phase: designer
- Authorizing main agent: main-agent
- Date: 2026-04-05 10:59 CST

## Goal

- Produce the bounded designer artifact set for `EXR-READ-11A`: `01_designer_core.md` and `03_designer_ktest.md`, plus `02_designer_async.md` only if the component has concurrency or serialization obligations that later roles cannot safely infer from the core spec.

## Read Set

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/README.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/DESIGNER_SPEC_TEMPLATE.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/DESIGNER.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-READ-11A/00_architect.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-MOUNT-09/01_designer_core.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-05B/01_designer_core.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-CHAIN-03B/01_designer_spec.md`
- `kernel/src/fs/fs_impls/exfat_refactor/fat.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/io.rs`
- `/home/halifuda/linux/fs/exfat/file.c`
- `/home/halifuda/linux/fs/exfat/inode.c`

## Write Set

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-READ-11A/01_designer_core.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-READ-11A/02_designer_async.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-READ-11A/03_designer_ktest.md`

## Forbidden Files

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/*`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-READ-11A/00_architect.md`
- all production Rust code

## Required Inputs

- Role protocol files accompanying this packet:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/DESIGNER.md`
- Required artifact/style inputs:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/DESIGNER_SPEC_TEMPLATE.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-READ-11A/00_architect.md`

## Semantic Prior Inputs

- Use:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/Microsoft-exFAT-spec.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/linux-exFAT-implementation-summary.md`
  - Linux `file.c` and `inode.c` as needed for read mapping behavior
- Precedence:
  - Microsoft exFAT rules first
  - Linux exFAT implementation guidance second
- Semantic focus:
  - mapping existing regular-file offsets to physical placement
  - respecting contiguous and FAT-backed chain semantics
  - keeping buffered reads and page-cache policy out of scope

## Local Architectural Prior Inputs

- Use integration constraints derived from:
  - `EXR-READ-11A/00_architect.md`
  - `EXR-MOUNT-09/01_designer_core.md`
  - `EXR-INODE-05B/01_designer_core.md`
  - `EXR-CHAIN-03B/01_designer_spec.md`
- Local focus:
  - consume mounted shared state instead of opening a second mount path
  - keep `PageCacheBackend` ownership in `EXR-PGCACHE-11B`
  - keep buffered `read_at` behavior in `EXR-READ-11B`

## Quality Prior Inputs

- Use `Q-DESIGN` from `kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_CODE_QUALITY_PRIORS.md`
- In scope:
  - mapping-only boundary
  - explicit helper justification for any new inode or fs accessor
  - checker-owned local ktest obligations
- Out of scope:
  - creator-local naming or formatting choices

## Prior Delivery Notes

- Keep the component about placement, not copying bytes.
- If `02_designer_async.md` is omitted, say explicitly why the serial contract is enough.
- Do not specify buffered reads, page-cache hooks, directory lookup, or write growth here.

## Temporary Interfaces And Exit Plan

- No temporary interface is authorized.

## Helper Justification

- Any helper or accessor must name the read-mapping caller that needs it now and explain why the helper is better than keeping the state private until a later component.

## Allowed Commands

- read-only shell inspection commands only

## Parallelism Classification

- Lane class:
  - command-free
- May overlap with:
  - `EXR-DIR-10` designer work
  - `EXR-MOUNT-09` checker, reviewer, and final-checker flow
  - other command-free lanes with disjoint write sets
- Known conflicts:
  - any lane writing `EXR-READ-11A` designer artifacts

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

- Stop after writing the required designer artifacts for `EXR-READ-11A`.
- Do not write creator artifacts, update the board, or touch production code.

## Escalation Rule

- If the design cannot stay confined to logical-to-physical mapping for existing reads, stop and report exactly what pressure is trying to pull buffered reads, page-cache behavior, directory lookup, or write-side growth into this component.
