<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-MOUNT-09-ARCH-20260404-1501`
- Packet file: `kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-MOUNT-09/20260404-1501-architect-packet.md`
- Supersedes: none
- Role: architect
- Component: `EXR-MOUNT-09`
- Phase: architect
- Authorizing main agent: main-agent
- Date: 2026-04-04 15:01 CST

## Goal

- Write the architect artifact for `EXR-MOUNT-09` in `00_architect.md`. The component must own mount bootstrap and shared filesystem state, including root-seeded shared state and root-discovered table loading, while staying out of inode metadata shaping, page-cache backend behavior, and directory lookup policy.

## Read Set

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/README.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/ARCHITECT_TEMPLATE.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/ARCHITECT.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-SYSROOT-06/00_architect.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-05B/00_architect.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07B/00_architect.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BITMAP-08A/00_architect.md`
- `kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `kernel/src/fs/fs_impls/exfat/upcase_table.rs`
- `kernel/src/fs/fs_impls/exfat/bitmap.rs`
- `/home/halifuda/linux/fs/exfat/super.c`
- `/home/halifuda/linux/fs/exfat/namei.c`

## Write Set

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-MOUNT-09/00_architect.md`

## Forbidden Files

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/*`
- any file under `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-MOUNT-09/` other than `00_architect.md`
- all production Rust code

## Required Inputs

- Role protocol files accompanying this packet:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/ARCHITECT.md`
- Required artifact/style inputs:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/ARCHITECT_TEMPLATE.md`
  - accepted architect artifacts for `EXR-SYSROOT-06`, `EXR-INODE-05B`, `EXR-BITMAP-08A`, and current architect artifact for `EXR-UPCASE-07B`

## Semantic Prior Inputs

- Use:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/Microsoft-exFAT-spec.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/linux-exFAT-implementation-summary.md`
  - Linux `super.c` and `namei.c` as needed for mount/bootstrap ownership
- Precedence:
  - Microsoft exFAT rules first
  - Linux exFAT implementation guidance second
- Semantic focus:
  - mount-time opening and root seeding
  - ownership of shared runtime state
  - loading accepted root-discovered auxiliary tables into mount-owned state

## Local Architectural Prior Inputs

- Use selected integration constraints from:
  - accepted `SYSROOT`, `INODE-05B`, and `BITMAP-08A` architect artifacts
  - current `UPCASE-07B` architect boundary
  - main-agent notes that exact opened-inode lookup was intentionally deferred to `EXR-MOUNT-09`
- Local focus:
  - `EXR-MOUNT-09` owns shared filesystem state and open sequencing
  - `EXR-UPCASE-07B` owns fold-and-hash service only
  - `EXR-DIR-10` owns directory lookup policy
  - `EXR-PGCACHE-11B` and `READ` components own page-cache and mapping behavior

## Quality Prior Inputs

- Use `Q-ARCH` from `kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_CODE_QUALITY_PRIORS.md`
- In scope:
  - small dependency-safe split
  - clear ownership of shared state
  - preventing bleed into later directory or page-cache components
- Out of scope:
  - creator-local implementation detail

## Prior Delivery Notes

- Keep mount ownership explicit but narrow.
- Make sure the architect artifact names what stays out of scope just as clearly as what enters mount ownership.
- Expose any ready-next parallel wave after mount architecting, rather than only a linear chain.

## Temporary Interfaces And Exit Plan

- No temporary interface is authorized.

## Helper Justification

- No helper API is pre-authorized by this packet beyond the eventual mount-owned shared-state boundary.

## Allowed Commands

- read-only shell inspection commands only

## Parallelism Classification

- Lane class:
  - command-free
- May overlap with:
  - `EXR-UPCASE-07B` designer work
  - other command-free lanes with disjoint write sets
- Known conflicts:
  - any lane writing `EXR-MOUNT-09` architect artifacts

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

## Stop Condition

- Stop after writing `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-MOUNT-09/00_architect.md`.
- Do not write designer artifacts, update the task board, or touch production code.

## Escalation Rule

- If the component cannot stay confined to mount bootstrap and shared-state ownership, stop and report exactly what pressure is trying to pull directory policy, page-cache behavior, or write-path allocation into this component.
