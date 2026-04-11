<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-BITMAP-21-20260410-1455-REVIEW`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-BITMAP-21/20260410-1455-reviewer-packet.md`
- Supersedes: None
- Role: `reviewer`
- Component: `EXR-BITMAP-21`
- Phase: `review`
- Authorizing main agent: `main-agent`
- Date: `2026-04-10 14:55 CST`

## Goal

- Review the landed allocation-bitmap owner boundary after successful checker evidence, focusing on owner-private boundary discipline, maintainability, local invariant expression, and residual bitmap-facing risks.

## Architectural Unit Context

- Functional goal: `ExfatFs`-owned validated allocation bitmap snapshot plus occupancy/accounting queries
- Final architectural owner: `ExfatFs`
- Expected landing form: owner-internal `AllocationBitmap` state in `bitmap.rs` plus owner methods in `fs.rs`

## Required Resolution Questions

- Confirm the landed shape still matches the designer boundary and remains a read-only bitmap snapshot rather than drifting toward allocator policy or mount sequencing.
- Look for local correctness, maintainability, invariant-expression, or test-quality issues in `bitmap.rs`, the bitmap-facing parts of `fs.rs`, and `mod.rs`.
- If a bounded in-scope production edit materially improves quality, make it and record it; otherwise leave production code untouched.

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BITMAP-21/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BITMAP-21/02_designer_async.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BITMAP-21/03_designer_ktest.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BITMAP-21/10_creator_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BITMAP-21/11_checker_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/mod.rs`

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BITMAP-21/30_reviewer_report.md`

## Forbidden Files

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/`
- Sibling component artifacts.

## Required Inputs

- Skill to use: `$exfat-subagent-workflow`
- Required role reference: `references/reviewer.md`

## Semantic Prior Inputs

- Use the accepted designer constraints only.
- Preserve `AllocationBitmap` as one immutable owner-local snapshot.
- Do not reopen Linux or Microsoft exFAT behavior beyond the designer-approved validation, occupancy, and accounting semantics.

## Integration Prior Inputs

- Treat checker evidence as authoritative for runtime behavior.
- Reviewer work is bounded code-quality review, not another validation pass.
- The checker already added local `bitmap.rs` regressions; prefer preserving that locality rather than moving tests around unless a small edit clearly improves readability.

## Workflow Prior Inputs

- Command-free reviewer lane.
- If no production edits are needed, say so explicitly in the report.
- If production edits are made, keep them strictly local to `bitmap.rs`, the bitmap-facing parts of `fs.rs`, or `mod.rs`.

## Quality Prior Inputs

- Use the reviewer-role quality slice from `$exfat-subagent-workflow`.

## Temporary Interfaces And Exit Plan

- Preserve the row as a read-only snapshot boundary.
- Do not widen this row into allocator search, FAT mutation, directory traversal, or mount/open sequencing.

## Helper Justification

- Small helper or documentation reshaping is allowed only if it clearly improves local readability or invariant protection without widening scope.

## Allowed Commands

- Read-only shell inspection commands under `/home/halifuda/asterinas`

## Parallelism Classification

- Lane class: `command-free review`
- May overlap with other command-free lanes only
- Known conflicts:
  - `bitmap.rs`
  - `fs.rs`
  - `mod.rs`

## Execution Environment

- Host workspace only

## Execution Lock

- None

## Stop Condition

- Stop after writing `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BITMAP-21/30_reviewer_report.md`

## Escalation Rule

- If review would require edits outside `bitmap.rs`, `fs.rs`, or `mod.rs`, or suggests a broader architectural correction, report that and stop.
