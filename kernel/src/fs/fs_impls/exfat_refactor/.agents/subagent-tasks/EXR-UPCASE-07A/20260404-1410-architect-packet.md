<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-UPCASE-07A-ARCH-20260404-1410`
- Packet file: `kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-UPCASE-07A/20260404-1410-architect-packet.md`
- Supersedes: none
- Role: architect
- Component: `EXR-UPCASE-07A`
- Phase: architect
- Authorizing main agent: main-agent
- Date: 2026-04-04 14:10 CST

## Goal

- Write the architect artifact for `EXR-UPCASE-07A` in `00_architect.md`. The component must cover only on-disk upcase-table loading and validation using the root-entry discovery facts from `EXR-SYSROOT-06`. It must not own case folding, name hashing, mount policy, NLS policy, or general filename conversion.

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
- `kernel/src/fs/fs_impls/exfat/upcase_table.rs`
- `/home/halifuda/linux/fs/exfat/nls.c`
- `/home/halifuda/linux/fs/exfat/super.c`

## Write Set

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07A/00_architect.md`

## Forbidden Files

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/*`
- any file under `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07A/` other than `00_architect.md`
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
  - legacy Asterinas upcase loader in `upcase_table.rs`
  - Linux upcase loader in `nls.c`

## Semantic Prior Inputs

- Use:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/Microsoft-exFAT-spec.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/linux-exFAT-implementation-summary.md`
  - Linux `nls.c` as needed for exact loading and checksum behavior
- Precedence:
  - Microsoft exFAT rules first
  - Linux exFAT implementation guidance second
- Semantic focus:
  - upcase-table entry semantics
  - table size and checksum validation
  - loading the on-disk table contents only

## Local Architectural Prior Inputs

- Use selected integration constraints from:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_ARCHITECT_PRIORS.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-SYSROOT-06/00_architect.md`
- Local architectural focus:
  - `EXR-SYSROOT-06` owns discovery of the upcase root entry
  - `EXR-UPCASE-07A` owns loading and validating the table bytes from that descriptor
  - `EXR-UPCASE-07B` owns later case folding and name-hash services

## Quality Prior Inputs

- Use `Q-ARCH` from `kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_CODE_QUALITY_PRIORS.md`
- In scope:
  - small dependency-safe split
  - preserving one canonical loaded-table surface
  - preventing overlap with `UPCASE-07B`
- Out of scope:
  - creator-local implementation detail
  - final naming or formatting detail

## Prior Delivery Notes

- Keep this packet narrow around the first upcase component only: load and validate the table from the already-discovered root entry.
- Do not widen into fallback name conversion policy, case folding, or name hashing. Those belong later.
- Use legacy Asterinas and Linux only as split pressure and algorithm references, not as reasons to keep mount-time coupling.

## Temporary Interfaces And Exit Plan

- No temporary interface is authorized.

## Helper Justification

- No helper API is pre-authorized by this packet beyond the eventual loaded-table surface.
- If the architect believes a short helper is needed, the artifact must name whether the caller is `EXR-UPCASE-07B` or the local loader only.

## Allowed Commands

- read-only shell inspection commands only

## Parallelism Classification

- Lane class:
  - command-free
- May overlap with:
  - `EXR-BITMAP-08A` architect work
  - `EXR-SYSROOT-06` design work
  - other command-free lanes with disjoint write sets
- Known conflicts:
  - any lane writing `EXR-UPCASE-07A` architect artifacts

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

- Stop after writing `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07A/00_architect.md`.
- Do not write designer artifacts, update the task board, or touch production code.

## Escalation Rule

- If the component cannot stay confined to on-disk table loading and validation, stop and report exactly what pressure is trying to pull case folding, name hashing, mount policy, or fallback table behavior into this component.
