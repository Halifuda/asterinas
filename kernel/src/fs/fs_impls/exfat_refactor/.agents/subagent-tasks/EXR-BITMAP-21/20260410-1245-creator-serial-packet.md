<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-BITMAP-21-20260410-1245-CREATE-SERIAL`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-BITMAP-21/20260410-1245-creator-serial-packet.md`
- Supersedes: None
- Role: `creator`
- Component: `EXR-BITMAP-21`
- Phase: `serial creator`
- Authorizing main agent: `main-agent`
- Date: `2026-04-10 12:45 CST`

## Goal

- Implement `ExfatFs`-owned validated allocation-bitmap state plus read-only occupancy/accounting queries without widening into allocation mutation, FAT mutation, directory scanning, or mount/open sequencing.

## Architectural Unit Context

- Functional goal: `ExfatFs`-owned allocation bitmap state and read-only accounting
- Final architectural owner: `ExfatFs`
- Expected landing form: owner-internal `AllocationBitmap` state plus owner methods
- Parent unit: `EXR-DIR-ENGINE-19`
- Interfaces served: later `EXR-FS-OPEN-22`, later `EXR-ALLOC-27`, and later filesystem accounting consumers

## Required Resolution Questions

- Add an owner-local `AllocationBitmap` state type in `bitmap.rs`.
- Consume a raw singleton `Bitmap` candidate without re-scanning directories in this component.
- Validate geometry, bitmap length, and bitmap-file cluster ownership before publication.
- Implement canonical read-only occupancy and used/free cluster accounting queries.
- Keep mutation, dirty tracking, FAT writes, and mount/open sequencing out of this pass.

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BITMAP-21/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BITMAP-21/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BITMAP-21/02_designer_async.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BITMAP-21/03_designer_ktest.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/dentry.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/io.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fat.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/super_block.rs`

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BITMAP-21/10_creator_serial.md`

## Forbidden Files

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/`
- Sibling component artifacts.

## Required Inputs

- Skill to use: `$exfat-subagent-workflow`
- Required role reference: `references/creator.md`

## Semantic Prior Inputs

- Use the accepted designer constraints only. Do not reopen Linux or Microsoft exFAT behavior beyond the designer-approved bitmap validation and occupancy/accounting semantics.

## Integration Prior Inputs

- `EXR-DIR-ENGINE-19` is accepted and now provides the raw `Bitmap` singleton boundary; consume that boundary without reintroducing directory scanning here.
- `EXR-UPCASE-20` is accepted and also lives under `ExfatFs`, but stays out of scope for this pass except as neighboring owner state in `fs.rs`.

## Workflow Prior Inputs

- Command-free creator lane.
- This is the loop's only creator round.
- Do not run compile or test commands; checker will own executable verification and the required local ktests.

## Quality Prior Inputs

- Use the creator-role quality slice from `$exfat-subagent-workflow`.
- Keep publication atomic and the installed bitmap immutable after validation.

## Temporary Interfaces And Exit Plan

- Do not widen this pass into allocation search, cluster marking, freeing, dirty-byte tracking, FAT mutation, or mount/open sequencing.
- Do not introduce a second occupancy helper surface when `cluster_is_allocated()` and used/free counts are enough.

## Helper Justification

- Small owner-private helpers in `bitmap.rs` and owner wiring in `fs.rs` are allowed when they keep validation and accounting readable.
- Reject helpers whose main effect is to invent allocator policy or a second public occupancy API early.

## Allowed Commands

- Read-only shell inspection commands under `/home/halifuda/asterinas`

## Parallelism Classification

- Lane class: `command-free production edit`
- May overlap with command-free lanes only
- Known conflicts:
  - `fs.rs`
  - `mod.rs`

## Execution Environment

- Host workspace only

## Execution Lock

- None

## Stop Condition

- Stop after writing `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BITMAP-21/10_creator_serial.md`

## Escalation Rule

- If the implementation requires edits outside `bitmap.rs`, `fs.rs`, or `mod.rs`, or suggests the component boundary itself is underspecified, report that and stop.
