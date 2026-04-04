<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-BITMAP-08A-REVIEW-FOLLOWUP-20260404-1616`
- Packet file: `kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-BITMAP-08A/20260404-1616-reviewer-followup-packet.md`
- Supersedes: none
- Role: reviewer
- Component: `EXR-BITMAP-08A`
- Phase: reviewer follow-up
- Authorizing main agent: main-agent
- Date: 2026-04-04 16:16 CST

## Goal

- Perform a bounded follow-up review of `bitmap.rs` before the next big loop. Focus on whether the current free pure helper functions are an acceptable local implementation shape or should be folded into a more coherent owner, plus the usual reviewer checks for helper discipline and temporary surfaces. Do not run commands.

## Read Set

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/REVIEWER.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_CODE_QUALITY_PRIORS.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/REVIEWER_REPORT_TEMPLATE.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BITMAP-08A/00_architect.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BITMAP-08A/01_designer_core.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BITMAP-08A/30_reviewer_report.md`
- `kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs`

## Write Set

- `kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BITMAP-08A/32_reviewer_followup.md`

## Forbidden Files

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/*`
- all non-`bitmap.rs` production files
- all checker artifacts
- all command-producing verification

## Required Inputs

- Role protocol files accompanying this packet:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/REVIEWER.md`
- Required artifact inputs:
  - the architect, designer, and prior reviewer artifacts listed in the read set
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/REVIEWER_REPORT_TEMPLATE.md`

## Semantic Prior Inputs

- Use only semantic constraints already captured by the accepted component artifacts.

## Local Architectural Prior Inputs

- Use the accepted component artifacts as the local boundary contract.
- Local focus:
  - `bitmap.rs` remains load-and-read-only occupancy only,
  - free helper shape should be judged by locality and ownership clarity, not by mechanical preference alone,
  - no allocation policy or mutation behavior belongs here.

## Quality Prior Inputs

- Use `Q-REVIEW`
- In scope:
  - local helper shape,
  - boundary hygiene,
  - temporary-surface confirmation,
  - comments and invariant clarity.
- Out of scope:
  - runtime verification,
  - broader redesign.

## Prior Delivery Notes

- The user specifically asked whether pure free functions here are really necessary.
- If the current free helpers are already the clearest local shape, say so explicitly in the report instead of forcing a method conversion without a real readability gain.

## Temporary Interfaces And Exit Plan

- No temporary interface is intended in this file.
- If one is discovered, mark it explicitly and record its exit condition in the report.

## Helper Justification

- Keep or remove local helpers based on ownership clarity and repeated error-prone logic.
- Do not add new helpers unless they materially improve the existing read-only bitmap boundary.

## Allowed Commands

- none

## Parallelism Classification

- Lane class:
  - command-free
- May overlap with:
  - command-free reviewer work for disjoint files only
- Known conflicts:
  - any lane writing `bitmap.rs` or the assigned follow-up reviewer artifact

## Execution Environment

- Host or Docker:
  - host workspace only
- Required command prefix:
  - none
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

- Stop after the reviewer pass and `32_reviewer_followup.md`.
- Do not run final-checker commands or update the task board.

## Escalation Rule

- If this file appears to need broader redesign rather than bounded review cleanup, stop and report that instead of widening scope.
