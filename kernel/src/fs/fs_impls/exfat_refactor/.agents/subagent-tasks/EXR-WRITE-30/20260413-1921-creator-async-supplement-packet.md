<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-WRITE-30-20260413-1921-CREATE-ASYNC-SUPPLEMENT`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-WRITE-30/20260413-1921-creator-async-supplement-packet.md`
- Supersedes:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-WRITE-30/20260413-1914-creator-resize-serialization-packet.md`
- Role: `creator`
- Component: `EXR-WRITE-30`
- Phase: `serial creator repair`
- Authorizing main agent: `main-agent`
- Date: `2026-04-13 19:21 CST`

## Goal

- Land the post-split same-row async supplement for `EXR-WRITE-30`: repair the currently landed buffered `write_at` / committed-growth implementation so it satisfies the already accepted `02_designer_async.md` serialization contract, without introducing a new owner or a separate background protocol.

## Architectural Unit Context

- Functional goal: `ExfatInode` buffered write and committed-growth publication
- Final architectural owner: `ExfatInode`
- Expected landing form: owner methods plus owner-private helpers in `inode.rs`
- Parent unit:
  - `EXR-WRITE-30`
- Interfaces served:
  - current `InodeIo::write_at` on `ExfatInode`
  - the existing downstream `write_page_async()` seam that must remain future-owned by `EXR-SYNC-31`

## Required Resolution Questions

- Check whether the serially landed buffered `write_at` + committed-growth shape still permits overlapping buffered writes to race on stale inode snapshots.
- If yes, add the narrowest owner-private inode-local serialization seam needed so later readers observe either the old byte stream or one fully applied buffered-write call result.
- Preserve the existing call-local `ExfatInodeWriteState` model.
- Do not add a background writer, deferred publish queue, filesystem-global mutation coordinator, or new sync owner.
- Do not reopen allocation search, direct I/O, or resize/truncate/deallocation ownership.

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/02_designer_async.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/03_designer_ktest.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/10_creator_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/12_creator_serial_repair.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/13_checker_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/14_creator_serial_repair.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/16_creator_serial_repair.md`

## Forbidden Files

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/allocator.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fat.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/`
- sibling component artifacts

## Required Inputs

- Skill to use: `$exfat-subagent-workflow`
- Required role reference: `references/creator.md`
- Treat `02_designer_async.md` as the authority for what must be repaired; do not reinterpret this as a new architect or designer task.
- `EXR-RESIZE-37` now owns the deferred resize/truncate work; this packet must not wait for or recreate that row.

## Semantic Prior Inputs

- `EXR-WRITE-30` already has an accepted async/serialization contract.
  - This packet repairs implementation drift against that contract; it does not create a new concurrency owner.
- `ExfatInode` remains the only owner of buffered write and committed-growth publication inside `EXR-WRITE-30`.
- `EXR-RESIZE-37` now owns resize/truncate publication and the missing release/reclaim handshake.
- `EXR-SYNC-31` remains the downstream owner of durable ordering and `write_page_async()`.
- `EXR-ALLOC-27` remains the committed-allocation owner, and `EXR-PGCACHE-26` remains the inode-local cache owner.

## Integration Prior Inputs

- The current risk observed by the main agent is that `write_at` snapshots state at call start and publishes at call end without one explicit inode-local serialization boundary spanning that buffered-write call.
- The intended repair is an owner-private serialization seam inside `ExfatInode`, not a filesystem-global gate.
- The current `Inode::resize` implementation is still deferred and returns `EOPNOTSUPP`; treat that as background context owned by `EXR-RESIZE-37`, not as a blocker for this packet.

## Workflow Prior Inputs

- Command-free creator repair lane.
- This packet starts from the post-split buffered-write shape that is already checked under `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/13_checker_serial.md`.
- Do not run compile, test, format, Docker, KVM, or QEMU commands.

## Quality Prior Inputs

- Use the creator-role quality slice from `$exfat-subagent-workflow`.
- Prefer one explicit owner-private serialization seam over distributed partial locking.
- Do not create a background queue, deferred publish state, or filesystem-global coordinator.

## Temporary Interfaces And Exit Plan

- An owner-private inode-local serialization seam is allowed if needed to enforce the accepted async contract.
  - It must remain local to `ExfatInode`.
  - It must not become a filesystem-global writer gate.
- No temporary public concurrency surface is authorized.

## Helper Justification

- Allowed helper surfaces may:
  - serialize `write_at` and `resize` publication through one inode-local owner-private boundary,
  - keep the call-local `ExfatInodeWriteState` pattern intact,
  - and ensure committed allocation and page-cache-visible EOF changes do not leak half-applied state.
- Reject helpers whose main effect is to invent a standalone mutation manager or shift ownership to `ExfatFs`.

## Allowed Commands

- Read-only shell inspection commands under `/home/halifuda/asterinas`

## Parallelism Classification

- Lane class: `command-free production edit`
- May overlap with:
  - artifact-only planning lanes whose write sets stay outside `inode.rs`
- Known conflicts:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
  - checker and reviewer lanes for `EXR-WRITE-30`

## Execution Environment

- Host workspace only
- This task is command-free.
  - Do not add compile or runtime commands on your own.

## Execution Lock

- None

## Stop Condition

- Stop after writing `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/16_creator_serial_repair.md`
- Do not proceed into checker work.

## Escalation Rule

- If repairing the serialization gap appears to require a new architect/designer decision or a new owner outside `ExfatInode`, report that exact gap and stop instead of inventing it.
