<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-BITMAP-21-20260407-1125-DESIGN`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-BITMAP-21/20260407-1125-designer-packet.md`
- Supersedes: None
- Role: `designer`
- Component: `EXR-BITMAP-21`
- Phase: `designer`
- Authorizing main agent: `main-agent`
- Date: `2026-04-07 11:25 CST`

## Goal

- Produce the designer artifact set for `EXR-BITMAP-21`, specifying the `ExfatFs`-owned read-only `AllocationBitmap` runtime state and occupancy/accounting query surface without widening into allocation mutation, FAT mutation, directory scanning, or mount/open sequencing.

## Architectural Unit Context

- Functional goal: load/validate the allocation bitmap and provide stable read-only occupancy and free-space accounting queries.
- Final architectural owner: `ExfatFs`.
- Expected landing form: owner-internal state (`AllocationBitmap`) plus owner methods.
- Interfaces served: future `EXR-FS-OPEN-22`, `EXR-ALLOC-27`, and later sync/write-side work.

## Required Resolution Questions

- Point to the accepted architect artifact and preserve the owner-first boundary.
- Specify validated bitmap state, payload validation inputs, occupancy predicates, and accounting operations.
- Specify how this unit consumes raw `Bitmap` candidates from `DirectoryEngine` without becoming directory scanning.
- Keep allocation search, set/clear, dirty tracking, FAT mutation, trim/discard, and persistence ordering out of this component.
- Decide whether `02_designer_async.md` is needed; if omitted, record why in `01_designer_core.md`.
- Define checker-owned ktest obligations for bitmap validation, occupancy queries, accounting, and boundary non-widening.

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/DESIGNER.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/DESIGNER_SPEC_TEMPLATE.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BITMAP-21/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-ENGINE-19/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-ENGINE-19/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_ARCHITECT_PRIORS.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_CODE_QUALITY_PRIORS.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/dentry.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/super_block.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat/bitmap.rs`
- Read-only inspection commands inside `/home/halifuda/asterinas` are allowed.

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BITMAP-21/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BITMAP-21/02_designer_async.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BITMAP-21/03_designer_ktest.md`

## Forbidden Files

- Production code, `COMPONENT_INDEX.md`, main-agent handoffs, and other components' artifacts.

## Required Inputs

- Role-scoped protocol files: `COMMON_SUBAGENT.md`, `DESIGNER.md`.
- Accepted architect artifact: `EXR-BITMAP-21/00_architect.md`.

## Semantic Prior Inputs

- Use accepted architect artifact and accepted bitmap/dentry/geometry boundaries only. Do not read Linux source.

## Integration Prior Inputs

- Use `ASTERINAS_ARCHITECT_PRIORS.md` profile `I-DESIGN`.
- Treat legacy `exfat/bitmap.rs` as integration context only, not as a license to include allocator mutation.

## Workflow Prior Inputs

- Command-free designer lane. May overlap with `EXR-UPCASE-20` designer and active reviewer/checker lanes because write sets are disjoint.

## Quality Prior Inputs

- Use `ASTERINAS_CODE_QUALITY_PRIORS.md` profile `Q-DESIGN`.

## Temporary Interfaces And Exit Plan

- No temporary production surface is expected unless explicitly justified with an exit plan.
- Allocation mutation belongs to `EXR-ALLOC-27`; mount-time installation belongs to `EXR-FS-OPEN-22`.

## Helper Justification

- Bitmap helpers must be justified by `ExfatFs` allocation-bitmap ownership and named later consumers.

## Allowed Commands

- Read-only shell commands only. No build/test/QEMU/format commands.

## Parallelism Classification

- Lane class: command-free.
- May overlap with: `EXR-UPCASE-20` designer and active reviewer/checker lanes.

## Execution Environment

- Host read-only inspection from `/home/halifuda/asterinas`.

## Execution Lock

- No execution lock is needed.

## Stop Condition

- Stop after writing the assigned designer artifact set. If `02_designer_async.md` is omitted, explicitly record why in `01_designer_core.md`.

## Escalation Rule

- If the allocation-bitmap boundary cannot be specified without allocator mutation, FAT mutation, directory scanning, or mount/open sequencing, report the gap instead of widening.
