<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Core

## Metadata

- Component ID: `EXR-DENTRY-WRITE-28`
- Title: `DirectoryEngine` Write-Side Directory Entry Mutation
- Status: `Specified`
- Author: designer
- Date: 2026-04-12
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-DENTRY-WRITE-28/20260412-2049-designer-packet.md`
- Based on architect artifact: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DENTRY-WRITE-28/00_architect.md`

## Scope

- In scope:
  - Define `DirectoryEngine` as an `ExfatFs`-internal write-side owner for directory-entry mutation.
  - Consume validated `ExfatDentrySet` values as the file-record boundary.
  - Consume committed allocation results from `EXR-ALLOC-27` only as already-decided growth facts.
  - Specify slot discovery, record placement and removal, overwrite rules, tombstoning, and directory-side handling when a write cannot stay in place.
  - Keep the landing zone in `kernel/src/fs/fs_impls/exfat_refactor/directory.rs`.
- Out of scope:
  - Namespace policy, name canonicalization, or inode publication.
  - Allocation search, reservation intent, or reservation visibility.
  - Sync ordering, writeback policy, or a standalone directory-write manager.
  - Reworking the read-only record stream beyond what the write methods need as a foundation.

## Module Specification

- Dependencies:
  - `EXR-DIR-ENGINE-19` for the existing `DirectoryEngine` owner and directory scan state.
  - `EXR-FILESET-04B` for the validated file-record boundary and serialized set bytes.
  - `EXR-ALLOC-27` for committed allocation results used only when directory growth is already decided.
  - `DirectoryRecordLocation` and related owner-private location facts for write placement.
  - `ExfatChain` and `ExfatSuperBlock` for directory-chain traversal and cluster sizing.
- Interfaces provided:
  - Owner-private write methods on `DirectoryEngine` that place, rewrite, remove, and tombstone validated directory records.
  - Owner-private helpers that discover a writable slot range inside one directory and map a committed allocation result onto a directory growth step.
  - A narrow relocation path for the case where a validated set does not fit in place but growth has already been committed upstream.
