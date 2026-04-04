<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-UPCASE-07B-CREATE-20260404-1559`
- Packet file: `kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-UPCASE-07B/20260404-1559-creator-retry-packet.md`
- Supersedes: `EXR-UPCASE-07B-CREATE-20260404-1508`
- Role: creator
- Component: `EXR-UPCASE-07B`
- Phase: serial creator retry
- Authorizing main agent: main-agent
- Date: 2026-04-04 15:59 CST

## Goal

- Repair the single checker-confirmed `EXR-UPCASE-07B` consumer-path defect in `fileset.rs`: the production validation path must consume the canonical upcase-backed name-hash service instead of retaining a raw-UTF-16 checksum comparison. Keep the repair narrow, update the creator artifact, and stop.

## Read Set

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/CREATOR.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07B/00_architect.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07B/01_designer_core.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07B/03_designer_ktest.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07B/10_creator_serial.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07B/11_checker_serial.md`
- `kernel/src/fs/fs_impls/exfat_refactor/fileset.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/upcase_table.rs`

## Write Set

- `kernel/src/fs/fs_impls/exfat_refactor/fileset.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07B/12_creator_serial_retry.md`

## Forbidden Files

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/*`
- all checker and reviewer artifacts other than the assigned creator artifact
- `kernel/src/fs/fs_impls/exfat_refactor/upcase_table.rs`
- all production files outside the write set

## Required Inputs

- Role protocol files accompanying this packet:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/CREATOR.md`
- Required artifact inputs:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07B/00_architect.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07B/01_designer_core.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07B/03_designer_ktest.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07B/10_creator_serial.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07B/11_checker_serial.md`

## Semantic Prior Inputs

- Use only the semantic constraints already distilled by the architect and designer artifacts.
- Semantic focus:
  - `fileset.rs` must stop validating `NameHash` with a raw UTF-16 checksum path,
  - the canonical answer is the loaded-table-backed hash service on `ExfatUpcaseTable`,
  - no fallback, lookup policy, or mount behavior belongs in this repair.

## Local Architectural Prior Inputs

- Use the accepted architect and designer artifacts plus the checker finding as the complete local contract.
- Local focus:
  - `fileset.rs` needs an explicit upcase-backed validation boundary,
  - do not widen into mount, dir, or sysroot work,
  - do not add a second overlapping public normalization helper.

## Quality Prior Inputs

- Use `Q-CREATE` from `kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_CODE_QUALITY_PRIORS.md`
- In scope:
  - narrow visibility,
  - direct expression of the validation invariant,
  - avoiding speculative new helpers or accessors,
  - keeping temporary staging surfaces explicit if any must remain.
- Out of scope:
  - unrelated quality cleanup in `upcase_table.rs`,
  - test writing,
  - compile or runtime verification.

## Prior Delivery Notes

- This packet is intentionally narrower than the original creator packet.
- Repair only the checker-confirmed consumer-path gap at `fileset.rs`.
- If the cleanest fix is to add an upcase-backed validation entry point, keep the old raw path out of the canonical production contract instead of hiding it behind another helper.

## Temporary Interfaces And Exit Plan

- The existing ktest-only `from_trusted_metadata` staging surface may remain.
- If this repair needs an additional ktest-only constructor or validation entry point, it must be marked as temporary and tied to later write-side ownership, but prefer reusing existing temporary surfaces instead of adding new ones.

## Helper Justification

- No new short helper or field-exposing accessor is authorized unless it is the narrowest way to express the upcase-backed validation boundary already required by the designer artifact.
- If an upcase-backed validation method is introduced, its caller boundary must be obvious from the code and artifact.

## Allowed Commands

- none

## Parallelism Classification

- Lane class:
  - command-free
- May overlap with:
  - the `EXR-UPCASE-07B` reviewer lane because that lane is report-only
- Known conflicts:
  - any lane writing `fileset.rs` or `12_creator_serial_retry.md`

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

- Stop after the serial repair pass and `12_creator_serial_retry.md`.
- Do not write checker tests, reviewer notes, or task-board updates.

## Escalation Rule

- If the repair appears to require editing `upcase_table.rs`, widening into lookup or mount policy, or running commands, stop and report that pressure instead of widening scope.
