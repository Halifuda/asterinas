<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-SYSROOT-06-DESIGN-20260404-1408`
- Packet file: `kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-SYSROOT-06/20260404-1408-designer-packet.md`
- Supersedes: none
- Role: designer
- Component: `EXR-SYSROOT-06`
- Phase: design
- Authorizing main agent: main-agent
- Date: 2026-04-04 14:08 CST

## Goal

- Write the bounded designer artifact set for `EXR-SYSROOT-06` so a later creator can implement a root-directory system-entry scanner without guessing about interfaces, validation ownership, or file layout. The design must stop at discovery facts for the `UPCASE` and `BITMAP` root entries and must not absorb concrete loading, mount state, or a general directory API.

## Read Set

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/README.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/DESIGNER_SPEC_TEMPLATE.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/DESIGNER.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-SYSROOT-06/00_architect.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-05B/01_designer_core.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-05B/02_designer_async.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-05B/03_designer_ktest.md`
- `kernel/src/fs/fs_impls/exfat_refactor/dentry.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/fat.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/io.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/super_block.rs`

## Write Set

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-SYSROOT-06/01_designer_core.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-SYSROOT-06/02_designer_async.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-SYSROOT-06/03_designer_ktest.md`

## Forbidden Files

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/*`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-SYSROOT-06/00_architect.md`
- all production Rust code

## Required Inputs

- Role protocol files accompanying this packet:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/DESIGNER.md`
- Required artifact/style inputs:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/DESIGNER_SPEC_TEMPLATE.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-SYSROOT-06/00_architect.md`
  - the recent split-designer examples from `EXR-INODE-05B`
- Required code/reference inputs:
  - current refactor read-side modules listed in the read set

## Semantic Prior Inputs

- Use:
  - prior-derived semantic constraints from `EXR-SYSROOT-06/00_architect.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/Microsoft-exFAT-spec.md` only as needed for exact root-entry semantics
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/linux-exFAT-implementation-summary.md` only as needed for the split between discovery and later loading
- Precedence:
  - Microsoft exFAT rules first
  - Linux implementation guidance second
- Semantic focus:
  - root-directory `UPCASE` and `BITMAP` entry identity
  - discovery result shape for later loaders
  - discovery-time validation versus loader-time validation

## Local Architectural Prior Inputs

- Use prior-derived integration constraints from `EXR-SYSROOT-06/00_architect.md`
- Additional local focus:
  - safe-Rust-only kernel boundary
  - reuse of accepted root-chain and root-metadata inputs instead of inventing a mount object
  - preserving immediate downstream readiness for `EXR-UPCASE-07A` and `EXR-BITMAP-08A`

## Quality Prior Inputs

- Use `Q-DESIGN` from `kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_CODE_QUALITY_PRIORS.md`
- In scope:
  - canonical interfaces
  - hidden implementation details
  - helper minimization
  - explicit temporary-surface prohibition unless justified
- Out of scope:
  - creator-local naming or formatting detail
  - checker implementation detail beyond test obligations

## Prior Delivery Notes

- Keep this design packet narrow around one creator-facing unit: a synchronous, read-only root-entry scanner and its result types.
- Do not reopen mount sequencing, open-inode lookup, page-cache, name folding, or allocation policy.
- The architect artifact already records the scheduler boundary; design work should translate that into precise interfaces and validation ownership.

## Temporary Interfaces And Exit Plan

- No temporary interface is authorized.
- If the component appears to need a staging-only helper or wrapper, stop and explain the pressure in the designer artifact instead of silently permitting it.

## Helper Justification

- One canonical scanner entry point is expected.
- One read-only result surface is expected for later loaders to consume.
- Any additional short helper or field-exposing accessor must name its downstream caller:
  - either the scanner implementation itself,
  - or a later `EXR-UPCASE-07A` / `EXR-BITMAP-08A` loader.
- If no such caller exists yet, the helper must not be specified.

## Allowed Commands

- read-only shell inspection commands only

## Parallelism Classification

- Lane class:
  - command-free
- May overlap with:
  - command-free planning or architect/design lanes with disjoint write sets
- Known conflicts:
  - any lane writing `EXR-SYSROOT-06` designer artifacts

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

- Write `01_designer_core.md` and `03_designer_ktest.md`.
- Write `02_designer_async.md` only if the component has meaningful concurrency, serialization, or lock-order obligations that later roles cannot safely infer from the core spec alone.
- If `02_designer_async.md` is omitted, say explicitly in the designer artifacts why no separate async artifact is needed.
- Do not implement code, write checker results, or update the task board.

## Escalation Rule

- If the design cannot stay confined to a synchronous scanner plus read-only discovery results, stop and report exactly which boundary pressure is forcing drift toward mount bootstrap, general directory APIs, or concrete `UPCASE` / `BITMAP` loading.
