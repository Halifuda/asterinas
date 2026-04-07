<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Core

## Metadata

- Component ID: EXR-INODE-CACHE-18
- Title: ExfatFs Opened-Inode Table And Validated InodeKey
- Status: `Specified`
- Author: designer
- Date: 2026-04-07
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-INODE-CACHE-18/20260407-1048-designer-packet.md`
- Based on architect artifact: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CACHE-18/00_architect.md`

## Scope

- In scope:
  - Define the `ExfatFs`-owned opened-inode table as the canonical reuse point for non-root `Arc<ExfatInode>` handles.
  - Define `InodeKey` as a validated owner-private value type derived only from trusted directory-location facts.
  - Reserve a dedicated root special-case slot that is not encoded as an ordinary `InodeKey`.
  - Specify the ordinary lookup, reuse, insert, and remove behavior needed for handle sharing under `ExfatFs`.
  - Record the owner-serialization boundary needed to publish canonical handles without racing into duplicate inode shells.
- Out of scope:
  - Mount/open sequencing, directory traversal, inode metadata ownership, page-cache behavior, read/write behavior, and namespace mutation.
  - Any helper module whose only purpose would be to make `InodeKey` look like a free-standing utility instead of an owner-owned boundary.
  - Root publication wiring for the VFS `FileSystem` surface; that remains the later `EXR-FS-OPEN-22` handoff.
  - Any separate helper shell for root, stats, or inode ownership.

## Module Specification

- Dependencies:
  - `EXR-FS-CORE-16` for the filesystem owner boundary.
  - `EXR-INODE-CORE-17` for the inode carrier that will be reused by the table.
  - The validated directory-location facts already accepted for inode construction.
  - The VFS `Inode` contract, only as the consumer of stable `Arc<ExfatInode>` handles.
- Interfaces provided:
  - An owner-private opened-inode table owned by `ExfatFs`.
  - An owner-private `InodeKey` value type that is comparable, copyable, and usable only for opened-inode identity.
  - A distinct owner-private root slot that stays outside the ordinary keyspace.
- Files or modules touched:
  - Implementation landing zone: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
  - If later wiring requires a declaration edit in `mod.rs`, that edit must remain a separate shared-file collision point and must not widen this component into directory traversal or mount sequencing.
- Hidden implementation details:
  - The exact table storage type, so long as it owns canonical strong `Arc<ExfatInode>` handles and returns clones on reuse.
  - The exact root-slot representation, so long as it remains separate from the keyed table and does not become a synthetic `InodeKey`.
  - The exact constructor name for `InodeKey`, so long as the type is only constructible from validated location facts and not from mutable inode metadata.

## Functional Specification

### InodeKey Boundary

- Inputs:
  - Validated parent-directory location facts.
  - The ordinal or byte-offset fact that identifies the file-record primary entry within that directory location.
- Actions:
  - Capture the parent-directory location plus the primary-entry ordinal as the canonical identity facts for a non-root inode.
  - Reject any attempt to derive the key from file size, timestamps, name text, start cluster, or other mutable inode contents.
  - Keep the key as a compact value type that exists only because `ExfatFs` needs a stable identity boundary for opened inodes.
- Outputs:
  - A validated `InodeKey` usable only by the opened-inode table.
- Postconditions:
  - Equal trusted location facts produce equal keys.
  - A different primary-entry location produces a different key.
  - Root does not produce an `InodeKey`.

### Ordinary Opened-Inode Table

- Inputs:
  - A validated `InodeKey`.
  - A fully constructed `Arc<ExfatInode>` for a non-root inode.
- Actions:
  - Use the key as the sole lookup identity for the ordinary opened-inode table.
  - Return the canonical stored `Arc<ExfatInode>` clone when the key is already present.
  - Publish a newly constructed inode only after the inode snapshot is complete and the publication step can be made atomic with respect to other lookups.
  - Remove entries only by the exact validated key that created them.
  - Never wrap the handle in a second ownership shell just to expose the table state.
- Outputs:
  - The canonical `Arc<ExfatInode>` handle for the requested key, or a newly published canonical handle if the key was absent.
- Postconditions:
  - Repeated opens or lookups for the same validated location share the same inode object while the table entry remains live.
  - A race between two creators for the same key resolves to a single canonical handle.

### Root Special Case

- Inputs:
  - The filesystem owner state for the root inode.
- Actions:
  - Keep root in a dedicated owner-private slot, not in the ordinary keyed table.
  - Treat root publication as a separate owner concern so `EXR-FS-OPEN-22` can wire VFS root exposure later without changing the ordinary keyspace.
  - Do not synthesize a fake root `InodeKey`.
