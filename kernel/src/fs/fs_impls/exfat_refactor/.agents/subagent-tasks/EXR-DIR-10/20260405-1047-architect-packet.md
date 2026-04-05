<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-DIR-10-ARCH-20260405-1047`
- Packet file: `kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-DIR-10/20260405-1047-architect-packet.md`
- Supersedes: none
- Role: architect
- Component: `EXR-DIR-10`
- Phase: architect
- Authorizing main agent: main-agent
- Date: 2026-04-05 10:47 CST

## Goal

- Write the architect artifact for `EXR-DIR-10` in `00_architect.md`. The component must own directory iteration and lookup over already-mounted shared state, consume validated file-record and upcase-backed name-hash surfaces, and stay out of mount bootstrap, page-cache-backed file reads, and namespace mutation.

## Read Set

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/README.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/ARCHITECT_TEMPLATE.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/ARCHITECT.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-MOUNT-09/00_architect.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-MOUNT-09/01_designer_core.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-05B/01_designer_core.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07B/01_designer_core.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FILESET-04B/00_architect.md`
- `kernel/src/fs/fs_impls/exfat_refactor/fileset.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/dentry.rs`
- `/home/halifuda/linux/fs/exfat/namei.c`
- `/home/halifuda/linux/fs/exfat/dir.c`

## Write Set

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-10/00_architect.md`

## Forbidden Files

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/*`
- any file under `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-10/` other than `00_architect.md`
- all production Rust code

## Required Inputs

- Role protocol files accompanying this packet:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/ARCHITECT.md`
- Required artifact/style inputs:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/ARCHITECT_TEMPLATE.md`
  - accepted `EXR-MOUNT-09` architect artifact and current designer boundary
  - accepted `EXR-INODE-05B`, `EXR-UPCASE-07B`, and `EXR-FILESET-04B` boundaries

## Semantic Prior Inputs

- Use:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/Microsoft-exFAT-spec.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/linux-exFAT-implementation-summary.md`
  - Linux `namei.c` and `dir.c` as needed for lookup and directory-iteration ownership
- Precedence:
  - Microsoft exFAT rules first
  - Linux exFAT implementation guidance second
- Semantic focus:
  - directory entry iteration from accepted directory-chain and fileset surfaces
  - case-folded name lookup through the accepted upcase-table service
  - separation between directory lookup and later namespace mutation

## Local Architectural Prior Inputs

- Use selected integration constraints from:
  - `EXR-MOUNT-09` architect and designer artifacts
  - `EXR-INODE-05B` designer core
  - `EXR-UPCASE-07B` designer core
  - `EXR-FILESET-04B` architect boundary
- Local focus:
  - directory lookup must consume the mount-owned shared state rather than rediscovering volume resources
  - lookup policy belongs here, not in mount
  - page-cache-backed regular-file reads belong to `EXR-READ-11A` and later components

## Quality Prior Inputs

- Use `Q-ARCH` from `kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_CODE_QUALITY_PRIORS.md`
- In scope:
  - narrow ownership boundary
  - explicit ready-now parallel relationship with `EXR-READ-11A`
  - avoiding bleed into mutation and page-cache work
- Out of scope:
  - creator-local implementation detail

## Prior Delivery Notes

- Keep the component small enough that it is about iteration and lookup only.
- Make the handoff explicit about what later namespace and read-side work should still own.
- Prefer one dependency-safe lookup/iteration slice over a wide "directory plus namespace" bucket.

## Temporary Interfaces And Exit Plan

- No temporary interface is authorized.

## Helper Justification

- No helper API is pre-authorized by this packet beyond the eventual directory/lookup boundary itself.

## Allowed Commands

- read-only shell inspection commands only

## Parallelism Classification

- Lane class:
  - command-free
- May overlap with:
  - `EXR-READ-11A` architect work
  - `EXR-MOUNT-09` creator/checker/reviewer flow
  - other command-free lanes with disjoint write sets
- Known conflicts:
  - any lane writing `EXR-DIR-10` architect artifacts

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

- Stop after writing `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-10/00_architect.md`.
- Do not write designer artifacts, update the task board, or touch production code.

## Escalation Rule

- If the split seems to require mount bootstrap details, page-cache-backed regular-file reads, or namespace mutation in the same component, stop and report the exact pressure instead of widening scope.
