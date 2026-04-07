<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-FS-CORE-16-20260407-1105-REVIEW`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-FS-CORE-16/20260407-1105-reviewer-packet.md`
- Supersedes: None
- Role: `reviewer`
- Component: `EXR-FS-CORE-16`
- Phase: `review`
- Authorizing main agent: `main-agent`
- Date: `2026-04-07 11:05 CST`

## Goal

- Review the accepted serial implementation and checker results for `EXR-FS-CORE-16`, make only bounded review-quality edits if needed, and write the reviewer report.

## Architectural Unit Context

- Functional goal: `ExfatFs` VFS `FileSystem` owner skeleton.
- Final architectural owner: `ExfatFs`.
- Expected landing form: trait-carrier type plus owner state in `fs.rs`.
- Parent unit: none.
- Interfaces served: VFS `FileSystem`, future `EXR-FS-OPEN-22`, future `EXR-SYNC-31`, sibling `EXR-INODE-CORE-17`.

## Required Resolution Questions

- Confirm the implementation still reads as a single `ExfatFs` owner boundary rather than a helper pile.
- Confirm `root_inode()` keeps the explicit `EXR-FS-OPEN-22` temporary seam and is not hidden behind a fake root shell.
- Confirm `sync()` remains a placeholder and does not begin real flush-order ownership.
- Confirm visibility, comments, helpers, and module wiring are narrowly justified.
- State explicitly whether production code changed and whether any direct edits were functional or semantic.

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/REVIEWER.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/REVIEWER_REPORT_TEMPLATE.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-CORE-16/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-CORE-16/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-CORE-16/03_designer_ktest.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-CORE-16/10_creator_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-CORE-16/11_checker_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-CORE-16/12_checker_serial_retry.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_CODE_QUALITY_PRIORS.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- Read-only inspection commands inside `/home/halifuda/asterinas` are allowed.

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-CORE-16/30_reviewer_report.md`

## Forbidden Files

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/`
- Sibling component artifacts.

## Required Inputs

- Role-scoped protocol files: `COMMON_SUBAGENT.md`, `REVIEWER.md`.
- Reviewer report template.
- Designer, creator, and checker artifacts listed above.

## Semantic Prior Inputs

- Use the accepted designer constraints only. Do not reopen Linux behavior.

## Integration Prior Inputs

- Use the local VFS `FileSystem` trait surface only as needed for review.

## Workflow Prior Inputs

- Command-free reviewer lane. Do not run build, test, format, Docker, KVM, or QEMU commands.
- Launch only after the serial checker retry has written `12_checker_serial_retry.md`.

## Quality Prior Inputs

- Use `ASTERINAS_CODE_QUALITY_PRIORS.md` profile `Q-REVIEW`.
- Apply the reviewer protocol checklist for temporary seams, visibility, helper justification, and boundary hygiene.

## Temporary Interfaces And Exit Plan

- Preserve the required `root_inode()` seam comment:
  - `// Temporary seam: EXR-FS-OPEN-22 will install the real root inode after EXR-INODE-CORE-17 lands.`
- Preserve placeholder `sync()` unless there is a narrow quality-only reason to adjust its expression.

## Helper Justification

- Reject or remove helper wrappers and field accessors that lack a named current caller or designer-backed reason.

## Allowed Commands

- Read-only shell inspection only.
- No build, test, format, Docker, KVM, or QEMU commands.

## Parallelism Classification

- Lane class: command-free.
- May overlap with `EXR-INODE-CORE-17` reviewer if the sibling reviewer does not edit `fs.rs` or `mod.rs`.
- Known conflicts: do not overlap production edits with later `EXR-INODE-CACHE-18` creator or `EXR-DIR-ENGINE-19` creator because those may touch `fs.rs` or `mod.rs`.

## Execution Environment

- Host read-only inspection and bounded edits under `/home/halifuda/asterinas`.

## Execution Lock

- No execution lock is needed.

## Stop Condition

- Stop after writing `EXR-FS-CORE-16/30_reviewer_report.md`.

## Escalation Rule

- If review needs semantic redesign or edits outside `fs.rs`/`mod.rs`, report the issue and stop.
