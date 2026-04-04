<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Core

## Metadata

- Component ID: `EXR-BITMAP-08A`
- Title: Allocation Bitmap Loader And Read-Only Occupancy Queries
- Status: `Specified`
- Author: `designer`
- Date: `2026-04-04`
- Task packet: `EXR-BITMAP-08A-DESIGN-20260404-1414`
- Based on architect artifact: `00_architect.md`

## Scope

- In scope:
  - Consume the validated bitmap discovery facts from `EXR-SYSROOT-06`.
  - Load the on-disk allocation bitmap bytes through the existing read-side I/O and chain helpers.
  - Validate that the bitmap is large enough for the volume geometry.
  - Validate that the bitmap file's own clusters are marked allocated before the bitmap becomes visible.
  - Expose a canonical read-only occupancy surface for cluster-id and bounded-range queries.
- Out of scope:
  - Bitmap search policy, first-free hint policy, or any other allocation strategy.
  - Mutation, dirty tracking, discard/trim policy, or writeback.
  - Root-directory rescanning or discovery of the bitmap entry itself.
  - Mount bootstrap, mount-owned shared state, or general directory APIs.
  - Any async work, background coordination, or concurrency policy beyond the synchronous load boundary.

## Module Specification

- Dependencies:
  - `EXR-IO-02`
  - `EXR-CHAIN-03B`
  - `EXR-SYSROOT-06`
  - `EXR-FILESET-04B` only as an upstream read-side validation boundary if a later creator needs to thread the bitmap file through the existing file-record surface; do not add a helper surface for it unless the need is proven.
- Interfaces provided:
  - One canonical loader entry point that accepts the validated bitmap discovery record and returns one loaded bitmap value.
  - One canonical read-only bitmap value type that owns the loaded bytes and the geometry facts needed for occupancy checks.
  - One occupancy query surface for single-cluster checks.
  - One occupancy query surface for bounded-range checks.
