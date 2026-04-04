<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-UPCASE-07B-REVIEW-20260404-1559`
- Packet file: `kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-UPCASE-07B/20260404-1559-reviewer-packet.md`
- Supersedes: none
- Role: reviewer
- Component: `EXR-UPCASE-07B`
- Phase: reviewer
- Authorizing main agent: main-agent
- Date: 2026-04-04 15:59 CST

## Goal

- Start the bounded reviewer pass for `EXR-UPCASE-07B` in parallel with the narrow repair creator lane. Review the current component implementation for boundary hygiene, helper discipline, temporary-surface clarity, and invariant expression, but keep this pass report-only so it does not conflict with the repair lane.

## Read Set

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/REVIEWER.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_CODE_QUALITY_PRIORS.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/REVIEWER_REPORT_TEMPLATE.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07B/00_architect.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07B/01_designer_core.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07B/03_designer_ktest.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07B/10_creator_serial.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07B/11_checker_serial.md`
- `kernel/src/fs/fs_impls/exfat_refactor/upcase_table.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/fileset.rs`

## Write Set

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07B/30_reviewer_report.md`

## Forbidden Files

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/*`
- all production files
- all checker artifacts
- all command-producing verification

## Required Inputs

- Role protocol files accompanying this packet:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/REVIEWER.md`
- Required artifact inputs:
  - architect, designer, creator, and checker artifacts listed in the read set
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/REVIEWER_REPORT_TEMPLATE.md`

## Semantic Prior Inputs

- Use only semantic constraints already captured by the architect, designer, and checker artifacts.
- Do not reopen broader exFAT prior material unless the component artifacts are internally inconsistent.

## Local Architectural Prior Inputs

- Use the accepted component artifacts as the main architectural contract.
- Local focus:
  - `ExfatUpcaseTable` should remain the canonical owner of table-backed fold-and-hash behavior,
  - `fileset.rs` should consume that surface without preserving an overlapping raw-hash contract,
  - no lookup policy, fallback policy, or mount policy belongs in this component.

## Quality Prior Inputs

- Use `Q-REVIEW`
- In scope:
  - helper discipline,
  - visibility and boundary hygiene,
  - temporary-surface clarity,
  - invariant expression in the consumer path,
  - whether `07B` still looks like one canonical normalization component.
- Out of scope:
  - the separately queued post-loop review topics about accessor-only helpers in `upcase_table.rs`,
  - the separately queued review of free pure functions in `sysroot.rs` and `bitmap.rs`,
  - runtime verification,
  - redesign beyond the current component.

## Prior Delivery Notes

- Keep this pass bounded and report-only so it can overlap safely with the repair creator lane.
- The checker-known defect at `fileset.rs#182` is already being repaired in parallel; do not spend the report only rediscovering that same issue unless it exposes a broader quality problem beyond the known fix.

## Temporary Interfaces And Exit Plan

- The existing ktest-only fileset builders are temporary staging surfaces for later write-side ownership.
- The reviewer should verify that those temporary surfaces are still explicitly marked and that the report echoes their future owner or removal condition if relevant.

## Helper Justification

- The canonical `name_hash` surface on `ExfatUpcaseTable` is already justified by the accepted designer spec.
- Any other short helper or field-exposing accessor should be called out only if its need is unsupported inside the current component boundary, but do not convert the deferred post-loop quality queue into a blocking rewrite here.

## Allowed Commands

- none

## Parallelism Classification

- Lane class:
  - command-free
- May overlap with:
  - the `EXR-UPCASE-07B` repair creator lane
- Known conflicts:
  - any lane writing `30_reviewer_report.md`

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
- Do not run final-checker commands or update the task board.

## Escalation Rule

- If the component appears to need a redesign rather than a bounded review report, stop and report that instead of widening scope.
