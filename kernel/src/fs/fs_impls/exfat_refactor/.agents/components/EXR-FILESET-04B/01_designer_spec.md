<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Specification

## Metadata

- Component ID: EXR-FILESET-04B
- Title: Validated File-Record Set And Raw Name Aggregation
- Status: `Specified`
- Author: main-agent
- Date: 2026-04-01
- Based on architect artifact: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FILESET-04B/00_architect.md`

## Scope

- In scope:
  - Define the validated multi-entry file-record set object for exFAT.
  - Enforce the ordered shape `File -> Stream -> Name+ -> benign secondary*`.
  - Preserve and serialize the ordered typed dentry sequence without reordering.
  - Aggregate raw UTF-16 name data from name dentries without upcase-table policy.
  - Verify and update the file-record checksum across the full set.
  - Provide a narrow assembly path from trusted file metadata and raw name data.
  - Add checker-owned ktests for validation, checksum, raw-name aggregation, and serialization.
- Out of scope:
  - Directory iteration or on-disk scanning.
  - Inode identity, inode key derivation, or namespace mutation policy.
  - FAT-chain decoding, allocation policy, or bitmap mutation.
  - Upcase-table loading, case folding, or canonical name lookup.
  - Any second record shape beyond one file-record set.

## Dependencies and Provided Interfaces

- Dependencies:
  - `EXR-BOOT-01` for shared kernel error conventions, checked arithmetic patterns, and exFAT constants already established there.
  - `EXR-DENTRY-04A` for typed single-entry decoding and the concrete `ExfatDentry` wrappers.
  - Existing kernel `Vec`, `Result`, `Arc`, and checksum helpers already used by the exFAT code.
- Files or modules touched:
  - `kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/fileset.rs`
- Interfaces provided:
  - A validated set type, working name `ExfatDentrySet`.
  - A raw-name helper that returns ordered UTF-16 code units gathered from the name dentries.
  - A constructor from already ordered typed dentries.
  - A narrow constructor for assembling a validated set from trusted file metadata and raw name data.
  - Accessors for the file and stream primary entries.
  - `verify_checksum`, `update_checksum`, and ordered byte serialization.
- Hidden implementation details:
  - Whether the raw-name helper returns a `Vec<u16>` or a small module-private wrapper type.
  - Whether the validated constructor takes a single ordered `Vec<ExfatDentry>` or a pair of primary/tail slices internally normalized into that vector.
  - Whether the file and stream accessors are implemented as getters, setters, or narrow replacement helpers.

## Data and Control Flow

- Construction flows in two ways:
  - Validation flow: a caller passes an already ordered vector of typed dentries into the set constructor.
  - Assembly flow: a caller passes trusted file metadata, trusted stream metadata, raw UTF-16 name data, and any benign secondary tail entries into the assembly helper.
- Validation flow:
  - The constructor checks the entry order and entry types.
  - The constructor checks the secondary count implied by the serialized sequence.
  - The constructor reconstructs raw name data from the name dentries.
  - The constructor verifies the stream `name_len` and `name_hash` against the reconstructed raw name data.
  - The constructor verifies the file checksum against the full serialized set.
- Assembly flow:
  - The assembly helper builds the ordered entry list.
  - The helper derives `num_secondary` from the final entry count.
  - The helper derives the raw-name length and name hash from the supplied raw name data.
  - The helper writes the file checksum before returning the validated set.
- Mutation flow:
  - Any helper that changes serialized bytes must leave checksum validity stale until `update_checksum` is called.
  - `update_checksum` recomputes the checksum over the current serialized bytes and writes it back into the file primary entry.
- Serialization flow:
  - `to_le_bytes` emits the current entries in order.
  - Serialization must not normalize, reorder, or case-fold any content.

## Functional Rules

### Constructor and Validation

- Precondition:
  - The caller supplies typed dentries or trusted components that already belong to one file record.
- Action:
  - Accept only a `File` primary entry in slot 0.
  - Accept only a `Stream` primary entry in slot 1.
  - Accept one or more `Name` dentries immediately after the stream entry.
  - Accept only benign secondary dentries after the name tail.
  - Reject `Unused`, `Deleted`, or any primary entry appearing after slot 0.
  - Reject a tail that contains `GenericPrimary` or any other non-benign variant.
  - Reject a record whose total entry count exceeds `u8::MAX + 1`.
  - Reject a record whose stream name length does not match the aggregated raw UTF-16 length.
  - Reject a record whose stream name hash does not match the raw-name checksum.
  - Reject a record whose file checksum does not match the checksum over all serialized entries.
- Postcondition:
  - The returned object is a validated file-record boundary that later inode and namespace code can trust internally.

### Raw Name Aggregation

- Precondition:
  - The set already passed structural validation, or the caller supplies a trusted ordered name tail for assembly.
- Action:
  - Concatenate the UTF-16 payloads from each `Name` dentry in order.
  - Stop at the first zero code unit in the logical name stream and ignore padding after that point.
  - Preserve the raw code units exactly as stored on disk.
  - Do not consult an upcase table or perform case folding.
- Postcondition:
  - Later consumers receive the raw logical name payload as stored in the file record.

### Checksum Handling

- Precondition:
  - The set holds an ordered sequence of typed dentries.
- Action:
  - Compute the checksum across the complete serialized set with the file checksum field treated as the checksum input exclusion point.
  - Write the computed checksum into the file primary entry.
  - Keep all other entry bytes unchanged.
- Postcondition:
  - `verify_checksum` succeeds immediately after `update_checksum` unless another mutator has changed the serialized bytes.

### Serialization

- Precondition:
  - The set is already validated, or the caller intentionally wants the current raw bytes without revalidation.
- Action:
  - Emit the entries in current order as a contiguous little-endian byte vector.
  - Copy each 32-byte dentry verbatim from its typed representation.
- Postcondition:
  - The serialized bytes round-trip back into the same ordered typed sequence when re-decoded by the existing dentry layer.

## Invariants

- The set always owns the entry order; it never derives meaning from directory position outside the record.
- The file primary entry remains at index 0 and the stream entry remains at index 1.
- The file checksum covers the entire record, not just the primary pair.
- Raw name data is stored and compared structurally, not canonically transformed.
- The component handles exactly one record shape and does not generalize to directory iteration or other exFAT record families.
- Any mutation that changes serialized bytes invalidates the checksum until `update_checksum` restores it.

## Concurrency Specification

- Shared state:
  - None beyond exclusive `&mut self` access and immutable inputs.
- Lock ordering:
  - None.
- Atomicity requirements:
  - Validation, checksum recomputation, and serialization are ordinary single-object operations with no internal locking.
  - No helper should depend on a shared cache or a cross-object writeback transaction.
- Forbidden interleavings:
  - No I/O under a lock.
  - No blocking operations under a lock.
  - No shared mutable global state.
- Allowed simplifications:
  - No dedicated concurrency implementation is required for this component.
  - No dedicated concurrency tests are required because the boundary is pure value management and mutable access is already exclusive.

## Pass Split

### Serial Creator Pass

- Required implementation obligations:
  - Add `fileset.rs` and wire it into `mod.rs`.
  - Define the validated file-record set type and the raw-name helper.
  - Implement ordered validation for `File -> Stream -> Name+ -> benign secondary*`.
  - Implement the checksum calculation and checksum update path.
  - Implement the narrow assembly helper from trusted file metadata and raw name data.
  - Implement ordered byte serialization for later write-side consumers.
  - Keep the API surface narrowly bounded to this one record shape.
- Explicit non-goals:
  - No directory traversal.
  - No inode mapping or namespace mutation policy.
  - No FAT-chain semantics.
  - No upcase-table integration.
  - No second record family or generalized dentry container.

### Serial Checker Pass

- Required checker-owned tests:
  - A success-path ktest that builds a valid set, verifies checksum, and serializes it back to bytes.
  - A raw-name aggregation ktest that proves multi-entry name data is concatenated in order.
  - A checksum-update ktest that mutates a field, observes a failed verification, then restores validity with `update_checksum`.
  - Negative-path ktests for wrong ordering, missing stream entry, name dentries after benign secondaries, unexpected primary entries in the tail, and checksum mismatch.
  - A serialization round-trip ktest that confirms the emitted bytes decode back into the same typed sequence.
- Observable properties that must pass before leaving the serial loop:
  - The constructor rejects malformed record shapes instead of repairing them silently.
  - Raw name data is preserved without upcase-policy dependence.
  - Serialized output preserves the current order and byte content exactly.
  - The checksum logic covers the full record and not just the primary pair.

### Concurrency Creator Pass

- Required implementation obligations:
  - No dedicated concurrency implementation required.
- Explicit non-goals:
  - Do not add locks, atomics, caches, or background maintenance for this component.

### Concurrency Checker Pass

- Required checker-owned concurrency tests:
  - No dedicated concurrency tests required.
- Observable properties that must pass before leaving the concurrency loop:
  - The component remains a pure validated value object with exclusive mutation only.

## Acceptance Notes

- Reviewer should verify that the raw-name helper stays policy-free and does not absorb upcase-table work.
- Reviewer should reject any attempt to move directory scanning, inode identity, or FAT-chain logic into this component.
- If the implementation starts needing more than one file-record shape, the scope is too broad and must be split instead of widened.
