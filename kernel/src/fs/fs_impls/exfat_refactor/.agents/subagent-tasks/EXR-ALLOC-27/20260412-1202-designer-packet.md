<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-ALLOC-27-20260412-1202-DESIGN`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-ALLOC-27/20260412-1202-designer-packet.md`
- Supersedes: None
- Role: `designer`
- Component: `EXR-ALLOC-27`
- Phase: `designer`
- Authorizing main agent: `main-agent`
- Date: `2026-04-12 12:02 CST`

## Goal

- Produce the split designer artifact set for `EXR-ALLOC-27` so later creator work can land an `ExfatFs`-owned allocation service that searches free space, reserves cluster runs, and coordinates bitmap plus FAT mutation without absorbing directory-entry writes, file-size growth policy, truncate semantics, or sync ordering.

## Architectural Unit Context

- Functional goal: `ExfatFs`-owned cluster allocation service
- Final architectural owner: `ExfatFs`
- Expected landing form: owner-internal `Allocator` service plus owner methods
- Architect artifact:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-ALLOC-27/00_architect.md`

## Required Resolution Questions

- Refine the architected allocator boundary into a creator-ready spec without reopening the owner question.
- State what owner-internal allocator state is needed now and what stable result shape later namespace/write rows will consume.
- Specify the boundary between free-space search, reservation intent, and the bitmap/FAT mutation handshake.
- Define any lock-order or sequencing expectations between allocation bitmap state, FAT mutation, and later filesystem owners.
- Decide whether a dedicated async artifact is required; if not, say why explicitly.
- Define checker-owned test obligations for free-space search, reservation, contiguous-versus-fragmented allocation behavior, and bitmap/FAT coherence.

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/DESIGNER_CORE_TEMPLATE.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/DESIGNER_ASYNC_TEMPLATE.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/DESIGNER_KTEST_TEMPLATE.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-ALLOC-27/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BITMAP-21/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BITMAP-21/10_creator_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BITMAP-21/30_reviewer_report.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FATVAL-03A/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-CHAIN-03B/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/linux-exFAT-implementation-summary.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/Microsoft-exFAT-spec.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fat.rs`

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-ALLOC-27/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-ALLOC-27/02_designer_async.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-ALLOC-27/03_designer_ktest.md`

## Forbidden Files

- production code
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/`
- creator, checker, advisor, and reviewer artifacts for any component

## Required Inputs

- Skill to use: `$exfat-subagent-workflow`
- Required role reference: `references/designer.md`
- Based-on architect artifact: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-ALLOC-27/00_architect.md`

## Semantic Prior Inputs

- `EXR-BITMAP-21` remains the owner of read-only allocation bitmap state and occupancy/accounting queries.
- `EXR-ALLOC-27` is the first owner of allocation search, reservation, and the bitmap/FAT mutation handshake under `ExfatFs`.
- Microsoft exFAT semantics come first; the Linux summary is orientation for implementation shape only.

## Integration Prior Inputs

- Consume existing bitmap state and FAT helpers as prerequisites. Do not redesign or re-own those rows.
- Keep later consumers such as `EXR-DENTRY-WRITE-28`, `EXR-NAMESPACE-29`, `EXR-WRITE-30`, and `EXR-SYNC-31` out of scope except as downstream interfaces the spec must serve.
- If the design needs a temporary reservation/result structure, name its future consumer and why it remains allocator-owned rather than becoming a free-standing manager API.

## Workflow Prior Inputs

- Command-free designer lane.
- This lane may overlap with the active `EXR-READ-OPS-25` creator lane and the `EXR-PGCACHE-26` designer lane because the write set is artifact-only and disjoint.
- Produce the standard split artifact set: core, async, and ktest.
- Stay designer-only; do not drift into creator implementation details beyond creator-ready obligations.

## Quality Prior Inputs

- Use the designer-role quality slice from `$exfat-subagent-workflow`.
- Keep the allocator boundary filesystem-owned and internal to `ExfatFs`.
- Reject drift into inode-local allocation helpers, directory-write helpers, file-growth policy, or hidden sync ordering.

## Temporary Interfaces And Exit Plan

- Do not authorize a standalone allocator crate/service, sync manager, or directory/publication helper in this designer pass.
- If the spec needs a temporary staging result or reservation handle, it must explicitly name the later owner or removal condition.

## Helper Justification

- Allowed helper surfaces are owner-private allocator helpers that:
  - search the published bitmap snapshot for candidate free runs,
  - reserve and commit allocation state under `ExfatFs`,
  - and expose a stable allocator-owned result for later namespace/write rows.
- They must remain subordinate to filesystem-owned allocation state.

## Allowed Commands

- Read-only shell inspection commands under `/home/halifuda/asterinas`

## Parallelism Classification

- Lane class: `command-free planning`
- May overlap with:
  - `EXR-READ-OPS-25` creator
  - `EXR-PGCACHE-26` designer

## Execution Environment

- Host workspace only

## Execution Lock

- None

## Stop Condition

- Stop after writing:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-ALLOC-27/01_designer_core.md`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-ALLOC-27/02_designer_async.md`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-ALLOC-27/03_designer_ktest.md`

## Escalation Rule

- If the architect artifact is still too coarse to specify a stable allocator boundary without deciding directory-entry mutation, file-growth/truncate policy, or sync ordering, report the exact missing handshake and stop instead of guessing.
