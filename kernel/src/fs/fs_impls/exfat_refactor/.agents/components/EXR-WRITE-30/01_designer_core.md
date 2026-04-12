<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Core

## Metadata

- Component ID: `EXR-WRITE-30`
- Title: `ExfatInode` buffered write and size mutation
- Status: `Specified`
- Author: designer
- Date: `2026-04-12`
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-WRITE-30/20260412-2215-designer-packet.md`
- Based on architect artifact: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-WRITE-30/00_architect.md`

## Scope

- In scope:
  - Define `ExfatInode::write_at` as the inode-owned buffered regular-file write surface.
  - Define `ExfatInode::resize` as the inode-owned size-growth and truncate surface for regular files.
  - Consume `EXR-PGCACHE-26` as the inode-local cache boundary for write-visible bytes and cache sizing.
  - Consume `EXR-ALLOC-27` as the only committed-growth handoff when allocation coverage must expand.
  - Consume `EXR-FILE-MAP-24` when the write path needs logical-to-physical translation without reopening mapping ownership.
  - Define the narrow owner-private helper shape allowed inside `inode.rs`, including one inode-local mutable state holder if the current copied snapshot fields need mutation.
- Out of scope:
  - Direct-I/O write support or an `O_DIRECT` bypass path.
  - Filesystem-wide sync ordering, durable flush semantics, or page-cache writeback ownership, which remain downstream in `EXR-SYNC-31`.
  - Directory-entry mutation, namespace publication, or opened-inode coordination.
  - Allocation search, reservation intent, or a public deallocator facade.
  - A write manager, dirty-writeback service, or any filesystem-global coordination layer.

## Module Specification

- Dependencies:
  - `EXR-INODE-CORE-17` for the stable `ExfatInode` carrier and filesystem back-reference.
  - `EXR-FILE-MAP-24` for inode-private logical-to-physical translation when the write owner needs it.
  - `EXR-READ-OPS-25` for the already accepted read-visible EOF and valid-size zero-fill contract that write-side mutation must preserve.
  - `EXR-PGCACHE-26` for inode-local page-cache attachment, cache sizing, and page-backed visibility.
  - `EXR-ALLOC-27` for committed allocation results and the `ExfatFs::allocate_clusters()` owner-private wrapper.
  - VFS `InodeIo::write_at`, `Inode::resize`, `VmReader`, and `PageCache`.
- Interfaces provided:
  - `ExfatInode::write_at`
  - `ExfatInode::resize`
  - Owner-private helpers inside `inode.rs` that may:
    - hold mutable file-state facts for one inode,
    - compute additional cluster demand,
    - fold committed allocation results into the inode-owned snapshot,
    - zero-fill a valid-size gap before publishing a larger initialized range,
    - keep page-cache sizing and visible bytes coherent with the new inode state.
- Files or modules touched:
  - Primary landing: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
  - Narrow owner-consumed helper adjustments in `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs` are acceptable only if the existing `ExfatFs` wrapper surface needs a small extension to stay owner-first.
- Hidden implementation details:
  - Whether mutable inode file facts stay inline on `ExfatInode` behind one owner-private guard or move into one small inode-local state struct.
  - Whether post-write visibility is maintained by cache-backed owner-private reads, synchronous owner-private write placement, or a combination of the two, so long as the visible byte-stream contract remains on `ExfatInode`.
  - Exact helper names and how write and resize share them, provided they remain owner-private to `ExfatInode`.

## Functional Specification

### Write Eligibility

- Preconditions:
  - The current inode is a regular file.
  - The caller supplies a readable `VmReader`.
- Actions:
  - Reject directory or other non-regular writes through the existing inode-visible error path instead of inventing partial semantics.
  - Treat zero-length writes as successful no-op writes.
  - Keep `StatusFlags::O_DIRECT` out of scope for this row; preserve an explicit unsupported answer instead of silently inventing a direct-I/O path.
- Postconditions:
  - This row remains the buffered regular-file write surface on `ExfatInode`.

### Operation

