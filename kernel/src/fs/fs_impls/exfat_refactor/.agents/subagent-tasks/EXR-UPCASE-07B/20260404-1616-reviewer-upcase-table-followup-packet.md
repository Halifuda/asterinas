<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-UPCASE-07B-REVIEW-UPCASE-TABLE-20260404-1616`
- Packet file: `kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-UPCASE-07B/20260404-1616-reviewer-upcase-table-followup-packet.md`
- Supersedes: none
- Role: reviewer
- Component: `EXR-UPCASE-07B`
- Phase: reviewer follow-up for `upcase_table.rs`
- Authorizing main agent: main-agent
- Date: 2026-04-04 16:16 CST

## Goal

- Perform a bounded follow-up review of the merged `upcase_table.rs` surface before the next big loop. Focus on helper discipline around the accessor-only surfaces, current boundary shape after `07A` plus `07B`, and whether any test-only staging surface still needs explicit temporary marking. Do not run commands.

## Read Set

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/REVIEWER.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_CODE_QUALITY_PRIORS.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/REVIEWER_REPORT_TEMPLATE.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07A/00_architect.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07A/01_designer_core.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07A/30_reviewer_report.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07B/00_architect.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07B/01_designer_core.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07B/30_reviewer_report.md`
- `kernel/src/fs/fs_impls/exfat_refactor/upcase_table.rs`

## Write Set

- `kernel/src/fs/fs_impls/exfat_refactor/upcase_table.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07B/32_reviewer_upcase_table_followup.md`

## Forbidden Files

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/*`
- all non-`upcase_table.rs` production files
- all checker artifacts
- all command-producing verification

## Required Inputs

- Role protocol files accompanying this packet:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/REVIEWER.md`
- Required artifact inputs:
  - the architect, designer, and reviewer artifacts listed in the read set
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/REVIEWER_REPORT_TEMPLATE.md`

## Semantic Prior Inputs

- Use only semantic constraints already captured by the accepted component artifacts.
- Do not reopen broader exFAT prior material unless the current artifacts conflict.

## Local Architectural Prior Inputs

- Use the accepted component artifacts as the local boundary contract.
- Local focus:
  - `ExfatUpcaseTable` remains the canonical owner of the loaded table plus table-backed fold-and-hash,
  - helper growth still requires a named downstream caller,
  - field-exposing accessors that only mirror stored members are suspect unless a packet-backed caller exists.

## Quality Prior Inputs

- Use `Q-REVIEW`
- In scope:
  - accessor-only helper discipline,
  - visibility hygiene,
  - narrowness of the merged table surface,
  - temporary-surface clarity if any test-only staging helper remains.
- Out of scope:
  - runtime verification,
  - broader redesign beyond bounded cleanup.

## Prior Delivery Notes

- The main question in this pass is whether `words()`, `byte_size()`, and `checksum()` are justified surfaces or only test-local exposure.
- If a helper is used only by local tests in the same file, prefer removing it and reading private state directly from the local test module.

## Temporary Interfaces And Exit Plan

- No temporary interface is currently intended in this file.
- If the reviewer discovers one, it must be marked explicitly with a TODO naming the removal condition.

## Helper Justification

- `name_hash()` is already justified by `EXR-UPCASE-07B`.
- Any accessor that only exposes stored fields without a named non-test caller should be removed or inlined.

## Allowed Commands

- none

## Parallelism Classification

- Lane class:
  - command-free
- May overlap with:
  - command-free reviewer work for disjoint files only
- Known conflicts:
  - any lane writing `upcase_table.rs` or the assigned follow-up reviewer artifact

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

- Stop after the reviewer pass and `32_reviewer_upcase_table_followup.md`.
- Do not run final-checker commands or update the task board.

## Escalation Rule

- If this file appears to need broader redesign rather than bounded review cleanup, stop and report that instead of widening scope.
