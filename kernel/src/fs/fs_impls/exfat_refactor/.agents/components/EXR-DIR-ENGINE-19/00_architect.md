<!-- SPDX-License-Identifier: MPL-2.0 -->

# Architect Handoff

## Metadata

- Component ID: `EXR-DIR-ENGINE-19`
- Title: `ExfatFs` directory record-stream owner boundary
- Status: `Architected`
- Author: architect
- Date: `2026-04-07`
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-DIR-ENGINE-19/20260407-1040-architect-packet.md`

## Functional Unit Definition

- Functional goal: read directory contents as `ExfatDentrySet` streams over an `ExfatChain`, so mount-time system-entry discovery and later `ExfatInode` directory behavior can share one validated record-stream service.
- Final architectural owner: `ExfatFs`
- Expected landing form: owner-internal service `DirectoryEngine`
- Boundary kind: stable architectural boundary
- Why this boundary is architecturally real: directory traversal is the shared read-side state machine that sits between on-disk directory bytes and the rest of the filesystem. It needs one owner because it carries scan position, directory-hint state, record validation flow, and future adjacency to write-side directory mutation, but it must not absorb name policy, bitmap policy, inode cache policy, or VFS directory methods.

## Purpose

This unit is the smallest functionally coherent directory service that can exist before the rest of the mount/open sequence is complete.
It should own the directory scan loop and the conversion from directory bytes to validated record sets, while staying inside `ExfatFs` as an internal service.

`read_metadata_bytes` stays an owner-private I/O primitive, `ExfatChain` stays the validated traversal value, and `ExfatDentrySet` stays the validation boundary.
`DirectoryEngine` should orchestrate those pieces, not replace them with free helpers.

## Why This Comes Now

The boot and record-boundary foundations are already accepted, so the next stable unit is the shared directory-stream service that consumes them.
This can land before upcase-table loading, allocation-bitmap loading, inode caching, or VFS directory operations because the service only needs read-side traversal and validated entry-set assembly.

That sequencing matters for the refactor board: mount-time system-entry discovery needs a directory-stream owner, but the owner must remain read-only until `EXR-DENTRY-WRITE-28` exists.

## Owner And Integration Convergence

- Interfaces, traits, services, or higher-level functions this unit ultimately serves: future `EXR-UPCASE-20`, future `EXR-BITMAP-21`, future `EXR-FS-OPEN-22`, future `EXR-DIR-OPS-23`, and later `EXR-DENTRY-WRITE-28`.
- If the unit is internal-only, why that internal ownership is still stable in the finished system: `DirectoryEngine` is not a staging helper. It is the stable `ExfatFs` runtime service that both mount-time discovery and later inode directory operations depend on, so the owner boundary remains useful after open and after read-only directory ops land.
- Known non-goals or nearby logic that must remain in the parent owner: case folding, name hashing, bitmap discovery and occupancy queries, inode cache identity, VFS `lookup`/`readdir_at`, and all namespace mutation stay outside this unit.

Boundary consumption rules:

- `read_metadata_bytes` should remain an owner-private transport helper reached only through `DirectoryEngine`.
- `ExfatChain` should be accepted as the validated traversal state, not re-derived inside the engine.
- `ExfatDentrySet` should remain the validated file-record boundary the engine returns or consumes, so the service does not collapse into a bag of raw parsing helpers.

## Dependency Contract

- Depends on: `EXR-IO-02`, `EXR-CHAIN-03B`, `EXR-FILESET-04B`, and the accepted raw dentry types from `EXR-DENTRY-04A`.
- Blocks: mount-time directory discovery in `EXR-FS-OPEN-22`, directory read ops in `EXR-DIR-OPS-23`, and the read-side preconditions for later `EXR-DENTRY-WRITE-28`.
- Can run in parallel with: `EXR-INODE-CACHE-18` architect work, because the write sets are disjoint, and later `EXR-UPCASE-20` / `EXR-BITMAP-21` design work, because those owners consume the directory engine rather than define it.
- Recommended parallel wave: Wave B, but only as a read-only lane. Do not force this unit into the `fs.rs` owner shell if that would create file collisions with `EXR-FS-CORE-16` or `EXR-FS-OPEN-22`.
- Stable pre-existing interfaces used: `read_metadata_bytes`, `ExfatChain`, `RawExfatDentry`, `ExfatDentry`, `ExfatDentrySet`, and `ExfatSuperBlock`.
- Prior sources or prior slices that materially shaped the split: `EXR-IO-02`, `EXR-CHAIN-03B`, `EXR-FILESET-04B`, `WORKSPACE-ARCH-RESET/00_architect.md`, `ASTERINAS_ARCHITECT_PRIORS.md` profile `I-ARCH`, `ASTERINAS_CODE_QUALITY_PRIORS.md` profile `Q-ARCH`, and `linux-exFAT-implementation-summary.md` topic "Directory record parsing and dentry-set validation".

## Recommended Work Slices

These are candidate creator slices for scheduler consideration, not the active global plan.

| Slice ID | Parent Unit Scope | Goal | Likely Write Set | Depends On | May Overlap With | Lane Class | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `WS-DIR-ENGINE-19-A` | `EXR-DIR-ENGINE-19` | Define the `DirectoryEngine` owner-private scan state and the read-only directory record stream that turns directory bytes into ordered raw dentries and validated `ExfatDentrySet` values. | `kernel/src/fs/fs_impls/exfat_refactor/directory.rs` | `EXR-IO-02`, `EXR-CHAIN-03B`, `EXR-FILESET-04B` | `EXR-INODE-CACHE-18` architect only at the scheduler level; not file-parallel with `fs.rs` work if the same file is used for wiring | creator | Keep this slice policy-free: no upcase folding, no bitmap interpretation, no VFS `lookup`/`readdir_at`, and no write-side mutation. |
| `WS-DIR-ENGINE-19-B` | `EXR-DIR-ENGINE-19` | Add the mount-time discovery helpers that walk the validated stream and expose system-entry candidates by raw dentry kind and validated record shape only. | `kernel/src/fs/fs_impls/exfat_refactor/directory.rs` | `WS-DIR-ENGINE-19-A` | None if the first slice keeps the file narrow; do not try to parallelize this with the first slice because it lands in the same file | creator | This is still read-only. It may recognize directory-system records, but it must leave upcase loading and bitmap loading to their own owners. |

## exFAT Concepts Covered

- Directory-chain traversal.
- Metadata-byte reads over block-aligned storage.
- Raw dentry decode and typed dentry classification.
- Validated multi-entry file-record streaming.
- Directory record-set boundaries and checksum-preserving validation.
- Mount-time system-entry discovery from the root directory.
- Read-only directory scanning only. No namespace mutation.

## Boundary Rejections

- Splitting directory iteration into a free helper module was rejected. That would be packet convenience, not a stable owner boundary.
- Folding case folding, name hashing, or name comparison into this unit was rejected. Those belong to `EXR-UPCASE-20`.
- Folding allocation-bitmap loading or occupancy queries into this unit was rejected. Those belong to `EXR-BITMAP-21`.
- Folding `lookup` and `readdir_at` into this unit was rejected. Those belong to `EXR-DIR-OPS-23` on `ExfatInode`.
- Folding `create`, `unlink`, `mkdir`, `rmdir`, or `rename` into this unit was rejected. Those are explicitly write-side work for `EXR-DENTRY-WRITE-28`.
- Hiding mount-time discovery inside `EXR-FS-OPEN-22` without a reusable directory service was rejected. `ExfatFs` needs a stable directory-stream owner, not just a one-off mount script.

## Target Files

- Existing files likely to change: none in this architect pass.
- New files expected: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs`
- Future wiring risk to watch: `fs.rs` will likely be the integration surface for `EXR-FS-CORE-16` and `EXR-FS-OPEN-22`, so keep `DirectoryEngine` isolated from that shell until the owner boundary is complete.

## Code Budget

- Target creator work-slice size: `180-240` lines
- Expected number of creator slices: `2`
- Reason if any single slice might exceed 500 lines: it should not. If it does, the slice has probably absorbed name policy, mount sequencing, or write-side mutation, which means the boundary has drifted.

## Exit Condition

Design work may start once the directory-stream owner is defined as an `ExfatFs`-internal `DirectoryEngine` that can walk an `ExfatChain`, use `read_metadata_bytes` to materialize directory content, and return validated `ExfatDentrySet`-backed records without upcase policy, bitmap policy, VFS directory ops, or write-side mutation.

## Risks

- The engine can accidentally become a thin helper pile if it does not own the scan loop and scan state.
- `fs.rs` and `inode.rs` are future collision points, so the safest landing zone is a dedicated `directory.rs` instead of shared owner-shell code.
- A read-only service can drift into name policy if mount-time discovery starts doing case folding or name comparison too early.
- A read-only service can drift into allocation policy if system-entry discovery starts interpreting bitmap contents instead of just surfacing the directory records that later owners need.
- Later write-side work must not reuse this artifact as justification for `create` or `rename` helpers. Those belong to `EXR-DENTRY-WRITE-28`.
