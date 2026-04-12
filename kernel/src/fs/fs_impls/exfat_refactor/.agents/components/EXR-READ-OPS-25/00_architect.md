<!-- SPDX-License-Identifier: MPL-2.0 -->

# Architect Handoff

## Metadata

- Component ID: `EXR-READ-OPS-25`
- Title: `ExfatInode` buffered regular-file read owner boundary
- Status: `Architected`
- Author: architect
- Date: `2026-04-12`
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-READ-OPS-25/20260412-1018-architect-packet.md`

## Functional Unit Definition

- Functional goal: implement the smallest coherent buffered `read_at` unit on `ExfatInode` that consumes inode-owned mapping output and performs actual byte transfer, short-read handling, EOF handling, and valid-size zero-fill policy for regular files.
- Final architectural owner: `ExfatInode`
- Owner class:
  - VFS trait carrier
- Expected landing form:
  - owner methods
- Boundary kind:
  - stable architectural boundary
- Why this boundary is architecturally real: buffered read is the stable VFS-visible data-path behavior for a regular-file inode. The inode already owns the file snapshot needed to answer read requests, and `EXR-FILE-MAP-24` already owns logical-to-physical translation. The remaining work is the actual read-side policy and byte transfer on the inode carrier itself, not a filesystem-global reader or a page-cache shell.

## Purpose

This unit turns the temporary `InodeIo::read_at` seam in `inode.rs` into the real buffered read path for regular files.
It should take the mapping result from `EXR-FILE-MAP-24`, copy data into the caller-provided `VmWriter`, stop at EOF, and apply the valid-size zero-fill rule for the unread initialized gap when the request extends past `valid_size` but still within the logical file size.

This unit must stay narrow: it owns the buffered read contract, but it does not own the page cache, write-side growth, truncate policy, allocator mutation, or filesystem-wide sync ordering.

## Why This Comes Now

`EXR-INODE-CORE-17` already established `ExfatInode` as the VFS carrier, and `EXR-FILE-MAP-24` already established the read-path translation layer that converts logical offsets into inode-owned file spans.
That means the next smallest coherent step is the read-side byte-transfer owner itself, because the existing `read_at` placeholder is now a temporary seam waiting to be absorbed rather than a long-lived staging boundary.

## Owner And Integration Convergence

- Interfaces, traits, services, or higher-level functions this unit ultimately serves:
  - VFS `InodeIo::read_at`
  - later page-cache read integration in `EXR-PGCACHE-26`
  - the regular-file read side of `ExfatInode`
- If the unit is internal-only, why that internal ownership is still stable in the finished system:
  - it is not internal-only; the read path is a stable user-visible inode behavior, but the owner is still `ExfatInode` rather than a filesystem-global service.
- Known non-goals or nearby logic that must remain in the parent owner:
  - page-cache ownership and cache policy
  - write-side growth, truncate, and allocation mutation
  - directory behavior and namespace mutation
  - sync ordering

Boundary consumption rules:

- `EXR-FILE-MAP-24` provides the source span and position information; this unit consumes that result and must not reopen logical-to-physical translation as a new owner boundary.
- EOF handling belongs here because it is part of read-side user-visible behavior, but the read path must stop at short-read semantics and not invent a separate buffering or caching owner.
- The valid-size gap is read-side policy here: bytes between `valid_size` and the logical file size are handled as zero-fill only because buffered read owns the final user-visible byte stream.
- `write_at` remains an explicit temporary seam for later write-side ownership; this row must not absorb it.

## Dependency Contract

- Depends on:
  - `EXR-INODE-CORE-17`
  - `EXR-FILE-MAP-24`
  - the VFS `InodeIo` contract
- Blocks:
  - the regular-file read path on `ExfatInode`
  - later page-cache read integration in `EXR-PGCACHE-26`
  - any later user-visible byte-stream read path that assumes buffered reads already exist
- Can run in parallel with:
  - sibling architect or designer work that does not widen `inode.rs` into cache or write ownership
- Recommended parallel wave:
  - Wave C, with regular-file buffered read kept separate from directory behavior and from later page-cache ownership
- Stable pre-existing interfaces used:
  - `InodeIo::read_at`
  - `VmWriter`
  - `ExfatInode`
  - `ExfatChain`-derived mapping output from `EXR-FILE-MAP-24`
  - inode-owned file size and valid-size facts
- Prior sources or prior slices that materially shaped the split:
  - `WORKSPACE-ARCH-RESET/00_architect.md`
  - `EXR-INODE-CORE-17/00_architect.md`
  - `EXR-FILE-MAP-24/00_architect.md`
  - `kernel/src/fs/vfs/fs_apis/inode.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/fs.rs`

## Recommended Work Slices

These are candidate creator slices for scheduler consideration, not the active global plan.

| Slice ID | Parent Unit Scope | Goal | Likely Write Set | Depends On | May Overlap With | Lane Class | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `WS-READ-OPS-25-TRANSFER` | `EXR-READ-OPS-25` | Implement the regular-file buffered byte-transfer path on `ExfatInode` using the mapping helpers and `VmWriter`, including partial-span copies across cluster boundaries. | `kernel/src/fs/fs_impls/exfat_refactor/inode.rs` | `EXR-INODE-CORE-17`, `EXR-FILE-MAP-24` | `WS-READ-OPS-25-BOUNDS` if the helper region remains disjoint inside `inode.rs` | creator | Keep the slice read-side only. Do not add page-cache hooks or write-side helpers. |
| `WS-READ-OPS-25-BOUNDS` | `EXR-READ-OPS-25` | Add the read-side bounds and policy helpers that decide EOF truncation, zero-fill length for the valid-size gap, and the final byte count returned to the caller. | `kernel/src/fs/fs_impls/exfat_refactor/inode.rs` | `EXR-INODE-CORE-17`, `EXR-FILE-MAP-24` | `WS-READ-OPS-25-TRANSFER` if both land in the same helper region in `inode.rs` | creator | This slice must stop at read policy and span derivation. It must not perform cache ownership, growth, or allocation mutation. |

## exFAT Concepts Covered

- Buffered regular-file reads on `ExfatInode`.
- Logical EOF versus valid-size zero-fill policy.
- Byte transfer into a caller-provided `VmWriter`.
- Short-read return behavior.
- Mapping output consumption from `EXR-FILE-MAP-24`.
- Read-only data-path behavior only; no cache, growth, or mutation ownership.

## Boundary Rejections

- Splits considered but rejected:
  - a filesystem-global read service separate from `ExfatInode`
  - a page-cache shell that would own regular-file reads before `EXR-PGCACHE-26`
  - folding write-side growth, truncate, or allocator policy into the read path
  - folding logical-to-physical translation back into this unit
- Why those rejected splits would be packet convenience, not real architecture:
  - they would hide the stable inode owner boundary that already owns the file snapshot needed for buffered reads
  - they would blur byte transfer with later cache ownership instead of letting the inode carrier own the user-visible read contract directly
  - they would recreate helper-first drift by turning a subordinate read seam into a fake long-lived service boundary

## Target Files

- Existing files likely to change:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- New files expected:
  - none

## Code Budget

- Target creator work-slice size: `160-260` lines
- Expected number of creator slices: `2`
- Reason if any single slice might exceed 500 lines: it should not. If a slice grows that large, buffered read policy has leaked into page-cache or write-side ownership and the unit should be re-sliced instead of expanded.

## Exit Condition

Design work may start once `ExfatInode` has a named owner-method path for buffered `read_at`, the mapping output from `EXR-FILE-MAP-24` is consumed rather than redefined, and the only unresolved read-side details are the explicit EOF, short-read, and valid-size zero-fill rules inside the inode owner.

## Risks

- `valid_size` can be mistaken for a cache hint instead of a read-side byte-stream boundary; the design must keep that policy on the inode read path.
- The temporary `read_at` seam can drift into a fake filesystem-global reader if the owner boundary is not named clearly.
- `inode.rs` is the likely shared landing zone for both read-side helper slices, so fake parallelism should be avoided if the helper region collides.
- Page-cache integration must remain a later owner boundary; if cache ownership appears in this unit, the row has grown beyond buffered read.
