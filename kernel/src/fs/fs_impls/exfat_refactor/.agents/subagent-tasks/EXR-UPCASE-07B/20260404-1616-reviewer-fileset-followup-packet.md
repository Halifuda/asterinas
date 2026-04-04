<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-UPCASE-07B-REVIEW-FILESET-20260404-1616`
- Packet file: `kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-UPCASE-07B/20260404-1616-reviewer-fileset-followup-packet.md`
- Supersedes: none
- Role: reviewer
- Component: `EXR-UPCASE-07B`
- Phase: reviewer follow-up for `fileset.rs`
- Authorizing main agent: main-agent
- Date: 2026-04-04 16:16 CST

## Goal

- Perform a bounded follow-up review of `fileset.rs` before the next big loop. Focus on the current canonical `NameHash` validation boundary, helper discipline, and especially any test-only temporary wrappers or validators still living in production code. If such wrappers remain, add explicit TODO comments with their exit condition. Do not run commands.

## Read Set

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/REVIEWER.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_CODE_QUALITY_PRIORS.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/REVIEWER_REPORT_TEMPLATE.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FILESET-04B/00_architect.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07B/00_architect.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07B/01_designer_core.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07B/11_checker_serial.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07B/12_creator_serial_retry.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07B/13_checker_serial_retry.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07B/30_reviewer_report.md`
- `kernel/src/fs/fs_impls/exfat_refactor/fileset.rs`

## Write Set

- `kernel/src/fs/fs_impls/exfat_refactor/fileset.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07B/33_reviewer_fileset_followup.md`

## Forbidden Files

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/*`
- all non-`fileset.rs` production files
- all checker artifacts
- all command-producing verification

## Required Inputs

- Role protocol files accompanying this packet:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/REVIEWER.md`
- Required artifact inputs:
  - the architect, designer, creator, checker, retry-checker, and reviewer artifacts listed in the read set
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/REVIEWER_REPORT_TEMPLATE.md`

## Semantic Prior Inputs

- Use only semantic constraints already captured by the accepted component artifacts.
- Do not reopen broader exFAT prior material unless the current artifacts conflict.

## Local Architectural Prior Inputs

- Use the accepted component artifacts as the local boundary contract.
- Local focus:
  - `ExfatDentrySet::new(..., &ExfatUpcaseTable)` is the canonical consumer boundary,
  - any structure-only or builder surface kept under `#[cfg(ktest)]` is temporary staging only,
  - test-only helpers left in production code must advertise their exit condition explicitly.

## Quality Prior Inputs

- Use `Q-REVIEW`
- In scope:
  - temporary-wrapper clarity,
  - TODO coverage for test-only staging surfaces,
  - helper discipline and visibility hygiene,
  - preserving the single canonical production validation path.
- Out of scope:
  - runtime verification,
  - broader redesign of file-record ownership.

## Prior Delivery Notes

- This pass exists mainly to harden the temporary staging signals after the `07B` repair loop.
- If a test-only helper remains in production code, add a short TODO with the future owner or removal condition rather than leaving only a generic "temporary" phrase.

## Temporary Interfaces And Exit Plan

- `new_structure_only()`, `from_trusted_metadata()`, and `from_trusted_metadata_with_upcase()` are expected staging surfaces if they remain.
- Each retained staging surface must carry a TODO comment that says it should move into dedicated test support or disappear once production file-record synthesis no longer depends on local ktests.

## Helper Justification

- No new production helper is authorized.
- Review may keep existing test-only staging helpers only when they remain explicitly temporary and narrowly justified.

## Allowed Commands

- none

## Parallelism Classification

- Lane class:
  - command-free
- May overlap with:
  - command-free reviewer work for disjoint files only
- Known conflicts:
  - any lane writing `fileset.rs` or the assigned follow-up reviewer artifact

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

- Stop after the reviewer pass and `33_reviewer_fileset_followup.md`.
- Do not run final-checker commands or update the task board.

## Escalation Rule

- If this file appears to need broader redesign rather than bounded review cleanup, stop and report that instead of widening scope.
