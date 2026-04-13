<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-WRITE-30-20260413-1914-CREATE-REPAIR`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-WRITE-30/20260413-1914-creator-resize-serialization-packet.md`
- Supersedes:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-WRITE-30/20260413-1755-creator-serial-packet.md`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-WRITE-30/20260413-1807-creator-repair-packet.md`
- Role: `creator`
- Component: `EXR-WRITE-30`
- Phase: `serial creator repair`
- Authorizing main agent: `main-agent`
- Date: `2026-04-13 19:14 CST`

## Goal

- Land the next `EXR-WRITE-30` creator slice in `inode.rs`: replace the temporary `resize` rejection with inode-owned grow/shrink publication for regular files, and close the same-row write-side serialization gap if the current `write_at` / `resize` shape still allows concurrent calls to race by snapshotting state without one owner-private inode-local serialization boundary.

## Architectural Unit Context

- Functional goal: `ExfatInode` buffered write, growth, and truncate publication
- Final architectural owner: `ExfatInode`
- Expected landing form: owner methods plus owner-private helpers in `inode.rs`
- Parent unit:
  - `EXR-WRITE-30`
- Interfaces served:
  - current `Inode::resize` on `ExfatInode`
  - the already landed `InodeIo::write_at` slice on `ExfatInode`
  - the inode-visible EOF, `valid_size`, and page-cache sizing contract already consumed by `EXR-READ-OPS-25` and `EXR-PGCACHE-26`

## Required Resolution Questions

- Replace the `EOPNOTSUPP` `resize` stub in `inode.rs` with inode-owned size mutation behavior for regular files.
- Keep the implementation bounded to:
  - no-op when `new_size` equals the current size,
  - growth publication that preserves zero-visible unwritten suffix semantics,
  - shrink/truncate publication that clamps visible EOF and inode-local page-cache sizing coherently.
- Inspect the currently landed `write_at` / `resize` shape against `02_designer_async.md`.
  - If concurrent write or resize calls can still race because they only snapshot state and publish later without one inode-local serialization boundary, add the narrowest owner-private serialization seam needed to close that gap inside `EXR-WRITE-30`.
- Keep the serialization repair owner-private to `ExfatInode`.
  - Do not invent a filesystem-global writer, background queue, or new sync owner.
- If shrink/truncate would require a broader free-cluster or deallocation owner than the accepted `EXR-WRITE-30` / `EXR-ALLOC-27` seams already expose, stop and report the exact missing handshake instead of inventing a new allocation or truncate service.

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/00_architect.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/02_designer_async.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/03_designer_ktest.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/10_creator_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/12_creator_serial_repair.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/13_checker_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-ALLOC-27/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-ALLOC-27/10_creator_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-26/02_designer_async.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/vfs/fs_apis/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat/inode.rs`

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/14_creator_serial_repair.md`

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
- Implement against the accepted `EXR-WRITE-30` designer set plus the landed `10` / `12` / `13` artifacts.
- Treat `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat/inode.rs` as Asterinas-local integration context only for page-cache and resize ordering instincts; it is not semantic authority over the refactor row.

## Semantic Prior Inputs

- `EXR-WRITE-30` remains buffered-only.
  - Keep `StatusFlags::O_DIRECT` unsupported; direct I/O belongs to `EXR-DIRECT-33`.
- `ExfatInode` remains the only owner of buffered write behavior, size mutation, and visible-byte publication for this row.
- `EXR-READ-OPS-25` remains the owner of read-visible EOF and zero-visible unwritten ranges.
  - This slice must preserve those semantics when it mutates `size` or `valid_size`.
- `EXR-PGCACHE-26` remains the inode-local page-cache owner.
  - This slice may resize or invalidate that cache but must not invent a cache manager or writeback owner.
- `EXR-ALLOC-27` remains the owner of allocation search, reservation, and committed allocation publication.
  - This slice may consume only the committed growth seam already exposed through `ExfatFs::allocate_clusters()`.
- `EXR-SYNC-31` remains the downstream owner of durable ordering and flush policy.

## Integration Prior Inputs

- The current landed `write_at` path snapshots one `ExfatInodeWriteState`, performs growth and disk writes, then publishes the new state at the end.
  - If that shape still permits two overlapping write or resize calls to race on stale inode snapshots, close that gap with one owner-private inode-local serialization boundary instead of sprinkling ad hoc locks.
- The current checked `write_at` slice already proves:
  - in-allocation buffered writes,
  - empty-file growth,
  - non-empty growth with chain stitching.
- The current unimplemented seam is `Inode::resize` in `inode.rs`.
- `EXR-WRITE-30` architect slicing already named `WS-WRITE-30-TRUNCATE` as the follow-on creator slice and allowed `inode.rs` / `fs.rs` overlap for that work.
- If a small `fs.rs` owner-private helper is genuinely needed to keep truncate/grow behavior owner-first, it is allowed.
  - Do not widen beyond a small helper seam.

## Workflow Prior Inputs

- Command-free creator repair lane.
- This is the current wave's active production slice.
- You are not alone in the codebase.
  - Do not revert or overwrite unrelated edits; adjust to the current workspace state.
- Do not run compile, test, format, Docker, KVM, or QEMU commands.
- Keep the slice narrow enough that checker can later prove it with local `inode.rs` ktests rather than a broad integration campaign.

## Quality Prior Inputs

- Use the creator-role quality slice from `$exfat-subagent-workflow`.
- Prefer one explicit owner-private serialization seam over distributed partial locking.
- Keep helper surfaces owner-private to `ExfatInode` or a very small `ExfatFs` seam.
- Do not create a public deallocator, truncate manager, writeback shell, or filesystem-global mutation coordinator.
- If implementing shrink correctly would require a broader semantic decision about cluster release than this packet provides, stop and report that exact gap instead of guessing.

## Temporary Interfaces And Exit Plan

- An owner-private inode-local serialization seam is allowed if needed to enforce the already accepted `02_designer_async.md` contract.
  - It must remain local to `ExfatInode`.
  - It must not become a filesystem-global writer gate.
- A very small `fs.rs` helper is allowed only if it keeps truncate/grow handoff owner-first.
  - It must remain owner-private to `ExfatFs`.
- No temporary public deallocation or sync surface is authorized.

## Helper Justification

- Allowed helper surfaces may:
  - compute resize-side growth demand from `new_size`,
  - clamp `size`, `valid_size`, and page-cache sizing coherently on shrink,
  - serialize `write_at` and `resize` publication through one inode-local owner-private boundary,
  - and keep the already landed committed-growth handoff and chain facts coherent during size mutation.
- Reject helpers whose main effect is to:
  - invent a standalone truncate/deallocation service,
  - move writeback ordering into this row,
  - or expose a reusable mutation coordinator outside `ExfatInode`.

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
  - creator lanes for `EXR-NAMESPACE-29`, `EXR-DIRECT-33`, and `EXR-INODE-META-36`

## Execution Environment

- Host workspace only
- This task is command-free.
  - Do not add compile or runtime commands on your own.

## Execution Lock

- None

## Stop Condition

- Stop after writing `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/14_creator_serial_repair.md`
- Do not proceed into checker work.

## Escalation Rule

- If coherent shrink/truncate publication requires a broader cluster-release owner or a new architect/designer decision, report the exact missing handshake and stop instead of widening scope.
