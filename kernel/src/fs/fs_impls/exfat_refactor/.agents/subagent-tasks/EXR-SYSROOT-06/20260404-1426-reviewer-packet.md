<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-SYSROOT-06-REVIEW-20260404-1426`
- Packet file: `kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-SYSROOT-06/20260404-1426-reviewer-packet.md`
- Supersedes: none
- Role: reviewer
- Component: `EXR-SYSROOT-06`
- Phase: reviewer
- Authorizing main agent: main-agent
- Date: 2026-04-04 14:26 CST

## Goal

- Perform bounded static review of the `EXR-SYSROOT-06` implementation after creator and checker completion. Focus on code quality, API boundaries, visibility hygiene, and invariant expression. Do not run commands.

## Read Set

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/REVIEWER.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_CODE_QUALITY_PRIORS.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-SYSROOT-06/00_architect.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-SYSROOT-06/01_designer_core.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-SYSROOT-06/03_designer_ktest.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-SYSROOT-06/10_creator_serial.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-SYSROOT-06/11_checker_serial.md`
- `kernel/src/fs/fs_impls/exfat_refactor/sysroot.rs`

## Write Set

- `kernel/src/fs/fs_impls/exfat_refactor/sysroot.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-SYSROOT-06/30_reviewer_report.md`

## Forbidden Files

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/*`
- all checker artifacts
- all non-`sysroot.rs` production files
- all command-producing verification

## Required Inputs

- Role protocol files accompanying this packet:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/REVIEWER.md`
- Required artifact inputs:
  - architect, designer, creator, and checker artifacts listed in the read set
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/REVIEWER_REPORT_TEMPLATE.md`

## Semantic Prior Inputs

- Use only semantic constraints already captured by the architect and designer artifacts.
- Do not reopen broader exFAT prior material unless a semantic drift question arises during review.

## Local Architectural Prior Inputs

- Use the accepted component artifacts as the main architectural contract.
- Local focus:
  - `sysroot.rs` must remain discovery-only
  - no general directory API
  - no mount bootstrap
  - no helper growth without a named downstream caller

## Quality Prior Inputs

- Use `Q-REVIEW`
- In scope:
  - boundary hygiene
  - visibility and helper discipline
  - narrowness of the public or module surface
  - comments and invariant clarity
- Out of scope:
  - runtime verification
  - broader redesign

## Prior Delivery Notes

- Keep review bounded to `sysroot.rs`.
- Prefer small direct edits over speculative redesigns.

## Temporary Interfaces And Exit Plan

- No temporary interface is authorized in this component.

## Helper Justification

- The scanner entry point and discovery result types are already justified by the accepted designer spec.
- Any other short helper should be removed or inlined unless the reviewer can point to a packet-backed downstream caller.

## Allowed Commands

- none

## Parallelism Classification

- Lane class:
  - command-free
- May overlap with:
  - command-free creator work for disjoint components
  - checker-owned runtime work in other components only if write sets stay disjoint
- Known conflicts:
  - any lane writing `sysroot.rs` or the reviewer artifact

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

- Stop after the reviewer pass and `30_reviewer_report.md`.
- Do not run final checker commands or update the task board.

## Escalation Rule

- If the component appears to need a redesign rather than a bounded review cleanup, stop and report that instead of widening scope.
