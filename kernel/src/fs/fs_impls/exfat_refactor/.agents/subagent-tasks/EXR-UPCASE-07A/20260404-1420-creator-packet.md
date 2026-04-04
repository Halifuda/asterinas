<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-UPCASE-07A-CREATE-20260404-1420`
- Packet file: `kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-UPCASE-07A/20260404-1420-creator-packet.md`
- Supersedes: none
- Role: creator
- Component: `EXR-UPCASE-07A`
- Phase: serial creator
- Authorizing main agent: main-agent
- Date: 2026-04-04 14:20 CST

## Goal

- Implement the first serial creator pass for `EXR-UPCASE-07A`: add the on-disk upcase-table loader and validated read-only table surface in `upcase_table.rs`, plus the creator handoff. Do not write tests, do not run commands, and do not widen into case folding, name hashing, fallback policy, or mount policy.

## Read Set

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/CREATOR.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07A/00_architect.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07A/01_designer_core.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07A/03_designer_ktest.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-SYSROOT-06/10_creator_serial.md`
- `kernel/src/fs/fs_impls/exfat_refactor/sysroot.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/io.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/fat.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/super_block.rs`

## Write Set

- `kernel/src/fs/fs_impls/exfat_refactor/upcase_table.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07A/10_creator_serial.md`

## Forbidden Files

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/*`
- `kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- all files under `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07A/` except `10_creator_serial.md`
- all tests and checker artifacts

## Required Inputs

- Role protocol files accompanying this packet:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/CREATOR.md`
- Required artifact inputs:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07A/00_architect.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07A/01_designer_core.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07A/03_designer_ktest.md`
- Required code/reference inputs:
  - the refactor read-side modules listed in the read set

## Semantic Prior Inputs

- Use only prior-derived semantic constraints from the architect and designer artifacts.
- Semantic focus:
  - consuming `ExfatSysRootUpcaseDiscovery`
  - reading the full discovered payload
  - validating structural size rules, completeness, and checksum
  - publishing one canonical read-only loaded-table surface

## Local Architectural Prior Inputs

- Use integration constraints derived by the architect and designer artifacts.
- Local focus:
  - no root rediscovery
  - no fallback policy
  - no case folding or name hashing

## Quality Prior Inputs

- Use `Q-CREATE`
- In scope:
  - one canonical loader entry point
  - one canonical read-only table surface
  - no speculative helper growth
- Out of scope:
  - tests
  - commands
  - module wiring

## Prior Delivery Notes

- Main-agent-approved deviation from the designer file list:
  - `mod.rs` wiring is intentionally reserved to the main agent so `EXR-UPCASE-07A` and `EXR-BITMAP-08A` creators can run with disjoint write sets.
- This does not authorize any other scope widening or contraction.

## Temporary Interfaces And Exit Plan

- No temporary interface is authorized.

## Helper Justification

- The canonical loader entry point and loaded-table surface are authorized because `EXR-UPCASE-07B` will consume them.
- No extra short helper is authorized unless the designer artifact already proves it is part of the canonical surface.

## Allowed Commands

- none

## Parallelism Classification

- Lane class:
  - command-free
- May overlap with:
  - `EXR-BITMAP-08A` creator work
  - `EXR-SYSROOT-06` checker work
  - other command-free lanes with disjoint write sets
- Known conflicts:
  - any lane writing `upcase_table.rs` or the `EXR-UPCASE-07A` creator artifact

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
  - `kernel/src/fs/fs_impls/exfat_refactor/upcase_table.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07A/10_creator_serial.md`
- Do not modify `mod.rs`, write tests, or update the board.

## Escalation Rule

- If implementing the specified loader truly requires edits outside the write set, stop and report the missing interface rather than widening scope.
