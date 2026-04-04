<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Core

## Metadata

- Component ID: `EXR-SYSROOT-06`
- Title: Root-Directory System-Entry Scanner
- Status: `Specified`
- Author: `designer`
- Date: `2026-04-04`
- Task packet: `EXR-SYSROOT-06-DESIGN-20260404-1408`
- Based on architect artifact: `00_architect.md`

## Scope

- In scope:
  - A synchronous, read-only scan of the exFAT root directory for the `BITMAP` and `UPCASE` system entries.
  - Discovery facts only: entry location, entry kind, validated start cluster, validated byte size, and upcase checksum.
  - Boundary validation that keeps later `EXR-UPCASE-07A` and `EXR-BITMAP-08A` loaders from rediscovering the same root facts.
  - Duplicate, missing, malformed, and wrong-kind root-entry detection at the scanner boundary.
- Out of scope:
  - Mount bootstrap, mount-owned shared state, or any filesystem-wide owner object.
  - General directory iteration, lookup, rename, or any namespace mutation.
  - Loading bitmap bytes or upcase-table bytes.
  - Page-cache ownership, buffered I/O, async work, or background coordination.
  - Name folding, hash policy, or any other lookup concern outside the root-entry discovery boundary.

## Module Specification

- Dependencies:
  - `EXR-IO-02`
  - `EXR-CHAIN-03B`
  - `EXR-DENTRY-04A`
  - `EXR-INODE-05B`
- Interfaces provided:
  - One canonical synchronous scan entry point in `sysroot.rs`.
  - One read-only aggregate of root discovery facts for later loaders.
  - One opaque primary-entry location token for stable diagnostics and downstream ownership checks.
  - Two typed discovery records, one for `BITMAP` and one for `UPCASE`.
- Files or modules touched:
  - `kernel/src/fs/fs_impls/exfat_refactor/sysroot.rs`
- Hidden implementation details:
  - Whether the scanner uses a private cursor, a small state machine, or a private iterator to advance through the root entry stream.
  - Whether unrelated root content is skipped by reusing already-validated file-record boundaries or by another private advancement helper.
  - Whether the primary-entry location token is stored as a packed key or as explicit cluster-plus-offset fields.
  - Whether the scanner stops as soon as both discoveries are found or finishes the directory scan and then returns the same aggregate.

The canonical surface must stay narrow. The later loaders should consume the returned discovery aggregate directly; no separate getter layer is specified unless a downstream caller proves it needs one.

## Functional Specification

### Operation

- Name: root-system-entry scan
- Inputs:
  - `block_device: &dyn BlockDevice`
  - `super_block: &ExfatSuperBlock`
  - `root_chain: ExfatChain`
- Preconditions:
  - `root_chain` is already validated by `EXR-CHAIN-03B`.
  - `super_block` already provides validated root geometry.
  - The caller wants only root-entry discovery facts, not loaded table contents.
  - The caller is not asking this component to own mount state or directory namespace state.
- Actions:
  - Walk the root directory entry stream in root-chain order.
  - Stop at end-of-directory.
  - Skip deleted or free entries.
  - Ignore unrelated root content only as needed to advance safely.
  - Recognize the root `BITMAP` entry and the root `UPCASE` entry.
  - Validate each recognized entry at discovery time only far enough to establish a legal root-only primary entry, a legal data-cluster start, and a representable byte size.
  - Preserve the `UPCASE` checksum field as a discovery fact.
  - Record the primary-entry location for each discovery.
  - Detect duplicate `BITMAP` or `UPCASE` entries and reject them.
  - Reject malformed root entries whose on-disk fields cannot be represented safely at the discovery boundary.
  - Never load the bitmap payload or the upcase table payload.
- Outputs:
  - `Result<ExfatSysRootFacts>`
- Postconditions:
  - The returned facts are read-only and value-like.
  - Later loaders can consume the same discovery facts without rescanning the root directory.
  - No mount state, page cache state, or directory API state is created or modified.

### Discovery Facts

