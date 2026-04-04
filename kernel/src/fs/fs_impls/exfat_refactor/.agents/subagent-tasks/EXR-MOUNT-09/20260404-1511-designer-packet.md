<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-MOUNT-09-DESIGN-20260404-1511`
- Packet file: `kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-MOUNT-09/20260404-1511-designer-packet.md`
- Supersedes: none
- Role: designer
- Component: `EXR-MOUNT-09`
- Phase: designer
- Authorizing main agent: main-agent
- Date: 2026-04-04 15:11 CST

## Goal

- Produce the bounded designer artifact set for `EXR-MOUNT-09`: `01_designer_core.md`, `02_designer_async.md`, and `03_designer_ktest.md` if the mount shared-state contract needs all three. Omit any artifact only if the component contract truly does not need it.

## Read Set

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/README.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/DESIGNER_SPEC_TEMPLATE.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/DESIGNER.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-MOUNT-09/00_architect.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-SYSROOT-06/01_designer_core.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-05B/01_designer_core.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-05B/02_designer_async.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07B/01_designer_core.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BITMAP-08A/01_designer_core.md`
- `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/sysroot.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/upcase_table.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs`
- `kernel/src/fs/fs_impls/exfat/upcase_table.rs`
- `kernel/src/fs/fs_impls/exfat/bitmap.rs`
- `/home/halifuda/linux/fs/exfat/super.c`
- `/home/halifuda/linux/fs/exfat/namei.c`

## Write Set

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-MOUNT-09/01_designer_core.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-MOUNT-09/02_designer_async.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-MOUNT-09/03_designer_ktest.md`

## Forbidden Files

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/*`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-MOUNT-09/00_architect.md`
- all production Rust code

## Required Inputs

- Role protocol files accompanying this packet:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/DESIGNER.md`
- Required artifact/style inputs:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/DESIGNER_SPEC_TEMPLATE.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-MOUNT-09/00_architect.md`

## Semantic Prior Inputs

- Use:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/Microsoft-exFAT-spec.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/linux-exFAT-implementation-summary.md`
  - Linux `super.c` and `namei.c` as needed for mount/bootstrap ordering
- Precedence:
  - Microsoft exFAT rules first
  - Linux exFAT implementation guidance second
- Semantic focus:
  - mount sequencing
  - shared filesystem state ownership
  - consuming accepted loader surfaces without rediscovery

## Local Architectural Prior Inputs

- Use integration constraints derived from:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-MOUNT-09/00_architect.md`
  - accepted `SYSROOT`, `INODE-05B`, `UPCASE-07B`, and `BITMAP-08A` designer/core artifacts
- Local focus:
  - mount owns bootstrap plus shared runtime state
  - no inode metadata shaping, no directory lookup policy, no page-cache behavior, no mutation
  - if lock-ordering or shared-state publication needs explicit treatment, capture it in `02_designer_async.md`

## Quality Prior Inputs

- Use `Q-DESIGN` from `kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_CODE_QUALITY_PRIORS.md`
- In scope:
  - clear shared-state contract
  - explicit concurrency and publication invariants when needed
  - implementable creator and checker split
- Out of scope:
  - creator-local formatting or naming trivia

## Prior Delivery Notes

- This component is more likely than prior read-only loaders to need a real `02_designer_async.md`; omit it only with an explicit reason.
- Keep the design mount-owned but narrow.
- Make the root-seeding order and shared-state publication contract explicit enough that later `DIR-10` and `READ-11A` can consume it without reopening mount ownership.

## Temporary Interfaces And Exit Plan

- No temporary interface is authorized.

## Helper Justification

- Any helper or accessor surface must name the downstream consumer and explain why the mount object needs it now.

## Allowed Commands

- read-only shell inspection commands only

## Parallelism Classification

- Lane class:
  - command-free
- May overlap with:
  - `EXR-UPCASE-07B` creator and later checker lanes
  - other command-free lanes with disjoint write sets
- Known conflicts:
  - any lane writing `EXR-MOUNT-09` designer artifacts

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

- Stop after writing the required designer artifacts for `EXR-MOUNT-09`.
- Do not write creator artifacts, update the board, or touch production code.

## Escalation Rule

- If the design cannot stay confined to mount bootstrap and shared-state ownership, stop and report exactly what pressure is trying to pull directory policy, page-cache behavior, or mutation into this component.
