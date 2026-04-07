<!-- SPDX-License-Identifier: MPL-2.0 -->

# Architect Handoff

## Metadata

- Component ID: `EXR-INODE-CACHE-18`
- Title: `ExfatFs` opened-inode table and validated `InodeKey`
- Status: `Architected`
- Author: architect
- Date: `2026-04-07`
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-INODE-CACHE-18/20260407-1040-architect-packet.md`

## Functional Unit Definition

- Functional goal: make `ExfatFs` the stable owner of the opened-inode table and the validated `InodeKey` used to reuse `Arc<ExfatInode>` handles across opens, lookups, and later root publication.
- Final architectural owner: `ExfatFs`
- Expected landing form: owner-internal state plus a validated value type
- Boundary kind: stable architectural boundary
- Why this boundary is architecturally real: opened-inode identity is a filesystem-wide coherence concern, not inode metadata. The filesystem owner must deduplicate inode carriers, preserve handle identity across repeated opens, and keep the identity rule separate from directory traversal and inode content state. The key itself is a validated value, while the table is owner-private runtime state.

## Purpose

This unit is the smallest coherent slice that gives `ExfatFs` a real opened-inode table and a stable identity model for non-root inodes.
It does not own directory traversal, inode metadata, or mount sequencing. It owns the filesystem-side cache that hands out shared `Arc<ExfatInode>` handles without creating a filesystem/inode ownership cycle.

## Why This Comes Now

`EXR-FS-CORE-16` already establishes `ExfatFs` as the filesystem owner, and `EXR-INODE-CORE-17` already establishes `ExfatInode` as the inode carrier with a weak filesystem back-reference.
The remaining missing owner boundary is the reuse layer between them: a table inside `ExfatFs` that can return the same inode object when the validated on-disk location is the same.

This can be designed now because the identity rule is already known from the accepted inode and fileset boundaries. The root special case still needs `EXR-FS-OPEN-22` to land cleanly, but the non-root table and the key semantics do not.

## Owner And Integration Convergence

- Interfaces, traits, services, or higher-level functions this unit ultimately serves: VFS `FileSystem` open/root handling, future inode-open reuse, and the filesystem-side part of `EXR-FS-OPEN-22`.
- If the unit is internal-only, why that internal ownership is still stable in the finished system: it is internal to `ExfatFs`, but it is stable because every later exFAT path that needs a live inode handle must come back through the same filesystem owner.
- Known non-goals or nearby logic that must remain in the parent owner: inode metadata, directory lookup, readdir, file mapping, page cache, and namespace mutation stay out of this boundary.

The `InodeKey` should represent the on-disk location of the primary directory entry, derived from trusted directory-location facts rather than from mutable inode contents.
In practice, that means the key is based on the validated directory chain position plus the entry offset or ordinal for the file-record primary entry.
It must not be derived from file size, timestamps, name text, or start cluster, because those are not identity facts.

Inference from the Linux summary and the legacy Asterinas hash shape: the root inode should be a distinguished owner slot, not a synthetic entry forced into the ordinary keyspace.
That keeps the generic key model reserved for real directory-entry locations and avoids pretending root is just another cached directory record.

## Dependency Contract

- Depends on: `EXR-FS-CORE-16`, `EXR-INODE-CORE-17`, validated directory-location facts from the fileset/directory work, and the VFS `FileSystem` contract.
- Blocks: `EXR-FS-OPEN-22` root publication, opened-inode reuse during mount/open sequencing, and any later path that expects stable `Arc<ExfatInode>` reuse from `ExfatFs`.
- Can run in parallel with: `EXR-DIR-ENGINE-19` architect work, because this unit consumes directory-location facts but does not own directory traversal.
- Recommended parallel wave: finalize the `ExfatFs` table/key contract now, then let `EXR-FS-OPEN-22` wire the root special case once the filesystem open sequence exists.
- Stable pre-existing interfaces used: VFS `FileSystem`, VFS `Inode`, the `ExfatFs` weak back-reference contract from `EXR-INODE-CORE-17`, and the directory-location facts already accepted for inode construction.
- Prior sources or prior slices that materially shaped the split: `EXR-FS-CORE-16`, `EXR-INODE-CORE-17`, `ASTERINAS_ARCHITECT_PRIORS.md` profile `I-ARCH`, `ASTERINAS_CODE_QUALITY_PRIORS.md` profile `Q-ARCH`, `linux-exFAT-implementation-summary.md` topic `Inode hashing and opened-inode identity`, and the legacy `exfat` `hash_index()` / root-hash behavior as integration context only.

## Recommended Work Slices

These are candidate creator slices for scheduler consideration, not the active global plan.

| Slice ID | Parent Unit Scope | Goal | Likely Write Set | Depends On | May Overlap With | Lane Class | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `WS-INODE-CACHE-18-OWNER` | `EXR-INODE-CACHE-18` | Define the `ExfatFs`-owned opened-inode table and the validated `InodeKey` model for non-root entries. | `kernel/src/fs/fs_impls/exfat_refactor/fs.rs` | `EXR-FS-CORE-16`, `EXR-INODE-CORE-17` | `EXR-DIR-ENGINE-19` architect work only; do not treat `fs.rs` as file-parallel with `EXR-FS-CORE-16` if both are landing in the same file. | creator | Keep the table owner-private. Return cloned `Arc<ExfatInode>` handles, never a separate ownership shell. |
| `WS-INODE-CACHE-18-LOOKUP` | `EXR-INODE-CACHE-18` | Add the non-root lookup/insert/remove contract that keeps the table coherent under `ExfatFs`. | `kernel/src/fs/fs_impls/exfat_refactor/fs.rs` | `WS-INODE-CACHE-18-OWNER` | none unless the shared `fs.rs` skeleton is still being written elsewhere | creator | This slice is only about table coherence and handle reuse. It must not pull in mount sequencing or directory traversal. |
| `WS-INODE-CACHE-18-ROOT` | `EXR-INODE-CACHE-18` | Wire the root special case as a dedicated `ExfatFs` slot or equivalent owner-private handle, not as a synthetic `InodeKey`. | `kernel/src/fs/fs_impls/exfat_refactor/fs.rs` | `EXR-FS-OPEN-22`, `EXR-FS-CORE-16`, `EXR-INODE-CORE-17` | `EXR-FS-OPEN-22` only after the root-handoff seam is settled | creator | This slice must wait for `EXR-FS-OPEN-22`. The root handle is part of filesystem open sequencing, not part of the generic keyspace. |

## exFAT Concepts Covered

- Opened-inode table ownership.
- Stable inode identity by primary directory-entry location.
- Parent-directory chain position and entry ordinal as identity facts.
- Root as a distinguished filesystem owner case.
- `Arc<ExfatInode>` reuse without a filesystem/inode reference cycle.
- Owner-private cache state versus validated value semantics.
- Filesystem-global serialization for cache mutation and handle publication.

## Boundary Rejections

- Splits considered but rejected:
  - a standalone helper-only `InodeKey` component
  - a synthetic root cache key that pretends root is a normal directory entry
  - a separate inode-shell owner that only forwards cache lookups
  - directory traversal or mount/open sequencing folded into the cache owner
- Why those rejected splits would be packet convenience, not real architecture:
  - they would recreate the old drift pattern where identity is treated as a free helper instead of a filesystem-owned coherence rule
  - they would blur the line between validated directory location facts and mutable inode metadata
  - they would create another ownership shell between `ExfatFs` and `ExfatInode` instead of letting `ExfatFs` own the reuse table directly

## Target Files

- Existing files likely to change: none yet
- New files expected: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`

## Code Budget

- Target creator work-slice size: `160-240` lines
- Expected number of creator slices: `2` to `3`
- Reason if any single slice might exceed 500 lines: it should not. If the slice grows that large, root handling or directory traversal has leaked into the cache owner and the unit must be re-sliced.

## Exit Condition

This design is ready for implementation when `ExfatFs` can own a validated non-root `InodeKey`, reuse opened inodes through an internal table, and keep the root handle outside the ordinary keyspace until `EXR-FS-OPEN-22` installs the final root sequencing.

## Risks

- The root special case can easily become a fake cache key if the owner model is not kept explicit.
- The table must not become a second place where directory semantics or name matching are re-implemented.
- A filesystem/inode cycle will appear if `ExfatInode` ever gets a strong back-reference to `ExfatFs`.
- Lock order must stay explicit. Cache mutation and handle publication should be serialized by the filesystem owner, and no implementation should do blocking I/O under that serialization point.
