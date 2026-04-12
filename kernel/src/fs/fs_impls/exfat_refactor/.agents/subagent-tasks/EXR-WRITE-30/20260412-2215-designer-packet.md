<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-WRITE-30-20260412-2215-DESIGN`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-WRITE-30/20260412-2215-designer-packet.md`
- Supersedes: None
- Role: `designer`
- Component: `EXR-WRITE-30`
- Phase: `designer`
- Authorizing main agent: `main-agent`
- Date: `2026-04-12 22:15 CST`

## Goal

- Produce the split designer artifact set for `EXR-WRITE-30` so later creator work can implement `ExfatInode` buffered write, growth, truncate, and resize behavior without guessing about owner boundaries, helper shape, allocation consumption, cache interaction, or checker obligations.

## Architectural Unit Context

- Functional goal: `ExfatInode` write-side file mutation
- Final architectural owner: `ExfatInode`
- Expected landing form: owner methods plus owner-private helpers in `inode.rs`
- Architect artifact:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/00_architect.md`

## Required Resolution Questions

- Specify the smallest inode-owned write-side surface that covers buffered `write_at`, growth, truncate, and resize without inventing a write manager.
- State exactly how the row consumes inode-local page cache, inode-private mapping helpers, committed allocation results, and later sync ownership without absorbing those owners.
- Keep allocation search/reservation inside `EXR-ALLOC-27`, read-side byte-stream policy inside `EXR-READ-OPS-25`, and durable flush ordering inside `EXR-SYNC-31`.
- Define narrow creator and checker obligations so later work does not guess where buffered write ends and allocator or sync ownership begins.
- State serialization, repeated-call, and size-mutation expectations for regular-file write-side behavior without creating a filesystem-global coordinator.

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/DESIGNER_CORE_TEMPLATE.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/DESIGNER_ASYNC_TEMPLATE.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/DESIGNER_KTEST_TEMPLATE.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FILE-MAP-24/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-READ-OPS-25/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-26/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-26/02_designer_async.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-26/03_designer_ktest.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-26/30_reviewer_report.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-ALLOC-27/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-ALLOC-27/02_designer_async.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-ALLOC-27/03_designer_ktest.md`
- `/home/halifuda/asterinas/kernel/src/fs/vfs/fs_apis/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/allocator.rs`

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/02_designer_async.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/03_designer_ktest.md`

## Forbidden Files

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- production code
- creator, checker, advisor, and reviewer artifacts for any component

## Required Inputs

- Skill to use: `$exfat-subagent-workflow`
- Required role reference: `references/designer.md`

## Semantic Prior Inputs

- Use the accepted architect boundary as authoritative.
- `ExfatInode` remains the write-visible owner of buffered mutation and size changes.
- `EXR-PGCACHE-26` remains the owner of inode-local page-cache attachment and backend behavior.
- `EXR-ALLOC-27` remains the owner of search, reservation intent, and committed allocation results.
- `EXR-SYNC-31` remains downstream; do not absorb durable flush ordering or persistence policy here.

## Integration Prior Inputs

- `EXR-FILE-MAP-24` already owns logical-to-physical translation; this row consumes that helper boundary rather than duplicating mapping logic.
- `EXR-READ-OPS-25` already owns buffered read policy; write-side zero-fill or valid-size rules may reference the same byte-stream contract but must not create a second read-policy owner.
- `EXR-PGCACHE-26` is accepted and should be treated as a consumed cache boundary, not as a staging owner to replace.
- `EXR-ALLOC-27` checker is still running. Designer work must use the architected/designed allocator boundary rather than speculate on checker-time implementation details.

## Workflow Prior Inputs

- Command-free designer lane.
- This is artifact-only planning and may overlap with the active `EXR-ALLOC-27` checker because the write set is disjoint.
- Produce the standard split artifact set: core, async, and ktest.
- Stay designer-only; do not drift into creator implementation details beyond creator-ready obligations.

## Quality Prior Inputs

- Use the designer-role quality slice from `$exfat-subagent-workflow`.
- Keep helper shape explicitly owner-private to `ExfatInode`.
- Reject drift into a write manager, allocator facade, or sync shell.

## Temporary Interfaces And Exit Plan

- Do not authorize a background writeback queue, sync shell, or allocator reservation wrapper in this designer pass.
- If a temporary seam seems necessary, stop and report it instead of inventing one silently.

## Helper Justification

- Allowed helper surfaces are owner-private helpers that:
  - coordinate one buffered write or size-mutation call on `ExfatInode`,
  - consume mapping helpers, page-cache access, and committed allocation results,
  - and stage dirty state for later sync ownership without defining the flush protocol.
- They must remain subordinate to `ExfatInode`.

## Allowed Commands

- Read-only shell inspection commands under `/home/halifuda/asterinas`

## Parallelism Classification

- Lane class: `command-free planning`
- May overlap with:
  - the active `EXR-ALLOC-27` checker

## Execution Environment

- Host workspace only

## Execution Lock

- None

## Stop Condition

- Stop after writing:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/01_designer_core.md`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/02_designer_async.md`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/03_designer_ktest.md`

## Escalation Rule

- If the architect artifact plus current upstream boundaries are still insufficient to specify write-side file mutation cleanly without reopening allocator ownership, read-policy ownership, or sync ordering, report the exact missing handshake and stop instead of guessing.
