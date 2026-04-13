<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-WRITE-30-20260413-1755-CREATE-SERIAL`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-WRITE-30/20260413-1755-creator-serial-packet.md`
- Supersedes: None
- Role: `creator`
- Component: `EXR-WRITE-30`
- Phase: `serial creator`
- Authorizing main agent: `main-agent`
- Date: `2026-04-13 17:55 CST`

## Goal

- Land the first `EXR-WRITE-30` creator slice in `inode.rs`: replace the temporary buffered `write_at` rejection with inode-owned buffered write behavior, introduce one call-local `ExfatInodeWriteState`, and consume committed growth through the existing `ExfatFs::allocate_clusters()` seam without absorbing `resize`, truncate, direct I/O, or sync ownership yet.

## Architectural Unit Context

- Functional goal: `ExfatInode` buffered write, growth, and truncate publication
- Final architectural owner: `ExfatInode`
- Expected landing form: owner methods plus owner-private helpers in `inode.rs`
- Interfaces served:
  - current `InodeIo::write_at` on `ExfatInode`
  - the existing read-visible byte stream and page-cache attachment already accepted under `EXR-READ-OPS-25` and `EXR-PGCACHE-26`
  - the allocator-owned committed growth seam under `ExfatFs::allocate_clusters()`

## Required Resolution Questions

- Replace the `EOPNOTSUPP` `write_at` stub in `inode.rs` with real buffered write behavior for regular files.
- Introduce one explicit call-local `ExfatInodeWriteState` as the sole mutable write-state carrier for this slice.
- Preserve the accepted read-visible zero-fill contract when a write begins beyond the current `valid_size`.
- Consume committed growth through `ExfatFs::allocate_clusters()` only when the requested write extends beyond current allocation coverage.
- Keep page-cache sizing and visible byte publication coherent with the new inode snapshot for this call.
- Leave `resize` / truncate as a later slice if implementing them cleanly would widen this pass beyond the packet boundary.

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/02_designer_async.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/03_designer_ktest.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FILE-MAP-24/10_creator_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-READ-OPS-25/30_reviewer_report.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-26/30_reviewer_report.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-ALLOC-27/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-ALLOC-27/10_creator_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/vfs/fs_apis/inode.rs`

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/10_creator_serial.md`

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
- Implement against the accepted `EXR-WRITE-30` designer set plus the already accepted read/page-cache/allocation seams; do not reopen architect or designer decisions unless the escalation rule triggers.

## Semantic Prior Inputs

- `EXR-WRITE-30` is buffered-only; keep `StatusFlags::O_DIRECT` unsupported in this slice.
- `ExfatInode` remains the only owner of buffered write behavior and visible byte publication.
- `EXR-READ-OPS-25` remains the owner of read-visible EOF and valid-size zero-fill semantics; this slice must preserve those semantics when it mutates `size` or `valid_size`.
- `EXR-PGCACHE-26` remains the inode-local page-cache owner; this slice may resize or dirty that cache but must not create a write manager.
- `EXR-ALLOC-27` remains the owner of allocation search and commit; this slice may consume only committed `AllocationResult` values through the existing `ExfatFs::allocate_clusters()` seam.
- `EXR-SYNC-31` remains the downstream owner of writeback ordering and persistence.

## Integration Prior Inputs

- Default this first slice to `WS-WRITE-30-WRITEAT` plus only the growth logic needed to make extending writes correct.
- Keep `resize` as a later slice if implementing it cleanly would materially enlarge the pass or force extra helper churn.
- The existing `ExfatFs::allocate_clusters()` wrapper already exists in `fs.rs`; prefer consuming it as-is and touch `fs.rs` only if a very small owner-private seam adjustment is required.
- Keep helper shape owner-private to `ExfatInode`; do not introduce a write manager, allocator facade, or sync shell.

## Workflow Prior Inputs

- Command-free creator lane.
- This is the current wave's active production creator lane now that `EXR-CHARSET-32` has cleared retry checking.
- Do not run compile, test, format, Docker, KVM, or QEMU commands; checker will own executable verification.
- Record any new owner-private helper, local type, or temporary seam in the creator artifact with its final owner or removal condition.

## Quality Prior Inputs

- Use the creator-role quality slice from `$exfat-subagent-workflow`.
- Prefer early returns and compact owner-private helpers over a broad mutation helper layer.
- Keep the write-state carrier call-local and explicit.
- If this slice cannot stay comfortably inside roughly `200-350` lines of new or heavily rewritten code, stop and report the missing split rather than swallowing `resize`/truncate.

## Temporary Interfaces And Exit Plan

- A call-local `ExfatInodeWriteState` is allowed and expected in this slice.
- A very small `fs.rs` seam adjustment is allowed only if it keeps committed allocation consumption owner-first.
- Do not add a public deallocator, truncate service, direct-I/O path, or writeback manager.
- Leave `resize` / truncate ownership explicit in the artifact as pending follow-on work if this first slice does not implement them.

## Helper Justification

- Allowed helpers may:
  - snapshot the inode's mutable write-side facts into one call-local state holder,
  - compute extra cluster demand from a target write end,
  - fold a committed `AllocationResult` into that local state,
  - zero-fill a valid-size gap before publication,
  - and publish the resulting inode-visible byte stream coherently.
- Reject helpers whose main effect is to invent a standalone write service or to move allocation/sync ownership away from their accepted owners.

## Allowed Commands

- Read-only shell inspection commands under `/home/halifuda/asterinas`

## Parallelism Classification

- Lane class: `command-free production edit`
- May overlap with:
  - artifact-only planning lanes whose write sets stay outside `inode.rs` and `fs.rs`
- Known conflicts:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
  - later checker or reviewer lanes for `EXR-WRITE-30`

## Execution Environment

- Host workspace only
- This task is command-free. Do not add compile or runtime commands on your own.

## Execution Lock

- None

## Stop Condition

- Stop after writing `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/10_creator_serial.md`.
- Do not proceed into checker work.

## Escalation Rule

- If implementation appears to require edits outside `inode.rs`, `fs.rs`, and the creator artifact, or if keeping `resize` out of this slice is impossible without breaking the write-side owner boundary, report the exact missing handshake and stop instead of widening scope.
