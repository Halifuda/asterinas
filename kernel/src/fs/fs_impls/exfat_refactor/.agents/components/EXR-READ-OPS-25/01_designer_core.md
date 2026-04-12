<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Core

## Metadata

- Component ID: `EXR-READ-OPS-25`
- Title: `ExfatInode` Buffered Regular-File Read Path
- Status: `Specified`
- Author: designer
- Date: `2026-04-12`
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-READ-OPS-25/20260412-1110-designer-packet.md`
- Based on architect artifact: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-READ-OPS-25/00_architect.md`

## Scope

- In scope:
  - Define the smallest owner-method buffered `read_at` path on `ExfatInode`.
  - Consume the current `EXR-FILE-MAP-24` mapping helper contract, including the temporary explicit `&dyn BlockDevice` and `&ExfatSuperBlock` traversal-context arguments.
  - Define where physically backed copying stops, where EOF truncation applies, and where valid-size zero-fill begins.
  - Define the byte-count contract returned to the caller-provided `VmWriter`.
  - Keep helper shape owner-private to `ExfatInode` in `inode.rs`.
- Out of scope:
  - Reopening logical-to-physical mapping ownership from `EXR-FILE-MAP-24`.
  - Page-cache ownership, cache coordination, or cache-backed read services.
  - Directory behavior, namespace mutation, write-side growth, truncate, allocator mutation, and sync ordering.
  - A filesystem-global reader or any public read helper outside `ExfatInode`.

## Module Specification

- Dependencies:
  - `EXR-INODE-CORE-17` for the stable `ExfatInode` carrier.
  - `EXR-FILE-MAP-24` for owner-private mapping output and temporary traversal-context shape.
  - VFS `InodeIo::read_at` and `VmWriter`.
  - Filesystem-owned traversal context reached through the inode owner boundary.
- Interfaces provided:
  - `ExfatInode::read_at` as the stable buffered regular-file read entry point.
  - Owner-private helpers inside `inode.rs` that:
    - decide the readable slice for one iteration,
    - copy one physically backed span into the caller writer,
    - and account for any zero-fill tail between `valid_size` and logical EOF.
- Files or modules touched:
  - Primary landing: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- Hidden implementation details:
  - Whether the read loop is expressed inline in `read_at` or through one or two owner-private helpers.
  - Whether the temporary traversal context is sourced once per call or threaded through narrower local helpers.
  - Exact helper names, so long as the owner remains `ExfatInode`.

## Functional Specification

### Read Eligibility

- Preconditions:
  - The current inode is a regular file.
  - The caller provides a writable `VmWriter`.
- Actions:
  - Reject non-regular-file reads through the existing inode-visible error path instead of inventing partial directory semantics.
  - Treat zero-length reads as successful no-op reads.
- Postconditions:
  - The component remains the regular-file buffered read surface on `ExfatInode`.

### Operation

- Name: `ExfatInode::read_at`
- Inputs:
  - A logical byte offset.
  - A caller-owned `VmWriter`.
  - VFS status flags, if required by the existing trait shape.
- Preconditions:
  - The inode snapshot already contains trusted size, valid-size, allocation, and chain facts.
  - The mapping helper contract from `EXR-FILE-MAP-24` is available.
- Actions:
  - Stop immediately with `0` when the request starts at or beyond logical EOF.
  - Obtain the temporary traversal context needed by the current mapping helper shape without widening that temporary seam into a new owner.
  - Repeatedly ask the mapping layer for the next physically backed range starting at the current logical offset.
  - For each mapped range, read the reported physical byte span and copy it into the caller writer.
  - Stop the physically backed copy loop when:
    - the writer has no remaining capacity,
    - the request reaches logical EOF,
    - or the current logical offset reaches `valid_size`, at which point physical copying ends.
  - If the request still has remaining logical bytes after physical copying and the current logical offset is below logical EOF but at or beyond `valid_size`, synthesize zero bytes into the caller writer for the bounded valid-size gap.
  - Return the total number of bytes made visible to the caller, counting both copied bytes and zero-filled bytes.
- Outputs:
  - The number of bytes read into the writer.
- Postconditions:
  - Read-visible EOF and short-read behavior live here rather than in the mapping layer.
  - The mapping layer stays a translation-only dependency.
  - Zero-fill is applied only inside the logical file size and only after physically backed bytes end.

### Mapping Consumption

- Inputs:
  - `PhysicalFileRange` from `EXR-FILE-MAP-24`.
  - Temporary explicit `&dyn BlockDevice` and `&ExfatSuperBlock` traversal-context arguments currently required by `map_physical_file_range()`.
- Preconditions:
  - The current mapping helper shape is accepted as a temporary dependency contract, not as a permanent second owner.
- Actions:
  - Treat `PhysicalFileRange` as one physically backed slice descriptor.
  - Consume its physical offset and bounded byte count to drive one read-copy iteration.
  - Do not reinterpret `PhysicalFileRange` as owning EOF, short-read, zero-fill, or retry policy.
- Postconditions:
  - `EXR-FILE-MAP-24` remains a subordinate translation layer.
  - `EXR-READ-OPS-25` becomes the first owner of user-visible byte-stream semantics.

