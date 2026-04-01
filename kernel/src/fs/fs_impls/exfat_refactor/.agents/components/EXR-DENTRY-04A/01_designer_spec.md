<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Specification

## Metadata

- Component ID: EXR-DENTRY-04A
- Title: Raw Dentry Layout And Typed Single-Entry Decode
- Status: `Specified`
- Author: main-agent
- Date: 2026-04-01
- Based on architect artifact: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DENTRY-04A/00_architect.md`

## Scope

- In scope:
  - Define the raw 32-byte dentry struct.
  - Define a typed `ExfatDentry` enum for one-entry classification.
  - Implement `TryFrom<RawExfatDentry>` or an equivalent single-entry decode path.
  - Cover representative typed variants needed by later file-record parsing.
  - Add checker-owned tests for representative entry kinds, deleted/unused handling, and fallback generic-primary/generic-secondary classification.
- Out of scope:
  - `ExfatDentrySet`
  - file-record checksum verification
  - name aggregation across multiple entries
  - directory scanning from disk
  - inode conversion

## Module Specification

- Dependencies:
  - accepted boot constants or local dentry constants needed for type-byte interpretation
  - basic byte-casting or `Pod` support already used in the kernel
- Interfaces provided:
  - `pub(super) const DENTRY_SIZE: usize = 32`
  - `RawExfatDentry`
  - `ExfatDentry` enum with at least:
    - `File`
    - `Stream`
    - `Name`
    - `Bitmap`
    - `Upcase`
    - `VendorExt`
    - `VendorAlloc`
    - `GenericPrimary`
    - `GenericSecondary`
    - `Deleted`
    - `Unused`
  - representative packed sub-structs for the concrete typed entry variants listed above
  - a single-entry decoding path from `RawExfatDentry` into `ExfatDentry`
- Files or modules touched:
  - `kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/dentry.rs`
- Hidden implementation details:
  - whether helpers are `const` classification functions or inline match logic,
  - whether concrete typed entry wrappers expose narrow accessors or fields directly inside the module.

## Functional Specification

- `RawExfatDentry` must remain a 32-byte packed representation of one on-disk directory entry.
- Single-entry decode:
  - reads the type byte,
  - returns `Unused` for `0x00`,
  - returns `Deleted` for `0x01..0x7F`,
  - returns concrete typed variants for special known kinds such as file, stream, name, bitmap, upcase, vendor-ext, and vendor-alloc,
  - falls back to `GenericPrimary` for unrecognized primary entries,
  - falls back to `GenericSecondary` for unrecognized secondary entries.
- This component must not decide whether a sequence of entries forms a valid file record.

## Invariants

- `size_of::<RawExfatDentry>() == DENTRY_SIZE`.
- One-entry decode is deterministic and based only on the current entry bytes.
- `Unused` and `Deleted` remain distinct states.
- Concrete known types are recognized before the broader primary/secondary fallback ranges.

## Concurrency Specification

- Shared state:
  - none beyond immutable input bytes
- Lock ordering:
  - none
- Atomicity requirements:
  - none beyond ordinary by-value decode
- Forbidden interleavings:
  - no caches, no shared mutable state, no multi-entry validation state machine
- Allowed simplifications such as a temporary big lock:
  - no concurrency work is required for this component

## Pass Split

### Serial Creator Pass

- Required implementation obligations:
  - add `dentry.rs`,
  - wire it into `mod.rs`,
  - implement the raw layout and single-entry decode path,
  - keep the concrete type surface just large enough for later file-record parsing.
- Explicit non-goals:
  - no dentry-set validation,
  - no checksum state machine,
  - no page-cache or inode logic.

### Serial Checker Pass

- Required checker-owned tests:
  - a size/layout test for `RawExfatDentry`,
  - representative decode tests for known concrete kinds,
  - a deleted/unused classification test,
  - a generic primary/secondary fallback test.
- Observable properties that must pass before leaving the serial loop:
  - all representative type-byte classifications match the spec,
  - concrete special kinds are not swallowed by the generic fallback ranges,
  - the module stays limited to one-entry decode.

### Concurrency Creator Pass

- Required implementation obligations:
  - no dedicated concurrency implementation required
- Explicit non-goals:
  - do not add state-machine or cache structures under the guise of concurrency work

### Concurrency Checker Pass

- Required checker-owned concurrency tests:
  - No dedicated concurrency tests required.
- Observable properties that must pass before leaving the concurrency loop:
  - the component remains pure value decoding.

## Acceptance Notes

- Reviewer should check that concrete special kinds are matched before the generic fallback ranges.
- Reviewer should reject any attempt to sneak `ExfatDentrySet` or checksum-state-machine logic into this component.
