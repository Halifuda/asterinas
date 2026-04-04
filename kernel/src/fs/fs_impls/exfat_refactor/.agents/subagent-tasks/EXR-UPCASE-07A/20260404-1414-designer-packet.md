<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-UPCASE-07A-DESIGN-20260404-1414`
- Packet file: `kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-UPCASE-07A/20260404-1414-designer-packet.md`
- Supersedes: none
- Role: designer
- Component: `EXR-UPCASE-07A`
- Phase: design
- Authorizing main agent: main-agent
- Date: 2026-04-04 14:14 CST

## Goal

- Write the bounded designer artifact set for `EXR-UPCASE-07A` so a later creator can implement on-disk upcase-table loading and validation without guessing about interfaces, validation ownership, or file layout. The design must stop before case folding, name hashing, fallback-table policy, mount policy, or charset conversion.

## Read Set

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/DESIGNER.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/DESIGNER_SPEC_TEMPLATE.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07A/00_architect.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-SYSROOT-06/01_designer_core.md`
- `kernel/src/fs/fs_impls/exfat_refactor/upcase_table.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/io.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/fat.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/super_block.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/dentry.rs`

## Write Set

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07A/01_designer_core.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07A/02_designer_async.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07A/03_designer_ktest.md`

## Forbidden Files

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/*`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07A/00_architect.md`
- all production Rust code

## Required Inputs

- Role protocol files accompanying this packet:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/DESIGNER.md`
- Required artifact inputs:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07A/00_architect.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-SYSROOT-06/01_designer_core.md`
- Required code/reference inputs:
  - refactor read-side modules listed in the read set

## Semantic Prior Inputs

- Use prior-derived semantic constraints from the architect artifact.
- Use broader Microsoft/Linux semantic sources only if the architect artifact is insufficient.
- Semantic focus:
  - consuming the `UPCASE` discovery descriptor from `SYSROOT`
  - loading and validating the on-disk table bytes
  - preserving a canonical loaded-table surface for `EXR-UPCASE-07B`

## Local Architectural Prior Inputs

- Use integration constraints derived by the architect artifact.
- Local focus:
  - `SYSROOT` owns discovery
  - `UPCASE-07A` owns loading and validation only
  - `UPCASE-07B` owns case-folding and name-hash behavior

## Quality Prior Inputs

- Use `Q-DESIGN`
- In scope:
  - canonical loaded-table interface
  - hidden implementation details
  - helper minimization
- Out of scope:
  - creator-local formatting detail
  - checker implementation detail beyond test obligations

## Prior Delivery Notes

- Keep this packet narrow around the first upcase component only.
- Do not specify case-folding APIs or a fallback-default-table policy.

## Temporary Interfaces And Exit Plan

- No temporary interface is authorized.

## Helper Justification

- One canonical loader entry point and one canonical loaded-table surface are expected.
- Any extra short helper must name whether the caller is local loading logic or later `EXR-UPCASE-07B`.

## Allowed Commands

- read-only shell inspection commands only

## Parallelism Classification

- Lane class:
  - command-free
- May overlap with:
  - `EXR-BITMAP-08A` design work
  - `EXR-SYSROOT-06` creator work
  - other command-free lanes with disjoint write sets
- Known conflicts:
  - any lane writing `EXR-UPCASE-07A` designer artifacts

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

- Write `01_designer_core.md` and `03_designer_ktest.md`.
- Write `02_designer_async.md` only if meaningful concurrency or shared-state obligations appear.
- If `02_designer_async.md` is omitted, say explicitly in the designer artifacts why no separate async artifact is needed.
- Do not implement code or update the board.

## Escalation Rule

- If the design cannot stay confined to table loading and validation, stop and report exactly what pressure is trying to pull case folding, name hashing, or policy decisions into this component.
