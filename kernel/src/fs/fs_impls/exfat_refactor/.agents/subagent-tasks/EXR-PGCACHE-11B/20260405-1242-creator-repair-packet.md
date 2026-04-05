<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-PGCACHE-11B-CREATE-20260405-1242`
- Packet file: `kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-PGCACHE-11B/20260405-1242-creator-repair-packet.md`
- Supersedes:
  - `EXR-PGCACHE-11B-CREATE-20260405-1216`
- Role: creator
- Component: `EXR-PGCACHE-11B`
- Phase: serial creator repair
- Authorizing main agent: main-agent
- Date: 2026-04-05 12:42 CST

## Goal

- Repair the creator-owned production issue reported by the serial checker for `EXR-PGCACHE-11B`, then record the repair in `12_creator_serial_retry.md`. The current blocking defect is a return-type mismatch in `ExfatRegularFileBackend::{read_page_async, write_page_async}` where the backend returns `BioEnqueueError` instead of the kernel `Error` expected by `PageCacheBackend`.

## Read Set

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/README.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/CREATOR_LOG_TEMPLATE.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/CREATOR.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_CODE_QUALITY_PRIORS.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-11B/00_architect.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-11B/01_designer_core.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-11B/10_creator_serial.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-11B/11_checker_serial.md`
- `kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/read.rs`
- `kernel/src/fs/vfs/page_cache.rs`

## Write Set

- `kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-11B/12_creator_serial_retry.md`

## Forbidden Files

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/*`
- checker, reviewer, and final-checker artifacts for `EXR-PGCACHE-11B`
- downstream `EXR-READ-11B` and write-path component artifacts

## Required Inputs

- Role protocol files accompanying this packet:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/CREATOR.md`
- Required artifact/style inputs:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/CREATOR_LOG_TEMPLATE.md`
  - creator artifact `10_creator_serial.md`
  - checker report `11_checker_serial.md`
- This is a bounded repair pass. Preserve the accepted backend split unless fixing the checker-reported production mismatch requires a narrower helper or error-conversion surface.

## Semantic Prior Inputs

- Use only the semantic constraints already captured in the architect, designer, and checker artifacts.

## Local Architectural Prior Inputs

- Use the local constraints already captured in the architect, designer, and checker artifacts.
- Local focus:
  - keep one canonical `PageCacheBackend` surface
  - preserve `valid_data_length` page-count ownership
  - repair only the production issue blocking checker execution

## Quality Prior Inputs

- Use `Q-CREATE` from `kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_CODE_QUALITY_PRIORS.md`
- In scope:
  - the blocking production mismatch in `fs.rs`
  - any tiny companion adjustment strictly required by that repair
- Out of scope:
  - new checker coverage
  - reviewer-level cleanup

## Prior Delivery Notes

- Keep the repair minimal. The checker already added the local test coverage; do not rework that coverage here.
- If the narrowest repair is only in `fs.rs`, leave `inode.rs` untouched and say so in the artifact.

## Temporary Interfaces And Exit Plan

- No temporary interface is authorized.

## Helper Justification

- No new helper is pre-authorized unless the repair needs one tiny error-conversion or boundary-preserving helper with a named caller in this component.

## Allowed Commands

- read-only shell inspection commands under `/home/halifuda/asterinas`

## Parallelism Classification

- Lane class:
  - command-free
- May overlap with:
  - no other delegated lane writing `EXR-PGCACHE-11B`
- Known conflicts:
  - checker, reviewer, or final-checker passes for `EXR-PGCACHE-11B`

## Execution Environment

- Host or Docker:
  - host workspace only
- Required command prefix:
  - read-only shell commands under `/home/halifuda/asterinas`
- Required working directory:
  - `/home/halifuda/asterinas`
- Isolation notes:
  - shared worktree; do not edit outside the write set
- This task is command-free with respect to build, test, and runtime work; the subagent may use read-only inspection commands only.

## Execution Lock

- not applicable

## Stop Condition

- Stop after applying the production repair, if needed, and writing `12_creator_serial_retry.md`.
- Do not write checker, reviewer, final-checker, or task-board artifacts.

## Escalation Rule

- If the checker-reported mismatch hides a wider architectural problem, stop and report the exact pressure instead of widening scope opportunistically.