- Outputs:
  - A separate canonical root handle once later open wiring publishes it.
- Postconditions:
  - The ordinary opened-inode map and the root slot remain disjoint.
  - Root identity is preserved by owner state, not by a fake cache key.

## Invariants

- `ExfatFs` is the single owner of the opened-inode table.
- `InodeKey` is a validated value type, not a helper namespace and not a cache shell.
- The key depends only on trusted directory-location facts.
- The key never depends on mutable inode metadata such as size, timestamps, names, or start cluster.
- The ordinary table returns cloned canonical `Arc<ExfatInode>` handles.
- The root special case stays outside the ordinary keyspace.
- The table never creates a filesystem/inode ownership cycle.
- No helper or accessor should exist solely to expose stored table fields without a named caller need.

## Concurrency Specification

- Shared state:
  - The opened-inode table, the root special-case slot, and any bookkeeping needed to keep canonical handles published exactly once.
- Lock ordering:
  - The filesystem-owner serialization boundary must be acquired before mutating the table or the root slot.
  - Disk I/O, directory traversal, and inode construction work must happen outside that critical section.
  - No inode-local or page-cache lock may be held across table publication.
- Atomicity requirements:
  - Lookup, reuse, insert, and remove are linearized by the owner boundary.
  - Root publication is linearized separately from ordinary keyed publication but uses the same owner serialization rule.
  - A racing lookup must see either the preexisting canonical handle or the fully published new one, never a partially initialized entry.
- Forbidden interleavings:
  - Do not let two creators publish two different handles for the same validated key.
  - Do not let root publication become a synthetic ordinary key insertion.
  - Do not perform blocking I/O while holding the owner serialization boundary.
- Allowed simplifications such as a temporary big lock:
  - A single filesystem-wide serialization lock or equivalent serialized critical section is acceptable for this component.
  - Detailed lock-order mechanics are recorded in `02_designer_async.md` rather than split into more helper APIs.

## Pass Split

### Serial Creator Pass

- Required implementation obligations:
  - Add the owner-private `InodeKey` boundary and the opened-inode table inside `ExfatFs`.
  - Use validated directory-location facts to build the key and reject metadata-derived identity.
  - Implement reuse-first lookup, exact-key remove, and canonical-handle publication for non-root inodes.
  - Reserve the separate root special-case slot without turning it into an ordinary key.
  - Keep the implementation local to the filesystem owner boundary and `fs.rs`.
- Explicit non-goals:
  - No mount/open sequencing.
  - No directory traversal.
  - No inode metadata owner work.
  - No page-cache, read/write, or namespace work.
  - No helper shell whose only job is to forward table calls.

### Serial Checker Pass

- Required checker-owned tests:
  - A key-boundary regression that proves `InodeKey` is derived from validated location facts and not from mutable inode metadata.
  - A reuse regression that proves repeated publication of the same key returns the same canonical `Arc<ExfatInode>` handle.
  - A remove regression that proves the exact key is required to evict the entry and that unrelated entries remain untouched.
  - A root-separation regression that proves the root special case is not encoded as a synthetic ordinary key.
- Observable properties that must pass before leaving the serial loop:
  - The table behaves like one filesystem owner boundary, not a set of unrelated helper wrappers.
  - Canonical handle identity is stable for the same validated key.
  - Root remains outside the ordinary keyspace.

### Concurrency Creator Pass

- Required implementation obligations:
  - No dedicated concurrency implementation beyond the owner serialization boundary recorded above.
- Explicit non-goals:
  - No lock-free map, atomics, background publication task, or per-inode locking.

### Concurrency Checker Pass

- Required checker-owned concurrency tests:
  - No dedicated concurrency tests required.
- Observable properties that must pass before leaving the concurrency loop:
  - The owner serialization boundary remains explicit, and the table does not gain a hidden concurrency protocol.

## Acceptance Notes

- The reviewer should confirm that `InodeKey` stays owner-owned and is not split into a free helper module.
- The reviewer should confirm that ordinary lookup/insert reuses a canonical `Arc<ExfatInode>` handle instead of creating duplicate inode shells.
- The reviewer should confirm that root is reserved as a distinct owner-private special case and not as a synthetic `InodeKey`.
- The reviewer should reject any attempt to widen this component into mount/open sequencing or directory traversal.
- Any later `mod.rs` wiring needed for the implementation must remain a shared-file collision point and not become part of this designer boundary.
