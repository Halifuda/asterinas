<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-DIR-OPS-23-20260412-0904-REVIEW`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-DIR-OPS-23/20260412-0904-reviewer-packet.md`
- Supersedes: None
- Role: `reviewer`
- Component: `EXR-DIR-OPS-23`
- Phase: `review`
- Authorizing main agent: `main-agent`
- Date: `2026-04-12 09:04 CST`

## Goal

- Review the currently landed `EXR-DIR-OPS-23` implementation after the invalid readdir misdiagnosis chain was pruned.
- Focus on production owner boundaries, local correctness risks, and whether the current landing form still matches the designer boundary.

## Architectural Unit Context

- Functional goal: `ExfatInode` read-only directory `lookup` and `readdir_at`
- Final architectural owner: `ExfatInode`
- Expected landing form: owner methods on `ExfatInode`, consuming filesystem-owned directory-stream, canonicalization, and canonical-child publication bridges

## Required Resolution Questions

- Confirm the current production shape in `inode.rs`, `fs.rs`, and `directory.rs` still matches the repaired designer boundary.
- Check for local bugs, owner-boundary drift, ownerless helpers, or unjustified temporary seams in the current landed shape.
- Distinguish clearly between:
  - acceptable-for-now owner-private helper or local record type,
  - document-and-defer seam,
  - and refactor-now ownerless surface.
- If a bounded in-scope production edit materially improves quality, make it and record it; otherwise leave production code untouched.

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-OPS-23/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-OPS-23/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-OPS-23/02_designer_async.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-OPS-23/03_designer_ktest.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-OPS-23/10_creator_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-OPS-23/12_creator_serial_repair.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-OPS-23/22_creator_serial_readdir_fs_lifetime_repair.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-OPS-23/23_advisor_directory_stream_owner_shape.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs`

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-OPS-23/30_reviewer_report.md`

## Forbidden Files

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/`
- Sibling component artifacts and packets outside `EXR-DIR-OPS-23`

## Required Inputs

- Skill to use: `$exfat-subagent-workflow`
- Required role reference: `references/reviewer.md`

## Semantic Prior Inputs

- Treat the pruned readdir misdiagnosis chain as intentionally removed and out of scope. Do not try to reconstruct or rely on deleted checker/advisor artifacts from the failed diagnosis loop.
- Treat the currently valid readdir continuity facts as:
  - the two `readdir_*` tests now keep `Arc<ExfatFs>` alive, and
  - the owner-shape recommendation is to keep `directory_stream` filesystem-owned with at most a thin inode-private wrapper.
- This is a bounded code-quality review, not a new runtime diagnosis.

## Workflow Prior Inputs

- Command-free reviewer lane.
- If no production edits are needed, say so explicitly in the report.
- If production edits are made, keep them strictly local to `inode.rs`, `fs.rs`, or `directory.rs`.
- Unless a production concern directly depends on them, do not spend the review budget on `#[cfg(ktest)]` fixture convenience surfaces.

## Quality Prior Inputs

- Use the reviewer-role quality slice from `$exfat-subagent-workflow`.

## Temporary Interfaces And Exit Plan

- Preserve `lookup` and `readdir_at` as `ExfatInode` methods.
- Preserve `directory_stream` as an `ExfatFs` bridge unless you find a concrete owner-boundary problem that forces a narrower inode-private wrapper recommendation.
- Do not widen into mount/open, namespace mutation, allocator policy, or regular-file mapping.

## Allowed Commands

- Read-only shell inspection commands under `/home/halifuda/asterinas`

## Parallelism Classification

- Lane class: `command-free review`
- May overlap with other command-free lanes only
- Known conflicts:
  - `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/directory.rs`

## Execution Environment

- Host workspace only

## Execution Lock

- None

## Stop Condition

- Stop after writing `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-OPS-23/30_reviewer_report.md`

## Escalation Rule

- If review would require broader architectural redesign or another runtime-diagnosis loop, record that and stop instead of widening scope.
