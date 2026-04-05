<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-READ-11A-REVIEW-20260405-1148`
- Packet file: `kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-READ-11A/20260405-1148-reviewer-redo-packet.md`
- Supersedes: none
- Role: reviewer
- Component: `EXR-READ-11A`
- Phase: reviewer redo
- Authorizing main agent: main-agent
- Date: 2026-04-05 11:48 CST

## Goal

- Re-run the reviewer pass for `EXR-READ-11A` after the delegated creator and checker passes. Focus on helper justification, boundary discipline, and whether the current implementation still matches the mapping-only slice. Write `32_reviewer_followup.md`.

## Read Set

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/README.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/REVIEWER_REPORT_TEMPLATE.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/REVIEWER.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_CODE_QUALITY_PRIORS.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-READ-11A/00_architect.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-READ-11A/01_designer_core.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-READ-11A/12_creator_serial_retry.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-READ-11A/13_checker_serial_retry.md`
- `kernel/src/fs/fs_impls/exfat_refactor/read.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/fat.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/mod.rs`

## Write Set

- `kernel/src/fs/fs_impls/exfat_refactor/read.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/fat.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-READ-11A/32_reviewer_followup.md`

## Forbidden Files

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/*`
- checker artifacts other than the required read-only checker retry artifact
- final-checker artifacts

## Required Inputs

- Role protocol files accompanying this packet:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/REVIEWER.md`
- Required artifact/style inputs:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/REVIEWER_REPORT_TEMPLATE.md`
  - delegated creator artifact `12_creator_serial_retry.md`
  - delegated checker artifact `13_checker_serial_retry.md`

## Semantic Prior Inputs

- Use only the semantic constraints already captured in the architect and designer artifacts.

## Local Architectural Prior Inputs

- Use the local constraints already captured in the architect and designer artifacts.
- Local focus:
  - placement-only boundary
  - one immutable read-view helper at most
  - no buffered-read or backend drift

## Quality Prior Inputs

- Use `Q-REVIEW` from `kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_CODE_QUALITY_PRIORS.md`
- In scope:
  - helper and accessor justification
  - visibility and API shape
  - keeping the mapper canonical and narrow
- Out of scope:
  - runtime verification

## Prior Delivery Notes

- This reviewer should treat the earlier local main-thread review as non-authoritative.
- Direct edits are allowed only within the write set and only when they tighten the existing component boundary.

## Temporary Interfaces And Exit Plan

- No temporary interface is authorized.

## Helper Justification

- Remove or inline any helper that no longer has a named caller-backed reason to exist in `EXR-READ-11A`.

## Allowed Commands

- none

## Parallelism Classification

- Lane class:
  - command-free
- May overlap with:
  - no other delegated lane writing `EXR-READ-11A`
- Known conflicts:
  - all creator, checker, and final-checker passes for this component

## Execution Environment

- Host or Docker:
  - host workspace only
- Required working directory:
  - `/home/halifuda/asterinas`
- Isolation notes:
  - shared worktree; do not edit outside the write set
- This task is command-free and must not add compile or runtime commands.

## Execution Lock

- not applicable

## Stop Condition

- Stop after any in-scope review edits and after writing `32_reviewer_followup.md`.
- Do not run final-checker commands or task-board updates.

## Escalation Rule

- If the component still appears too wide even after bounded review edits, report that boundary failure clearly instead of redesigning it.
