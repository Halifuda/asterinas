<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-NAMESPACE-29-20260412-2134-DESIGN`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-NAMESPACE-29/20260412-2134-designer-packet.md`
- Supersedes: None
- Role: `designer`
- Component: `EXR-NAMESPACE-29`
- Phase: `designer`
- Authorizing main agent: `main-agent`
- Date: `2026-04-12 21:34 CST`

## Goal

- Produce the split designer artifact set for `EXR-NAMESPACE-29` so later creator work can implement `ExfatInode` namespace mutation (`create`, `unlink`, `mkdir`, `rmdir`, and `rename`) without guessing about owner boundaries, helper shape, mutation sequencing, or checker obligations.

## Architectural Unit Context

- Functional goal: `ExfatInode` namespace mutation
- Final architectural owner: `ExfatInode`
- Expected landing form: owner methods in `inode.rs`
- Architect artifact:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-NAMESPACE-29/00_architect.md`

## Required Resolution Questions

- Specify the smallest inode-owned namespace mutation surface that covers `create`, `unlink`, `mkdir`, `rmdir`, and `rename`.
- State exactly how the row consumes `DirectoryEngine` write primitives, committed allocation results, opened-inode publication, and name folding without absorbing those owners.
- Keep directory-entry slot mutation inside `DirectoryEngine`, allocation search/reservation inside `Allocator`, and sync ordering inside `EXR-SYNC-31`.
- Define narrow creator and checker obligations so later work does not guess where namespace preflight ends and directory mutation or allocator ownership begins.
- State serialization and repeated-call expectations for namespace-visible mutation without inventing a namespace manager or background coordinator.

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/DESIGNER_CORE_TEMPLATE.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/DESIGNER_ASYNC_TEMPLATE.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/DESIGNER_KTEST_TEMPLATE.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-NAMESPACE-29/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-OPS-23/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DENTRY-WRITE-28/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DENTRY-WRITE-28/02_designer_async.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DENTRY-WRITE-28/03_designer_ktest.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-20/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-ALLOC-27/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/vfs/fs_apis/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-NAMESPACE-29/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-NAMESPACE-29/02_designer_async.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-NAMESPACE-29/03_designer_ktest.md`

## Forbidden Files

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- production code
- creator, checker, advisor, and reviewer artifacts for any component

## Required Inputs

- Skill to use: `$exfat-subagent-workflow`
- Required role reference: `references/designer.md`

## Semantic Prior Inputs

- Use the accepted architect boundary as authoritative.
- `DirectoryEngine` remains the owner of slot discovery, overwrite, tombstoning, and placement.
- `Allocator` remains the owner of search, reservation intent, and committed allocation results.
- `ExfatFs` remains the owner of opened-inode publication and canonical child-handle reuse.

## Integration Prior Inputs

- `EXR-DIR-OPS-23` already owns read-side lookup and readdir behavior; this row adds mutation, not a second read-only directory owner.
- `EXR-DENTRY-WRITE-28` is the consumed write-side directory boundary; namespace design must treat it as a service, not as a staging owner to replace.
- `EXR-UPCASE-20` is the accepted canonicalization owner. Keep name folding and hashing dependent on that service.
- `EXR-SYNC-31` remains downstream. Do not let namespace design absorb write ordering, durability, or flush policy.

## Workflow Prior Inputs

- Command-free designer lane.
- This is artifact-only planning and may overlap with the active `EXR-PGCACHE-26` checker and `EXR-ALLOC-27` creator because the write set is disjoint.
- Produce the standard split artifact set: core, async, and ktest.
- Stay designer-only; do not drift into creator implementation details beyond creator-ready obligations.

## Quality Prior Inputs

- Use the designer-role quality slice from `$exfat-subagent-workflow`.
- Keep helper shape explicitly owner-private to `ExfatInode`.
- Reject drift into a standalone namespace manager, allocator wrapper, or sync coordinator.

## Temporary Interfaces And Exit Plan

- Do not authorize a namespace service layer, background mutation queue, or sync/writeback shell in this designer pass.
- If a temporary seam seems necessary, stop and report it instead of inventing one silently.

## Helper Justification

- Allowed helper surfaces are owner-private helpers that:
  - perform namespace preflight and canonical-name preparation,
  - coordinate one mutation call with `DirectoryEngine`,
  - and consume committed allocation results or opened-inode publication without re-owning those services.
- They must remain subordinate to `ExfatInode`.

## Allowed Commands

- Read-only shell inspection commands under `/home/halifuda/asterinas`

## Parallelism Classification

- Lane class: `command-free planning`
- May overlap with:
  - the active `EXR-PGCACHE-26` checker
  - the active `EXR-ALLOC-27` creator

## Execution Environment

- Host workspace only

## Execution Lock

- None

## Stop Condition

- Stop after writing:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-NAMESPACE-29/01_designer_core.md`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-NAMESPACE-29/02_designer_async.md`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-NAMESPACE-29/03_designer_ktest.md`

## Escalation Rule

- If the architect artifact plus current upstream boundaries are still insufficient to specify namespace mutation cleanly without reopening allocator ownership, directory-write ownership, or sync ordering, report the exact missing handshake and stop instead of guessing.
