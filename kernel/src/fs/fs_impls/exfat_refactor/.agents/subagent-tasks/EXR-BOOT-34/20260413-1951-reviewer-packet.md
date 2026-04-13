<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-BOOT-34-20260413-1951-REVIEW`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-BOOT-34/20260413-1951-reviewer-packet.md`
- Supersedes: None
- Role: `reviewer`
- Component: `EXR-BOOT-34`
- Phase: `review`
- Authorizing main agent: `main-agent`
- Date: `2026-04-13 19:51 CST`

## Goal

- Review the landed `EXR-BOOT-34` boot-policy publication boundary after serial checker evidence, focusing on owner-private boundary discipline, helper shape, publication-state hygiene, and residual local risks in `fs.rs`.

## Architectural Unit Context

- Functional goal: `ExfatFs` boot-region fallback and persistent boot-flag policy
- Final architectural owner: `ExfatFs`
- Expected landing form: owner-private helpers and owner-private state in `fs.rs`

## Required Resolution Questions

- Confirm the landed boot-policy carriers and helpers remain owner-private to `ExfatFs` and do not read like a second boot parser, recovery worker, or sync manager.
- Review `BootSource`, `BootDirtyIntent`, `BootPolicySnapshot`, and `BootPolicyState` as either justified owner-private record shapes or ownerless convenience seams.
- Confirm `publish_boot_policy()` and `published_boot_dirty_intent()` are narrow enough for the current landing form and do not accidentally expose policy ownership to later consumers.
- Check whether the current `open_root_inode()` publication point matches the intended mount/open handoff without re-owning `EXR-FS-OPEN-22`.
- Look for local maintainability or boundary-quality risks in the landed `fs.rs` slice and in the checker-added local tests.
- If no findings remain after checker, say so explicitly in the reviewer report.

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BOOT-34/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BOOT-34/02_designer_async.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BOOT-34/03_designer_ktest.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BOOT-34/10_creator_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BOOT-34/11_checker_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BOOT-34/30_reviewer_report.md`

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
- `EXR-BOOT-34` owns publication of one stable boot-policy snapshot on `ExfatFs`; it does not own boot-sector parsing, backup checksum validation, background recovery, or filesystem-wide sync ordering.
- `PercentInUse` remains observational only.
- `ClearToZero` remains a persistent pre-mutation guard, not a transient sync-only flag.

## Integration Prior Inputs

- Treat `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BOOT-34/11_checker_serial.md` as authoritative runtime evidence.
- The serial checker already proved the local boot-policy slice with exact-name `#[ktest]` coverage; reviewer work is bounded code-quality review, not another validation pass.
- `fs.rs` currently also contains earlier same-day charset changes that are outside this component's ownership.
  - Treat those regions as background context only.
  - Do not turn unrelated charset staging into a reviewer finding unless it directly causes owner-boundary drift inside the landed boot-policy slice.

## Workflow Prior Inputs

- Command-free reviewer lane.
- Dispatch only after `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BOOT-34/11_checker_serial.md` exists.
- This is a report-only review lane. Do not edit production code in this packet.
- If no findings are discovered, state that explicitly.

## Quality Prior Inputs

- Use the reviewer-role quality slice from `$exfat-subagent-workflow`.
- Prioritize behavioural regression risk, owner-boundary drift, temporary-seam hygiene, and helper justification over style nits.

## Temporary Interfaces And Exit Plan

- The current boot-policy helpers may remain owner-private on `ExfatFs` if the report judges them justified for now.
- Do not suggest a generic boot-policy subsystem, second parser, or public API as a cleanup.
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

- Stop after writing `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BOOT-34/30_reviewer_report.md`

## Escalation Rule

- If the review suggests a broader architectural correction rather than a bounded quality finding, report that and stop.
