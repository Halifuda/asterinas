<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-DENTRY-WRITE-28-20260413-0652-CREATE-SERIAL`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-DENTRY-WRITE-28/20260413-0652-creator-serial-packet.md`
- Supersedes: None
- Role: `creator`
- Component: `EXR-DENTRY-WRITE-28`
- Phase: `serial creator`
- Authorizing main agent: `main-agent`
- Date: `2026-04-13 06:52 CST`

## Goal

- Implement the serial creator pass for `EXR-DENTRY-WRITE-28` so `DirectoryEngine` gains owner-private write-side directory-entry mutation primitives that consume validated `ExfatDentrySet` values and committed allocation results without absorbing namespace policy, inode publication, allocation search, or sync ordering.

## Architectural Unit Context

- Functional goal: write-side directory-entry mutation in `DirectoryEngine`
- Final architectural owner: `ExfatFs` internal `DirectoryEngine`
- Expected landing form: owner-private write methods and helpers in `directory.rs`
- Interfaces served:
  - later namespace mutation under `EXR-NAMESPACE-29`

## Required Resolution Questions

- Add the write-side `DirectoryEngine` helper surface in `directory.rs` without inventing a standalone directory-write manager.
- Implement slot discovery, in-place rewrite, tombstoning, and relocation/growth handling for validated `ExfatDentrySet` values.
- Consume committed allocation results only as already-decided growth facts; do not re-run allocation search or reservation.
- Keep all new helper and location-update logic owner-private to `DirectoryEngine`.
- If the full serial creator pass still proves too large for one owner-first pass, stop and report the exact reslice boundary instead of improvising a manager or widening scope.

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DENTRY-WRITE-28/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DENTRY-WRITE-28/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DENTRY-WRITE-28/02_designer_async.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DENTRY-WRITE-28/03_designer_ktest.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-ENGINE-19/10_creator_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-ENGINE-19/30_reviewer_report.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FILESET-04B/10_creator_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-ALLOC-27/10_creator_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-ALLOC-27/30_reviewer_report.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fileset.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/dentry.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/allocator.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/test_support.rs`

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DENTRY-WRITE-28/10_creator_serial.md`

## Forbidden Files

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fat.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/`
- sibling component artifacts

## Required Inputs

- Skill to use: `$exfat-subagent-workflow`
- Required role reference: `references/creator.md`

## Semantic Prior Inputs

- `DirectoryEngine` remains the stable `ExfatFs`-owned directory service.
- `EXR-FILESET-04B` remains the validated file-record boundary.
- `EXR-ALLOC-27` remains the sole owner of allocation search, reservation, and commit.

## Integration Prior Inputs

- `directory.rs` is the landing zone for the whole serial creator pass.
- The write path may consume `AllocationResult` and `ExfatDentrySet`, but it must not push policy back into allocator or fileset owners.
- Do not widen into namespace publication, canonicalization, sync ordering, or a new helper manager.

## Workflow Prior Inputs

- Command-free creator lane.
- You are not alone in the codebase. Do not revert or overwrite edits made by others; adjust your implementation to the current workspace state.
- Do not run compile, test, format, Docker, KVM, or QEMU commands; checker will own executable verification.

## Quality Prior Inputs

- Use the creator-role quality slice from `$exfat-subagent-workflow`.
- Keep helper naming and cursor/location updates local and explicit in the creator artifact.
- Record every new helper or local record/update shape together with its final owner or removal condition.

## Temporary Interfaces And Exit Plan

- Do not introduce a directory-write manager, namespace helper service, reservation lease, or sync shell.
- If a temporary helper or record/update shape is needed, it must stay owner-private to `DirectoryEngine` and name the future owner or removal condition in the creator artifact.

## Helper Justification

- Allowed helper surfaces may:
  - discover writable or reusable slot ranges inside one directory,
  - place or tombstone serialized validated record bytes,
  - and consume one committed allocation result when directory growth is already decided.
- They must remain subordinate to `DirectoryEngine`.

## Allowed Commands

- Read-only shell inspection commands under `/home/halifuda/asterinas`

## Parallelism Classification

- Lane class: `command-free production edit`
- May overlap with:
  - `EXR-PGCACHE-26` async audit
  - `EXR-WRITE-30` designer repair
- Known conflicts:
  - `directory.rs`

## Execution Environment

- Host workspace only

## Execution Lock

- None

## Stop Condition

- Stop after writing `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DENTRY-WRITE-28/10_creator_serial.md`

## Escalation Rule

- If the serial creator pass still appears to require edits outside `directory.rs` or a new manager/helper owner to remain coherent, report the exact missing handshake and stop instead of widening scope.
