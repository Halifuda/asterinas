<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-BITMAP-08A-CREATE-20260404-1420`
- Packet file: `kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-BITMAP-08A/20260404-1420-creator-packet.md`
- Supersedes: none
- Role: creator
- Component: `EXR-BITMAP-08A`
- Phase: serial creator
- Authorizing main agent: main-agent
- Date: 2026-04-04 14:20 CST

## Goal

- Implement the first serial creator pass for `EXR-BITMAP-08A`: add the allocation-bitmap loader and read-only occupancy surface in `bitmap.rs`, plus the creator handoff. Do not write tests, do not run commands, and do not widen into search policy, hints, dirty tracking, or mutation.

## Read Set

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/CREATOR.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BITMAP-08A/00_architect.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BITMAP-08A/01_designer_core.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BITMAP-08A/03_designer_ktest.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-SYSROOT-06/10_creator_serial.md`
- `kernel/src/fs/fs_impls/exfat_refactor/sysroot.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/io.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/fat.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/super_block.rs`

## Write Set

- `kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BITMAP-08A/10_creator_serial.md`

## Forbidden Files

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/*`
- `kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- all files under `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BITMAP-08A/` except `10_creator_serial.md`
- all tests and checker artifacts

## Required Inputs

- Role protocol files accompanying this packet:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/CREATOR.md`
- Required artifact inputs:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BITMAP-08A/00_architect.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BITMAP-08A/01_designer_core.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BITMAP-08A/03_designer_ktest.md`
- Required code/reference inputs:
  - the refactor read-side modules listed in the read set

## Semantic Prior Inputs

- Use only prior-derived semantic constraints from the architect and designer artifacts.
- Semantic focus:
  - consuming `ExfatSysRootBitmapDiscovery`
  - reading the discovered payload
  - validating geometry coverage and the bitmap file's own cluster coverage
  - publishing one canonical read-only occupancy surface

## Local Architectural Prior Inputs

- Use integration constraints derived by the architect and designer artifacts.
- Local focus:
  - no root rediscovery
  - no free-space search or hint policy
  - no mutation or dirty tracking

## Quality Prior Inputs

- Use `Q-CREATE`
- In scope:
  - one canonical loader entry point
  - one canonical read-only occupancy surface
  - no speculative helper growth
- Out of scope:
  - tests
  - commands
  - module wiring

## Prior Delivery Notes

- Main-agent-approved deviation from the designer file list:
  - `mod.rs` wiring is intentionally reserved to the main agent so `EXR-BITMAP-08A` and `EXR-UPCASE-07A` creators can run with disjoint write sets.
- This does not authorize any other scope widening or contraction.

## Temporary Interfaces And Exit Plan

- No temporary interface is authorized.

## Helper Justification

- The canonical loader entry point and read-only occupancy surface are authorized because later `EXR-BITMAP-08B` and mount work will consume them.
- No extra short helper is authorized unless the designer artifact already proves it is part of the canonical surface.

## Allowed Commands

- none

## Parallelism Classification

- Lane class:
  - command-free
- May overlap with:
  - `EXR-UPCASE-07A` creator work
  - `EXR-SYSROOT-06` checker work
  - other command-free lanes with disjoint write sets
- Known conflicts:
  - any lane writing `bitmap.rs` or the `EXR-BITMAP-08A` creator artifact

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

- Stop after writing:
  - `kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BITMAP-08A/10_creator_serial.md`
- Do not modify `mod.rs`, write tests, or update the board.

## Escalation Rule

- If implementing the specified loader truly requires edits outside the write set, stop and report the missing interface rather than widening scope.
