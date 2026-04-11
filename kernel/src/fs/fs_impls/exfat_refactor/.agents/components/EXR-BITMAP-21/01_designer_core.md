<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Core

## Metadata

- Component ID: `EXR-BITMAP-21`
- Title: `ExfatFs` Allocation Bitmap Owner State And Read-Only Accounting
- Status: `Specified`
- Author: designer
- Date: 2026-04-10
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-BITMAP-21/20260410-1050-designer-packet.md`
- Based on architect artifact: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BITMAP-21/00_architect.md`

## Scope

- In scope:
  - Define `AllocationBitmap` as `ExfatFs`-owned runtime state for one validated allocation-bitmap image.
  - Consume the raw singleton `Bitmap` candidate discovered by `DirectoryEngine`.
  - Load bitmap bytes through `read_metadata_bytes`, validate the image against normalized geometry, and publish the validated snapshot into `ExfatFs`.
  - Provide read-only occupancy and free-space accounting queries from the owner boundary.
  - Keep later allocation mutation, FAT mutation, dirty tracking, and mount sequencing out of this component.
- Out of scope:
  - Directory traversal, bitmap candidate discovery, and name policy.
  - Allocation search, cluster marking, freeing, discard, or dirty-byte tracking.
  - FAT mutation, mount/open sequencing, and root inode publication.
  - Any public helper surface that exists only to duplicate the canonical occupancy or accounting queries.

## Module Specification

- Dependencies:
  - `DirectoryEngine` raw singleton bitmap candidates.
  - `read_metadata_bytes` as the owner-private transport primitive for the bitmap payload.
  - `ExfatChain` for the bitmap file extent.
  - `ExfatSuperBlock` for normalized cluster geometry and cluster-count bounds.
  - The `ExfatFs` owner boundary that will store and serve the validated snapshot.
- Interfaces provided:
  - Owner-private bitmap loading and validation on `ExfatFs`.
  - The canonical read-only occupancy query `cluster_is_allocated()`.
  - The canonical derived accounting queries `used_cluster_count()` and `free_cluster_count()`.
  - A crate-local `AllocationBitmap` state type that stays internal to `ExfatFs`.
