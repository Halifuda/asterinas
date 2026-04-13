<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-WRITE-30-20260413-0650-DESIGN-REPAIR`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-WRITE-30/20260413-0650-designer-repair-packet.md`
- Supersedes: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-WRITE-30/20260412-2215-designer-packet.md`
- Role: `designer`
- Component: `EXR-WRITE-30`
- Phase: `designer repair`
- Authorizing main agent: `main-agent`
- Date: `2026-04-13 06:50 CST`

## Goal

- Repair the current `EXR-WRITE-30` designer set so the first creator does not have to invent the inode-local mutation/publication model. The revised spec must explicitly pin the mutable file-state holder, publication order, and expected serial-vs-async follow-up discipline for write-side file mutation.

## Architectural Unit Context

- Functional goal: `ExfatInode` write-side file mutation
- Final architectural owner: `ExfatInode`
- Expected landing form: owner methods plus owner-private helpers and one explicit owner-private mutation holder in `inode.rs`
- Architect artifact:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/00_architect.md`

## Required Resolution Questions

- Replace the current ambiguous "one guard or one small state struct may be acceptable" wording with one explicit creator-ready owner-local mutation model.
- State exactly which inode facts become part of the mutable write-side state holder and how those facts are published back to the visible inode snapshot.
- State whether the row's `02_designer_async.md` is only documenting serialization or whether the component should reserve a distinct later concurrency creator/checker patch sequence after serial closure.
- Keep `write_page_async()` and durable flush ordering outside this row and explicitly future-owned by `EXR-SYNC-31`.
- Keep allocator search/reservation outside the row while making the committed-allocation consumption handshake concrete enough that creator work does not guess.

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/02_designer_async.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/03_designer_ktest.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-26/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-26/02_designer_async.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-26/30_reviewer_report.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-READ-OPS-25/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-READ-OPS-25/02_designer_async.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-ALLOC-27/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-ALLOC-27/02_designer_async.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-ALLOC-27/30_reviewer_report.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/cinder-harbor-20260412-2126-cache-check-allocator-wave.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/vfs/fs_apis/inode.rs`

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/02_designer_async.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/03_designer_ktest.md`

## Forbidden Files

- production code
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- creator, checker, advisor, and reviewer artifacts for any component

## Required Inputs

- Skill to use: `$exfat-subagent-workflow`
- Required role reference: `references/designer.md`

## Semantic Prior Inputs

- `ExfatInode` remains the sole write-visible owner.
- `EXR-PGCACHE-26` remains the owner of inode-local cache attachment.
- `EXR-ALLOC-27` remains the owner of allocation search, reservation, and commit.
- `EXR-SYNC-31` remains the downstream owner of durable flush ordering and final writeback protocol.

## Integration Prior Inputs

- The current spec is too permissive around the mutation-holder shape; repair that, do not redesign the row.
- Keep `EXR-FILE-MAP-24` as the consumed translation boundary and `EXR-READ-OPS-25` as the consumed read-visible byte-stream contract.
- Use current `inode.rs` reality to decide what later creator work would otherwise have to guess.

## Workflow Prior Inputs

- Command-free designer repair lane.
- Stay designer-only; do not implement code or schedule follow-up work.
- It is acceptable to reserve a later concurrency patch loop if the repaired spec concludes one is really required after serial closure.

## Quality Prior Inputs

- Use the designer-role quality slice from `$exfat-subagent-workflow`.
- Replace ambiguity with one concrete owner-first shape.
- Keep helper and temporary-seam surfaces explicitly subordinate to `ExfatInode`.

## Temporary Interfaces And Exit Plan

- Do not authorize a write manager, filesystem-global coordinator, direct-I/O path, or sync shell.
- If a temporary seam remains necessary, name its future owner and code-comment requirement explicitly.
- If the repaired spec concludes that the row needs a distinct post-serial concurrency patch, say exactly what that patch would own and what it must not reopen.

## Helper Justification

- Allowed helper surfaces are owner-private helpers and one explicit owner-private mutation holder that keep file-state publication local to `ExfatInode`.

## Allowed Commands

- Read-only shell inspection commands under `/home/halifuda/asterinas`

## Parallelism Classification

- Lane class: `command-free planning`
- May overlap with:
  - `EXR-PGCACHE-26` async audit
  - `EXR-DENTRY-WRITE-28` creator lane

## Execution Environment

- Host workspace only

## Execution Lock

- None

## Stop Condition

- Stop after rewriting:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/01_designer_core.md`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/02_designer_async.md`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/03_designer_ktest.md`

## Escalation Rule

- If the current architect boundary plus accepted dependencies are still insufficient to pin a single creator-ready mutation/publication model, report the exact missing handshake and stop instead of leaving the ambiguity in place.
