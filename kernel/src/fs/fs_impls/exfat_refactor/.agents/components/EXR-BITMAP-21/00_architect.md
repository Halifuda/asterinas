<!-- SPDX-License-Identifier: MPL-2.0 -->

# Architect Handoff

## Metadata

- Component ID: `EXR-BITMAP-21`
- Title: `ExfatFs` allocation-bitmap owner boundary
- Status: `Architected`
- Author: architect
- Date: `2026-04-07`
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-BITMAP-21/20260407-1110-architect-packet.md`

## Functional Unit Definition

- Functional goal: load and validate the allocation bitmap, then provide read-only occupancy and free-space accounting queries through `ExfatFs` without absorbing allocation mutation, FAT mutation, directory streaming, or mount/open sequencing.
- Final architectural owner: `ExfatFs`
- Expected landing form: owner-internal state (`AllocationBitmap`) plus owner methods
- Boundary kind: stable architectural boundary
- Why this boundary is architecturally real: allocation occupancy is filesystem-wide runtime state with its own lifecycle. It is discovered from the root directory, validated once, and then consulted by the filesystem owner for read-only occupancy facts and later allocator decisions. That makes it a stable `ExfatFs` concern, but only as read-side state in this unit.

## Purpose

This unit is the smallest coherent filesystem-wide bitmap service that can exist before mutation ownership lands.
It should own the in-memory bitmap image, validation of the on-disk bitmap source, and derived read-only accounting such as used/free cluster counts.

`DirectoryEngine` remains the source of raw singleton bitmap candidates, not the directory scanner itself.
`AllocationBitmap` should accept that already-discovered candidate and materialize validated bitmap state, but it must not iterate directory records or interpret the rest of the root directory.

## Why This Comes Now

The directory-stream owner is already accepted, so bitmap ownership can now sit on top of a stable read-side discovery source instead of reusing directory traversal as a hidden allocator concern.
This component can land before `EXR-ALLOC-27` because it only needs validated geometry, metadata-byte reads, and raw bitmap candidates. It does not need allocation search, bit flipping, FAT updates, or writeback ordering.

That sequencing matters for the refactor board: mount-time bitmap discovery should terminate in `ExfatFs`-owned bitmap state, but the state should remain read-only until the allocator owner exists.

## Owner And Integration Convergence

- Interfaces, traits, services, or higher-level functions this unit ultimately serves: future `EXR-FS-OPEN-22`, future `EXR-ALLOC-27`, and later sync or volume-stat reporting on `ExfatFs`.
- If the unit is internal-only, why that internal ownership is still stable in the finished system: `AllocationBitmap` is not a staging helper. It is the persistent filesystem-wide occupancy view that later allocator, statfs, and sync code must consult through the same filesystem owner.
- Known non-goals or nearby logic that must remain in the parent owner:
  - mount/open sequencing stays in `ExfatFs`,
  - directory traversal stays in `DirectoryEngine`,
  - allocation search, mark, free, and dirty-tracking policy stay in `EXR-ALLOC-27`,
  - FAT mutation stays outside this unit,
  - write ordering and persistence semantics stay for later work.

Boundary consumption rules:

- `DirectoryEngine` should surface a raw singleton `Bitmap` candidate; this unit consumes that candidate and does not rescan the directory.
- `read_metadata_bytes` should remain the owner-private transport primitive for loading the bitmap payload once the candidate is known.
- `ExfatChain` and `ExfatSuperBlock` remain the validated traversal and geometry inputs; the bitmap owner must not re-derive chain semantics or directory semantics.

## Dependency Contract

- Depends on: `EXR-DIR-ENGINE-19`, `EXR-IO-02`, `EXR-CHAIN-03B`, `EXR-DENTRY-04A`, `EXR-FILESET-04B`, and the normalized `ExfatSuperBlock` geometry from the boot foundation.
- Blocks: read-only occupancy queries, free-space accounting, mount-time bitmap publication, and the later allocator owner that will consume this state.
- Can run in parallel with: `EXR-UPCASE-20` architect/design work and later `EXR-FS-OPEN-22` planning, because those owners consume the bitmap state rather than define it.
- Recommended parallel wave: Wave B, as part of the mount-critical read-side services.
- Stable pre-existing interfaces used: `DirectoryEngine` singleton bitmap candidates, `read_metadata_bytes`, `ExfatChain`, `ExfatSuperBlock`, `RawExfatDentry` / `ExfatDentry`, and `ExfatDentrySet`.
- Prior sources or prior slices that materially shaped the split: `EXR-DIR-ENGINE-19` architect/designer artifacts, `WORKSPACE-ARCH-RESET/00_architect.md`, `ASTERINAS_ARCHITECT_PRIORS.md` profile `I-ARCH`, `ASTERINAS_CODE_QUALITY_PRIORS.md` profile `Q-ARCH`, `linux-exFAT-implementation-summary.md` topic "Allocation bitmap scanning and free-space accounting", and the legacy `kernel/src/fs/fs_impls/exfat/bitmap.rs` only as integration context.

## Recommended Work Slices

These are candidate creator slices for scheduler consideration, not the active global plan.

| Slice ID | Parent Unit Scope | Goal | Likely Write Set | Depends On | May Overlap With | Lane Class | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `WS-BITMAP-21-A` | `EXR-BITMAP-21` | Define the `AllocationBitmap` owner-internal state and the load/validation path that consumes a raw singleton bitmap candidate from `DirectoryEngine` and materializes validated bitmap bytes. | `kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs` | `EXR-DIR-ENGINE-19`, `EXR-IO-02`, `EXR-CHAIN-03B`, `EXR-FILESET-04B` | `EXR-UPCASE-20` architect only at the scheduler level; no file-parallel overlap if the same `fs.rs` wiring is touched elsewhere | creator | Keep this slice strictly read-side. Validate size, alignment, and geometry, but do not add allocation search, bit flips, or FAT updates. |
| `WS-BITMAP-21-B` | `EXR-BITMAP-21` | Add read-only occupancy and free-space accounting queries over the validated bitmap state, including derived counts and range checks. | `kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs` | `WS-BITMAP-21-A` | none if the first slice stays isolated in `bitmap.rs` | creator | This is still read-only. It may expose counts and predicates, but it must not grow into allocator policy, dirty-range tracking, or trim/discard behavior. |

## exFAT Concepts Covered

- Allocation bitmap discovery from the root directory.
- Raw singleton `Bitmap` dentry handling.
- Bitmap payload validation and loading.
- Read-only cluster occupancy predicates.
- Derived free-space and used-cluster accounting.
- Bitmap lifecycle as `ExfatFs` runtime state.
- Read-side mount publication only. No allocation mutation.

## Boundary Rejections

- Splitting bitmap handling into a free helper module was rejected. That would be packet convenience, not a stable owner boundary.
- Folding directory traversal into this unit was rejected. `DirectoryEngine` already owns that record stream.
- Folding allocation search, cluster marking, freeing, dirty-byte tracking, or TRIM/discard behavior into this unit was rejected. Those are write-side allocator concerns for `EXR-ALLOC-27`.
- Folding FAT mutation into this unit was rejected. Bitmap occupancy and FAT writes are separate ownership duties.
- Hiding mount/open sequencing inside this unit was rejected. `ExfatFs` owns the mount path, but this component is only the bitmap state slice under that owner.

## Target Files

- Existing files likely to change: none in this architect pass
- New files expected: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs`
- Future wiring risk to watch: `fs.rs` will likely be the integration surface for `ExfatFs`, so keep bitmap ownership isolated from the filesystem shell until the owner boundary is complete.

## Code Budget

- Target creator work-slice size: `180-260` lines
- Expected number of creator slices: `2`
- Reason if any single slice might exceed 500 lines: it should not. If it does, the slice has probably absorbed mutation policy, directory scanning, or filesystem-open sequencing, which means the boundary has drifted.

## Exit Condition

Design work may start once the bitmap owner is defined as an `ExfatFs`-internal `AllocationBitmap` that can consume a raw singleton bitmap candidate, validate and materialize the bitmap image, and answer read-only occupancy and accounting queries without allocation mutation, FAT mutation, directory streaming, or mount/open sequencing.

## Risks

- The owner can accidentally become a directory-scanning helper pile if it reaches back into `DirectoryEngine` instead of consuming a prepared candidate.
- The bitmap state can drift into allocator policy if search, mark, free, or dirty-range logic is added before `EXR-ALLOC-27`.
- `fs.rs` and later allocator work are future collision points, so the safest landing zone is a dedicated `bitmap.rs` rather than shared owner-shell code.
- Read-only accounting can accidentally become write-side persistence logic if sync behavior is pulled in too early.
- Later write-side work must not reuse this artifact as justification for allocation mutation helpers. Those belong to `EXR-ALLOC-27`.
