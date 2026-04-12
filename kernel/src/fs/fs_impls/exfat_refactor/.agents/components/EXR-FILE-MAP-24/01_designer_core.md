<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Core

## Metadata

- Component ID: `EXR-FILE-MAP-24`
- Title: `ExfatInode` Read-Path Logical-To-Physical File Mapping
- Status: `Specified`
- Author: designer
- Date: `2026-04-11`
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-FILE-MAP-24/20260411-1613-designer-packet.md`
- Based on architect artifact: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FILE-MAP-24/00_architect.md`

## Scope

- In scope:
  - Define the smallest owner-private helper set on `ExfatInode` that translates a regular-file logical byte offset into a cluster position, in-cluster offset, and physically mappable byte span.
  - Consume the accepted `ExfatChain` walking boundary instead of inventing a new mapping owner.
  - Consume inode-owned size facts, valid-size facts, allocated-size facts, and cluster geometry as read-side inputs.
  - Keep the helper surface subordinate to `ExfatInode` in `inode.rs`.
- Out of scope:
  - Directory traversal, directory enumeration, and namespace mutation.
  - Mount/open sequencing, root publication, and opened-inode cache ownership.
  - Actual byte-copying, page-cache behavior, zero-fill policy, and EOF policy.
  - Allocation growth, truncate, write-side mutation, and dirty-state management.
  - A standalone mapping service or read shell that would replace the inode owner boundary.

## Module Specification

- Dependencies:
  - `EXR-INODE-CORE-17` for the stable `ExfatInode` owner and its copied inode facts.
  - `EXR-CHAIN-03B` for read-only cluster walking and logical-offset-to-chain-position helpers.
  - `ExfatFs` cluster geometry and superblock state reached through the inode owner.
  - The VFS `Inode` carrier context for later read-side callers.
- Interfaces provided:
  - Owner-private helper(s) inside `ExfatInode` that can:
    - reconstruct or consume the inode-owned chain state for a logical offset,
    - return the cluster containing that offset plus the in-cluster byte offset,
    - derive the physically mappable span for a logical request without performing the read itself.
  - A small private result shape is acceptable if it keeps the later reader from guessing about the helper boundary.
- Files or modules touched:
  - Primary landing: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- Hidden implementation details:
  - Whether the helper set is split into one offset-to-position helper plus one span helper, or a single helper with a small private return type.
  - Whether the offset helper reconstructs `ExfatChain` from inode-owned facts on demand or consumes a caller-supplied chain snapshot.
  - The exact local names, so long as the helpers remain owner-private to `ExfatInode` and do not become a separate service boundary.

## Functional Specification

### Offset Translation

- Inputs:
  - A regular-file logical byte offset.
  - The inode-owned chain facts copied into `ExfatInode`.
  - The filesystem cluster geometry.
- Preconditions:
  - The inode already represents a regular file.
  - The inode snapshot contains trusted chain identity facts and trusted size facts.
  - The cluster geometry is normalized and already owned by `ExfatFs`.
- Actions:
  - Use the inode-owned chain facts as the starting point for read-path translation.
  - Use accepted `ExfatChain` walking to advance to the cluster containing the requested logical offset.
  - Compute the in-cluster byte offset from the logical offset and the cluster size.
  - Preserve the chain boundary as read-only; do not expand it into a separate owner.
- Outputs:
  - The cluster position that contains the requested offset.
  - The byte offset within that cluster.
- Postconditions:
  - The helper returns a pure translation result and does not perform data transfer.
  - Repeated calls with the same inode snapshot and the same logical offset produce the same translation result.

### Physically Mappable Span

- Inputs:
  - A logical request offset and request length.
  - The inode-owned logical file size, valid size, and allocated-size facts.
  - The filesystem cluster geometry and the offset translation result.
- Preconditions:
  - The request is for a regular file and not for directory behavior.
  - The caller wants the maximal read-side span that is physically backed at the requested logical offset.
- Actions:
  - Bound the span by inode-owned file-size and valid-size facts.
  - Bound the span by the allocated on-disk region that the inode snapshot already owns.
  - Bound the span by cluster geometry so the later read caller can work one physically backed slice at a time.
  - Leave zero-fill and EOF policy to later read owners instead of resolving them here.
- Outputs:
  - The maximum physically mappable byte span for the request.
- Postconditions:
  - The helper can return zero when no physical bytes are mappable at the requested offset.
  - The helper never crosses into actual byte-copying, cache policy, or allocation growth.

### Boundary Consumption

- Inputs:
  - `ExfatChain` read-only walking support.
  - Inode-owned size facts copied from trusted construction inputs.
  - `ExfatFs` cluster geometry.
