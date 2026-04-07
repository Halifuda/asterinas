<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-INODE-CORE-17-20260407-1105-REVIEW`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-INODE-CORE-17/20260407-1105-reviewer-packet.md`
- Supersedes: None
- Role: `reviewer`
- Component: `EXR-INODE-CORE-17`
- Phase: `review`
- Authorizing main agent: `main-agent`
- Date: `2026-04-07 11:05 CST`

## Goal

- Review the accepted serial implementation and checker results for `EXR-INODE-CORE-17`, make only bounded review-quality edits if needed, and write the reviewer report.

## Architectural Unit Context

- Functional goal: `ExfatInode` VFS metadata carrier.
- Final architectural owner: `ExfatInode`.
- Expected landing form: trait-carrier type plus owner state in `inode.rs`.
- Parent unit: `EXR-FS-CORE-16` owner boundary.
- Interfaces served: VFS `Inode`, VFS `InodeIo`, future inode cache and data-path owners.

## Required Resolution Questions

- Confirm the implementation still reads as a stable `ExfatInode` carrier rather than a dentry-set, chain, or cache-key wrapper.
- Confirm `Weak<ExfatFs>` remains the only filesystem ownership edge and no strong cycle was introduced.
- Confirm data-path and mutation methods remain explicit temporary seams or rejections.
- Confirm visibility, comments, helpers, and metadata snapshot invariants are narrowly expressed.
- State explicitly whether production code changed and whether any direct edits were functional or semantic.

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/REVIEWER.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/REVIEWER_REPORT_TEMPLATE.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CORE-17/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CORE-17/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CORE-17/03_designer_ktest.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CORE-17/10_creator_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CORE-17/11_checker_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CORE-17/12_checker_serial_retry.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_CODE_QUALITY_PRIORS.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- Read-only inspection commands inside `/home/halifuda/asterinas` are allowed.

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CORE-17/30_reviewer_report.md`

## Forbidden Files

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
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

- Use the local VFS `Inode`, `InodeIo`, and `FileSystem` trait surfaces only as needed for review.

## Workflow Prior Inputs

- Command-free reviewer lane. Do not run build, test, format, Docker, KVM, or QEMU commands.
- Launch only after the serial checker retry has written `12_checker_serial_retry.md`.

## Quality Prior Inputs

- Use `ASTERINAS_CODE_QUALITY_PRIORS.md` profile `Q-REVIEW`.
- Apply the reviewer protocol checklist for temporary seams, visibility, helper justification, and boundary hygiene.

## Temporary Interfaces And Exit Plan

- Preserve explicit data-path temporary seam behavior for later `EXR-READ-OPS-25`, `EXR-WRITE-30`, and `EXR-PGCACHE-26`.
- Preserve setter rejection behavior until a later owner specifies write-side persistence.

## Helper Justification

- Reject or remove helper wrappers and field accessors that lack a named current caller or designer-backed reason.

## Allowed Commands

- Read-only shell inspection only.
- No build, test, format, Docker, KVM, or QEMU commands.

## Parallelism Classification

- Lane class: command-free.
- May overlap with `EXR-FS-CORE-16` reviewer if the sibling reviewer does not edit `inode.rs`.
- Known conflicts: do not overlap production edits with later `EXR-INODE-CACHE-18` creator if it requires inode API changes.

## Execution Environment

- Host read-only inspection and bounded edits under `/home/halifuda/asterinas`.

## Execution Lock

- No execution lock is needed.

## Stop Condition

- Stop after writing `EXR-INODE-CORE-17/30_reviewer_report.md`.

## Escalation Rule

- If review needs semantic redesign or edits outside `inode.rs`, report the issue and stop.