- `ExfatSysRootFacts` contains exactly the root discovery records needed by later loaders.
- The bitmap discovery record preserves:
  - the primary-entry location token,
  - the discovered start cluster,
  - the discovered byte size.
- The upcase discovery record preserves:
  - the primary-entry location token,
  - the discovered start cluster,
  - the discovered byte size,
  - the upcase checksum field.
- The scanner does not attach loaded bytes, decoded table content, or any allocation-bitmap occupancy state to the discovery aggregate.

### Validation Ownership

- Discovery-time validation belongs here:
  - root-only entry identity,
  - duplicate detection,
  - safe directory advancement,
  - representable location facts,
  - legal start-cluster facts,
  - representable byte-size facts,
  - checksum field preservation for `UPCASE`.
- Loader-time validation belongs later:
  - bitmap content and cluster-marking checks,
  - upcase-table byte-content checks,
  - any allocation or case-folding behavior that depends on the loaded payloads.

## Invariants

- The scanner is synchronous and read-only.
- The scanner owns no mutable shared state.
- The scanner never produces mount bootstrap state.
- The scanner never exposes a general directory API.
- The scanner never returns loaded bitmap or upcase payload bytes.
- One root `BITMAP` discovery and one root `UPCASE` discovery are the only canonical outputs.
- Each discovery record retains the original primary-entry location used for later diagnostics or ownership checks.
- The `UPCASE` checksum is preserved as a fact, not interpreted as a loaded-table substitute.
- Later loaders may trust the discovery facts as boundary-validated inputs, but they still own content validation.

## Concurrency Specification

- Shared state:
  - None introduced by this component.
- Lock ordering:
  - None.
- Atomicity requirements:
  - The scan is one synchronous call that returns a complete read-only aggregate.
  - The caller must not observe partial mutation because the scanner does not mutate shared state.
- Forbidden interleavings:
  - None beyond ordinary single-threaded call ordering.
- Allowed simplifications such as a temporary big lock:
  - None needed.

No separate async artifact is needed because the component introduces no background work, no awaitable I/O contract, no shared mutable state, and no lock-ordering obligations. The full serialization story is recorded here in the synchronous concurrency specification and the pass split below.

## Pass Split

### Serial Creator Pass

- Required implementation obligations:
  - Add the canonical synchronous root-system-entry scanner and the read-only discovery aggregate in `sysroot.rs`.
  - Preserve the primary-entry location, start cluster, byte size, and `UPCASE` checksum facts exactly as discovered.
  - Keep the scanner boundary narrow: discovery only, no table loading, no mount ownership, no general directory API.
  - Reject duplicate, missing, malformed, or wrong-kind entries at the scanner boundary.
  - Keep unrelated root content outside the result surface.
- Explicit non-goals:
  - No mount bootstrap or superblock mutation.
  - No general directory lookup, iteration API, or namespace mutation.
  - No bitmap or upcase content materialization.
  - No async work, tasks, channels, atomics, or registry mutation.

### Serial Checker Pass

- Required checker-owned tests:
  - A mixed-root regression that proves the scanner finds the `BITMAP` and `UPCASE` facts without turning into a general directory API.
  - A duplicate-entry regression for either `BITMAP` or `UPCASE`.
  - A missing-entry regression for either required entry.
  - A malformed-entry regression that exercises an illegal start cluster, a bad size, or another structurally invalid root payload.
  - A regression that proves the `UPCASE` checksum is preserved as discovery data and not treated as loaded content.
- Observable properties that must pass before leaving the serial loop:
  - The scanner returns one read-only discovery aggregate.
  - Later loaders can reuse the same facts without rescanning.
  - The tests do not need page cache, VFS, mount sequencing, or async harnesses.

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

- The scanner should remain narrower than directory scanning. If it starts needing lookup semantics or namespace policy, the boundary has drifted.
- The scanner should remain narrower than loading. If it begins reading bitmap bytes or upcase-table contents, the later loader components are being pulled in too early.
- The discovery aggregate should be the only cross-component surface here; no extra helper should be added unless a later loader names the need.
- The root-entry location token exists so later code can report stable diagnostics without re-walking the root directory.