### EOF, Short-Read, And Zero-Fill Rules

- EOF:
  - Logical EOF is `self.size()`.
  - Reads starting at or beyond logical EOF return `0`.
  - Reads that begin before EOF are truncated to the remaining logical file size.
- Short-read:
  - Return the number of bytes already copied or zero-filled if the request is satisfied only partially because it hit EOF or writer capacity.
  - Do not invent a retry loop, cache owner, or prefetch owner.
- Valid-size zero-fill:
  - The zero-fill region begins at `self.valid_size` and ends at logical EOF.
  - Zero-fill is visible only when the logical request extends into that region.
  - Zero-fill does not grant `EXR-FILE-MAP-24` ownership of unbacked bytes; it is purely read-path presentation policy on `ExfatInode`.

## Invariants

- `read_at` remains an `ExfatInode` method and does not become a filesystem-global reader.
- `EXR-FILE-MAP-24` remains responsible only for translation and physically backed span derivation.
- Logical EOF, short-read accounting, and valid-size zero-fill belong to `EXR-READ-OPS-25`.
- The helper surface remains owner-private in `inode.rs`.
- No helper in this row introduces page-cache ownership, write-side mutation, or allocator policy.
- Repeated reads on the same inode snapshot and the same logical request produce the same visible byte stream.

## Concurrency Specification

- Shared state:
  - The immutable inode snapshot carried by `ExfatInode`.
  - Filesystem-owned traversal context reached through the inode back-reference.
  - The caller-owned `VmWriter`.
- Lock ordering:
  - No new lock hierarchy is introduced here.
  - If the implementation needs an owner-local sequencing point around mapping plus byte-copy, that sequencing belongs entirely inside the read call and must not escape as a new shared lock owner.
  - Do not hold any future cache or write-side mutation guard while reading physical bytes for this row.
- Atomicity requirements:
  - One `read_at` call should observe one coherent inode snapshot.
  - The returned byte count must match the exact bytes copied plus zero-filled for that call.
  - Repeated calls on the same snapshot and offset must remain deterministic.
- Forbidden interleavings:
  - Do not let the mapping helper retain hidden mutable state across iterations.
  - Do not publish a read cursor, page-cache state, or shared temporary buffer outside the call.
  - Do not combine buffered read sequencing with write-side growth or truncate policy.
- Allowed simplifications:
  - Per-call traversal-context lookup is acceptable.
  - Per-iteration mapping plus copy is acceptable even if the call spans multiple clusters.
  - A temporary local zero buffer is acceptable if it remains call-local and subordinate to `ExfatInode`.

## Pass Split

### Serial Creator Pass

- Required implementation obligations:
  - Replace the temporary `read_at` rejection with a real buffered regular-file read path in `inode.rs`.
  - Consume `map_physical_file_range()` as the current translation boundary.
  - Copy physically backed bytes into `VmWriter`.
  - Apply EOF truncation, short-read accounting, and valid-size zero-fill in the inode owner.
  - Keep any new helpers owner-private to `ExfatInode`.
- Explicit non-goals:
  - No page-cache integration.
  - No public read service.
  - No write-side or allocator behavior.
  - No mapping-layer redesign.

### Serial Checker Pass

- Required checker-owned tests:
  - A regression that confirms `read_at` copies physically backed bytes for a regular file and stops at EOF.
  - A regression that confirms reads spanning past `valid_size` return copied bytes followed by zero-filled bytes within logical EOF.
  - A regression that confirms reads starting at or beyond logical EOF return `0`.
  - A regression that confirms repeated calls on the same snapshot return the same byte stream and byte count.
- Observable properties that must pass before leaving the serial loop:
  - Read-visible policy lives in `ExfatInode`.
  - Mapping remains translation-only.
  - No page-cache or write-side ownership appears in the implementation.

### Concurrency Creator Pass

- Required implementation obligations:
  - No dedicated concurrency implementation beyond the per-call buffered read loop described above.
- Explicit non-goals:
  - No shared read cursor.
  - No cache coordination owner.
  - No background read buffering service.

### Concurrency Checker Pass

- Required checker-owned concurrency tests:
  - No dedicated concurrency-only tests are required beyond repeated-call determinism on one stable snapshot.
- Observable properties that must pass before leaving the concurrency loop:
  - The component remains a call-local `ExfatInode` read path with no extra concurrency machinery.

## Acceptance Notes

- Reviewers should confirm that `read_at` stays on `ExfatInode` and does not become a filesystem-global reader or page-cache shell.
- Reviewers should confirm that EOF, short-read, and valid-size zero-fill ownership begin here and are not pushed back into `EXR-FILE-MAP-24`.
- Reviewers should confirm that the current explicit traversal-context arguments are consumed as a temporary dependency contract rather than widened into a permanent second owner.
- Reviewers should reject any implementation that folds write-side growth, truncate, allocator mutation, or sync ordering into this row.
- Creator work should be treated as shared-file work in `inode.rs`; fake parallel lanes inside the same helper region should be avoided.
