<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-PGCACHE-11B-REVIEW-20260405-1216`
- Packet file: `kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-PGCACHE-11B/20260405-1216-reviewer-packet.md`
- Supersedes: none
- Role: reviewer
- Component: `EXR-PGCACHE-11B`
- Phase: reviewer
- Authorizing main agent: main-agent
- Date: 2026-04-05 12:16 CST

## Goal

- Review `EXR-PGCACHE-11B` after the delegated creator and checker passes. Focus on helper justification, backend ownership, and whether the page-cache surface stayed narrow. Write `30_reviewer_report.md`.

## Read Set

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/README.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/REVIEWER_REPORT_TEMPLATE.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/REVIEWER.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_CODE_QUALITY_PRIORS.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-11B/00_architect.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-11B/01_designer_core.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-11B/10_creator_serial.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-11B/11_checker_serial.md`
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
- checker artifacts other than the required read-only checker artifact
- final-checker artifacts

## Required Inputs

- Role protocol files accompanying this packet:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/REVIEWER.md`
- Required artifact/style inputs:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/REVIEWER_REPORT_TEMPLATE.md`
  - delegated creator artifact `10_creator_serial.md`
  - delegated checker artifact `11_checker_serial.md`

## Semantic Prior Inputs

- Use only the semantic constraints already captured in the architect and designer artifacts.

## Local Architectural Prior Inputs

- Use the local constraints already captured in the architect and designer artifacts.
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
- Do not treat legacy `exfat` layering as mandatory if a narrower refactor-local surface is cleaner.

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
  - command-free planning lanes with disjoint write sets
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