- Files or modules touched:
  - Primary landing: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs`
  - Owner wiring: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- Hidden implementation details:
  - Whether the bitmap image is stored as packed bytes, words, or another internal bitvec form.
  - Whether used-cluster accounting is cached after validation or derived on demand from the immutable image.
  - The exact private field names on `AllocationBitmap` and `ExfatFs`, so long as the owner still exposes one canonical read-only occupancy query and one canonical pair of derived accounting queries.

## Functional Specification

### Operation

- Name: `ExfatFs::load_allocation_bitmap`
- Inputs:
  - A raw singleton `Bitmap` candidate discovered from the root directory.
  - The validated `ExfatChain` and normalized `ExfatSuperBlock` geometry for that bitmap file.
- Actions:
  - Read the bitmap payload through `read_metadata_bytes`.
  - Validate that the payload covers the data-cluster range described by the superblock.
  - Reject malformed size, impossible extent geometry, or any bitmap source that cannot be materialized as one coherent image.
  - Ensure the bitmap file's own clusters are represented as allocated in the loaded image.
  - Publish the validated image into the owner state only after the image is fully checked.
- Postconditions:
  - `ExfatFs` owns one validated immutable allocation-bitmap snapshot.
  - No caller can observe a half-loaded or partially validated bitmap.

### Operation

- Name: `ExfatFs::cluster_is_allocated`
- Inputs:
  - A data-cluster number in the valid exFAT cluster range.
- Actions:
  - Map cluster `2` to bit `0`.
  - Treat a set bit as allocated or bad.
  - Treat a clear bit as free.
  - Reject out-of-range cluster numbers instead of silently clamping them.
- Postconditions:
  - Callers can ask occupancy questions without seeing the raw bitmap image or any mutation policy.

### Operation

- Name: `ExfatFs::used_cluster_count`
- Inputs:
  - No additional inputs beyond the owner state.
- Actions:
  - Count allocated clusters from the validated bitmap image.
  - Ignore padding bits beyond the valid cluster range.
- Postconditions:
  - The returned count matches the validated image and the current superblock geometry.

### Operation

- Name: `ExfatFs::free_cluster_count`
- Inputs:
  - No additional inputs beyond the owner state.
- Actions:
  - Derive free clusters as the total valid cluster count minus the used-cluster count.
  - Do not consult the FAT or any future allocator policy.
- Postconditions:
  - Free-space reporting can be derived from the same owner-owned snapshot that answers occupancy queries.

## Invariants

- `AllocationBitmap` is owner-internal to `ExfatFs`.
- The bitmap image is validated once and then treated as read-only state until a later write-side owner exists.
- Cluster numbering starts at `2`; bit `0` corresponds to cluster `2`.
- Occupancy and accounting ignore padding bits outside the valid cluster range.
- The bitmap owner does not rescan the directory once the raw singleton candidate has been discovered.
- No allocation search, cluster flipping, dirty tracking, or FAT mutation lives in this component.
- `cluster_is_allocated()` is the canonical occupancy query; `free_cluster_count()` is derived from the same snapshot rather than from a separate free-space helper.

## Concurrency Specification

- Shared state:
  - The owner-owned validated bitmap image.
  - Any cached derived counts that are computed from that image.
- Lock ordering:
  - Bitmap load and publication must occur under the filesystem-owner serialization boundary used by `ExfatFs`.
  - The load path may not perform blocking I/O while holding a lock that requires non-blocking progress.
- Atomicity requirements:
  - Readers must see either no bitmap or one fully validated bitmap snapshot.
  - Derived counts must correspond to the same snapshot that answers occupancy queries.
- Forbidden interleavings:
  - No query may observe a partially loaded image.
  - No mutation path may race with read-only accounting inside this component.
- Allowed simplifications such as a temporary big lock:
  - A single owner-side publication boundary is sufficient because the bitmap becomes immutable after validation.

## Pass Split

### Serial Creator Pass

- Required implementation obligations:
  - Add the `AllocationBitmap` owner state in `bitmap.rs`.
  - Implement bitmap loading from a raw singleton candidate using `read_metadata_bytes`.
  - Validate geometry, length, and bitmap-file cluster ownership before publication.
  - Implement the canonical read-only occupancy query.
  - Implement the canonical used/free accounting queries.
  - Keep mutation, dirty tracking, discard, and FAT writes out of the component.
- Explicit non-goals:
  - No free-space search.
  - No bit flips or cluster allocation.
  - No dirty-range bookkeeping.
  - No temporary helper shell for a second occupancy API.

### Serial Checker Pass

- Required checker-owned tests:
  - Verify that malformed bitmap images are rejected before publication.
  - Verify that a valid bitmap reports occupancy correctly for the first and last valid clusters.
  - Verify that out-of-range cluster queries are rejected.
  - Verify that used-cluster and free-cluster counts match the same validated image.
- Observable properties that must pass before leaving the serial loop:
  - The owner loads one coherent bitmap snapshot.
  - Occupancy queries and accounting queries agree with each other.
  - Padding bits and invalid clusters do not leak into accounting.

### Concurrency Creator Pass

- Required implementation obligations:
  - No dedicated concurrency implementation is required beyond the owner-side publication boundary already described here.
- Explicit non-goals:
  - Do not add background refresh, lock-free publication, or per-query mutation tracking.

### Concurrency Checker Pass

- Required checker-owned concurrency tests:
  - No dedicated concurrency tests required.
- Observable properties that must pass before leaving the concurrency loop:
  - The bitmap remains immutable after publication and does not require a separate async protocol.

## Acceptance Notes

- Reviewers should confirm that `cluster_is_allocated()` is the canonical occupancy surface and that free-space reporting is derived from the same bitmap snapshot.
- Reviewers should reject any attempt to move allocation search, cluster marking, or dirty-byte tracking into this component.
- Reviewers should confirm that the bitmap image is owner-owned state, not a directory-scanning helper.
- Later allocator work should consume this component, not redefine it.
