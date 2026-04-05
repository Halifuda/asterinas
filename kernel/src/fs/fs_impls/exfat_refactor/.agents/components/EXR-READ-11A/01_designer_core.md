<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Core

## Metadata

- Component ID: `EXR-READ-11A`
- Title: Logical-To-Physical Mapping For Existing Regular-File Reads
- Status: `Specified`
- Author: `designer`
- Date: `2026-04-05`
- Task packet: `EXR-READ-11A-DESIGN-20260405-1059`
- Based on architect artifact: `00_architect.md`

## Scope

- In scope:
  - Translate a validated regular file's logical byte offset into on-disk cluster placement.
  - Respect contiguous and FAT-backed placement using the accepted `NoFatChain`-to-mode split.
  - Consume immutable inode read facts plus superblock geometry facts without reopening mount state.
  - Publish one narrow physical-placement boundary that later read execution can consume.
- Out of scope:
  - Buffered `read_at`, page-cache ownership, direct I/O, or zero-fill policy.
  - Allocation growth, truncation, or any write-side splicing path.
  - Directory lookup, namespace mutation, or root discovery.
  - Async coordination, background work, or a second mapping helper surface.

## Module Specification

- Dependencies:
  - `EXR-MOUNT-09`
  - `EXR-CHAIN-03B`
  - `EXR-INODE-05B`
- Interfaces provided:
  - One canonical read-mapping entry point in `read.rs` that maps a logical byte offset inside an existing regular file to physical cluster placement.
  - One narrow inode read-view accessor, only if needed, that exposes the read-mapping facts as an immutable bundle instead of reopening the shell.
  - One small placement type that carries the physical cluster and byte offset within that cluster.
- Files or modules touched:
  - `kernel/src/fs/fs_impls/exfat_refactor/read.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- Hidden implementation details:
  - Whether the placement helper returns a tuple or a small private struct internally.
  - Whether the inode read-view accessor returns a dedicated bundle type or a private borrow-only view.
  - Whether contiguous placement is computed directly in `read.rs` or delegated to a private helper that wraps `ExfatChain::walk_to_cluster_at_offset(...)`.

## Functional Specification

### Operation

- Name: map logical read offset to physical placement
- Inputs:
  - `inode_read_view`
  - `block_device: &dyn BlockDevice`
  - `super_block: &ExfatSuperBlock`
  - `offset: usize`
- Preconditions:
  - The inode view already represents an accepted existing regular file.
  - The file has validated chain facts and validated size facts.
  - `super_block` is already normalized.
  - `offset` is a logical byte offset into the file's existing data range.
- Actions:
  - Reject directory shells and other non-regular-file inputs.
  - If `offset` is at or beyond valid data length, return no placement instead of inventing a physical target.
  - If the file is contiguous, compute the destination cluster arithmetically from the chain head and cluster size.
  - If the file is FAT-backed, use the read-only chain helper to walk to the cluster containing the offset.
  - Compute the byte offset within the destination cluster from the logical offset remainder.
  - Keep the operation read-only; do not copy file bytes or touch page-cache state.
- Outputs:
  - `Result<Option<ExfatReadPlacement>>`
- Postconditions:
  - `Some(...)` means the logical offset lands inside existing file data and can be translated to a physical cluster location.
  - `None` means the caller reached logical EOF for this component's purposes.
  - The result contains placement facts only, not buffered-read policy.

## Invariants

- `NoFatChain` maps to contiguous placement and never forces a FAT walk.
- FAT-backed placement depends only on read-only chain walking.
- The read boundary is based on `valid_data_length`, not `data_length`.
- Directory shells are not valid inputs for this component.
- The placement result never points outside the valid data-cluster range.
- This component does not mutate mount state, inode state, FAT state, or page cache state.

## Concurrency Specification

- Shared state:
  - Borrowed `ExfatSuperBlock`.
  - Borrowed inode read facts.
  - Borrowed `BlockDevice` only when the chain is FAT-backed.
- Lock ordering:
  - None introduced here.
- Atomicity requirements:
  - Inputs are treated as immutable for the duration of the call.
  - FAT-backed multi-hop placement is not snapshot-atomic across external writers; that limitation is inherited from `EXR-CHAIN-03B`.
- Forbidden interleavings:
  - No mount publication, no writeback, no page-cache coordination, and no hidden second mount path.
- Allowed simplifications such as a temporary big lock:
  - No separate async artifact is needed.
  - Any residual serialization assumption is recorded here: callers must keep the underlying volume stable while the chain is being walked.

## Pass Split

### Serial Creator Pass

- Required implementation obligations:
  - Add the canonical read-mapping entry point in `read.rs`.
  - Add the smallest read-view boundary needed from `inode.rs` so the mapper can consume chain and size facts without exposing the whole shell.
  - Reuse the accepted chain helper and superblock geometry helper instead of re-deriving placement policy.
  - Keep the placement type narrow and read-only.
- Explicit non-goals:
  - No buffered read implementation.
  - No page-cache backend ownership.
  - No allocation, truncation, directory lookup, or write-side logic.

### Serial Checker Pass

- Required checker-owned tests:
  - A contiguous-file regression that proves a logical offset maps to the expected cluster and byte offset without consulting the FAT.
  - A FAT-backed regression that proves the mapper walks the chain to the correct destination placement.
  - An EOF regression that proves offsets at or beyond valid data length return no placement.
  - A directory rejection regression that proves non-regular-file shells are rejected.
- Observable properties that must pass before leaving the serial loop:
  - The mapper returns placement facts only.
  - Contiguous placement does not require block-device reads.
  - FAT-backed placement respects the accepted chain facts and the logical offset remainder.

### Concurrency Creator Pass

- Required implementation obligations:
  - None beyond the read-only borrow contract already recorded above.
- Explicit non-goals:
  - No concurrent publication protocol.
  - No shared mutable cache ownership.

### Concurrency Checker Pass

- Required checker-owned concurrency tests:
  - No dedicated concurrency tests required.
- Observable properties that must pass before leaving the concurrency loop:
  - None; the component is a read-only translation boundary.

## Acceptance Notes

- Keep the inode boundary narrow: one read-view surface is enough if it is justified by `EXR-READ-11A`, and separate getters should not appear unless a downstream caller proves they are needed.
- If the implementation starts needing buffered read behavior, page-cache hints, or allocation growth, the component has drifted and should be split again.
- The serial contract is sufficient here, so `02_designer_async.md` is intentionally omitted.
