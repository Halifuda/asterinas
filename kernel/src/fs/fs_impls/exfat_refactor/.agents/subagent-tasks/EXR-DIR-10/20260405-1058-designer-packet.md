<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-DIR-10-DESIGN-20260405-1058`
- Packet file: `kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-DIR-10/20260405-1058-designer-packet.md`
- Supersedes: none
- Role: designer
- Component: `EXR-DIR-10`
- Phase: designer
- Authorizing main agent: main-agent
- Date: 2026-04-05 10:58 CST

## Goal

- Produce the bounded designer artifact set for `EXR-DIR-10`: `01_designer_core.md` and `03_designer_ktest.md`, plus `02_designer_async.md` only if the component has concurrency or publication obligations that later roles cannot safely infer from the core spec.

## Read Set

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/README.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/DESIGNER_SPEC_TEMPLATE.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/DESIGNER.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-10/00_architect.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-MOUNT-09/01_designer_core.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07B/01_designer_core.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-05B/01_designer_core.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FILESET-04B/01_designer_spec.md`
- `kernel/src/fs/fs_impls/exfat_refactor/fileset.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/linux/fs/exfat/namei.c`
- `/home/halifuda/linux/fs/exfat/dir.c`

## Write Set

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-10/01_designer_core.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-10/02_designer_async.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-10/03_designer_ktest.md`

## Forbidden Files

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/*`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-10/00_architect.md`
- all production Rust code

## Required Inputs

- Role protocol files accompanying this packet:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/DESIGNER.md`
- Required artifact/style inputs:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/DESIGNER_SPEC_TEMPLATE.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-10/00_architect.md`

## Semantic Prior Inputs

- Use:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/Microsoft-exFAT-spec.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/linux-exFAT-implementation-summary.md`
  - Linux `namei.c` and `dir.c` as needed for lookup and iteration behavior
- Precedence:
  - Microsoft exFAT rules first
  - Linux exFAT implementation guidance second
- Semantic focus:
  - directory iteration from validated directory records
  - canonical name-hash and name-comparison behavior
  - keeping lookup separated from mutation and regular-file reads

## Local Architectural Prior Inputs

- Use integration constraints derived from:
  - `EXR-DIR-10/00_architect.md`
  - `EXR-MOUNT-09/01_designer_core.md`
  - `EXR-UPCASE-07B/01_designer_core.md`
  - `EXR-INODE-05B/01_designer_core.md`
  - `EXR-FILESET-04B/01_designer_spec.md`
- Local focus:
  - mounted shared-state consumption only
  - no second mount path
  - no page-cache-backed file reads
  - no namespace mutation

## Quality Prior Inputs

- Use `Q-DESIGN` from `kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_CODE_QUALITY_PRIORS.md`
- In scope:
  - a single canonical directory entry-point
  - explicit caller-facing helper boundaries only when a named downstream caller needs them
  - checker-owned local ktest obligations
- Out of scope:
  - creator-local naming or formatting choices

## Prior Delivery Notes

- Keep the directory component read-only.
- If `02_designer_async.md` is omitted, say explicitly why no extra concurrency artifact is required.
- Do not specify create, unlink, rename, or file-read behavior here.

## Temporary Interfaces And Exit Plan

- No temporary interface is authorized.

## Helper Justification

- Any helper or accessor must name the downstream directory caller that needs it now and explain why the mounted shared-state boundary cannot stay narrower.

## Allowed Commands

- read-only shell inspection commands only

## Parallelism Classification

- Lane class:
  - command-free
- May overlap with:
  - `EXR-READ-11A` designer work
  - `EXR-MOUNT-09` checker, reviewer, and final-checker flow
  - other command-free lanes with disjoint write sets
- Known conflicts:
  - any lane writing `EXR-DIR-10` designer artifacts

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

- Stop after writing the required designer artifacts for `EXR-DIR-10`.
- Do not write creator artifacts, update the board, or touch production code.

## Escalation Rule

- If the design cannot stay confined to iteration and lookup over mounted shared state, stop and report exactly what pressure is trying to pull mutation, mount bootstrap, or read-path behavior into this component.
