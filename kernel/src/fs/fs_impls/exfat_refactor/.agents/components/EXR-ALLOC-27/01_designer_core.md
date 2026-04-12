<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Core

## Metadata

- Component ID: `EXR-ALLOC-27`
- Title: `ExfatFs` Cluster Allocation Service Boundary
- Status: `Specified`
- Author: designer
- Date: `2026-04-12`
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-ALLOC-27/20260412-1202-designer-packet.md`
- Based on architect artifact: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-ALLOC-27/00_architect.md`

## Scope

- In scope:
  - Define `Allocator` as an `ExfatFs`-owned internal service for free-space search, reservation intent, and bitmap/FAT commit coordination.
  - Specify the owner-internal state needed to search published allocation facts without turning allocation into a standalone manager.
  - Define the stable result shape later namespace and write owners will consume.
  - Define the boundary between candidate search, reservation, and on-disk mutation.
  - Keep later dentry publication, file-growth policy, truncate semantics, and sync ordering out of this component.
- Out of scope:
  - Directory-entry writes or namespace publication.
  - Inode-local allocation helpers or file-size policy.
  - Truncate/free semantics or filesystem-wide sync ordering.
  - A public allocator crate or a free-standing reservation manager.

## Module Specification

- Dependencies:
  - `EXR-BITMAP-21` for read-only allocation facts and occupancy/accounting queries.
  - `EXR-FATVAL-03A` and `EXR-CHAIN-03B` for FAT value decoding and chain shape.
  - `ExfatSuperBlock` geometry and cluster validation.
  - The `ExfatFs` owner boundary that already serializes bitmap publication.
- Interfaces provided:
  - An owner-private `Allocator` service under `ExfatFs`.
  - An owner-private reservation/result type that later namespace and write rows can consume.
  - A commit path that coordinates allocation bitmap updates with FAT mutation when fragmentation requires it.
