<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-ALLOC-27-20260412-2231-REVIEW`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-ALLOC-27/20260412-2231-reviewer-packet.md`
- Supersedes: None
- Role: `reviewer`
- Component: `EXR-ALLOC-27`
- Phase: `review`
- Authorizing main agent: `main-agent`
- Date: `2026-04-12 22:31 CST`

## Goal

- Review the checked allocator landing after exact filtered proof, focusing on owner-boundary discipline, committed-result shape, metadata-write helper hygiene, and whether the checked implementation still matches the accepted designer contract.

## Architectural Unit Context

- Functional goal: `ExfatFs`-owned cluster allocation service
- Final architectural owner: `ExfatFs`
- Expected landing form: owner-internal service plus owner methods across `allocator.rs`, `bitmap.rs`, `fat.rs`, and `fs.rs`

## Required Resolution Questions

- Confirm the landed shape still keeps allocation search, reservation intent, and commit under `ExfatFs` instead of widening into a standalone free-space manager.
- Confirm `AllocationResult` remains the only small committed result later namespace and write rows need to consume.
- Review the sector-aligned metadata-write repair in `bitmap.rs` and `fat.rs` as either acceptable owner-local hygiene or boundary drift.
- Look for local correctness or maintainability risks in the allocator-owned write set after the checker fix.
- If no findings remain after checker, say so explicitly in the reviewer report.

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-ALLOC-27/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-ALLOC-27/02_designer_async.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-ALLOC-27/03_designer_ktest.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-ALLOC-27/10_creator_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-ALLOC-27/11_checker_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/allocator.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fat.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/mod.rs`

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-ALLOC-27/30_reviewer_report.md`

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
- `EXR-ALLOC-27` remains the only owner of allocation search, reservation intent, and bitmap/FAT commit under `ExfatFs`.
- Later namespace and write rows may consume only the committed `AllocationResult`, not bitmap or FAT internals.

## Integration Prior Inputs

- Treat `11_checker_serial.md` as the authoritative runtime proof for contiguous preference, fragmented fallback, reservation privacy, and commit coherence.
- Reviewer work is a bounded quality pass, not another checker rerun.
- The sector-aligned metadata-write repair is in scope because it was a checker-recorded local production fix inside the authorized write set.

## Workflow Prior Inputs

- Command-free reviewer lane.
- Dispatch only after `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-ALLOC-27/11_checker_serial.md` exists.
- This is a report-only review lane. Do not edit production code in this packet.
- If no findings are discovered, state that explicitly.

## Quality Prior Inputs

- Use the reviewer-role quality slice from `$exfat-subagent-workflow`.
- Prioritize behavioral regressions, owner-boundary drift, and misleading temporary surfaces over style nits.

## Temporary Interfaces And Exit Plan

- Keep the committed result shape small and copyable.
- If the sector-aligned metadata-write helpers are acceptable for now, say why they remain subordinate to allocator ownership.
- Do not suggest a public reservation API, a deallocator facade, or a sync shell as cleanup.

## Helper Justification

- Reviewer work is report-only. Any reshaping recommendation must appear as a finding or note rather than as a code edit.

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

- Stop after writing `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-ALLOC-27/30_reviewer_report.md`

## Escalation Rule

- If the review suggests a broader architectural correction rather than a bounded quality finding, report that and stop.