- Preconditions:
  - The inode already owns the copied regular-file metadata snapshot.
- Actions:
  - Treat `ExfatChain` as the accepted traversal boundary and not as a new owner.
  - Treat the inode snapshot as the source of truth for the file sizes used to cap mapping.
  - Treat `ExfatFs` as the source of cluster geometry only.
- Postconditions:
  - The unit remains a read-path translation layer for `ExfatInode`, not a file-data path.

## Invariants

- `ExfatInode` remains the final owner of this mapping layer.
- `ExfatChain` remains the accepted traversal boundary, not a promoted mapping service.
- The helper surface stays owner-private in `inode.rs`.
- No helper in this component performs byte-copying, zero-fill, page-cache access, or allocation mutation.
- The mapping layer can be called repeatedly on the same snapshot without changing inode state.
- The later read owner remains responsible for EOF policy, short-read policy, and any decision to synthesize bytes beyond the physically backed span.

## Concurrency Specification

- Shared state:
  - The immutable `ExfatInode` snapshot fields.
  - The borrowed filesystem geometry reached through `ExfatFs`.
  - The borrowed block-device and superblock context that later readers will already own when they call into the helpers.
- Lock ordering:
  - No new locks or mutexes are introduced by this component.
  - If a later caller serializes read-path work, that caller should acquire its own higher-level guard before invoking the helpers, then release it before any actual byte-copy or cache interaction.
- Atomicity requirements:
  - Each helper call should observe one coherent inode snapshot and one coherent cluster-geometry snapshot.
  - Repeated calls on the same snapshot must remain deterministic.
  - Multi-hop `ExfatChain` traversal remains read-only and can still surface external writer interference as a traversal error, as defined by the chain owner.
- Forbidden interleavings:
  - Do not share a mutable chain cursor across calls.
  - Do not hold read-path policy or page-cache state inside the mapping helper.
  - Do not couple logical-to-physical translation to allocation growth or write-side mutation.
- Allowed simplifications such as a temporary big lock:
  - A caller-local serialization guard is sufficient if a later read owner wants to keep its own mapping and copy phases ordered.
  - Per-call reconstruction of `ExfatChain` is acceptable if it keeps the inode boundary simple.

## Pass Split

### Serial Creator Pass

- Required implementation obligations:
  - Add the owner-private mapping helper set to `inode.rs`.
  - Reuse `ExfatChain` and inode-owned size facts instead of introducing a new mapping owner.
  - Keep the helpers crate-private or more restrictive unless a later caller is explicitly named in the packet.
  - Keep the landing zone inside `ExfatInode` and do not widen into read-loop, zero-fill, or page-cache behavior.
- Explicit non-goals:
  - No public mapping service.
  - No data copying.
  - No read-policy decision making.
  - No growth, truncate, or allocation mutation.
  - No directory or mount/open behavior.

### Serial Checker Pass

- Required checker-owned tests:
  - Verify that a logical offset resolves to the expected cluster position and in-cluster byte offset for a regular file.
  - Verify that the physically mappable span is bounded by the inode snapshot’s file-size, valid-size, and allocated-size facts.
  - Verify that a request crossing a cluster boundary is capped at the boundary the helper reports, rather than silently spilling into later copy logic.
  - Verify that repeated calls on the same inode snapshot and logical offset return the same mapping result.
- Observable properties that must pass before leaving the serial loop:
  - Mapping stays read-only.
  - The helper never claims zero-fill or EOF policy ownership.
  - The helper never mutates inode state or allocation state.

### Concurrency Creator Pass

- Required implementation obligations:
  - No dedicated concurrency implementation is required beyond the per-call, owner-private helper shape defined above.
- Explicit non-goals:
  - No new lock hierarchy.
  - No shared mapping cursor.
  - No cache publication boundary.

### Concurrency Checker Pass

- Required checker-owned concurrency tests:
  - No dedicated concurrency tests required.
- Observable properties that must pass before leaving the concurrency loop:
  - The mapping layer remains a deterministic inode-private translation helper set with no extra concurrency machinery.

## Acceptance Notes

- Reviewers should confirm that this component stays subordinate to `ExfatInode` and does not become a standalone read service.
- Reviewers should confirm that helper boundaries stop at address translation and physically mappable span derivation.
- Reviewers should reject any attempt to fold byte-copying, zero-fill, EOF policy, page cache, or allocator mutation into this row.
- The likely write-set collision is `inode.rs`, so any creator slices should be treated as shared-file work rather than fake parallel files.
