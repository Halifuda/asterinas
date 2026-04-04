<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Core

## Metadata

- Component ID: `EXR-UPCASE-07B`
- Title: Canonical Upcase-Backed Case Folding And Name Hashing
- Status: `Specified`
- Author: `designer`
- Date: `2026-04-04`
- Task packet: `EXR-UPCASE-07B-DESIGN-20260404-1501`
- Based on architect artifact: `00_architect.md`

## Scope

- In scope:
  - Consume the canonical loaded upcase-table value from `EXR-UPCASE-07A`.
  - Fold logical UTF-16 name units through that table.
  - Derive the exFAT `NameHash` from the folded UTF-16 bytes.
  - Replace the provisional raw-UTF-16 hash path used by `fileset.rs`.
  - Expose one canonical read-only service surface for later filename work.
- Out of scope:
  - Upcase-table loading, discovery, fallback selection, or mount bootstrap.
  - Root-directory scanning or lookup orchestration.
  - Charset conversion, UTF-8/NLS translation, or path normalization policy.
  - Namespace mutation, rename policy, or dentry-set writeback.
  - A second overlapping helper for fold-only or hash-only behavior unless a later caller is explicitly named.

## Module Specification

- Dependencies:
  - `EXR-UPCASE-07A`
  - `EXR-FILESET-04B`
  - The Microsoft exFAT `NameHash` rule.
  - Linux `fs/exfat/nls.c` and `fs/exfat/namei.c` as algorithmic reference only.
- Interfaces provided:
  - One canonical read-only service on the loaded upcase table that accepts logical UTF-16 units and returns the folded-or-hashed result needed by `fileset.rs`.
  - One internal table-backed folding primitive if the hash operation needs it, but only as an implementation detail behind the canonical service.
  - No separate public helper surface for fallback policy, mount policy, or name comparison.
- Files or modules touched:
  - `kernel/src/fs/fs_impls/exfat_refactor/upcase_table.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/fileset.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- Hidden implementation details:
  - Whether folding is performed by direct table lookup on each UTF-16 unit or by a pre-expanded table representation.
  - Whether the canonical service hashes folded units in one pass or exposes an internal fold step that the hash step immediately consumes.
  - How the loaded table value stores its trusted bytes or words, provided the surface remains read-only.

The canonical boundary must stay narrow: later code should consume one trusted table-backed service instead of reimplementing raw UTF-16 hashing or requesting a separate policy layer.

## Functional Specification

### Operation

- Name: canonical upcase-backed fold-and-hash
- Inputs:
  - A loaded upcase-table value produced by `EXR-UPCASE-07A`.
  - A logical UTF-16 name slice from `fileset.rs` or another later filename consumer.
- Preconditions:
  - The upcase-table value is already validated and read-only.
  - The caller is not asking for mount policy, root discovery, or fallback selection.
  - The caller wants the exFAT name-hash contract, not a raw code-unit checksum.
- Actions:
  - Fold each logical UTF-16 unit through the loaded table.
  - Treat the loaded table as the single source of case-folding truth for the volume.
  - Derive the exFAT `NameHash` from the folded UTF-16 bytes, matching the Microsoft exFAT rule and the Linux upcase-then-hash behavior.
  - Preserve the caller-visible name length boundary used by the stream entry.
  - Reject or surface any structural mismatch that prevents the loaded table from being used as a trusted folding source.
- Outputs:
  - A folded-and-hashed result suitable for populating or validating the stream entry `NameHash`.
  - No mutation of the loaded table and no fallback synthesis.
- Postconditions:
  - The produced hash reflects the folded UTF-16 bytes, not the raw input bytes.
  - `fileset.rs` no longer owns a second provisional hash implementation.
  - The loaded table remains read-only and reusable for later filename consumers.

### Canonical Service Surface

- `ExfatUpcaseTable` is the canonical owner of the loaded table and the only component that may perform table-backed folding for this slice.
- `fileset.rs` is the first consumer and uses that service when validating or synthesizing stream-entry metadata.
- If an internal fold-only helper exists, it must remain private to the table module and justified only as a support step for the canonical hash service.
- A second exported fold-only or hash-only helper is not warranted for this component.

## Invariants

- The loaded upcase table is immutable after construction.
- Folding is always table-backed, never raw-UTF-16 identity behavior.
- Name hashing always runs on folded UTF-16 bytes.
- The canonical service does not load tables, discover the root entry, or choose fallback policy.
- `fileset.rs` must not retain an independent raw-UTF-16 checksum path once the canonical service is wired in.
- The service stays usable by later directory and rename code without exposing mount ownership.

## Concurrency Specification

- Shared state:
  - None introduced by this component.
- Lock ordering:
  - None.
- Atomicity requirements:
  - The loaded upcase table is published once and then treated as read-only.
  - Folding and hashing operate on an already-published immutable value.
- Forbidden interleavings:
  - No background loading, partial publication, or shared mutable cache mutation.
  - No I/O or blocking under a spinlock because this component has no lock-bearing workflow.
- Allowed simplifications such as a temporary big lock:
  - None needed.

No separate async artifact is needed because this component is synchronous, read-only, and has no awaitable contract or shared mutable state. Those serialization assumptions are recorded here and in the serial pass split below.

## Pass Split

### Serial Creator Pass

- Required implementation obligations:
  - Add the canonical upcase-backed fold-and-hash service to the loaded table boundary.
  - Route `fileset.rs` to that service instead of the raw-UTF-16 checksum path.
  - Preserve the loaded table as the only canonical source of folding truth.
  - Keep any fold-only helper private unless the hash service truly needs it internally.
- Explicit non-goals:
  - No mount policy, no root discovery, no fallback/default table, and no directory mutation.
  - No second public helper surface for the same normalization semantics.
  - No async tasks, channels, atomics, or background coordination.

### Serial Checker Pass

- Required checker-owned tests:
  - A representative fold-and-hash regression that proves the canonical service uppercases before hashing.
  - A mismatch regression that proves raw UTF-16 bytes do not produce the accepted `NameHash` when folding changes a code unit.
  - A `fileset.rs` regression that proves stream-entry validation or synthesis now uses the canonical table-backed service.
  - A full-surface regression that proves names beyond the legacy assumptions still route through the canonical service without truncation of the loaded table boundary.
- Observable properties that must pass before leaving the serial loop:
  - The hash is derived from folded UTF-16 bytes, not raw input bytes.
  - `fileset.rs` no longer depends on a private provisional hash helper for the canonical path.
  - The tests stay local and do not require mount sequencing, directory mutation, or async harnesses.

### Concurrency Creator Pass

- Required implementation obligations:
  - None.
- Explicit non-goals:
  - No concurrency pass is needed for this component.

### Concurrency Checker Pass

- Required checker-owned concurrency tests:
  - No dedicated concurrency tests required.
- Observable properties that must pass before leaving the concurrency loop:
  - None; the synchronous read-only boundary already captures the full contract.

## Acceptance Notes

- The main acceptance pressure is boundary hygiene: this component should own table-backed folding and hash derivation, but not mount policy, discovery, or lookup orchestration.
- If the implementation starts exposing both fold-only and hash-only public helpers, that should be justified against the named downstream caller; otherwise one canonical service is the clearer contract.
- `fileset.rs` should consume the canonical service directly so the provisional raw-UTF-16 hash path does not survive by accident.
- The reviewer should pay special attention to whether the final surface still looks like a single table-backed normalization service rather than two overlapping helpers.
