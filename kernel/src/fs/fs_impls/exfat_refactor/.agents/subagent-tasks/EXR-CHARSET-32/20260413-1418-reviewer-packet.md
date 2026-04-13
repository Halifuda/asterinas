<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-CHARSET-32-20260413-1418-REVIEW`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-CHARSET-32/20260413-1418-reviewer-packet.md`
- Supersedes: None
- Role: `reviewer`
- Component: `EXR-CHARSET-32`
- Phase: `review`
- Authorizing main agent: `main-agent`
- Date: `2026-04-13 14:18 CST`

## Goal

- Review the landed `ExfatFs` charset boundary after checker evidence, focusing on owner-private boundary discipline, legacy read-side consumer migration quality, maintainability, and residual local risks.

## Architectural Unit Context

- Functional goal: `ExfatFs`-owned charset and visible-name conversion boundary
- Final architectural owner: `ExfatFs`
- Expected landing form: owner-private converted-name / converted-label value types and owner methods in `fs.rs`, with narrow consumer migration in `inode.rs`

## Required Resolution Questions

- Confirm the landed shape still matches the designer boundary and does not widen into a generic text helper subsystem.
- Review the new converted-name, converted-label, and visible-name decode surfaces as either justified owner-private helpers or ownerless convenience seams.
- Confirm the `inode.rs` migration remains narrow and does not reopen `EXR-DIR-OPS-23` ownership or inject local conversion policy back into read-side code.
- Look for local correctness, maintainability, or test-quality risks in `fs.rs` and the narrow `inode.rs` consumer migration.
- If no findings remain after checker, say so explicitly in the reviewer report.

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-CHARSET-32/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-CHARSET-32/02_designer_async.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-CHARSET-32/03_designer_ktest.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-CHARSET-32/10_creator_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-CHARSET-32/11_checker_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-OPS-23/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-CHARSET-32/30_reviewer_report.md`

## Forbidden Files

- production code
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/`
- sibling component artifacts

## Required Inputs

- Skill to use: `$exfat-subagent-workflow`
- Required role reference: `references/reviewer.md`

## Semantic Prior Inputs

- Use the accepted designer constraints only.
- `EXR-UPCASE-20` remains the sole fold/hash owner; review should reject any charset helper that starts carrying canonicalization or hash state.
- Accepted read-side `lookup` and `readdir_at` remain `EXR-DIR-OPS-23` behavior; this review is about consumer migration quality, not read-side redesign.

## Integration Prior Inputs

- Treat checker evidence as authoritative for runtime behavior.
- Reviewer work is bounded code-quality review, not another validation pass.
- The current converted-name / converted-label / visible-name decode surfaces are in scope because they are new owner-private helper boundaries introduced by creator.

## Workflow Prior Inputs

- Command-free reviewer lane.
- Dispatch only after `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-CHARSET-32/11_checker_serial.md` exists.
- This is a report-only review lane. Do not edit production code in this packet.
- If no findings are discovered, state that explicitly.

## Quality Prior Inputs

- Use the reviewer-role quality slice from `$exfat-subagent-workflow`.
- Prioritize behavioural regression risk, owner-boundary drift, and temporary-seam hygiene over style nits.

## Temporary Interfaces And Exit Plan

- Keep charset encode/decode helpers owner-private to `ExfatFs`.
- Do not suggest a generic Unicode helper module, second text subsystem, or VFS-surface signature change as a “cleanup.”
- If a borderline helper remains acceptable for now, say why and name the likely future owner or removal condition already implied by the checked artifacts.

## Helper Justification

- Reviewer work is report-only. Any suggested reshaping must appear as a finding or note in the report rather than as a code edit.

## Allowed Commands

- Read-only shell inspection commands under `/home/halifuda/asterinas`

## Parallelism Classification

- Lane class: `command-free review`
- May overlap with other command-free lanes only
- Known conflicts: none beyond its own reviewer artifact

## Execution Environment

- Host workspace only

## Execution Lock

- None

## Stop Condition

- Stop after writing `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-CHARSET-32/30_reviewer_report.md`

## Escalation Rule

- If the review suggests a broader architectural correction rather than a bounded quality finding, report that and stop.
