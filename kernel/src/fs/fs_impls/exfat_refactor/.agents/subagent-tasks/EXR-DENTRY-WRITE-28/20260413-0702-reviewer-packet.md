<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-DENTRY-WRITE-28-20260413-0702-REVIEW`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-DENTRY-WRITE-28/20260413-0702-reviewer-packet.md`
- Supersedes: None
- Role: `reviewer`
- Component: `EXR-DENTRY-WRITE-28`
- Phase: `review`
- Authorizing main agent: `main-agent`
- Date: `2026-04-13 07:02 CST`

## Goal

- Review the landed `EXR-DENTRY-WRITE-28` directory-write implementation after checker evidence, focusing on owner-boundary discipline, helper shape, temporary-surface hygiene, and local correctness risk.

## Architectural Unit Context

- Functional goal: write-side directory-entry mutation in `DirectoryEngine`
- Final architectural owner: `ExfatFs` internal `DirectoryEngine`
- Expected landing form: owner-private write methods and helpers in `directory.rs`

## Required Resolution Questions

- Confirm the landed shape still matches the designer boundary and has not widened into namespace policy, allocation search, or a standalone manager.
- Review every new helper, local record/update shape, or temporary seam in `directory.rs` and decide whether it is justified owner-private landing or packet-convenience drift.
- If no findings remain after checker, say so explicitly in the reviewer report.

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DENTRY-WRITE-28/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DENTRY-WRITE-28/02_designer_async.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DENTRY-WRITE-28/03_designer_ktest.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DENTRY-WRITE-28/10_creator_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DENTRY-WRITE-28/11_checker_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs`

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DENTRY-WRITE-28/30_reviewer_report.md`

## Forbidden Files

- production code
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/`
- sibling component artifacts

## Required Inputs

- Skill to use: `$exfat-subagent-workflow`
- Required role reference: `references/reviewer.md`

## Semantic Prior Inputs

- `DirectoryEngine` remains the owner of placement, relocation, and tombstoning.
- `EXR-ALLOC-27` remains consumed only through committed allocation results.
- Namespace policy, inode publication, and sync ordering remain outside the row.

## Integration Prior Inputs

- Treat the checker artifact as the authoritative runtime proof for this review lane.
- Review work is bounded code-quality review, not a second verification pass.
- The current helper and location-update shape in `directory.rs` is in scope because this row is likely to be consumed by `EXR-NAMESPACE-29`.

## Workflow Prior Inputs

- Command-free reviewer lane.
- Dispatch only after `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DENTRY-WRITE-28/11_checker_serial.md` exists.
- This is a report-only review lane. Do not edit production code in this packet.
- If no findings are discovered, state that explicitly.

## Quality Prior Inputs

- Use the reviewer-role quality slice from `$exfat-subagent-workflow`.
- Prioritize owner-boundary drift, unjustified helper surfaces, and misleading temporary seams over style notes.

## Temporary Interfaces And Exit Plan

- Keep the directory-write implementation owner-private to `DirectoryEngine`.
- If any temporary helper remains acceptable for now, say why and name the likely owner or removal condition.
- Do not suggest a namespace service or sync layer as cleanup.

## Helper Justification

- Reviewer work is report-only. Any suggested reshaping must appear as a finding or explicit no-finding conclusion in the report.

## Allowed Commands

- Read-only shell inspection commands under `/home/halifuda/asterinas`

## Parallelism Classification

- Lane class: `command-free review`
- May overlap with other command-free lanes only

## Execution Environment

- Host workspace only

## Execution Lock

- None

## Stop Condition

- Stop after writing `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DENTRY-WRITE-28/30_reviewer_report.md`

## Escalation Rule

- If the review suggests a broader architectural correction rather than a bounded quality finding, report that and stop.
