<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Core

## Metadata

- Component ID: `EXR-DIR-ENGINE-19`
- Title: `DirectoryEngine` Read-Only Record Stream
- Status: `Specified`
- Author: designer
- Date: 2026-04-07
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-DIR-ENGINE-19/20260407-1048-designer-packet.md`
- Based on architect artifact: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-ENGINE-19/00_architect.md`

## Scope

- In scope:
  - Define `DirectoryEngine` as an `ExfatFs`-internal, read-only directory record-stream owner.
  - Carry a validated `ExfatChain` plus private scan cursor state for walking directory bytes in on-disk order.
  - Group raw directory dentries into validated `ExfatDentrySet` file records without introducing name policy or bitmap policy.
  - Surface singleton system-entry candidates only by raw dentry kind and validated record shape.
  - Keep the scan service read-only and stable enough for later `EXR-UPCASE-20`, `EXR-BITMAP-21`, `EXR-FS-OPEN-22`, `EXR-DIR-OPS-23`, and `EXR-DENTRY-WRITE-28` consumers.
- Out of scope:
  - Upcase folding, name comparison, or name hashing policy.
  - Allocation bitmap loading, occupancy policy, or free-space discovery.
  - VFS directory methods, inode cache policy, or namespace mutation.
  - Write-side directory mutation.
  - Any helper surface that does not have a named future consumer in this packet.
- `02_designer_async.md` is not needed because this component has no independent async contract, no cross-call shared mutable state, and no lock-order rule beyond the single-owner serialization already implied by `ExfatFs`.
  The residual serialization assumption is simple: one `DirectoryEngine` instance is driven serially by its owning filesystem context, and later filesystem-wide serialization belongs to `ExfatFs`, not to this component.

## Module Specification

- Dependencies:
  - `read_metadata_bytes` as the owner-private transport primitive.
  - `ExfatChain` as the validated directory traversal state.
  - `RawExfatDentry`, `ExfatDentry`, and `ExfatDentrySet` as the record-decoding and record-validation boundary.
  - `ExfatSuperBlock` for geometry, offset translation, and end-of-directory bounds.
- Interfaces provided:
  - A crate-local `DirectoryEngine` owner service.
  - A private scan cursor that advances in directory-entry order and remembers where the next record begins.
  - A canonical record-stream result that preserves on-disk order and returns either a validated file-record set or a raw singleton system-entry candidate.
  - A singleton candidate path for raw `Bitmap` and `Upcase` dentries only.
- Files or modules touched:
  - Primary landing: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs`
  - Any module declaration change should stay narrow and file-local if the new module needs to be wired in.
- Hidden implementation details:
  - Exact buffering size, whether the cursor is byte-based or dentry-based, and whether one read spans one cluster or multiple clusters.
  - Whether the engine keeps one in-flight record accumulator or a small private queue of raw dentries.
  - The exact internal name of the scan-state fields, so long as the owner boundary remains a single `DirectoryEngine` and not a helper pile.

## Functional Specification

### Record Stream

- Precondition:
  - The caller supplies a validated `ExfatChain` for a directory.
  - The caller is prepared to drive the stream serially.
- Action:
  - Read directory bytes through `read_metadata_bytes`.
  - Decode raw 32-byte dentries in on-disk order.
  - Accumulate ordered dentries until a full record boundary is known.
  - Emit validated `ExfatDentrySet` values for `File -> Stream -> Name+ -> benign secondary*` records.
  - Emit raw singleton candidates for `Bitmap` and `Upcase` dentries without interpreting their payloads.
  - Treat `Deleted` dentries as skipped tombstones and `Unused` as the end-of-directory marker.
- Postcondition:
  - The caller sees a faithful directory record stream, not a normalized name service and not a bitmap loader.

### System-Entry Candidate Output

- Precondition:
  - The stream has encountered a singleton `Bitmap` or `Upcase` dentry.
- Action:
  - Surface the typed raw entry itself as the candidate.
  - Preserve the raw on-disk fields exactly as read.
  - Do not classify the entry by name text, do not case-fold, and do not load the bitmap or upcase table here.
- Postcondition:
  - Later owners can consume the candidate using only the raw typed entry and the already validated directory record context.

## Invariants

- The engine is owner-internal to `ExfatFs`; it is not a public directory API.
- The scan state is read-only and advances monotonically through the directory chain.
- `ExfatChain` remains the validated traversal input; the engine does not re-derive chain semantics.
- Raw dentry bytes are preserved in on-disk order until the validated record boundary is known.
- `ExfatDentrySet` remains the validation boundary for file records.
- Singleton `Bitmap` and `Upcase` entries are surfaced as raw candidates only; no policy is embedded in the engine.
- `Deleted` entries do not become durable records, and `Unused` terminates the scan.

## Concurrency Specification

- Shared state:
  - None beyond the engine’s own private cursor and record accumulator.
- Lock ordering:
  - None introduced by this component.
- Atomicity requirements:
  - Each record emission must leave the private scan cursor in a self-consistent next-record position.
  - No emission may depend on partially exposed mutable state from another caller.
- Forbidden interleavings:
  - No concurrent readers on the same engine instance.
  - No writeback, no background mutation, and no hidden filesystem-global state.
- Allowed simplifications such as a temporary big lock:
  - None required.

## Pass Split

### Serial Creator Pass

- Required implementation obligations:
  - Add the `DirectoryEngine` owner service in `directory.rs`.
  - Carry the validated `ExfatChain` and a private scan cursor for directory traversal.
  - Implement the read-only record stream that groups raw dentries into validated `ExfatDentrySet` file records.
  - Surface singleton `Bitmap` and `Upcase` candidates as raw typed dentries only.
  - Keep the scan state policy-free and leave name folding, bitmap loading, and VFS behavior to later owners.
- Explicit non-goals:
  - No directory mutation.
  - No inode cache or VFS directory ops.
  - No name policy, bitmap policy, or upcase-table policy.
  - No temporary helper surfaces unless a later consumer is named here.

### Serial Checker Pass

- Required checker-owned tests:
  - Verify that a record stream can advance across block and cluster boundaries without losing record order.
  - Verify that a valid `File -> Stream -> Name+ -> benign secondary*` sequence is emitted as a validated `ExfatDentrySet`.
  - Verify that singleton `Bitmap` and `Upcase` entries surface as raw candidates without policy interpretation.
  - Verify that `Deleted` entries are skipped and `Unused` terminates the stream.
  - Verify that malformed record boundaries are rejected instead of being repaired or merged silently.
- Observable properties that must pass before leaving the serial loop:
  - The engine preserves on-disk ordering.
  - The engine emits validated file records only after the full record shape is known.
  - The engine does not absorb upcase or bitmap policy.

### Concurrency Creator Pass

- Required implementation obligations:
  - No dedicated concurrency implementation is required.

- Explicit non-goals:
  - Do not add per-record locking, atomics, async state machines, or background retry logic.

### Concurrency Checker Pass

- Required checker-owned concurrency tests:
  - No dedicated concurrency tests required.
- Observable properties that must pass before leaving the concurrency loop:
  - N/A.

## Acceptance Notes

- Reviewers should confirm that the engine stays owner-internal and does not become a hidden public directory API.
- Reviewers should reject any attempt to fold name comparison, bitmap loading, or upcase-table handling into the scan service.
- If future work needs more than raw singleton candidates and validated file records, the component has been widened too far and should be re-sliced instead of expanded.
