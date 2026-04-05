<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Core

## Metadata

- Component ID: `EXR-WRITE-13A`
- Title: Writable Regular-File Allocation Growth And Metadata Publication
- Status: `Specified`
- Author: `designer`
- Date: `2026-04-05`
- Task packet: `EXR-WRITE-13A-DESIGN-20260405-1224`
- Based on architect artifact: `00_architect.md`

## Scope

- In scope:
  - Grow a writable regular file by allocating additional clusters and publishing the larger allocation boundary.
  - Extend or publish the cluster chain so the new allocation becomes visible as a coherent file image.
  - Preserve the distinction between allocated length and valid-data length.
  - Update inode and mount-visible metadata consistently after allocation succeeds.
  - Consume the accepted chain and bitmap boundaries instead of re-deriving placement policy.
- Out of scope:
  - Buffered write-copy policy, page-cache writeback, or direct I/O.
  - Read-side zero-fill behavior or logical-to-physical read mapping.
  - Truncate, shrink, or tail-cluster free policy.
  - Namespace mutation, directory creation or removal, or lookup policy.
  - Mount bootstrap or root discovery.

## Module Specification

- Dependencies:
  - `EXR-MOUNT-09`
  - `EXR-INODE-05B`
  - `EXR-CHAIN-03B`
  - `EXR-BITMAP-08B`
  - `EXR-SBGEOM-15`
- Interfaces provided:
  - One canonical growth-and-publication entry point in `fs.rs` for writable regular files.
  - One narrow inode publication surface in `inode.rs`, only if needed to keep the updated allocation and size facts coherent.
  - One bitmap mutation surface in `bitmap.rs` for reserving and marking newly allocated clusters.
- Files or modules touched:
  - `kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- Hidden implementation details:
  - Whether the canonical growth entry point returns the updated inode shell or mutates the validated inode state in place.
  - Whether bitmap reservation and chain publication are coordinated by one private transaction helper or by a short synchronous sequence inside the canonical entry point.
  - Whether chain extension is expressed as a private wrapper over the accepted `ExfatChain` helpers or by a small allocator-specific helper that exists only for the growth caller.

## Functional Specification

### Operation

- Name: grow regular-file allocation boundary
- Inputs:
  - mount-owned filesystem state
  - validated writable regular-file inode state
  - requested new file length
- Preconditions:
  - The inode already passed the accepted metadata and chain boundaries.
  - The inode represents a writable regular file, not a directory or root shell.
  - The requested length is greater than or equal to the current allocated length.
  - The caller is not asking this component to initialize file bytes, zero-fill gaps, or shrink the file.
- Actions:
  - Compute the additional cluster coverage needed for the requested length.
  - Reserve the required clusters through the bitmap boundary.
  - Extend the cluster chain or publish the newly allocated chain segment so the allocation becomes visible as one coherent file image.
  - Update the inode-visible allocation metadata to the new length.
  - Preserve the existing valid-data boundary unless a later buffered-write component separately advances it.
  - Publish the new allocation only after the bitmap and chain state agree.
- Outputs:
  - `Result<()>`.
- Postconditions:
  - The file's allocation boundary grows to the requested size.
  - The file's valid-data length does not advance as a side effect of allocation alone.
  - The bitmap, chain, and inode metadata describe the same enlarged allocation boundary.
  - No buffered write policy or truncate/shrink policy is introduced.

## Invariants

- Allocation growth never reassigns buffered-write responsibility.
- `valid_data_length <= data_length` remains true after growth.
- A directory shell or root shell does not enter this component.
- The accepted chain mode remains a fact the growth path consumes; it is not recomputed from scratch.
- Bitmap publication and chain publication must agree before the inode boundary is exposed as enlarged.
- The growth path does not manufacture page-cache state, writeback policy, or directory mutation state.

## Concurrency Specification

- Shared state:
  - Mount-owned filesystem state.
  - The allocation bitmap boundary.
  - The mutable inode publication state for the writable regular file being expanded.
- Lock ordering:
  - Serialize growth through one private allocation-and-publication critical section.
  - If the implementation uses narrower locks, the order is: mount or volume allocation guard, then bitmap mutation, then chain publication, then inode metadata publication.
  - No page-cache lock, buffered-write lock, or truncate lock may be held while waiting on allocation or chain bookkeeping.
- Atomicity requirements:
  - The requested growth is all-or-nothing from the caller's point of view.
  - No reader may observe a new inode size without the bitmap and chain having already agreed on the new allocation.
  - A failed allocation leaves the prior inode size and chain metadata intact.
- Forbidden interleavings:
  - No partial chain publication.
  - No bitmap reservation that becomes visible before the matching inode update can succeed.
  - No concurrent truncate or buffered-write path inside this component.
- Allowed simplifications such as a temporary big lock:
  - A temporary synchronous growth lock is acceptable if it is private to the component.
  - The lock must not escape the canonical growth path or become a general filesystem lock.

## Pass Split

### Serial Creator Pass

- Required implementation obligations:
  - Add one canonical writable-regular-file growth entry point in `fs.rs`.
  - Use the accepted bitmap and chain boundaries to extend allocation without re-deriving placement policy.
  - Keep the valid-data boundary separate from the allocation boundary.
  - Publish the enlarged chain and metadata only after allocation succeeds.
  - Add only the smallest inode publication helper needed to keep the growth result coherent.
- Explicit non-goals:
  - No buffered write-copy logic.
  - No zero-fill or hole-initialization policy.
  - No truncate, shrink, or tail-free logic.
  - No directory or namespace mutation.
  - No page-cache backend ownership.

### Serial Checker Pass

- Required checker-owned tests:
  - A contiguous-growth regression that proves additional clusters are reserved and the inode-visible allocation boundary grows while valid-data length stays unchanged.
  - A chain-publication regression that proves a growth requiring chain extension publishes the accepted chain facts rather than inventing a second allocation path.
  - A boundary regression that proves a directory or root shell is rejected by the growth surface.
  - A failure-atomicity regression that proves allocation failure does not expose a partially grown inode or partially linked chain.
- Observable properties that must pass before leaving the serial loop:
  - The growth path updates allocation and metadata together.
  - The valid-data boundary remains distinct from the allocation boundary.
  - The tests do not need buffered write copying, truncate handling, or page-cache behavior.

### Concurrency Creator Pass

- Required implementation obligations:
  - None beyond the synchronous publication boundary already recorded above.
- Explicit non-goals:
  - No background allocator, retry queue, or deferred publication protocol.
  - No shared mutable cache ownership in this component.

### Concurrency Checker Pass

- Required checker-owned concurrency tests:
  - No dedicated concurrency tests required.
- Observable properties that must pass before leaving the concurrency loop:
  - None; the component is satisfied by the synchronous allocation-and-publication contract recorded in the core and async specs.

## Acceptance Notes

- The component should remain growth-only. If it starts copying user bytes, zero-filling gaps, or freeing tail clusters, the split has drifted into later write-side lanes.
- The allocation boundary and the valid-data boundary must stay visibly distinct so later buffered-write work can advance initialized length without re-owning allocation growth.
- If a helper surface is proposed only to expose one stored inode fact, it needs a named downstream caller; otherwise it should not appear.
