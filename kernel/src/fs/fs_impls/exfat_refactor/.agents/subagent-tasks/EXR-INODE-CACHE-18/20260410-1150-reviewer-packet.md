<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-INODE-CACHE-18-20260410-1150-REVIEW`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-INODE-CACHE-18/20260410-1150-reviewer-packet.md`
- Supersedes: None
- Role: `reviewer`
- Component: `EXR-INODE-CACHE-18`
- Phase: `review`
- Authorizing main agent: `main-agent`
- Date: `2026-04-10 11:50 CST`

## Goal

- Review the opened-inode cache implementation after successful checker evidence, focusing on owner-private boundary discipline, maintainability, and residual local risks.

## Architectural Unit Context

- Functional goal: `ExfatFs`-owned opened-inode table keyed by `InodeKey`, with a separate root slot
- Final architectural owner: `ExfatFs`
- Expected landing form: owner-internal state plus validated `InodeKey` in `fs.rs`

## Required Resolution Questions

- Confirm the landed shape still matches the designer boundary and does not widen into `EXR-FS-OPEN-22`.
- Look for local correctness, maintainability, or test-quality issues in `fs.rs`.
- If a bounded in-scope production edit materially improves quality, make it and record it; otherwise leave production code untouched.

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CACHE-18/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CACHE-18/03_designer_ktest.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CACHE-18/10_creator_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CACHE-18/11_checker_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CACHE-18/12_creator_serial_repair.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CACHE-18/13_checker_serial_retry.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CACHE-18/30_reviewer_report.md`

## Forbidden Files

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/`
- Sibling component artifacts.

## Required Inputs

- Skill to use: `$exfat-subagent-workflow`
- Required role reference: `references/reviewer.md`

## Semantic Prior Inputs

- Use the accepted designer constraints only. Do not reopen Linux or Microsoft exFAT behavior.

## Integration Prior Inputs

- Treat the retry checker evidence as authoritative for runtime behavior. Reviewer work is bounded code-quality review, not another validation pass.

## Workflow Prior Inputs

- Command-free reviewer lane.
- If no production edits are needed, say so explicitly in the report.
- If production edits are made, keep them strictly local to `fs.rs`.

## Quality Prior Inputs

- Use the reviewer-role quality slice from `$exfat-subagent-workflow`.

## Temporary Interfaces And Exit Plan

- Preserve the temporary root seam for `EXR-FS-OPEN-22`.
- Do not add public cache helpers or synthetic root keys.

## Helper Justification

- Small helper/documentation reshaping is allowed only if it clearly improves local readability or invariants without widening scope.

## Allowed Commands

- Read-only shell inspection commands under `/home/halifuda/asterinas`

## Parallelism Classification

- Lane class: `command-free review`
- May overlap with other command-free lanes only
- Known conflicts:
  - `fs.rs`

## Execution Environment

- Host workspace only

## Execution Lock

- None

## Stop Condition

- Stop after writing `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CACHE-18/30_reviewer_report.md`

## Escalation Rule

- If review would require edits outside `fs.rs` or suggests a broader architectural correction, report that and stop.
