<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-UPCASE-07B-REVIEW-DUPLICATE-HELPERS-20260404-1622`
- Packet file: `kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-UPCASE-07B/20260404-1622-reviewer-duplicate-helpers-scan-packet.md`
- Supersedes: none
- Role: reviewer
- Component: `EXR-UPCASE-07B`
- Phase: report-only cross-module helper-duplication scan
- Authorizing main agent: main-agent
- Date: 2026-04-04 16:22 CST

## Goal

- Perform a bounded report-only scan for potentially duplicated helpers across the current `exfat_refactor` wave's touched modules: `upcase_table.rs`, `fileset.rs`, `sysroot.rs`, and `bitmap.rs`, with optional comparison to `boot_sector.rs` only when needed to classify a checksum-style overlap. Classify candidates as real duplicates, acceptable local parallels, or not duplicates.

## Read Set

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/REVIEWER.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_CODE_QUALITY_PRIORS.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/REVIEWER_REPORT_TEMPLATE.md`
- `kernel/src/fs/fs_impls/exfat_refactor/upcase_table.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/fileset.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/sysroot.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/boot_sector.rs`

## Write Set

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07B/35_reviewer_duplicate_helpers_scan.md`

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
  - files listed in the read set
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/REVIEWER_REPORT_TEMPLATE.md`

## Semantic Prior Inputs

- Use the current code only.
- Distinguish carefully among:
  - true duplicate helpers with the same contract and same natural owner,
  - local helpers that share arithmetic shape but serve different on-disk semantics,
  - structurally similar helpers that are justified by different boundary owners.

## Local Architectural Prior Inputs

- Local focus:
  - do not recommend collapsing helpers across component boundaries unless one owner is clearly canonical,
  - flag only overlaps that materially hurt clarity or widen surfaces without need.

## Quality Prior Inputs

- Use `Q-REVIEW`
- In scope:
  - duplicate-helper scan,
  - local helper ownership judgment,
  - concise cleanup recommendations if justified.
- Out of scope:
  - code edits,
  - runtime verification,
  - broad refactoring.

## Prior Delivery Notes

- The user specifically wants other possible duplicate helpers checked in addition to `checksum32`.
- A short list of candidates with disposition is better than vague commentary.

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

- Stop after writing `35_reviewer_duplicate_helpers_scan.md`.

## Escalation Rule

- If the scan appears to require a wider corpus than the read set, report that gap instead of widening scope.
