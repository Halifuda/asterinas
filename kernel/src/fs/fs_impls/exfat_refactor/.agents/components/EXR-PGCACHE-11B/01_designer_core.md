<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Core

## Metadata

- Component ID: `EXR-PGCACHE-11B`
- Title: Page-Cache Backend Integration For Regular Files
- Status: `Specified`
- Author: `designer`
- Date: `2026-04-05`
- Task packet: `EXR-PGCACHE-11B-DESIGN-20260405-1134`
- Based on architect artifact: `00_architect.md`

## Scope

- In scope:
  - A single canonical page-cache backend surface for refactored exFAT regular files.
  - Page-level read and write hooks that service already-accepted regular-file data.
  - Cache-size coordination from the file's visible length, not from allocated length.
  - Use of the accepted `EXR-READ-11A` placement boundary rather than re-deriving physical mapping.
  - A regular-file runtime attachment that owns the `PageCache` and keeps backend ownership narrow.
- Out of scope:
  - Buffered `read_at` and read-side zero-fill policy, which remain with `EXR-READ-11B`.
  - Write-side growth, truncation, or allocation splicing.
  - Directory lookup, namespace mutation, rename policy, or mount bootstrap.
  - A second mount path or any helper that reopens filesystem state.

## Module Specification

- Dependencies:
  - `EXR-MOUNT-09`
  - `EXR-INODE-05B`
  - `EXR-READ-11A`
- Interfaces provided:
  - One canonical `PageCacheBackend` implementation for the mounted exFAT filesystem object.
  - One regular-file runtime attachment that owns the `PageCache` and borrows mount-owned state.
  - One page-count and cache-capacity rule derived from `valid_data_length` rather than `data_length`.