- Name: `ExfatInode::write_at`
- Inputs:
  - A logical byte offset.
  - A caller-owned `VmReader`.
  - VFS status flags.
- Preconditions:
  - The inode already carries trusted size, valid-size, allocation, and chain facts.
  - The inode-local page cache is already attached for regular files.
- Actions:
  - Compute the write end with overflow checking and derive the final logical size for the call.
  - If the final logical size exceeds the current allocated coverage, request exactly the additional committed allocation facts needed through the `ExfatFs` owner boundary and fold the returned `AllocationResult` into the inode-owned chain and allocation snapshot before publishing the larger coverage.
  - Resize the inode-local page cache before publishing a larger logical EOF.
  - If the write begins beyond the current `valid_size`, zero-initialize the gap `[old_valid_size, offset)` in the same inode-owned visible byte source that later buffered reads and mappings will observe.
  - Copy the caller bytes through the inode-owned buffered write path.
  - Publish the new `size`, `valid_size`, allocation facts, and dirty timestamp state only after the gap and written bytes are visible for this call.
- Outputs:
  - The number of bytes written from the caller reader.
- Postconditions:
  - Successful writes are immediately visible through the inode-owned byte stream on the same inode snapshot.
  - The row consumes page cache and committed allocation results without re-homing either owner.
  - Durable writeback ordering still remains outside this component.

### Operation

- Name: `ExfatInode::resize`
- Inputs:
  - A new logical file size.
- Preconditions:
  - The current inode is a regular file.
- Actions:
  - Return success without mutation when `new_size` equals the current logical size.
  - For growth:
    - consume committed allocation results only when `new_size` exceeds current allocated coverage,
    - resize the inode-local page cache to `new_size`,
    - publish the larger logical size without advancing `valid_size` past the already initialized byte range.
  - For shrink or truncate:
    - clamp `size` and `valid_size` to `new_size`,
    - resize the inode-local page cache down to `new_size`,
    - release or detach clusters that are no longer owned only through owner-private helpers subordinate to `ExfatInode`; do not turn truncate into a public allocation manager.
  - Mark inode-owned timestamp or dirty-state updates needed later by `EXR-SYNC-31`.
- Outputs:
  - None.
- Postconditions:
  - Reads past the new EOF are no longer visible after shrink.
  - Grown-but-unwritten bytes remain zero-visible through the accepted read owner until a later write initializes them.
  - `resize` remains an inode-owned file mutation, not a sync or allocator service.

### Allowed Helper Shape

- Owner-private helpers may:
  - hold mutable file-state facts such as `size`, `valid_size`, `start_cluster`, `cluster_count`, `chain_mode`, and `allocated_size`,
  - compute how many additional clusters a growth request needs,
  - fold a committed `AllocationResult` into the inode snapshot,
  - zero-fill unwritten gaps before `valid_size` advances,
  - keep page-cache size and read-after-write visibility coherent.
- Owner-private helpers must not:
  - become a standalone write manager,
  - absorb allocation search or reservation,
  - introduce a deallocator or allocator facade that escapes `ExfatInode`,
  - define durable flush ordering,
  - create a background dirty-write queue.

## Invariants

- `write_at` and `resize` stay on `ExfatInode`, not on `ExfatFs` or a new writer service.
- `EXR-ALLOC-27` remains the only owner of allocation search, reservation intent, and commit; this row consumes only committed results.
- `EXR-PGCACHE-26` remains the inode-local cache owner; this row may resize or dirty that cache but does not re-home it.
- `EXR-READ-OPS-25` remains the owner of read-visible EOF, short-read, and valid-size zero-fill semantics; this row must preserve those semantics when it changes `size` or `valid_size`.
- Successful read-after-write visibility on one inode snapshot must stay coherent.
- `write_page_async()` and durable persistence ordering remain downstream to `EXR-SYNC-31`.
- No direct-I/O path or public write service is introduced here.

## Concurrency Specification

- Shared state:
  - The inode-owned mutable file-state snapshot.
  - The inode-local `PageCache`.
  - The filesystem-owned committed-allocation service reached through `ExfatFs`.
