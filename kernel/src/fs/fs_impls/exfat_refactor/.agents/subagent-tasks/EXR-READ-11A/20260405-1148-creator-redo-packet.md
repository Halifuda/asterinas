<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-READ-11A-CREATE-20260405-1148`
- Packet file: `kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-READ-11A/20260405-1148-creator-redo-packet.md`
- Supersedes: none
- Role: creator
- Component: `EXR-READ-11A`
- Phase: serial creator redo
- Authorizing main agent: main-agent
- Date: 2026-04-05 11:48 CST

## Goal

- Re-run the creator pass for `EXR-READ-11A` using the existing code as the starting point, but owned by a narrow creator subagent. Keep or refine the current implementation in `read.rs`, `inode.rs`, `fat.rs`, and `mod.rs` so it cleanly matches the accepted mapping-only spec. Write the delegated creator artifact `12_creator_serial_retry.md`.

## Read Set

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/README.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/CREATOR_LOG_TEMPLATE.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/CREATOR.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_CODE_QUALITY_PRIORS.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-READ-11A/00_architect.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-READ-11A/01_designer_core.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-READ-11A/03_designer_ktest.md`
- `kernel/src/fs/fs_impls/exfat_refactor/read.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/fat.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/mod.rs`

## Write Set

- `kernel/src/fs/fs_impls/exfat_refactor/read.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/fat.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-READ-11A/12_creator_serial_retry.md`

## Forbidden Files

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/*`
- checker, reviewer, and final-checker artifacts for `EXR-READ-11A`
- downstream `EXR-PGCACHE-11B` and `EXR-READ-11B` artifacts

## Required Inputs

- Role protocol files accompanying this packet:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/CREATOR.md`
- Required artifact/style inputs:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/CREATOR_LOG_TEMPLATE.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-READ-11A/00_architect.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-READ-11A/01_designer_core.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-READ-11A/03_designer_ktest.md`
- Existing code should be treated as the starting implementation, not as an authoritative acceptance proof.

## Semantic Prior Inputs

- Use only the semantic constraints already captured in the architect and designer artifacts.
- Do not reopen a broader semantic prior corpus in this pass.

## Local Architectural Prior Inputs

- Use the local constraints already captured in the architect and designer artifacts.
- Local focus:
  - the component must stay at logical-to-physical placement only
  - no buffered `read_at`
  - no page-cache backend ownership
  - no allocation growth

## Quality Prior Inputs

- Use `Q-CREATE` from `kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_CODE_QUALITY_PRIORS.md`
- In scope:
  - helper justification
  - narrow immutable read-view boundary
  - keeping tests local to `read.rs`
- Out of scope:
  - reviewer-level cleanup beyond the assigned files

## Prior Delivery Notes

- Keep this pass narrow. It is a delegated redo of the creator ownership, not a redesign of the component.
- If the current code already fits the spec, minimal or no production edits are acceptable, but the delegated creator artifact must still explain that judgment.

## Temporary Interfaces And Exit Plan

- No temporary interface is authorized.

## Helper Justification

- `ExfatInodeMeta::read_view()` is allowed only if the creator still agrees that `EXR-READ-11A` is the named downstream caller and that one immutable read-view surface is enough.
- `ExfatChain::current_cluster_id()` is allowed only if the creator still agrees it is the narrowest way to publish the mapped destination cluster without reopening chain internals.

## Allowed Commands

- none

## Parallelism Classification

- Lane class:
  - command-free
- May overlap with:
  - no other delegated lane writing `EXR-READ-11A`
- Known conflicts:
  - checker, reviewer, or final-checker passes for `EXR-READ-11A`

## Execution Environment

- Host or Docker:
  - host workspace only
- Required working directory:
  - `/home/halifuda/asterinas`
- Isolation notes:
  - shared worktree; do not edit outside the write set
- This task is command-free and must not add compile or runtime commands on its own.

## Execution Lock

- not applicable

## Stop Condition

- Stop after updating the owned production files, if needed, and writing `12_creator_serial_retry.md`.
- Do not write checker, reviewer, or task-board artifacts.

## Escalation Rule

- If the current code shape appears to need a larger split or a second creator component, stop and report the exact pressure instead of widening scope.
