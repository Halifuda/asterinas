<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Core

## Metadata

- Component ID: `EXR-READ-11B`
- Title: Buffered Regular-File Read Execution And Read-Side Zero-Fill
- Status: `Specified`
- Author: `designer`
- Date: `2026-04-05`
- Task packet: `EXR-READ-11B-DESIGN-20260405-1134`
- Based on architect artifact: `00_architect.md`

## Scope

- In scope:
  - Serve buffered `read_at` for already-mounted exFAT regular files.
  - Consume the `EXR-READ-11A` placement boundary for the initialized prefix of the read.
  - Zero-fill the unread portion of a request that falls between `valid_data_length` and `data_length`.
  - Keep the regular-file read path separate from page-cache backend ownership.
- Out of scope:
  - Logical-to-physical mapping, chain walking, or any other re-derivation of placement.
  - Page-cache backend ownership, backend registration, or page-count policy.
  - Allocation growth, truncation, writeback, direct I/O, or namespace behavior.
  - Directory lookup, root discovery, or mount bootstrap.

## Module Specification

- Dependencies:
  - `EXR-MOUNT-09`
  - `EXR-READ-11A`
  - `EXR-PGCACHE-11B`
- Interfaces provided:
  - One canonical buffered-read entry point that serves regular-file `InodeIo::read_at` and consumes the accepted placement boundary instead of remapping file contents.
  - One narrow inode buffered-read facts accessor, only if needed, that extends the `EXR-READ-11A` read view with the visible file length needed to separate EOF from zero-fill.
  - One read-side zero-fill path that fills the unread tail of a request without taking over backend ownership or write-side growth.
- Files or modules touched:
  - `kernel/src/fs/fs_impls/exfat_refactor/read.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- Hidden implementation details:
  - Whether the buffered-read entry point lives beside the placement helper in `read.rs` or is dispatched from the inode trait implementation into a private helper.
  - Whether the inode buffered-read facts accessor returns a dedicated immutable bundle or a borrow-only view that extends the `EXR-READ-11A` facts.
  - Whether zero-fill is applied while assembling page-backed reads or immediately after the initialized prefix has been copied, provided the visible bytes satisfy the same contract.

## Functional Specification

### Operation

- Name: buffered regular-file read with zero-fill
- Inputs:
  - `inode_buffered_read_view`
  - `offset`
  - read destination or writer sink
  - page-cache-backed file access provided through the accepted backend contract
- Preconditions:
  - The inode view already represents an accepted existing regular file.
  - The inode view exposes both the initialized-data limit and the visible file length.
  - `EXR-READ-11A` has already accepted the placement boundary for the initialized prefix.
  - `EXR-PGCACHE-11B` has already provided the page-cache backend contract used for page-level I/O.
- Actions:
  - If `offset` is at or beyond `data_length`, return EOF immediately.
  - If `offset` is inside the initialized prefix, use the accepted placement boundary to fetch the readable bytes for that prefix.
  - If the request crosses from initialized data into the zero-fill range, fill the tail with zeros rather than trying to remap file contents.
  - If `offset` starts inside the zero-fill range but still before `data_length`, return zeros for the readable span until EOF.
  - Never re-derive cluster placement, and never claim page-cache backend ownership.
- Outputs:
  - `Result<usize>` or the equivalent number of bytes made visible to the caller.
- Postconditions:
  - Bytes in the initialized prefix are read from the accepted placement boundary.
  - Bytes in `[valid_data_length, data_length)` are zero-filled when visible to the caller.
  - Bytes at or beyond `data_length` are not exposed and terminate the read as EOF.
  - The component does not mutate allocation size, valid-data length, mount state, or backend ownership.

## Invariants

- `data_length` is the visible EOF boundary for buffered reads.
- `valid_data_length` is the upper bound for placement-backed reads.
- Zero-fill is used for unread file contents inside the visible file length, not as a substitute for growth.
- Buffered reads consume the placement result from `EXR-READ-11A`; they do not recompute it.
- Directory shells remain rejected at the inode boundary and do not become buffered-read inputs.
- This component does not publish, own, or reconfigure the page-cache backend.

## Concurrency Specification

- Shared state:
  - Mount-owned filesystem state.
  - Immutable inode buffered-read facts.
  - The accepted page-cache backend contract.
- Lock ordering:
  - No new lock ordering is introduced here.
  - If the buffered-read path needs page-cache or inode locking, it must follow the backend owner's established order and must not hold a read-path lock across backend I/O.
- Atomicity requirements:
  - The read contract is read-only from the filesystem point of view.
  - The buffered-read path may observe concurrent state only through the backend contract already owned by `EXR-PGCACHE-11B`.
- Forbidden interleavings:
  - No mount publication.
  - No logical-to-physical remapping.
  - No backend ownership takeover.
  - No allocation growth or truncation.
- Allowed simplifications such as a temporary big lock:
  - No separate async artifact is needed.
  - Any residual serialization assumption is inherited from the page-cache backend contract and does not need a new component-local protocol.

## Pass Split

### Serial Creator Pass

- Required implementation obligations:
  - Add one canonical buffered-read entry point for regular files.
  - Add the smallest inode buffered-read facts accessor needed to expose `data_length` alongside the `EXR-READ-11A` placement facts.
  - Use the accepted placement boundary for initialized data instead of re-running mapping logic.
  - Apply read-side zero-fill for the visible but uninitialized tail of the file.
- Explicit non-goals:
  - No mapping helper redesign.
  - No page-cache backend ownership.
  - No write-side growth, truncate, or namespace work.

### Serial Checker Pass

- Required checker-owned tests:
  - A contiguous-file buffered-read regression that proves the read path returns the expected initialized bytes through the accepted placement boundary.
  - A FAT-backed buffered-read regression that proves the read path still returns the expected bytes when the initialized prefix is placement-backed rather than contiguous.
  - A zero-fill regression that proves bytes in the visible file range beyond `valid_data_length` are returned as zeros rather than as stale data or an error.
  - An EOF regression that proves offsets at or beyond `data_length` return no bytes.
- Observable properties that must pass before leaving the serial loop:
  - Buffered reads copy initialized data and zero-fill only the unread visible tail.
  - The read path does not re-derive placement or claim backend ownership.
  - The tests validate caller-visible bytes, not internal page-cache implementation details.

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
  - None; this component is a read execution policy layered over already-owned placement and backend contracts.

## Acceptance Notes

- Keep the inode boundary narrow: the buffered-read facts accessor is justified only because `EXR-READ-11B` needs `data_length` in addition to the `EXR-READ-11A` placement facts.
- Keep zero-fill separate from growth. If the design starts extending the file to satisfy a read, it has crossed into write-side behavior.
- The serial contract is sufficient here, so `02_designer_async.md` is intentionally omitted.
- If the buffered-read path starts needing its own backend abstraction, the boundary has drifted into `EXR-PGCACHE-11B`.
