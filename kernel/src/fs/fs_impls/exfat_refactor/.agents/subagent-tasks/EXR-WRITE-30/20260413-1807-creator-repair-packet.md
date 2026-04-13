<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-WRITE-30-20260413-1807-CREATE-REPAIR`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-WRITE-30/20260413-1807-creator-repair-packet.md`
- Supersedes: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-WRITE-30/20260413-1755-creator-serial-packet.md`
- Role: `creator`
- Component: `EXR-WRITE-30`
- Phase: `serial creator repair`
- Authorizing main agent: `main-agent`
- Date: `2026-04-13 18:07 CST`

## Goal

- Repair the just-landed `EXR-WRITE-30` write-side slice so extending writes on already non-empty files append newly committed clusters onto the existing file chain coherently, instead of only updating the in-memory cluster count.

## Architectural Unit Context

- Functional goal: `ExfatInode` buffered write, committed growth, and visible-byte publication
- Final architectural owner: `ExfatInode`
- Expected landing form: owner methods plus owner-private helpers in `inode.rs`
- Prior creator artifact:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/10_creator_serial.md`

## Required Resolution Questions

- Preserve the existing first-slice `write_at` behavior for in-allocation writes, valid-size gap zero-fill, and empty-file growth.
- Repair extending writes on already non-empty files so newly committed allocation is actually reachable from the preexisting file chain after the call returns.
- Keep the repair owner-private to `ExfatInode`; do not move chain stitching into `DirectoryEngine`, a filesystem-global writer, or a new public allocator API.
- Handle the chain-mode transition coherently:
  - if an existing contiguous chain can stay contiguous after growth, preserve that cheaper representation;
  - otherwise materialize a FAT-backed combined chain and publish `ChainMode::FatBacked` only after the combined chain is coherent.
- Keep `resize`, truncate, direct I/O, and sync ordering out of scope.

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/02_designer_async.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/03_designer_ktest.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/10_creator_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-ALLOC-27/10_creator_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fat.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs`

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/12_creator_serial_repair.md`

## Forbidden Files

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/allocator.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/`
- sibling component artifacts

## Required Inputs

- Skill to use: `$exfat-subagent-workflow`
- Required role reference: `references/creator.md`

## Semantic Prior Inputs

- `EXR-WRITE-30` remains buffered-only; keep `StatusFlags::O_DIRECT` unsupported.
- `EXR-ALLOC-27` remains the only owner of search, reservation, and committed allocation.
- This repair is about post-allocation chain reachability only; it must not reopen allocation ownership.
- `EXR-SYNC-31` still owns durable ordering, so this repair must not invent a flush shell.

## Integration Prior Inputs

- Prefer a narrow owner-private repair in `inode.rs`.
- It is acceptable to reuse the `directory.rs` chain-materialization pattern as inspiration, but do not move or share that helper across owners.
- Existing `fat.rs` already exposes `write_next_fat_value`; prefer consuming that seam from `inode.rs` rather than widening the write set.
- If a local ktest for non-empty extending-write growth is straightforward inside `inode.rs`, add it now so checker can prove the repaired path. Do not widen scope if constructing the fixture would become a separate mini-project.

## Workflow Prior Inputs

- Command-free creator repair lane.
- You are not alone in the codebase. Do not revert or overwrite others' edits; adjust to the current workspace state.
- Do not run compile, test, format, Docker, KVM, or QEMU commands.

## Quality Prior Inputs

- Use the creator-role quality slice from `$exfat-subagent-workflow`.
- Prefer one owner-private chain-append/materialization helper over ad hoc link writes scattered across `write_at`.
- Record the exact transition rule for contiguous-preserving growth versus forced FAT-backed materialization.

## Temporary Interfaces And Exit Plan

- Do not add a public append-chain helper on `ExfatFs`.
- Do not add a generic file-chain materializer shared with `DirectoryEngine`.
- Any new helper must stay owner-private to `ExfatInode` and serve only buffered write growth.

## Helper Justification

- Allowed helper surfaces may:
  - collect the current file-chain clusters when growth must stitch onto an existing non-empty chain,
  - decide whether the combined chain can stay contiguous,
  - materialize a FAT-backed combined chain with `write_next_fat_value` when needed,
  - and publish the updated `start_cluster`, `cluster_count`, and `chain_mode` only after the chain is coherent.

## Allowed Commands

- Read-only shell inspection commands under `/home/halifuda/asterinas`

## Parallelism Classification

- Lane class: `command-free production edit`
- May overlap with:
  - artifact-only planning lanes whose write sets stay outside `inode.rs`
- Known conflicts:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
  - later checker or reviewer lanes for `EXR-WRITE-30`

## Execution Environment

- Host workspace only

## Execution Lock

- None

## Stop Condition

- Stop after writing `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/12_creator_serial_repair.md`

## Escalation Rule

- If coherent chain-append repair still requires edits outside `inode.rs` or a new public filesystem seam, report the exact missing handshake and stop instead of widening scope.