- Files or modules touched:
  - `kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- Hidden implementation details:
  - Whether the loader stores the loaded bytes as a contiguous buffer, a packed bit-slice, or another private read-only representation.
  - Whether the loader walks the bitmap file cluster-by-cluster or accumulates a validated chain view before reading bytes.
  - Whether the range query is implemented by iterating the covered clusters or by a private bit-scan helper.

The canonical surface must stay narrow. Later callers should not need a second discovery path, a free-space search helper, or a mutation-oriented accessor to use the loaded bitmap.

## Functional Specification

### Operation

- Name: allocation-bitmap load-and-query
- Inputs:
  - `block_device: &dyn BlockDevice`
  - `super_block: &ExfatSuperBlock`
  - `bitmap_facts: &ExfatSysRootBitmapDiscovery`
- Preconditions:
  - `bitmap_facts` already came from the validated root-directory discovery boundary in `EXR-SYSROOT-06`.
  - `bitmap_facts` already carries a legal root-entry location token, a legal start cluster, and a representable byte size.
  - The caller wants only the loaded allocation bitmap and occupancy queries, not search policy or mutation.
- Actions:
  - Read the bitmap payload through the existing metadata I/O and chain helpers.
  - Load exactly the discovered bitmap byte size.
  - Validate that the loaded payload is large enough to represent every data cluster on the volume.
  - Reject undersized payloads before exposing any bitmap surface.
  - Validate that every cluster used by the bitmap file itself is marked allocated in the loaded bitmap.
  - Reject malformed cluster coverage before exposing any bitmap surface.
  - Preserve the loaded bytes as a read-only bitmap surface for later occupancy queries.
- Outputs:
  - `Result<ExfatAllocationBitmap>`
- Postconditions:
  - The returned bitmap is read-only.
  - Occupancy queries observe only the validated data-cluster range.
  - Reserved clusters and one-past-end cluster ids remain out of range instead of being treated as real volume space.
  - No search cursor, free-space hint, dirty state, or write path is created.

### Canonical Surface

- `ExfatAllocationBitmap` contains the loaded bitmap bytes and the geometry facts needed to answer occupancy queries.
- The single-cluster query surface answers whether one legal data-cluster id is allocated.
- The bounded-range query surface answers whether a legal half-open cluster range is fully allocated.
- The bitmap surface never exposes a mutation API, a free-space search API, or any persisted allocation policy.

### Validation Ownership

- Loader-time validation belongs here:
  - bitmap byte-size sufficiency against the volume geometry,
  - chain-backed payload loading,
  - cluster coverage for the bitmap file itself,
  - cluster-id bounds for later occupancy queries.
- Discovery-time validation belongs to `EXR-SYSROOT-06`:
  - root-entry identity,
  - start-cluster legality,
  - representable byte size,
  - duplicate and missing root-entry handling.
- Allocation-policy validation belongs later:
  - search ordering,
  - hint advancement,
  - mutation semantics.

## Invariants

- The bitmap surface is read-only after creation.
- The bitmap surface never rescans the root directory.
- The bitmap surface never invents a search cursor or a first-free hint.
- The bitmap surface rejects out-of-range cluster ids instead of interpreting tail padding as real clusters.
- The bitmap surface never exposes bytes as writable state.
- The bitmap file's own clusters are validated as allocated before the surface becomes visible.
- An oversized on-disk bitmap is acceptable if it still covers the full data-cluster range.
- An undersized on-disk bitmap is rejected.

## Concurrency Specification

- Shared state:
  - None introduced by this component.
- Lock ordering:
  - None.
- Atomicity requirements:
  - The load is one synchronous call that either returns a complete read-only bitmap or fails.
  - Callers must not observe a partially initialized bitmap surface.
- Forbidden interleavings:
  - None beyond ordinary single-threaded call ordering.
- Allowed simplifications such as a temporary big lock:
  - None needed.

No separate async artifact is needed because the component introduces no background work, no awaitable I/O contract, no shared mutable state, and no lock-ordering obligations. The full serialization story is recorded here in the synchronous concurrency specification and the pass split below.

## Pass Split

### Serial Creator Pass

- Required implementation obligations:
  - Add one loader entry point that consumes the validated bitmap discovery facts and returns one loaded bitmap surface.
  - Add one canonical read-only occupancy surface for cluster-id and bounded-range queries.
  - Reject undersized bitmap payloads.
  - Reject bitmap payloads whose own cluster coverage is not fully marked allocated.
  - Keep the bitmap surface read-only and boundary-validated.
- Explicit non-goals:
  - No free-space search, no hint policy, and no allocation or deallocation logic.
  - No dirty tracking, discard/trim policy, or writeback support.
  - No root-directory discovery and no mount bootstrap.
  - No async coordination or lock-management behavior.

### Serial Checker Pass

- Required checker-owned tests:
  - A happy-path load regression that proves the bitmap loads from the validated discovery facts and answers at least one occupied and one free cluster query correctly.
  - An undersized-bitmap regression that proves the loader rejects a bitmap smaller than the minimum byte size required by the volume geometry.
  - A self-coverage regression that proves the loader rejects a bitmap whose own clusters are not marked allocated.
  - An out-of-range-query regression that proves reserved cluster ids and one-past-end ids are rejected instead of being treated as real volume space.
  - An oversized-bitmap regression that proves extra on-disk bytes beyond the minimum do not cause rejection by themselves.
- Observable properties that must pass before leaving the serial loop:
  - The returned surface is read-only.
  - The tests exercise only the loader and occupancy surface, not search or mutation.
  - The tests do not require page cache, mount sequencing, VFS trait coverage, or async harnesses.

### Concurrency Creator Pass

- Required implementation obligations:
  - None.
- Explicit non-goals:
  - No concurrency pass is needed for this component.

### Concurrency Checker Pass

- Required checker-owned concurrency tests:
  - No dedicated concurrency tests required.
- Observable properties that must pass before leaving the concurrency loop:
  - None; the synchronous read-only boundary already captures the full contract.

## Acceptance Notes

- The loader should stay narrower than allocation policy. If it starts computing free-space search results or hints, it has crossed into `EXR-BITMAP-08B`.
- The loader should stay narrower than discovery. If it starts rescanning the root directory, it is duplicating `EXR-SYSROOT-06`.
- The loaded surface should remain a pure read-only value. Any write path, dirty-byte tracking, or mutation helper belongs outside this component.
- The canonical occupancy surface should be sufficient for later consumers without adding extra getter layers unless a downstream component proves a specific need.
