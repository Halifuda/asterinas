<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-BITMAP-21-20260407-1110-ARCH`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-BITMAP-21/20260407-1110-architect-packet.md`
- Supersedes: None
- Role: `architect`
- Component: `EXR-BITMAP-21`
- Phase: `architect`
- Authorizing main agent: `main-agent`
- Date: `2026-04-07 11:10 CST`

## Goal

- Produce the architect artifact for `EXR-BITMAP-21`: the owner-first boundary for `ExfatFs`-owned allocation-bitmap runtime state and read-only occupancy queries, without absorbing allocation mutation, FAT mutation, directory streaming, or mount/open sequencing.

## Architectural Unit Context

- Functional goal: load/validate the allocation bitmap and provide stable read-only occupancy and accounting queries.
- Final architectural owner: `ExfatFs`.
- Expected landing form: owner-internal state (`AllocationBitmap`) plus owner methods.
- Parent unit: none. This is a tracked functional unit in the live board.
- Interfaces served: future `EXR-FS-OPEN-22`, `EXR-ALLOC-27`, and later sync/write-side work.

## Required Resolution Questions

- What is the allocation-bitmap owner boundary, and what stays in `DirectoryEngine`, `Allocator`, or later mutation owners?
- How should the unit consume `DirectoryEngine` singleton bitmap candidates without becoming directory scanning itself?
- Which services are stable now: bitmap loading/validation, read-only occupancy queries, and free-space accounting?
- What mutation behavior must remain explicitly out of scope until `EXR-ALLOC-27`?
- What file landing zones and future collision risks should the designer know?

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/ARCHITECT.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/ARCHITECT_TEMPLATE.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/WORKSPACE-ARCH-RESET/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-ENGINE-19/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-ENGINE-19/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_ARCHITECT_PRIORS.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_CODE_QUALITY_PRIORS.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/linux-exFAT-implementation-summary.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/dentry.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fat.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/super_block.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat/bitmap.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat/fat.rs`
- Read-only inspection commands inside `/home/halifuda/asterinas` are allowed.

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BITMAP-21/00_architect.md`

## Forbidden Files

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/`
- Production code under `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/`
- Other role artifacts outside the write set.

## Required Inputs

- Role-scoped protocol files: `COMMON_SUBAGENT.md`, `ARCHITECT.md`.
- Board reset artifact: `WORKSPACE-ARCH-RESET/00_architect.md`.
- Accepted `EXR-DIR-ENGINE-19` architect/designer artifacts.

## Semantic Prior Inputs

- Use `linux-exFAT-implementation-summary.md` topic “Allocation bitmap scanning and free-space accounting” for orientation only.
- Legacy Asterinas `exfat/bitmap.rs` and FAT coupling references are integration context, not a license to widen into allocator mutation.
- No direct Linux source reads are authorized. If exact Linux bitmap scanning or trim behavior is needed, stop and report.

## Integration Prior Inputs

- Use `ASTERINAS_ARCHITECT_PRIORS.md` profile `I-ARCH`, especially mount/open sequence and allocation/runtime owner context.
- Treat `DirectoryEngine` as the source of raw bitmap singleton candidates, not as part of this component.

## Workflow Prior Inputs

- Command-free architect lane.
- May overlap with `EXR-UPCASE-20` architect and runtime checker lanes because write sets are disjoint.
- Workflow priors may shape work slices only after owner and read-only bitmap boundary are resolved.

## Quality Prior Inputs

- Use `ASTERINAS_CODE_QUALITY_PRIORS.md` profile `Q-ARCH`.
- Keep mutation policy out of this unit; allocation search/mark/free belongs to `EXR-ALLOC-27`.

## Temporary Interfaces And Exit Plan

- No production temporary interface is authorized in this architect pass unless the artifact names the future owner/removal condition.

## Helper Justification

- Any helper-like surface must be justified by `AllocationBitmap` ownership and future consumers, not by packet convenience.

## Allowed Commands

- Read-only shell commands only.
- No build/test/QEMU/format commands.

## Parallelism Classification

- Lane class: command-free.
- May overlap with: `EXR-UPCASE-20` architect, pending reviewer packets, and checker lanes with disjoint write sets.
- Known conflicts: none beyond forbidden write sets.

## Execution Environment

- Host read-only inspection from `/home/halifuda/asterinas`.

## Execution Lock

- No execution lock is needed.

## Stop Condition

- Stop after writing `components/EXR-BITMAP-21/00_architect.md`.

## Escalation Rule

- If the allocation-bitmap boundary cannot be defined without allocator mutation, FAT mutation, directory streaming, or mount/open sequencing, stop and report the missing dependency instead of widening the component.
