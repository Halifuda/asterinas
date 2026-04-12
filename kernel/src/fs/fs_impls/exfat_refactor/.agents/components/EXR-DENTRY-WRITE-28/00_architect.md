<!-- SPDX-License-Identifier: MPL-2.0 -->

# Architect Handoff

## Metadata

- Component ID: `EXR-DENTRY-WRITE-28`
- Title: `DirectoryEngine` write-side directory-entry mutation boundary
- Status: `Architected`
- Author: architect
- Date: `2026-04-12`
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-DENTRY-WRITE-28/20260412-1217-architect-packet.md`

## Functional Unit Definition

- Functional goal: update on-disk exFAT directory file-record sets through `DirectoryEngine` by consuming validated `ExfatDentrySet` values and committed allocation results, so later create/delete/rename work can mutate directory entries without reopening fileset ownership or allocation ownership.
- Final architectural owner: `ExfatFs`
- Owner class:
  - structure owner
- Expected landing form:
  - `DirectoryEngine` write methods plus narrow owner-private helpers
- Boundary kind:
  - stable architectural boundary
- Why this boundary is architecturally real:
  - directory-entry mutation is the write-side companion to the already accepted `DirectoryEngine` read stream. The same filesystem-owned directory service must own slot discovery, record placement and removal, overwrite decisions, tombstoning, and on-disk serialization boundaries because those operations share the directory scan state and the validated file-record contract. Keeping that mutation surface inside `ExfatFs` preserves the owner boundary while avoiding a separate write manager, namespace helper service, or sync layer.

## Purpose

This unit is the smallest functionally coherent write-side directory service that can exist before namespace mutation work lands.
It should own the primitives that rewrite validated directory record sets in place or into newly committed space, while staying inside `DirectoryEngine` as an internal `ExfatFs` service.

`ExfatDentrySet` remains the validation and serialization boundary, `EXR-ALLOC-27` remains the owner of allocation search and committed allocation results, and `DirectoryEngine` write methods should orchestrate those pieces rather than replace them with free helpers.

## Why This Comes Now

The read-side directory engine already exists, the validated file-record boundary already exists, and the board reset already assigns write-side directory mutation to `DirectoryEngine` instead of to a standalone manager.
That makes the next stable unit the write-side mutation boundary that consumes validated sets and committed allocation outcomes without pulling in namespace policy, inode publication, or sync ordering.

## Owner And Integration Convergence

- Interfaces, traits, services, or higher-level functions this unit ultimately serves:
  - later `EXR-NAMESPACE-29` `create`, `unlink`, `mkdir`, `rmdir`, and `rename`
  - later `EXR-SYNC-31` only as an eventual consumer of durable metadata state
- If the unit is internal-only, why that internal ownership is still stable in the finished system:
  - `DirectoryEngine` is not a transient helper. It is the stable `ExfatFs` runtime service that both read-side directory traversal and write-side directory mutation depend on, so the owner boundary remains useful after namespace methods land.
- Known non-goals or nearby logic that must remain in the parent owner:
  - name policy and lookup semantics
  - inode publication and opened-inode reuse policy
  - allocation search, reservation, and commit ownership
  - file-size policy and file-data writeback
  - sync ordering and durability policy

Boundary consumption rules:

- `ExfatDentrySet` should be consumed as the validated file-record unit. This row must not reopen file-record validation or raw-name aggregation.
- Committed allocation results from `EXR-ALLOC-27` should be consumed as already-decided growth facts. This row must not re-own allocation search or reservation.
- On-disk serialization should remain a file-record concern first, with `DirectoryEngine` only placing the serialized bytes into the chosen slot range and applying tombstone or overwrite policy at the directory boundary.
- `DirectoryEngine` may maintain owner-private location and scan state, but it must not become a namespace policy owner or a separate writeback subsystem.

## Dependency Contract

- Depends on:
  - `EXR-DIR-ENGINE-19`
  - `EXR-FILESET-04B`
  - `EXR-ALLOC-27`
- Blocks:
  - later namespace mutation work in `EXR-NAMESPACE-29`
  - any later directory mutation path that assumes stable create/delete/rename primitives
- Can run in parallel with:
  - `EXR-DIR-OPS-23` architect and creator work, because the read-only inode surface and the write-side mutation surface are distinct
  - `EXR-ALLOC-27` design work, provided the committed-result shape stays explicit and does not pull allocation search back into this row
- Recommended parallel wave:
  - Wave D, after `EXR-ALLOC-27` is specified, with this row kept separate from inode namespace wiring
- Stable pre-existing interfaces used:
  - `DirectoryEngine`
  - `DirectoryRecordLocation`
  - `DirectoryFileRecord`
  - `ExfatDentrySet`
  - committed allocation results from `EXR-ALLOC-27`
- Prior sources or prior slices that materially shaped the split:
  - `WORKSPACE-ARCH-RESET/00_architect.md`
  - `EXR-DIR-ENGINE-19/00_architect.md`
  - `EXR-FILESET-04B/00_architect.md`
  - `directory.rs`
  - `fileset.rs`
  - `linux-exFAT-implementation-summary.md` topics “Directory entry engine” and “Directory record parsing and dentry-set validation”

## Recommended Work Slices

These are candidate creator slices for scheduler consideration, not the active global plan.

| Slice ID | Parent Unit Scope | Goal | Likely Write Set | Depends On | May Overlap With | Lane Class | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `WS-DENTRY-WRITE-28-A` | `EXR-DENTRY-WRITE-28` | Define the write-side slot discovery and record placement/removal primitives inside `DirectoryEngine`, consuming validated `ExfatDentrySet` values and a committed allocation result only as inputs. | `kernel/src/fs/fs_impls/exfat_refactor/directory.rs` | `EXR-DIR-ENGINE-19`, `EXR-FILESET-04B`, `EXR-ALLOC-27` | `WS-DENTRY-WRITE-28-B` only if the file is kept small enough; otherwise treat as sequential work on the same owner file | creator | Keep this slice focused on directory slot selection and in-place record rewriting. Do not add namespace policy, inode publication, or allocation search. |
| `WS-DENTRY-WRITE-28-B` | `EXR-DENTRY-WRITE-28` | Add tombstoning, overwrite rules, and the on-disk serialization boundary for rewritten directory records, including the directory-side handling needed when a write cannot stay in-place. | `kernel/src/fs/fs_impls/exfat_refactor/directory.rs` | `EXR-DIR-ENGINE-19`, `EXR-FILESET-04B`, `EXR-ALLOC-27` | `WS-DENTRY-WRITE-28-A` because both slices want the same `directory.rs` owner region | creator | This slice should remain subordinate to `DirectoryEngine` and `ExfatDentrySet`; it must not become a new manager or absorb sync ordering. |

## exFAT Concepts Covered

- Directory-chain mutation.
- Directory slot discovery and reuse.
- Multi-entry file-record rewrite using validated file-record sets.
- Tombstone and overwrite handling for directory entries.
- Serialized record placement as the mutation boundary.
- Directory growth using already committed allocation results.
- Write-side directory mutation only; no namespace policy.

## Boundary Rejections

- Splitting write-side mutation into a standalone directory-write manager was rejected. That would be packet convenience, not a stable owner boundary.
- Folding name policy or canonical lookup into this unit was rejected. Those belong to `EXR-UPCASE-20` and later `EXR-NAMESPACE-29`.
- Folding inode publication or opened-inode reuse into this unit was rejected. Those remain `ExfatFs` and `ExfatInode` concerns outside the mutation primitive.
- Folding allocation search or reservation into this unit was rejected. That belongs to `EXR-ALLOC-27`; this row only consumes committed allocation outcomes.
- Folding file-size policy, buffered data writes, or sync ordering into this unit was rejected. Those belong to later inode write and sync owners.

## Target Files

- Existing files likely to change:
  - `kernel/src/fs/fs_impls/exfat_refactor/directory.rs`
- New files expected:
  - none

## Code Budget

- Target creator work-slice size: `180-260` lines
- Expected number of creator slices: `2`
- Reason if any single slice might exceed 500 lines:
  - It should not. If it does, the slice has likely absorbed allocation search, namespace policy, or sync ordering, which means the boundary has drifted away from `DirectoryEngine` mutation and needs to be re-sliced.

## Exit Condition

Design work may start once the write-side directory owner is defined as an `ExfatFs`-internal `DirectoryEngine` surface that can accept validated `ExfatDentrySet` values, consume committed allocation results, and perform slot discovery plus record placement, removal, tombstoning, and overwrite handling without namespace policy, inode publication, allocation ownership, or sync ordering.

## Risks

- The boundary can drift into namespace policy if create/delete/rename semantics start living in the mutation primitive instead of in later `ExfatInode` owner methods.
- The boundary can drift into allocation ownership if slot growth starts performing search or reservation instead of consuming a committed allocation result.
- The boundary can drift into serialization ownership if `DirectoryEngine` starts rebuilding file-record content instead of placing already validated bytes.
- `directory.rs` is already the obvious landing zone for the read-side engine, so write-side helpers must stay subordinate to that owner rather than becoming a detached manager.
