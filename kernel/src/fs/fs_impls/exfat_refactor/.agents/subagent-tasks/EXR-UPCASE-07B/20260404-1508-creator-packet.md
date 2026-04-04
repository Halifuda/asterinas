<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-UPCASE-07B-CREATE-20260404-1508`
- Packet file: `kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-UPCASE-07B/20260404-1508-creator-packet.md`
- Supersedes: none
- Role: creator
- Component: `EXR-UPCASE-07B`
- Phase: serial creator
- Authorizing main agent: main-agent
- Date: 2026-04-04 15:08 CST

## Goal

- Implement the canonical upcase-backed fold-and-hash service for `EXR-UPCASE-07B`, wire `fileset.rs` to consume it instead of the provisional raw-UTF-16 checksum path, and record the work in `10_creator_serial.md`. This is the only creator round for the current main-agent loop.

## Read Set

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/CREATOR.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07B/00_architect.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07B/01_designer_core.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07B/03_designer_ktest.md`
- `kernel/src/fs/fs_impls/exfat_refactor/upcase_table.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/fileset.rs`

## Write Set

- `kernel/src/fs/fs_impls/exfat_refactor/upcase_table.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/fileset.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07B/10_creator_serial.md`

## Forbidden Files

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/*`
- all checker, reviewer, or mount artifacts
- `kernel/src/fs/fs_impls/exfat_refactor/mod.rs` unless you first stop and report why it is necessary
- all other production files outside the write set

## Required Inputs

- Role protocol files accompanying this packet:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/CREATOR.md`
- Required artifact inputs:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07B/00_architect.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07B/01_designer_core.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07B/03_designer_ktest.md`

## Semantic Prior Inputs

- Use only the semantic constraints distilled by the architect and designer artifacts.
- Semantic focus:
  - fold logical UTF-16 units through the loaded upcase table
  - derive exFAT `NameHash` from folded UTF-16 bytes
  - remove the provisional raw-UTF-16 canonical hash path in `fileset.rs`

## Local Architectural Prior Inputs

- Use integration constraints derived from the accepted architect and designer artifacts.
- Local focus:
  - one canonical read-only fold-and-hash service
  - `fileset.rs` consumes the canonical service directly
  - no mount policy, lookup policy, fallback-table behavior, or extra overlapping helper surface

## Quality Prior Inputs

- Use `Q-CREATE` from `kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_CODE_QUALITY_PRIORS.md`
- In scope:
  - narrow visibility
  - checked arithmetic and error propagation
  - avoiding unjustified extra helpers or field-exposing accessors
- Out of scope:
  - test writing
  - compile or runtime verification

## Prior Delivery Notes

- Keep this creator pass inside the `07B` fold-and-hash boundary only.
- Prefer one canonical service surface over two overlapping exported helpers.
- If a private fold-only helper is needed inside `upcase_table.rs`, keep it private and document why the canonical hash service still remains the external contract.

## Temporary Interfaces And Exit Plan

- No temporary interface is authorized.

## Helper Justification

- No new public accessor or short wrapper is authorized unless the designer artifacts already prove a named downstream caller need.
- Internal private helpers are allowed only when they make the canonical fold-and-hash service clearer and do not become a second exported contract.

## Allowed Commands

- none

## Parallelism Classification

- Lane class:
  - command-free
- May overlap with:
  - `EXR-MOUNT-09` command-free planning lanes only
- Known conflicts:
  - any lane writing `upcase_table.rs`, `fileset.rs`, or the `EXR-UPCASE-07B` creator artifact

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

- Stop after the serial creator pass and `10_creator_serial.md`.
- Do not write checker tests, reviewer notes, or task-board updates.

## Escalation Rule

- If the implementation appears to require mount policy, lookup policy, a second exported helper surface, or edits outside the write set, stop and report the pressure instead of widening scope.
