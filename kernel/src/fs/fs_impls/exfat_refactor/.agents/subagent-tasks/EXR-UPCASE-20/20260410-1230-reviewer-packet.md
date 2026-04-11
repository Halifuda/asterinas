<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-UPCASE-20-20260410-1230-REVIEW`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-UPCASE-20/20260410-1230-reviewer-packet.md`
- Supersedes: None
- Role: `reviewer`
- Component: `EXR-UPCASE-20`
- Phase: `review`
- Authorizing main agent: `main-agent`
- Date: `2026-04-10 12:30 CST`

## Goal

- Review the landed upcase-table ownership and canonicalization services after checker evidence, focusing on owner-private boundary discipline, maintainability, and residual local risks.

## Architectural Unit Context

- Functional goal: `ExfatFs`-owned validated upcase table plus UTF-16 folding and exFAT name hashing
- Final architectural owner: `ExfatFs`
- Expected landing form: owner-private `UpcaseTable` state and owner methods in `fs.rs`

## Required Resolution Questions

- Confirm the landed shape still matches the designer boundary and does not widen into directory traversal, mount sequencing, or a generic text helper module.
- Look for local correctness, maintainability, or test-quality issues in `fs.rs`.
- If a bounded in-scope production edit materially improves quality, make it and record it; otherwise leave production code untouched.

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-20/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-20/02_designer_async.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-20/03_designer_ktest.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-20/10_creator_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-20/11_checker_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-20/30_reviewer_report.md`

## Forbidden Files

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/`
- Sibling component artifacts.

## Required Inputs

- Skill to use: `$exfat-subagent-workflow`
- Required role reference: `references/reviewer.md`

## Semantic Prior Inputs

- Use the accepted designer constraints only. Do not reopen Linux or Microsoft exFAT behavior beyond the designer-approved upcase-table and folded-name-hash semantics.

## Integration Prior Inputs

- Treat checker evidence as authoritative for runtime behavior. Reviewer work is bounded code-quality review, not another validation pass.

## Workflow Prior Inputs

- Command-free reviewer lane.
- If no production edits are needed, say so explicitly in the report.
- If production edits are made, keep them strictly local to `fs.rs`.

## Quality Prior Inputs

- Use the reviewer-role quality slice from `$exfat-subagent-workflow`.

## Temporary Interfaces And Exit Plan

- Keep the upcase table immutable after publication.
- Do not add a generic text helper module, fallback locale table, or mount/open sequencing shell.

## Helper Justification

- Small helper or documentation reshaping is allowed only if it clearly improves local readability or invariant protection without widening scope.

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

- Stop after writing `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-20/30_reviewer_report.md`

## Escalation Rule

- If review would require edits outside `fs.rs` or suggests a broader architectural correction, report that and stop.