- Files or modules touched:
  - Primary landing: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs`
- Hidden implementation details:
  - Exact helper names and whether the write path reuses the existing scan cursor or keeps a separate owner-private placement cursor.
  - Whether in-place rewrite is expressed as a replacement helper or as a single write-or-tombstone helper.
  - How the owner-private location facts are updated after a relocation, so long as the logic stays inside `DirectoryEngine`.

## Functional Specification

### Write Boundary

- Precondition:
  - The caller already has a validated `ExfatDentrySet`.
  - The caller either targets an existing directory location or supplies a committed allocation result when growth is required.
- Action:
  - Discover the slot range needed for the validated set.
  - If the set fits in the current location, rewrite the existing record in place.
  - If the current location must be vacated, tombstone the old slot range before or as part of placing the new bytes, depending on the chosen owner-private write order.
  - If the set does not fit in place and the directory must grow, consume the committed allocation result and extend the directory chain without searching for free space again.
  - Serialize the validated set bytes exactly as provided by `EXR-FILESET-04B`.
- Postcondition:
  - The directory now contains the validated record bytes at the chosen slot range.
  - The write path has not reopened file-record validation, name-policy decisions, or allocation search.

### Slot Discovery And Placement

- Precondition:
  - The directory write target is already known to belong to the owning directory.
- Action:
  - Search only within the directory-local slot space needed for the write.
  - Prefer in-place placement when the current slot range can still represent the validated set.
  - Use tombstoned or otherwise reusable directory space before extending the chain.
  - Treat committed allocation results as a fixed growth fact, not as a search input.
- Postcondition:
  - The chosen slot range is stable enough for later namespace consumers to reference by location.

### Overwrite And Tombstone Rules

- Precondition:
  - The write path is replacing, removing, or relocating an existing validated record.
- Action:
  - Overwrite only directory-local bytes that belong to the record being mutated.
  - Tombstone stale record slots instead of leaving them as live entries.
  - Preserve the validated record shape supplied by `ExfatDentrySet`.
  - Keep checksum and serialized-record handling inside the fileset boundary, not here.
- Postcondition:
  - Old slots are either rewritten or tombstoned.
  - No stale live entry remains reachable as if it were a separate record.

### Growth Handling

- Precondition:
  - The validated set cannot remain in place.
  - A committed allocation result is available from `EXR-ALLOC-27`.
- Action:
  - Use the committed allocation result to make room for the directory-chain extension.
  - Do not perform allocation search, reservation, or commit inside this row.
  - Write the serialized set after the growth decision has already been made upstream.
- Postcondition:
  - Directory growth is a consumer of committed allocation facts, not a second allocator.

## Invariants

- `DirectoryEngine` remains an `ExfatFs`-internal service.
- `ExfatDentrySet` remains the validated file-record boundary.
- `EXR-ALLOC-27` remains the owner of search, reservation, and commit facts.
- Directory mutation stays subordinate to `DirectoryEngine`; it does not become a namespace service.
- The write path does not derive file-record validity, inode identity, or name policy from scratch.
- Any relocation that needs more room must consume a committed allocation result instead of re-running allocation logic.

## Concurrency Specification

- Shared state:
  - The owning directory-chain state inside `DirectoryEngine`.
  - The owner-private placement cursor or location facts, if retained.
- Lock ordering:
  - No new lock order is introduced by this component.
  - Any filesystem-wide serialization remains the responsibility of `ExfatFs`, not a separate write manager.
- Atomicity requirements:
  - A caller should observe either the old directory state or the fully written new record state, not a partially interpreted record shape.
  - A write that needs growth should consume a committed allocation result as one owner-local handoff.
- Forbidden interleavings:
  - No directory write should race with a background allocator or a deferred reservation publish queue.
  - No in-flight write should expose a half-updated namespace policy decision.
- Allowed simplifications:
  - One `ExfatFs` owner-local critical section is sufficient.
  - No dedicated async protocol is required for this component.

## Pass Split

### Serial Creator Pass

- Required implementation obligations:
  - Add the write-side `DirectoryEngine` helpers in `directory.rs`.
  - Implement slot discovery, in-place overwrite, tombstoning, and write placement for validated `ExfatDentrySet` values.
  - Add the directory-growth path that consumes a committed allocation result without re-owning allocation search.
  - Keep the helper surface owner-private and subordinate to `DirectoryEngine`.
- Explicit non-goals:
  - No namespace policy.
  - No inode publication.
  - No allocation search or reservation.
  - No separate write manager or sync layer.

### Serial Checker Pass

- Required checker-owned tests:
  - Verify slot reuse over tombstoned directory space.
  - Verify an in-place rewrite preserves the record location when the set still fits.
  - Verify directory growth uses a committed allocation result instead of running allocation search.
  - Verify the write path consumes a validated `ExfatDentrySet` and does not absorb namespace policy.
- Observable properties that must pass before leaving the serial loop:
  - Write placement remains inside `DirectoryEngine`.
  - Stale slots are tombstoned or rewritten, not left live.
  - Growth is a consumer of committed allocation facts only.

### Concurrency Creator Pass

- Required implementation obligations:
  - No dedicated async implementation is required beyond the owner-local serialization already described here.
- Explicit non-goals:
  - No background directory writer.
  - No deferred tombstone queue.
  - No cross-call reservation lease.

### Concurrency Checker Pass

- Required checker-owned concurrency tests:
  - No dedicated concurrency tests are required.
- Observable properties that must pass before leaving the concurrency loop:
  - Directory mutation remains serialized through `ExfatFs`.
  - No intermediate write state escapes as a public protocol.

## Acceptance Notes

- Reviewers should confirm that this row stays inside `DirectoryEngine` and does not become a standalone directory-write manager.
- Reviewers should confirm that `EXR-ALLOC-27` is only consumed through committed allocation results.
- Reviewers should reject any design that moves namespace policy, inode publication, or sync ordering into this boundary.
- This component has two expected creator slices in the same `directory.rs` landing zone, so the slices should be sequenced rather than treated as file-parallel work.
