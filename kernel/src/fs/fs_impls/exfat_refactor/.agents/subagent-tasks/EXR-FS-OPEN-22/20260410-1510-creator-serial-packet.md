<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-FS-OPEN-22-20260410-1510-CREATE-SERIAL`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-FS-OPEN-22/20260410-1510-creator-serial-packet.md`
- Supersedes: None
- Role: `creator`
- Component: `EXR-FS-OPEN-22`
- Phase: `serial creator`
- Authorizing main agent: `main-agent`
- Date: `2026-04-10 15:10 CST`

## Goal

- Implement the `ExfatFs` owner-side mount/open sequence that installs mount prerequisites, publishes the canonical root inode, and removes the indefinite `root_inode()` seam without widening into later directory operations, allocator mutation, or sync behavior.

## Architectural Unit Context

- Functional goal: `ExfatFs` mount/open sequencing and root publication
- Final architectural owner: `ExfatFs`
- Expected landing form: owner methods plus sequencing invariants in `fs.rs`
- Interfaces served: VFS `FileSystem::root_inode()`, later directory operations, and later mounted-exFAT readiness work

## Required Resolution Questions

- Replace the temporary `root_inode()` seam with a real owner-owned root publication path.
- Define the creator-side mount/open sequence in `fs.rs` that consumes opened-inode cache, `DirectoryEngine`, `UpcaseTable`, and `AllocationBitmap` in the designer-approved order.
- Keep the root special case distinct from the ordinary opened-inode keyspace.
- Ensure root publication cannot surface a partially prepared root.
- Do not widen into later directory ops, namespace mutation, allocator mutation policy, data-path behavior, or sync ordering.

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-OPEN-22/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-OPEN-22/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-OPEN-22/02_designer_async.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-OPEN-22/03_designer_ktest.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/dentry.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fileset.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fat.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/super_block.rs`

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-OPEN-22/10_creator_serial.md`

## Forbidden Files

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/`
- Sibling component artifacts.

## Required Inputs

- Skill to use: `$exfat-subagent-workflow`
- Required role reference: `references/creator.md`

## Semantic Prior Inputs

- Use the accepted designer constraints only.
- Keep `ExfatFs` as the only mount/open owner.
- Keep `DirectoryEngine` read-only, `UpcaseTable` canonicalization-only, and `AllocationBitmap` read-only in this pass.

## Integration Prior Inputs

- `EXR-INODE-CACHE-18`, `EXR-DIR-ENGINE-19`, `EXR-UPCASE-20`, and `EXR-BITMAP-21` are accepted and must be consumed as already-owned boundaries rather than redefined here.
- The root special case must stay distinct from ordinary keyed opened-inode entries.
- This is now the loop's only creator round; no other creator should be opened on `fs.rs`.

## Workflow Prior Inputs

- Command-free creator lane.
- Do not run compile or test commands; checker will own executable verification and required local ktests.
- If the full designer scope does not fit cleanly in one bounded pass inside `fs.rs`, stop and report the exact oversize or missing handshake instead of widening into another file.

## Quality Prior Inputs

- Use the creator-role quality slice from `$exfat-subagent-workflow`.
- Prefer small owner-private helpers in `fs.rs` over long monolithic sequencing blocks when they clarify ordering or invariants.

## Temporary Interfaces And Exit Plan

- The old `todo!` seam in `root_inode()` should not remain an indefinite placeholder after this pass.
- Do not invent a separate mount object, root-scanner owner, or fake root carrier.
- Do not widen into later lookup/readdir, namespace mutation, allocator policy, page-cache, or sync logic.

## Helper Justification

- Small owner-private helpers in `fs.rs` are allowed when they make prerequisite order and root publication clearer.
- Reject helpers whose main effect is to create a new long-lived mount shell or to expose owner-internal state without need.

## Allowed Commands

- Read-only shell inspection commands under `/home/halifuda/asterinas`

## Parallelism Classification

- Lane class: `command-free production edit`
- May overlap with command-free planning lanes only
- Known conflicts:
  - `fs.rs`

## Execution Environment

- Host workspace only

## Execution Lock

- None

## Stop Condition

- Stop after writing `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-OPEN-22/10_creator_serial.md`

## Escalation Rule

- If the implementation requires edits outside `fs.rs`, or suggests the component boundary itself is underspecified, report that and stop.
