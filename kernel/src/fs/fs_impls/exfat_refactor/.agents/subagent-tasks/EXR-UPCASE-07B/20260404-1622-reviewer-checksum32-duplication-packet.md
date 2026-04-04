<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-UPCASE-07B-REVIEW-CHECKSUM32-20260404-1622`
- Packet file: `kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-UPCASE-07B/20260404-1622-reviewer-checksum32-duplication-packet.md`
- Supersedes: none
- Role: reviewer
- Component: `EXR-UPCASE-07B`
- Phase: report-only reviewer check for `checksum32`
- Authorizing main agent: main-agent
- Date: 2026-04-04 16:22 CST

## Goal

- Perform a bounded report-only review of `upcase_table.rs:checksum32` and determine what semantic contract it implements, whether it duplicates any helper already present in `exfat_refactor`, and whether any cleanup is warranted. Compare it against the closest nearby checksum helpers, but do not edit production code in this pass.

## Read Set

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/REVIEWER.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_CODE_QUALITY_PRIORS.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/REVIEWER_REPORT_TEMPLATE.md`
- `kernel/src/fs/fs_impls/exfat_refactor/upcase_table.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/boot_sector.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/fileset.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07A/00_architect.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07B/00_architect.md`

## Write Set

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07B/34_reviewer_checksum32_duplication.md`

## Forbidden Files

- all production files
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/*`
- all checker artifacts
- all command-producing verification

## Required Inputs

- Role protocol files accompanying this packet:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/REVIEWER.md`
- Required artifact inputs:
  - files and artifacts listed in the read set
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/REVIEWER_REPORT_TEMPLATE.md`

## Semantic Prior Inputs

- Use only semantic constraints already captured by the current implementation and architect artifacts.
- Focus on semantic identity versus mere algorithmic similarity:
  - upcase-table checksum on raw table payload bytes,
  - boot-region checksum with skipped mutable fields,
  - file-record checksum and `NameHash` helpers as distinct contracts.

## Local Architectural Prior Inputs

- Local focus:
  - helper reuse is desirable only when it preserves clarity and semantic boundaries,
  - do not recommend merging helpers that happen to use similar rotate-right accumulation but authenticate different on-disk objects with different exclusions or widths.

## Quality Prior Inputs

- Use `Q-REVIEW`
- In scope:
  - duplicate-helper classification,
  - semantic-boundary clarity,
  - whether a cleanup recommendation is justified.
- Out of scope:
  - code edits,
  - runtime verification,
  - broader redesign.

## Prior Delivery Notes

- This is a report-only pass. The main deliverable is a clear answer to whether `checksum32` is a duplicate helper or only superficially similar to other checksum routines.

## Temporary Interfaces And Exit Plan

- No temporary interface is in scope for this pass.

## Helper Justification

- No new helper is authorized.

## Allowed Commands

- none

## Parallelism Classification

- Lane class:
  - command-free
- May overlap with:
  - command-free reviewer lanes with disjoint artifact write sets
- Known conflicts:
  - any lane writing the assigned report artifact

## Execution Environment

- Host or Docker:
  - host workspace only
- Required command prefix:
  - none
- Required working directory:
  - `/home/halifuda/asterinas`
- Isolation notes:
  - shared worktree; do not edit outside the write set

## Execution Lock

- not applicable

## Stop Condition

- Stop after writing `34_reviewer_checksum32_duplication.md`.

## Escalation Rule

- If the comparison appears to require a wider corpus than the read set, report that gap instead of widening scope.
