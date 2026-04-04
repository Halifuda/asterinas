<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Core

## Metadata

- Component ID: `EXR-UPCASE-07A`
- Title: On-Disk Upcase Table Loader And Validator
- Status: `Specified`
- Author: `designer`
- Date: `2026-04-04`
- Task packet: `EXR-UPCASE-07A-DESIGN-20260404-1414`
- Based on architect artifact: `00_architect.md`

## Scope

- In scope:
  - Loading the on-disk exFAT upcase table identified by `EXR-SYSROOT-06`.
  - Validating the discovered start cluster, table size, cluster-chain reachability, and checksum before the table becomes visible to later code.
  - Producing one canonical, read-only loaded-table surface for later case-insensitive name work.
  - Preserving the full discovered table payload without truncating it to any legacy prefix.
- Out of scope:
  - Case folding, name hashing, or any name-normalization API.
  - Fallback or built-in default-table policy.
  - Mount bootstrap, mount-owned shared state, or filesystem-wide policy ownership.
  - Charset conversion, UTF-8/NLS translation, or lookup behavior.
  - Root-directory discovery of the `UPCASE` entry.

## Module Specification

- Dependencies:
  - `EXR-IO-02`
  - `EXR-CHAIN-03B`
  - `EXR-FILESET-04B`
  - `EXR-SYSROOT-06`
- Interfaces provided:
  - One canonical loader entry point in `upcase_table.rs` that accepts the validated `UPCASE` discovery facts from `EXR-SYSROOT-06`.
  - One canonical loaded-table value type that owns the validated table bytes or words and exposes a read-only view for later consumers.
  - No separate helper surface for fallback policy, case folding, or name hashing.
- Files or modules touched:
  - `kernel/src/fs/fs_impls/exfat_refactor/upcase_table.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- Hidden implementation details:
  - Whether the table is buffered all at once or read in cluster-sized chunks before canonicalization.
  - Whether the canonical surface stores validated bytes, validated little-endian words, or another equivalent read-only representation.
  - Whether checksum accumulation happens during streaming read or after a private buffer fill.

The canonical surface must stay narrow. Later consumers should receive the loaded table directly and must not need a second discovery pass or a separate fallback/default-table chooser.

## Functional Specification

### Operation

- Name: upcase-table load and validate
- Inputs:
  - `block_device: &dyn BlockDevice`
  - `super_block: &ExfatSuperBlock`
  - `upcase_facts: &ExfatSysRootUpcaseDiscovery`
- Preconditions:
  - `upcase_facts` came from `EXR-SYSROOT-06` and already carries the validated start cluster, table size, primary-entry location, and checksum.
  - The caller wants a loaded table, not root discovery.
  - The caller is not asking for fallback-table policy, case folding, or name hashing.
- Actions:
  - Derive the on-disk payload from the discovered start cluster and discovered size.
  - Read exactly the discovered table payload from the cluster chain in on-disk order.
  - Reject a chain that is too short, malformed, or otherwise unable to supply the full payload.
  - Reject table sizes that cannot describe a complete on-disk upcase payload, including odd-length or otherwise structurally invalid sizes.
  - Compute the on-disk checksum over the loaded payload and compare it with the discovery checksum.
  - Materialize one canonical read-only loaded-table value only after the payload and checksum checks succeed.
  - Preserve the full discovered table, not a truncated compatibility subset.
- Outputs:
  - `Result<ExfatUpcaseTable>`
- Postconditions:
  - The returned table is the canonical loaded surface for later case-insensitive work.
  - The returned value is read-only and value-like; later code may borrow its raw table view but may not mutate the canonical contents in place.
  - No fallback/default table is synthesized when validation fails.
  - No mount-owned state, page-cache state, or directory API state is created or modified.

### Loaded Table Surface

- `ExfatUpcaseTable` contains exactly the validated table payload and the metadata later consumers need to trust that payload.
- The loaded surface preserves:
  - the full validated payload in on-disk order,
  - the original discovery checksum or an equivalent preserved checksum fact,
  - the discovered byte size or an equivalent validated size fact.
- The loaded surface does not embed:
  - a fallback table,
  - case-folding results,
  - name hashes,
  - mount policy.

### Validation Ownership

- Discovery facts are trusted here only for identity and location:
  - the start cluster,
  - the payload size,
  - the checksum fact,
  - the primary-entry location for later diagnostics.
- Loader-time validation belongs here:
  - cluster-chain reachability for the discovered size,
  - payload completeness,
  - checksum comparison,
  - structural table-size validity.
- Later case-folding behavior belongs in `EXR-UPCASE-07B`.

## Invariants

- The loader is synchronous and read-only with respect to shared filesystem state.
- The loader never rescans the root directory.
- The loader never chooses or synthesizes a built-in fallback table.
- The canonical loaded table preserves the full validated payload, not just a compatibility prefix.
- The canonical surface is immutable after construction.
- The checksum comparison is against the preserved discovery checksum, not against a later derived policy value.
- Any later case-folding consumer must work from this loaded surface rather than from a second on-disk read.

## Concurrency Specification

- Shared state:
  - None introduced by this component.
- Lock ordering:
  - None.
- Atomicity requirements:
  - The load is one synchronous all-or-nothing operation.
  - The caller must not observe a partially published table value.
- Forbidden interleavings:
  - No background loading, partial publication, shared cache mutation, or mount-state mutation.
- Allowed simplifications such as a temporary big lock:
  - None needed.

No separate async artifact is needed because the component has no background work, awaitable contract, or shared mutable state. Any serialization assumptions are recorded here in the synchronous concurrency specification and in the serial pass split below.

## Pass Split

### Serial Creator Pass

- Required implementation obligations:
  - Add the canonical loader entry point in `upcase_table.rs`.
  - Add the canonical loaded-table value type that owns the validated payload.
  - Wire the new module into `mod.rs`.
  - Validate payload completeness, table-size structure, and checksum before publishing the loaded surface.
  - Preserve the full discovered table payload for later consumers.
- Explicit non-goals:
  - No case-folding API, no name-hash API, and no charset conversion.
  - No fallback/default-table policy.
  - No root-directory rediscovery or mount bootstrap.
  - No async tasks, channels, atomics, or background coordination.

### Serial Checker Pass

- Required checker-owned tests:
  - A valid-load regression that proves the loader accepts a real upcase discovery record and returns a canonical read-only table surface.
  - A checksum-mismatch regression that proves the loader rejects the same payload when the preserved checksum fact is wrong.
  - A malformed-discovery regression that exercises an illegal start cluster, an invalid size, or another structurally invalid discovery fact.
  - A short-read or truncated-chain regression that proves the loader rejects incomplete on-disk payloads.
  - A full-payload regression that proves the canonical table keeps the discovered payload past the legacy 128-entry prefix boundary.
- Observable properties that must pass before leaving the serial loop:
  - The loader returns one canonical loaded-table value, not a fallback or a partial result.
  - The returned surface stays read-only and still exposes the full validated payload to later code.
  - The tests do not need mount sequencing, page-cache ownership, directory mutation, or async harnesses.

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

- The loader must stay narrower than case-insensitive name handling. If it starts exposing folding or hash logic, the boundary has drifted into `EXR-UPCASE-07B`.
- The loader must stay narrower than policy. If it starts deciding whether to fall back to a built-in table, that decision belongs outside this component.
- The canonical loaded-table surface should be the only cross-component output here; no extra helper should be added unless later code names the need.
- The design should preserve the full table payload so later consumers are not forced to rediscover or re-read the on-disk bytes.
