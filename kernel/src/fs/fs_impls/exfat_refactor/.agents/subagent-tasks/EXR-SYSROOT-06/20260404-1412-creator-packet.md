<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-SYSROOT-06-CREATE-20260404-1412`
- Packet file: `kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-SYSROOT-06/20260404-1412-creator-packet.md`
- Supersedes: none
- Role: creator
- Component: `EXR-SYSROOT-06`
- Phase: serial creator
- Authorizing main agent: main-agent
- Date: 2026-04-04 14:12 CST

## Goal

- Implement the first serial creator pass for `EXR-SYSROOT-06`: add the synchronous read-only root-directory system-entry scanner and its discovery result types exactly as specified, without writing tests, without running commands, and without widening into mount/bootstrap or the later `UPCASE` / `BITMAP` loaders.

## Read Set

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/CREATOR.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-SYSROOT-06/00_architect.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-SYSROOT-06/01_designer_core.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-SYSROOT-06/03_designer_ktest.md`
- `kernel/src/fs/fs_impls/exfat_refactor/dentry.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/fat.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/io.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/super_block.rs`

## Write Set

- `kernel/src/fs/fs_impls/exfat_refactor/sysroot.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-SYSROOT-06/10_creator_serial.md`

## Forbidden Files

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/*`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-SYSROOT-06/00_architect.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-SYSROOT-06/01_designer_core.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-SYSROOT-06/03_designer_ktest.md`
- `kernel/src/fs/fs_impls/exfat_refactor/boot_sector.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/dentry.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`

## Required Inputs

- Role protocol files accompanying this packet:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/CREATOR.md`
- Required artifact inputs:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-SYSROOT-06/00_architect.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-SYSROOT-06/01_designer_core.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-SYSROOT-06/03_designer_ktest.md`
- Required code/reference inputs:
  - refactor read-side modules listed in the read set

## Semantic Prior Inputs

- Use only prior-derived semantic constraints from the architect and designer artifacts.
- Do not reopen broader semantic priors unless the spec is insufficient.
- Semantic focus:
  - root-only discovery of `BITMAP` and `UPCASE` entries
  - preservation of location, start cluster, byte size, and `UPCASE` checksum facts
  - duplicate, missing, malformed, and wrong-kind detection at the scanner boundary

## Local Architectural Prior Inputs

- Use only integration constraints derived by the architect and designer artifacts.
- Local architectural focus:
  - keep the component synchronous and read-only
  - keep the implementation self-contained in `sysroot.rs`
  - wire it through `mod.rs` only

## Quality Prior Inputs

- Use `Q-CREATE` from `kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_CODE_QUALITY_PRIORS.md`
- In scope:
  - purposeful helper surfaces only
  - no speculative getters
  - top-down readability
  - narrow visibility
- Out of scope:
  - test authoring
  - compile or runtime verification

## Prior Delivery Notes

- This packet is intentionally narrower than the designer artifacts:
  - implement only the production scanner and module wiring,
  - do not implement checker tests,
  - do not invent mount-owned state or any general directory API.

## Temporary Interfaces And Exit Plan

- No temporary interface is authorized.

## Helper Justification

- The canonical scanner entry point and the read-only discovery result types are authorized because later `EXR-UPCASE-07A` and `EXR-BITMAP-08A` loaders need them.
- No other short helper or field-exposing accessor is authorized unless the designer artifact already proves that it is part of the canonical surface.

## Allowed Commands

- none

## Parallelism Classification

- Lane class:
  - command-free
- May overlap with:
  - command-free architect and designer lanes with disjoint write sets
- Known conflicts:
  - any lane writing `sysroot.rs`, `mod.rs`, or the `EXR-SYSROOT-06` creator artifact

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
- This task does not include a command-producing checker stage.

## Stop Condition

- Stop after writing:
  - `kernel/src/fs/fs_impls/exfat_refactor/sysroot.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-SYSROOT-06/10_creator_serial.md`
- Do not write tests, checker artifacts, reviewer notes, or task-board updates.

## Escalation Rule

- If the specified scanner cannot be implemented inside the authorized write set without touching `dentry.rs`, `inode.rs`, or another production file, stop and report exactly which missing interface is forcing that wider edit.
