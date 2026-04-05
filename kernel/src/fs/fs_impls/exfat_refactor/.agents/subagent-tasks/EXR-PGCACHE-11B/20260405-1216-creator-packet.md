<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-PGCACHE-11B-CREATE-20260405-1216`
- Packet file: `kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-PGCACHE-11B/20260405-1216-creator-packet.md`
- Supersedes: none
- Role: creator
- Component: `EXR-PGCACHE-11B`
- Phase: serial creator
- Authorizing main agent: main-agent
- Date: 2026-04-05 12:16 CST

## Goal

- Implement the first creator pass for `EXR-PGCACHE-11B`. Add the smallest acceptable regular-file page-cache backend surface for the refactored exFAT path, using accepted mount state plus `EXR-READ-11A` placement facts, and write `10_creator_serial.md`.

## Read Set

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/README.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/CREATOR_LOG_TEMPLATE.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/CREATOR.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_CODE_QUALITY_PRIORS.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-11B/00_architect.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-11B/01_designer_core.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-11B/03_designer_ktest.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-READ-11A/12_creator_serial_retry.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-READ-11A/32_reviewer_followup.md`
- `kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/read.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- `kernel/src/fs/vfs/page_cache.rs`
- `kernel/src/fs/fs_impls/exfat/inode.rs`
- `kernel/src/fs/fs_impls/exfat/fs.rs`

## Write Set

- `kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/read.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-11B/10_creator_serial.md`

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
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-11B/00_architect.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-11B/01_designer_core.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-11B/03_designer_ktest.md`
- The current `EXR-READ-11A` artifacts are authoritative for placement ownership:
  - do not re-derive logical-to-physical mapping outside `read.rs`

## Semantic Prior Inputs

- Use only the semantic constraints already captured in the architect and designer artifacts.
- Treat legacy `exfat` sources as implementation references only, not as acceptance criteria.

## Local Architectural Prior Inputs

- Use the local constraints already captured in the architect and designer artifacts.
- Local focus:
  - one canonical `PageCacheBackend` surface
  - regular-file runtime owns the `PageCache`
  - backend page count comes from `valid_data_length`
  - no buffered `read_at`, zero-fill policy, growth, or truncate behavior

## Quality Prior Inputs

- Use `Q-CREATE` from `kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_CODE_QUALITY_PRIORS.md`
- In scope:
  - narrow backend ownership
  - helper justification
  - local tests or scaffolding only if they are needed by this component boundary
- Out of scope:
  - reviewer-level cleanup outside the assigned files

## Prior Delivery Notes

- Keep this pass narrow. The component should integrate with the existing page-cache trait, not redesign inode or mount ownership.
- Minimal production edits are acceptable if they fully establish the backend contract and the creator artifact explains why.

## Temporary Interfaces And Exit Plan

- No temporary interface is authorized.

## Helper Justification

- Any new helper must name a real cross-module caller inside this component boundary.
- Short field-exposing accessors are allowed only if they keep mount-owned state and regular-file runtime facts narrower than re-opening internals manually.

## Allowed Commands

- read-only shell inspection commands under `/home/halifuda/asterinas`

## Parallelism Classification

- Lane class:
  - command-free
- May overlap with:
  - command-free planning lanes with disjoint write sets
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
- This task is still command-free with respect to compile and runtime work; the subagent may use read-only inspection commands but must not add build, test, or runtime commands.

## Execution Lock

- not applicable

## Stop Condition

- Stop after updating the owned production files, if needed, and writing `10_creator_serial.md`.
- Do not write checker, reviewer, final-checker, or task-board artifacts.

## Escalation Rule

- If the smallest acceptable backend seems to require buffered `read_at`, growth, truncate, or a second mapping path, stop and report the exact pressure instead of widening scope.