- Lock ordering:
  - If creator work introduces an inode-local mutation guard, it must serialize publication of `size`, `valid_size`, and allocation facts.
  - Do not hold a page-cache page guard or future sync guard while calling `ExfatFs::allocate_clusters()`.
  - Do not let truncate or growth keep a filesystem-global lock alive after the committed allocation facts have been consumed.
- Atomicity requirements:
  - A successful write publishes one coherent combination of visible bytes, `size`, `valid_size`, and allocation facts.
  - A successful resize publishes the new EOF only after page-cache sizing and inode-state updates for that call are ready.
  - Later readers must observe either the old byte stream or the fully applied new byte stream for one call, not a half-grown or half-truncated inode snapshot.
- Forbidden interleavings:
  - Do not advance `valid_size` before the gap `[old_valid_size, offset)` has become zero-visible when the write skips forward.
  - Do not publish a larger size before the required committed allocation result has been folded into the inode snapshot.
  - Do not let buffered write drift into a background flush queue or sync shell.
- Allowed simplifications:
  - One inode-local mutation guard is acceptable.
  - Synchronous allocation consumption is acceptable.
  - A small owner-private mutable state struct is acceptable if it keeps mutation local to `ExfatInode`.

## Pass Split

### Serial Creator Pass

- Required implementation obligations:
  - Replace the temporary `write_at` and `resize` rejections with real inode-owned buffered mutation in `inode.rs`.
  - Consume `ExfatFs::allocate_clusters()` only when growth exceeds current allocation coverage.
  - Keep page-cache sizing and visible bytes coherent with new inode state.
  - Zero-fill unwritten valid-size gaps before publishing them as initialized.
  - Keep any new helpers owner-private to `ExfatInode`.
- Explicit non-goals:
  - No `O_DIRECT` support.
  - No background writeback or dirty-flush protocol.
  - No directory or namespace mutation.
  - No public write or deallocation manager.
  - No sync ordering or durability policy.

### Serial Checker Pass

- Required checker-owned tests:
  - A regression that confirms a buffered write inside existing allocation updates the visible byte stream and returns the correct byte count.
  - A regression that confirms a write beginning beyond `valid_size` zero-fills the unwritten gap before the new bytes and updates `size` and `valid_size` coherently.
  - A regression that confirms `resize` shrink truncates the visible EOF and page-cache sizing.
  - A regression that confirms growth beyond current allocation coverage consumes committed allocation results only when needed and preserves zero-visible unwritten suffix bytes.
- Observable properties that must pass before leaving the serial loop:
  - Write-side mutation remains inode-owned.
  - Committed allocation results are the only growth handoff.
  - Page cache remains inode-local.
  - No sync-ordering or direct-I/O behavior appears in the row.

### Concurrency Creator Pass

- Required implementation obligations:
  - No dedicated async protocol is required beyond the inode-local serialization boundary described above.
  - Preserve post-write visibility without introducing a background queue or filesystem-global coordinator.
  - Leave `write_page_async()` as a downstream future-owner seam instead of absorbing `EXR-SYNC-31`.
- Explicit non-goals:
  - No background flush worker.
  - No deferred publish queue.
  - No standalone dirty-state coordinator.

### Concurrency Checker Pass

- Required checker-owned concurrency tests:
  - No dedicated concurrency-only tests are required for this component.
- Observable properties that must pass before leaving the concurrency loop:
  - The component remains a synchronous inode-owned mutation boundary.
  - No extra concurrency machinery appears beyond local serialization of one inode mutation call.

## Acceptance Notes

- Reviewers should confirm that buffered write and size mutation stay on `ExfatInode`.
- Reviewers should confirm that `EXR-ALLOC-27`, `EXR-PGCACHE-26`, and `EXR-READ-OPS-25` remain consumed owners rather than reimplemented services.
- Reviewers should confirm that valid-size gap handling preserves the already accepted read-visible zero-fill contract.
- Reviewers should reject any attempt to add a write manager, allocator facade, direct-I/O path, or sync shell.
- Creator work should be treated as shared-file work in `inode.rs`, not as fake parallel lanes against the same mutation state.
