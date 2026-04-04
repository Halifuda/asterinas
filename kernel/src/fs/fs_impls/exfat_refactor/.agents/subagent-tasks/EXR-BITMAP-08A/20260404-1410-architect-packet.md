<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-BITMAP-08A-ARCH-20260404-1410`
- Packet file: `kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-BITMAP-08A/20260404-1410-architect-packet.md`
- Supersedes: none
- Role: architect
- Component: `EXR-BITMAP-08A`
- Phase: architect
- Authorizing main agent: main-agent
- Date: 2026-04-04 14:10 CST

## Goal

- Write the architect artifact for `EXR-BITMAP-08A` in `00_architect.md`. The component must cover only allocation-bitmap loading, validation, and read-only occupancy queries using the root-entry discovery facts from `EXR-SYSROOT-06`. It must not own allocation policy, free-space hints, dirty tracking, or alloc/free mutation.

## Read Set

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/README.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/ARCHITECT_TEMPLATE.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/ARCHITECT.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-SYSROOT-06/00_architect.md`
- `kernel/src/fs/fs_impls/exfat_refactor/super_block.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/fat.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/io.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/dentry.rs`
- `kernel/src/fs/fs_impls/exfat/bitmap.rs`
- `/home/halifuda/linux/fs/exfat/balloc.c`
- `/home/halifuda/linux/fs/exfat/super.c`

## Write Set

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BITMAP-08A/00_architect.md`

## Forbidden Files

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/*`
- any file under `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BITMAP-08A/` other than `00_architect.md`
- all production Rust code

## Required Inputs

- Role protocol files accompanying this packet:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/ARCHITECT.md`
- Required artifact/style inputs:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/ARCHITECT_TEMPLATE.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-SYSROOT-06/00_architect.md`
- Required code/reference inputs:
  - current refactor read-side modules listed in the read set
  - legacy Asterinas bitmap loader in `bitmap.rs`
  - Linux bitmap loader in `balloc.c`

## Semantic Prior Inputs

- Use:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/Microsoft-exFAT-spec.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/linux-exFAT-implementation-summary.md`
  - Linux `balloc.c` as needed for exact loading and validation behavior
- Precedence:
  - Microsoft exFAT rules first
  - Linux exFAT implementation guidance second
- Semantic focus:
  - allocation bitmap entry semantics
  - expected bitmap size against volume geometry
  - loading the bitmap bytes and read-only occupancy validation only

## Local Architectural Prior Inputs

- Use selected integration constraints from:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_ARCHITECT_PRIORS.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-SYSROOT-06/00_architect.md`
- Local architectural focus:
  - `EXR-SYSROOT-06` owns discovery of the bitmap root entry
  - `EXR-BITMAP-08A` owns loading and validating the bitmap bytes from that descriptor
  - `EXR-BITMAP-08B` owns later allocation cursor policy, mutation, and dirty tracking

## Quality Prior Inputs

- Use `Q-ARCH` from `kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_CODE_QUALITY_PRIORS.md`
- In scope:
  - small dependency-safe split
  - preserving one canonical read-only bitmap surface
  - preventing overlap with `BITMAP-08B`
- Out of scope:
  - creator-local implementation detail
  - final naming or formatting detail

## Prior Delivery Notes

- Keep this packet narrow around the first bitmap component only: load and validate the on-disk bitmap, then expose read-only occupancy queries.
- Do not widen into allocation search policy, free-space hints, or mutation. Those belong later.
- Use legacy Asterinas and Linux only as split pressure and algorithm references, not as reasons to keep mount-time coupling.

## Temporary Interfaces And Exit Plan

- No temporary interface is authorized.

## Helper Justification

- No helper API is pre-authorized by this packet beyond the eventual loaded read-only bitmap surface.
- If the architect believes a short helper is needed, the artifact must name whether the caller is local read-only query code or the later `EXR-BITMAP-08B` mutating layer.

## Allowed Commands

- read-only shell inspection commands only

## Parallelism Classification

- Lane class:
  - command-free
- May overlap with:
  - `EXR-UPCASE-07A` architect work
  - `EXR-SYSROOT-06` design work
  - other command-free lanes with disjoint write sets
- Known conflicts:
  - any lane writing `EXR-BITMAP-08A` architect artifacts

## Execution Environment

- Host or Docker:
  - host workspace only
- Required command prefix:
  - read-only shell commands under `/home/halifuda/asterinas`
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

- Stop after writing `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BITMAP-08A/00_architect.md`.
- Do not write designer artifacts, update the task board, or touch production code.

## Escalation Rule

- If the component cannot stay confined to bitmap loading, validation, and read-only occupancy queries, stop and report exactly what pressure is trying to pull allocation policy, free-space hints, or dirty-state mutation into this component.