- Files or modules touched:
  - `kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- Hidden implementation details:
  - Whether the backend trait is implemented directly on `ExfatFs` or on a small private backend carrier that `ExfatFs` owns.
  - Whether the regular-file runtime object keeps `PageCache` inline or inside a private runtime struct.
  - Whether cache capacity is fixed at construction or adjusted through a narrow size-coordination helper, provided the visible-length rule stays the same.

## Functional Specification

### Operation

- Name: attach page-cache backend to a regular file
- Inputs:
  - mount-owned filesystem state
  - accepted regular-file inode metadata from `EXR-INODE-05B`
  - accepted physical-placement facts from `EXR-READ-11A`
- Preconditions:
  - The inode already represents an accepted existing regular file.
  - The mount object already owns the shared filesystem state.
  - The read-mapping boundary already exists and is authoritative for logical-to-physical placement.
- Actions:
  - Create the regular-file runtime object with a page cache owned by that object.
  - Bind the runtime object to the mounted filesystem state without reopening or rediscovering anything.
  - Size the page cache from the file's visible length, not the allocated length.
  - Keep the backend surface narrow enough that buffered read policy does not move into this component.
- Outputs:
  - A backend-ready regular-file runtime object or equivalent attachment surface.
- Postconditions:
  - The live inode/file object can service page-cache requests without owning mount bootstrap or buffered read policy.
  - The page-cache backend remains the canonical place for page-level I/O only.

### Operation

- Name: service page-cache read and write requests
- Inputs:
  - `page_index`
  - cached page frame
  - mounted filesystem state
  - regular-file runtime object
- Preconditions:
  - The request targets a regular file that already has backend ownership attached.
  - The request is page-level, not buffered `read_at`.
- Actions:
  - If `page_index` is below the backend page count, translate the page through `EXR-READ-11A` and issue the corresponding block-device I/O.
  - If `page_index` is at or beyond the backend page count, do not invent disk I/O for this component; let the page cache's zero-page behavior handle the gap.
  - Keep reads and writes within the accepted backend contract only.
  - Avoid rewalking the FAT or re-deriving the placement policy here.
- Outputs:
  - The normal `PageCacheBackend` completion handle for the requested page.
- Postconditions:
  - Page-cache reads and writes stay within the accepted visible-length boundary.
  - Physical placement still comes from `EXR-READ-11A`, not from a second mapping helper.

### Operation

- Name: compute backend page count
- Inputs:
  - `valid_data_length`
  - page size
- Preconditions:
  - `valid_data_length` already came from the accepted inode metadata boundary.
- Actions:
  - Round the visible length up to the next page boundary.
  - Use that rounded page count as the backend-visible range.
  - Do not substitute allocated length or chain length.
- Outputs:
  - The backend page count and initial cache-capacity expectation.
- Postconditions:
  - Pages beyond that count are outside the backend-visible range.
  - Cache sizing remains aligned with the file's readable length.

## Invariants

- Backend page count is derived from visible length, not allocated length.
- The backend does not own buffered `read_at`, zero-fill policy, or page-copy policy.
- No second mount path is introduced.
- Any physical placement comes from `EXR-READ-11A`.
- The regular-file runtime object owns the page cache; the mount object does not become a cache manager.
- Directory shells and namespace behavior stay outside this component.

## Concurrency Specification

- Shared state:
  - Mounted filesystem state borrowed from `EXR-MOUNT-09`.
  - Immutable inode facts borrowed from `EXR-INODE-05B`.
  - The page cache owned by the regular-file runtime object.
- Lock ordering:
  - Do not hold a filesystem-global lock across block-device I/O.
  - Do not call back into page-cache APIs while holding a backend-local state lock.
  - Keep any size-coordination helper narrower than the backend I/O path itself.
- Atomicity requirements:
  - Cache-size coordination must use one visible-length snapshot at a time.
  - Page-level I/O is only as atomic as the existing `PageCacheBackend` and block-device contracts allow.
- Forbidden interleavings:
  - No buffered `read_at`, no write-side growth, and no mount bootstrap inside the backend path.
  - No backend helper may reopen the filesystem to recover state that is already owned by the mount object.
- Allowed simplifications such as a temporary big lock:
  - No separate async artifact is needed.
  - The existing page-cache manager already provides the scheduling boundary; this component only supplies the backend contract that it calls.

## Pass Split

### Serial Creator Pass

- Required implementation obligations:
  - Add the canonical backend surface on the mounted exFAT filesystem object.
  - Attach a regular-file page cache to the live inode/runtime object.
  - Keep backend page count and cache capacity aligned to visible length.
  - Route page-level I/O through `EXR-READ-11A` placement.
  - Keep buffered `read_at` and zero-fill policy out of this component.
- Explicit non-goals:
  - No buffered `read_at`.
  - No write-side growth, truncation, or allocation splicing.
  - No directory lookup, rename, or namespace mutation.
  - No second mount path or bootstrap helper.

### Serial Checker Pass

- Required checker-owned tests:
  - A regular-file regression that proves backend page count tracks visible length rather than allocated length.
  - A contiguous-file regression that proves page-level reads use the accepted `EXR-READ-11A` placement boundary.
  - A FAT-backed regression that proves page-level reads still use `EXR-READ-11A` rather than a new mapping shortcut.
  - A past-EOF regression that proves pages at or beyond the backend-visible range are left to the page cache's zero-page behavior.
  - A backend-scope regression that proves buffered `read_at` and write-side growth are not required for this component's contract.
- Observable properties that must pass before leaving the serial loop:
  - The backend reports the expected page count from visible length.
  - Page-level I/O is routed through the accepted placement boundary.
  - Out-of-range pages do not force this component to invent disk I/O.

### Concurrency Creator Pass

- Required implementation obligations:
  - None beyond the single backend scheduling boundary already recorded above.
- Explicit non-goals:
  - No new publication protocol.
  - No shared mutable cache ownership outside the regular-file runtime object.

### Concurrency Checker Pass

- Required checker-owned concurrency tests:
  - No dedicated concurrency tests required.
- Observable properties that must pass before leaving the concurrency loop:
  - None; the existing page-cache manager already owns scheduling and this component only provides the backend contract.

## Acceptance Notes

- Keep the backend surface singular. If the design starts wanting both a filesystem-level helper and a second inode-level helper with overlapping semantics, the boundary is too broad.
- The later buffered-read component, `EXR-READ-11B`, still owns `read_at` copy policy and read-side zero-fill behavior.
- `02_designer_async.md` is intentionally omitted because this component does not add a new async or publication protocol beyond the existing `PageCacheBackend` contract and the page-cache manager's own scheduling.