- Files or modules touched:
  - Primary landing: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/allocator.rs`
  - Owner wiring: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- Hidden implementation details:
  - Whether `Allocator` is stored directly on `ExfatFs` or constructed as a lightweight owner-private helper per call.
  - Whether the allocator keeps a search hint or reuses the filesystem's existing allocation cursor fields.
  - Exact helper names, so long as the owner remains `ExfatFs`.

## Functional Specification

### Owner Boundary

- `Allocator` is a filesystem-wide mutable state helper, not an inode helper.
- It searches only the published allocation bitmap snapshot and normalized superblock geometry.
- It does not discover directory entries, publish names, or decide file-size policy.
- It does not own bitmap state, FAT value decoding, or directory write semantics.

### Search And Reservation

- Search phase:
  - Scan the published bitmap for a free candidate run starting from the current owner hint or the caller-provided allocation request.
  - Prefer a contiguous run when one is available.
  - Fall back to a fragmented run only when a contiguous run cannot satisfy the request.
- Reservation intent:
  - Record the chosen candidate in an owner-private reservation object before any on-disk mutation begins.
  - Keep the reservation object internal to `ExfatFs`; it is not a public free-space lease.
  - Do not expose the reservation as a cross-owner lock or as a long-lived background handle.
- Stable result shape:
  - Publish a small copyable result that later namespace and write owners can consume.
  - The result should carry only the facts they need: `start_cluster`, `cluster_count`, and `chain_mode`.
  - Later owners may derive directory-entry size, stream flags, and follow-up write policy from that result, but they must not need bitmap internals.

### Bitmap/FAT Commit Handshake

- Commit phase:
  - Reserve first, then mutate bitmap and FAT state under the same filesystem-owner serialization boundary.
  - Treat bitmap updates and FAT writes as one owner-local commit sequence, not as separately visible public phases.
  - Publish the stable allocation result only after the commit succeeds.
- Contiguous allocation:
  - When the candidate run is contiguous, the allocator may commit the allocation bitmap without introducing a FAT chain.
  - The stable result should carry `ChainMode::Contiguous` so later owners know the run is arithmetic.
- Fragmented allocation:
  - When the candidate run is fragmented, the allocator must materialize the FAT chain as part of the same commit sequence.
  - The stable result should carry `ChainMode::FatBacked` so later owners know the run must be traversed through FAT links.
- Failure handling:
  - If the commit fails, no partial allocation result may escape to later owners.
  - The reservation object is discarded and the filesystem remains in its previous published state.

## Invariants

- `Allocator` remains owner-internal to `ExfatFs`.
- Allocation search consults published bitmap facts; it does not redefine bitmap ownership.
- Reservation intent is internal and temporary; it is not a public lease or a standalone manager API.
- The stable result consumed by later owners is intentionally small and copyable.
- Directory-entry publication, inode growth, truncate policy, and sync ordering remain outside the allocator boundary.
- Fragmentation changes chain mode, but it does not change owner boundaries.

## Concurrency Specification

- Shared state:
  - The published allocation bitmap snapshot owned by `ExfatFs`.
  - The allocator's filesystem-wide search hint, if one is retained.
  - The transient reservation/result state created during one allocation request.
- Lock ordering:
  - Search may consult bitmap state only while inside the filesystem-owner serialization boundary.
  - Commit must keep bitmap and FAT mutation inside the same owner-local critical section.
  - Do not introduce a separate reservation lock that outlives the allocator call.
- Atomicity requirements:
  - A caller should observe either no allocation result or one fully committed result.
  - The bitmap and FAT must not diverge in the published state.
  - Later namespace and write owners may only consume a committed result.
- Forbidden interleavings:
  - Do not let directory publication observe a half-committed reservation.
  - Do not split bitmap flipping from FAT mutation into independently publishable steps.
  - Do not add inode-local allocation state or a hidden background manager.
- Allowed simplifications:
  - A single `ExfatFs` owner lock is sufficient for this component.
  - Per-call reservation construction is acceptable.
  - A small owner-private search hint is acceptable if it stays inside `ExfatFs`.

## Pass Split

### Serial Creator Pass

- Required implementation obligations:
  - Add the owner-internal `Allocator` state under `ExfatFs`.
  - Implement free-space search over the published bitmap snapshot.
  - Record reservation intent before mutation.
  - Commit bitmap and FAT changes as one owner-local sequence.
  - Return a stable result shape for later namespace and write owners.
- Explicit non-goals:
  - No directory-entry writes.
  - No inode growth policy.
  - No truncate/free policy.
  - No sync ordering.

### Serial Checker Pass

- Required checker-owned tests:
  - Verify free-space search finds a contiguous run when one exists.
  - Verify fragmented allocation is selected only when contiguous space is insufficient.
  - Verify reservation intent does not escape before commit.
  - Verify bitmap and FAT state remain coherent after commit.
- Observable properties that must pass before leaving the serial loop:
  - Search, reservation, and commit stay inside `ExfatFs`.
  - Later consumers receive only the stable committed result.
  - No partial allocation state is published.

### Concurrency Creator Pass

- Required implementation obligations:
  - No dedicated async protocol is required beyond the owner-local serialization boundary already described here.
- Explicit non-goals:
  - No background allocator thread.
  - No deferred reservation publication.
  - No cross-call reservation lease.

### Concurrency Checker Pass

- Required checker-owned concurrency tests:
  - No dedicated concurrency tests are required for this component.
- Observable properties that must pass before leaving the concurrency loop:
  - Allocation remains serialized through `ExfatFs`.
  - A reservation is either fully committed or not visible at all.

## Acceptance Notes

- Reviewers should confirm that allocation is an `ExfatFs`-internal service, not a standalone free-space manager.
- Reviewers should confirm that the stable result shape is small enough for later namespace and write owners to consume directly.
- Reviewers should reject any design that moves directory publication, inode growth, truncate policy, or sync ordering into this row.
- Reviewers should confirm that bitmap search, reservation intent, and commit are distinct steps with a single owner boundary.

