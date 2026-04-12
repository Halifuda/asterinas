<!-- SPDX-License-Identifier: MPL-2.0 -->

# Architect Handoff

## Metadata

- Component ID: `EXR-NAMESPACE-29`
- Title: `ExfatInode` namespace mutation owner boundary
- Status: `Architected`
- Author: architect
- Date: 2026-04-12
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-NAMESPACE-29/20260412-2126-architect-packet.md`

## Functional Unit Definition

- Functional goal: implement `ExfatInode::create`, `unlink`, `mkdir`, `rmdir`, and `rename` as the stable VFS namespace-mutation surface that consumes the accepted read-side directory ownership, the specified `DirectoryEngine` write primitives, committed allocation results, and the existing opened-inode publication boundary.
- Final architectural owner: `ExfatInode`
- Owner class:
  - VFS trait carrier
- Expected landing form:
  - owner methods plus narrow owner-private helpers
- Boundary kind:
  - stable architectural boundary
- Why this boundary is architecturally real:
  - namespace mutation is a single inode-visible contract in the finished filesystem. The VFS caller does not ask for a separate namespace manager; it asks the inode owner to create, remove, and rename children under one directory identity. Those operations must combine name validation, read-side directory resolution, write-side directory entry mutation, allocation consumption, and child publication without splitting the responsibility across unrelated helper owners.

## Purpose

This unit is the smallest coherent `ExfatInode` slice that owns namespace-visible mutation without absorbing the directory scanner, the allocator, or sync ordering.
It converts the already accepted read-only directory surface and the new write-side directory entry primitives into the final VFS namespace methods for exFAT inodes.

The owner boundary should stay narrow:

1. `ExfatInode` owns the namespace method contract and the inode-local decision flow.
2. `DirectoryEngine` owns slot placement, overwrite, tombstoning, and serialized directory-record mutation.
3. `ExfatFs` owns opened-inode publication and the canonical `InodeKey` reuse boundary.
4. `Allocator` owns search, reservation intent, and committed allocation results.
5. `UpcaseTable` owns canonical name folding and hashing.
6. `EXR-SYNC-31` remains the downstream owner of persistence ordering.

## Why This Comes Now

The boundary is stable now because the prerequisite owners already exist as real architecture:

- `EXR-DIR-OPS-23` gives `ExfatInode` the read-side directory owner surface for lookup and enumeration.
- `EXR-DENTRY-WRITE-28` gives `DirectoryEngine` the write-side directory mutation primitives.
- `EXR-UPCASE-20` gives `ExfatFs` the filesystem-wide canonicalization service.
- `EXR-ALLOC-27` gives `ExfatFs` committed allocation results without letting namespace work absorb free-space search.
- `EXR-INODE-CACHE-18` and the `ExfatFs` publication boundary keep repeated child handles canonical.

That makes namespace mutation the next coherent inode-owned step, rather than another staging manager.

## Owner And Integration Convergence

- Interfaces, traits, services, or higher-level functions this unit ultimately serves:
  - VFS `Inode::create`
  - VFS `Inode::unlink`
  - VFS `Inode::mkdir`
  - VFS `Inode::rmdir`
  - VFS `Inode::rename`
  - the read-side `lookup` path already owned by `ExfatInode`
  - the `ExfatFs` opened-inode reuse boundary
  - the `DirectoryEngine` write-side mutation service
- If the unit is internal-only, why that internal ownership is still stable in the finished system:
  - it is not internal-only; namespace mutation is a stable VFS-visible inode behavior, and the inode carrier is the final owner that VFS expects to call.
- Known non-goals or nearby logic that must remain in the parent owner:
  - directory record validation and record-set construction
  - read-only directory streaming
  - canonical name folding and name hashing
  - allocation search and reservation
  - opened-inode table ownership
  - filesystem-wide sync ordering
  - buffered file writeback and page-cache policy

Boundary consumption rules:

- `ExfatInode` may decide when a mutation needs a target name, a parent directory, or a canonical child handle, but it must consume those as validated inputs from the already-owned services.
- `DirectoryEngine` remains the only owner of directory-entry slot discovery and in-place mutation mechanics.
- `Allocator` may return committed allocation facts, but namespace mutation must not re-own search or reservation.
- `ExfatFs` remains the canonical reuse point for child inode publication and `InodeKey`-based handle reuse.
- `rename` may coordinate two directory mutations, but it must not become a separate namespace service or a sync coordinator.

## Dependency Contract

- Depends on:
  - `EXR-INODE-CORE-17`
  - `EXR-DIR-OPS-23`
  - `EXR-DENTRY-WRITE-28`
  - `EXR-UPCASE-20`
  - `EXR-ALLOC-27`
  - the existing opened-inode publication boundary under `ExfatFs`
  - the VFS `Inode` mutation contract
- Blocks:
  - user-visible exFAT namespace mutation
  - later sync-ordering work in `EXR-SYNC-31`
  - any write-side directory path that assumes inode-owned namespace behavior
- Can run in parallel with:
  - read-only inode work that stays outside namespace mutation
  - allocator work only if it does not widen into namespace policy
  - directory-entry write work only if `DirectoryEngine` remains the mutation owner and `inode.rs` only consumes that service
- Recommended parallel wave:
  - Wave D after `EXR-DENTRY-WRITE-28` and `EXR-ALLOC-27` are both specified, with namespace mutation kept on `ExfatInode` and directory rewrite kept on `DirectoryEngine`
- Stable pre-existing interfaces used:
  - `ExfatInode`
  - `ExfatFs`
  - `DirectoryEngine`
  - `ExfatDentrySet`
  - `InodeKey`
  - committed allocation results from `Allocator`
- Prior sources or prior slices that materially shaped the split:
  - `WORKSPACE-ARCH-RESET/00_architect.md`
  - `EXR-INODE-CORE-17/00_architect.md`
  - `EXR-DIR-OPS-23/00_architect.md`
  - `EXR-DIR-OPS-23/01_designer_core.md`
  - `EXR-DENTRY-WRITE-28/00_architect.md`
  - `EXR-UPCASE-20/00_architect.md`
  - `EXR-ALLOC-27/01_designer_core.md`
  - `linux-exFAT-implementation-summary.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/directory.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/fs.rs`

## Recommended Work Slices

These are candidate slices for scheduler consideration, not the active global plan.

| Slice ID | Parent Unit Scope | Goal | Likely Write Set | Depends On | May Overlap With | Lane Class | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `WS-NAMESPACE-29-RESOLVE` | `EXR-NAMESPACE-29` | Implement the shared namespace preflight and resolution path for `create`, `unlink`, `mkdir`, `rmdir`, and `rename`, including canonical name handling and child-handle publication reuse. | `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`, `kernel/src/fs/fs_impls/exfat_refactor/fs.rs` | `EXR-INODE-CORE-17`, `EXR-DIR-OPS-23`, `EXR-UPCASE-20`, `EXR-INODE-CACHE-18` | later mutation slices in `inode.rs` because the shared helpers and method dispatch will likely land in the same owner file | creator | Keep this slice read/resolve oriented. It may prepare the owner-private namespace scaffolding, but it must not absorb directory-entry mutation or allocation search. |
| `WS-NAMESPACE-29-MUTATE` | `EXR-NAMESPACE-29` | Implement the write-backed mutation flow for `create`, `unlink`, `mkdir`, and `rmdir` using the `DirectoryEngine` write boundary and committed allocation results. | `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`, `kernel/src/fs/fs_impls/exfat_refactor/directory.rs` | `EXR-DENTRY-WRITE-28`, `EXR-ALLOC-27`, `EXR-UPCASE-20` | `WS-NAMESPACE-29-RESOLVE` because both slices want the same inode owner region | creator | Keep directory slot mutation and inode-owner method flow distinct from allocation search and from sync ordering. |
| `WS-NAMESPACE-29-RENAME` | `EXR-NAMESPACE-29` | Implement `rename` as the cross-directory namespace mutation that coordinates source removal and destination publication under the same inode owner boundary. | `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`, `kernel/src/fs/fs_impls/exfat_refactor/directory.rs` | `EXR-DENTRY-WRITE-28`, `EXR-ALLOC-27`, `EXR-DIR-OPS-23` | `WS-NAMESPACE-29-MUTATE` and `WS-NAMESPACE-29-RESOLVE` because rename will likely share the same helper region and mutation helpers | creator | Treat rename as the highest-collision method. If it expands beyond one inode-owner region, stop and re-slice instead of inventing a namespace manager. |

## exFAT Concepts Covered

- Namespace-visible child creation and removal.
- Directory-backed rename, including overwrite and source/destination coordination.
- Directory-entry mutation through `DirectoryEngine` write methods.
- Canonical name folding and hash reuse for mutation-time name matching.
- Opened-inode publication and canonical child-handle reuse.
- Committed allocation results for directory growth or new child materialization.
- Inode-owned VFS mutation methods, not a standalone namespace service.

## Boundary Rejections

- Splits considered but rejected:
  - a standalone namespace manager layered above `ExfatInode`
  - a free-standing create/unlink/mkdir/rmdir/rename helper service
  - pulling allocation search or reservation into namespace mutation
  - pulling write-side slot discovery or tombstoning out of `DirectoryEngine`
  - folding sync ordering into this row
  - reusing the legacy Asterinas `exfat` implementation as the semantic target
- Why those rejected splits would be packet convenience, not real architecture:
  - the VFS contract is inode-owned, so a separate namespace owner would insert an unnecessary staging layer
  - directory mutation and allocation already have stable owners; namespace work should consume them, not absorb them
  - rename and create/unlink/mkdir/rmdir share the same inode-local decision boundary, so splitting them into a new manager would be a fake boundary rather than a stable owner

## Target Files

- Existing files likely to change:
  - `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/directory.rs`
- New files expected:
  - none

## Code Budget

- Target creator work-slice size: `200-350` lines
- Expected number of creator slices: `2-3`
- Reason if any single slice might exceed 500 lines:
  - it should not. If a slice grows that large, namespace mutation has likely swallowed helper publication, directory rewrite, or sync concerns, which means the boundary has drifted and needs to be re-sliced.

## Exit Condition

Design work may start once `ExfatInode` is understood as the single namespace-mutation owner that consumes:

1. read-side directory ownership from `EXR-DIR-OPS-23`,
2. write-side directory-entry mutation from `EXR-DENTRY-WRITE-28`,
3. canonical name folding from `EXR-UPCASE-20`,
4. committed allocation results from `EXR-ALLOC-27`,
5. opened-inode publication from `ExfatFs`,
6. and later sync ordering from `EXR-SYNC-31` only as a downstream dependency.

Observable readiness means the designer can specify `create`, `unlink`, `mkdir`, `rmdir`, and `rename` as one inode-owned namespace surface without reopening allocator ownership or inventing a namespace manager.

## Risks

- `inode.rs` is the likely collision point for every namespace method, so fake file-parallel slicing would be risky.
- `rename` is the method most likely to cross into both source and destination directory mutation; if the helpers start spreading into a separate service, the boundary has drifted.
- Child-handle reuse must stay on the `ExfatFs` publication path; if namespace mutation starts re-owning inode identity, it will collide with `EXR-INODE-CACHE-18`.
- Write-order and durability questions belong to `EXR-SYNC-31`; if those details are pulled into this row, the architecture has absorbed a downstream owner.
