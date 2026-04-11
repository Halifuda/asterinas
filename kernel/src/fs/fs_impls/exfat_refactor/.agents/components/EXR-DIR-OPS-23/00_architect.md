<!-- SPDX-License-Identifier: MPL-2.0 -->

# Architect Handoff

## Metadata

- Component ID: `EXR-DIR-OPS-23`
- Title: `ExfatInode` read-only directory operations
- Status: `Architected`
- Author: architect
- Date: `2026-04-10`
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-DIR-OPS-23/20260410-1510-architect-packet.md`

## Functional Unit Definition

- Functional goal: implement the smallest coherent `ExfatInode` unit that owns read-only directory `lookup` and `readdir_at` behavior after mount/open has published the root inode.
- Final architectural owner: `ExfatInode`
- Owner class:
  - VFS trait carrier
- Expected landing form:
  - owner methods
- Boundary kind:
  - stable architectural boundary
- Why this boundary is architecturally real: `lookup` and `readdir_at` are VFS-visible directory behaviors on one inode instance. They depend on inode identity, the weak filesystem back-reference, and the already accepted filesystem-owned services for record streaming and name folding, but the final user-visible contract still belongs on the inode trait carrier. That makes this a stable `ExfatInode` boundary rather than a separate lookup service or scanner owner.

## Purpose

This unit is the read-only directory behavior that turns a published directory `ExfatInode` into a usable VFS directory node.
It should consume `DirectoryEngine` and `UpcaseTable` through the owning filesystem, reuse the opened-inode publication rules already accepted under `ExfatFs`, and stop before namespace mutation, write-side directory updates, allocator policy, or file-data behavior begin.

## Why This Comes Now

`EXR-INODE-CORE-17` already established `ExfatInode` as the VFS carrier, `EXR-DIR-ENGINE-19` already established the shared record stream, `EXR-UPCASE-20` already established canonical name folding and hashing, and `EXR-FS-OPEN-22` now names the mount/open boundary that publishes the ready root.
That means the next smallest coherent user-visible step is read-only directory behavior on `ExfatInode` itself, with root publication consumed as an input rather than absorbed into this row.

## Owner And Integration Convergence

- Interfaces, traits, services, or higher-level functions this unit ultimately serves:
  - VFS `Inode::lookup`
  - VFS `Inode::readdir_at`
  - the published-root path from `EXR-FS-OPEN-22`
  - the opened-inode reuse path already owned by `ExfatFs`
- If the unit is internal-only, why that internal ownership is still stable in the finished system:
  - it is not internal-only; `lookup` and `readdir_at` are stable VFS-facing behaviors on the inode carrier.
- Known non-goals or nearby logic that must remain in the parent owner:
  - namespace mutation and write-side directory entry updates
  - allocator and bitmap mutation policy
  - mount/open sequencing and root publication
  - regular-file mapping and data-path behavior
  - page-cache and sync ordering

Boundary consumption rules:

- The published root from `EXR-FS-OPEN-22` is an already-ready `ExfatInode` input; this unit must not reopen mount sequencing or root publication.
- `DirectoryEngine` remains the `ExfatFs`-owned read-only record stream; this unit consumes it and must not repackage it as a separate VFS-facing owner.
- `UpcaseTable` remains the filesystem-wide canonicalization prerequisite for name-sensitive lookup; this unit consumes it through `ExfatFs` rather than reimplementing case folding or hashing inside `ExfatInode`.
- Opened-inode reuse remains owned by `ExfatFs`; this unit may depend on that reuse path when lookup resolves a child, but it must not pull cache ownership into `ExfatInode`.

## Dependency Contract

- Depends on:
  - `EXR-INODE-CORE-17`
  - `EXR-DIR-ENGINE-19`
  - `EXR-UPCASE-20`
  - `EXR-FS-OPEN-22`
  - the VFS `Inode` contract
- Blocks:
  - read-only VFS directory access on exFAT
  - later namespace mutation work in `EXR-NAMESPACE-29`
  - any later directory path that assumes canonical lookup over published inode handles
- Can run in parallel with:
  - `EXR-FILE-MAP-24` architect work, because both are read-only `ExfatInode` behaviors with different functional targets
  - sibling planning work that does not reopen `inode.rs` ownership or mount sequencing
- Recommended parallel wave:
  - Wave C after `EXR-FS-OPEN-22` is specified and root publication is named, with read-only directory behavior kept separate from regular-file mapping
- Stable pre-existing interfaces used:
  - `Inode::lookup`
  - `Inode::readdir_at`
  - `ExfatInode`
  - `DirectoryEngine`
  - `DirectoryRecord`
  - `UpcaseTable`
  - the opened-inode table/root publication contract under `ExfatFs`
- Prior sources or prior slices that materially shaped the split:
  - `WORKSPACE-ARCH-RESET/00_architect.md`
  - `EXR-INODE-CORE-17/00_architect.md`
  - `EXR-INODE-CACHE-18/00_architect.md`
  - `EXR-DIR-ENGINE-19/00_architect.md`
  - `EXR-UPCASE-20/00_architect.md`
  - `EXR-FS-OPEN-22/00_architect.md`
  - `EXR-FS-OPEN-22/01_designer_core.md`
  - `COMPONENT_INDEX.md`
  - `kernel/src/fs/vfs/fs_apis/inode.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/directory.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/fs.rs`

## Recommended Work Slices

These are candidate slices for scheduler consideration, not the globally active plan.

| Slice ID | Parent Unit Scope | Goal | Likely Write Set | Depends On | May Overlap With | Lane Class | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `WS-DIR-OPS-23-LOOKUP` | `EXR-DIR-OPS-23` | Implement `ExfatInode::lookup` for directory inodes by consuming `DirectoryEngine` record streaming, `UpcaseTable` name folding/hash services, and `ExfatFs` opened-inode reuse. | `kernel/src/fs/fs_impls/exfat_refactor/inode.rs` | `EXR-INODE-CORE-17`, `EXR-DIR-ENGINE-19`, `EXR-UPCASE-20`, `EXR-FS-OPEN-22` | `WS-DIR-OPS-23-READDIR` if both land in the same `inode.rs` region; do not force file-parallelism where method bodies are adjacent | creator | Keep the slice read-only and directory-only. Do not add create/unlink/rename helpers and do not invent a lookup service owner. |
| `WS-DIR-OPS-23-READDIR` | `EXR-DIR-OPS-23` | Implement `ExfatInode::readdir_at` for directory inodes by iterating the shared directory record stream and projecting stable dirent output without absorbing mutation policy. | `kernel/src/fs/fs_impls/exfat_refactor/inode.rs` | `EXR-INODE-CORE-17`, `EXR-DIR-ENGINE-19`, `EXR-FS-OPEN-22` | `WS-DIR-OPS-23-LOOKUP` because both are expected to collide in `inode.rs` method landings | creator | This slice may share owner-private directory helpers inside `ExfatInode`, but those helpers must stay subordinate to the inode owner and not become a separate service boundary. |

## exFAT Concepts Covered

- Published root and later directory inodes as VFS directory carriers.
- Read-only directory record streaming through `DirectoryEngine`.
- Upcase-table-backed name folding and name-hash use for lookup.
- Opened-inode reuse for looked-up children.
- `lookup` and `readdir_at` as `ExfatInode` methods.
- Read-only directory traversal only; no mutation or allocator policy.

## Boundary Rejections

- Splits considered but rejected:
  - a standalone lookup service separate from `ExfatInode`
  - a user-facing directory-scanner owner layered over `DirectoryEngine`
  - folding mount/open sequencing or root publication back into this row
  - folding create/unlink/mkdir/rmdir/rename into read-only directory ops
  - folding file read/write or file-mapping behavior into directory ops
- Why those rejected splits would be packet convenience, not real architecture:
  - they would hide the stable VFS owner boundary on `ExfatInode`
  - they would reintroduce helper-first drift instead of consuming the already accepted filesystem-owned services
  - they would blur the clear line between read-only directory behavior and later mutation or data-path owners

## Target Files

- Existing files likely to change:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- New files expected:
  - none

## Code Budget

- Target creator work-slice size: `200-300` lines
- Expected number of creator slices: `2`
- Reason if any single slice might exceed 500 lines: it should not. If it does, namespace mutation, mount sequencing, or general directory-service reshaping has leaked into this unit and the work should be re-sliced instead of expanded.

## Exit Condition

Design work may start once `EXR-DIR-OPS-23` is understood as exactly the `ExfatInode` read-only directory surface: `lookup` and `readdir_at` consuming the published root, `DirectoryEngine`, `UpcaseTable`, and filesystem-owned opened-inode reuse, with no mount sequencing, mutation, allocator policy, or data-path logic folded in.

## Risks

- `lookup` can drift into a fake service boundary if name matching or child publication is moved out of `ExfatInode` and into a helper owner.
- `readdir_at` can accidentally grow write-side directory semantics if offset handling is treated as a precursor to mutation.
- `inode.rs` is the likely shared landing zone for both read-only directory methods, so fake parallel slicing should be avoided if the method bodies or shared helpers collide.
- The published-root dependency must remain explicit; if this unit starts constructing root itself, it has crossed back into `EXR-FS-OPEN-22`.
