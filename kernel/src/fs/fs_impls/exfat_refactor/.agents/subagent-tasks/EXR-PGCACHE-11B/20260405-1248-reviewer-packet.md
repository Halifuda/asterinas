<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-PGCACHE-11B-REVIEW-20260405-1248`
- Packet file: `kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-PGCACHE-11B/20260405-1248-reviewer-packet.md`
- Supersedes:
  - `EXR-PGCACHE-11B-REVIEW-20260405-1216`
- Role: reviewer
- Component: `EXR-PGCACHE-11B`
- Phase: reviewer
- Authorizing main agent: main-agent
- Date: 2026-04-05 12:48 CST

## Goal

- Review `EXR-PGCACHE-11B` after the repaired creator and checker retry passes. Focus on helper justification, backend ownership, and whether the page-cache surface stayed narrow. Write `30_reviewer_report.md`.

## Read Set

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/README.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/REVIEWER_REPORT_TEMPLATE.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/REVIEWER.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_CODE_QUALITY_PRIORS.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-11B/00_architect.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-11B/01_designer_core.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-11B/12_creator_serial_retry.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-11B/13_checker_serial_retry.md`
- `kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/read.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- `kernel/src/fs/vfs/page_cache.rs`

## Write Set

- `kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/read.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-11B/30_reviewer_report.md`

## Forbidden Files

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/*`
- checker artifacts other than the required read-only checker retry artifact
- final-checker artifacts

## Required Inputs

- Role protocol files accompanying this packet:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/REVIEWER.md`
- Required artifact/style inputs:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/REVIEWER_REPORT_TEMPLATE.md`
  - delegated creator repair artifact `12_creator_serial_retry.md`
  - delegated checker retry artifact `13_checker_serial_retry.md`

## Semantic Prior Inputs

- Use only the semantic constraints already captured in the architect, designer, and checker artifacts.

## Local Architectural Prior Inputs

- Use the local constraints already captured in the architect, designer, and checker artifacts.
- Local focus:
  - singular backend ownership
  - visible-length page-count rule
  - no buffered-read, zero-fill-policy, or growth drift

## Quality Prior Inputs

- Use `Q-REVIEW` from `kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_CODE_QUALITY_PRIORS.md`
- In scope:
  - helper and accessor justification
  - visibility and API shape
  - keeping page-cache backend ownership canonical and narrow
- Out of scope:
  - runtime verification

## Prior Delivery Notes

- Direct edits are allowed only within the write set and only when they tighten the existing component boundary.
- The checker retry already owns runtime verification. Do not rerun tests here.

## Temporary Interfaces And Exit Plan

- No temporary interface is authorized.

## Helper Justification

- Remove or inline any helper that lacks a concrete caller-backed reason inside the accepted backend boundary.

## Allowed Commands

- read-only shell inspection commands under `/home/halifuda/asterinas`

## Parallelism Classification

- Lane class:
  - command-free
- May overlap with:
  - no other delegated lane writing `EXR-PGCACHE-11B`
- Known conflicts:
  - creator, checker, and final-checker passes for this component

## Execution Environment

- Host or Docker:
  - host workspace only
- Required command prefix:
  - read-only shell commands under `/home/halifuda/asterinas`
- Required working directory:
  - `/home/halifuda/asterinas`
- Isolation notes:
  - shared worktree; do not edit outside the write set
- This task is command-free with respect to compile and runtime work; the subagent may use read-only inspection commands but must not add build, test, or runtime commands.

## Execution Lock

- not applicable

## Stop Condition

- Stop after any in-scope review edits and after writing `30_reviewer_report.md`.
- Do not run final-checker commands or task-board updates.

## Escalation Rule

- If the component still appears too wide even after bounded review edits, report that boundary failure clearly instead of redesigning it.
