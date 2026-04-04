<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Ktest

## Metadata

- Component ID: `EXR-SYSROOT-06`
- Title: Root-Directory System-Entry Scanner
- Status: `Specified`
- Author: `designer`
- Date: `2026-04-04`
- Task packet: `EXR-SYSROOT-06-DESIGN-20260404-1408`
- Based on architect artifact: `00_architect.md`

## Purpose

Define the minimal checker-owned regression coverage needed to prove that the root scanner discovers the `BITMAP` and `UPCASE` root entries, preserves their discovery facts, and stays out of mount, page-cache, and general directory behavior.

## Test Ownership

- Owner: checker
- Test style: `#[ktest]`
- Placement: local to `sysroot.rs`
- Helper touch: tests may inspect module-local private fields if the result aggregate keeps them private; no production getter surface is required for this component

## Required Coverage

### Scenario 1: Mixed-root discovery preserves the bitmap and upcase facts

- Test intent:
  - Confirm the scanner can find the two required root system entries even when the root directory also contains unrelated content.
- Suggested test shape:
  - Build a small synthetic root fixture with valid `BITMAP` and `UPCASE` entries plus at least one unrelated non-target entry.
- Assertions:
  - The scan succeeds.
  - The bitmap discovery record is present and matches the on-disk start cluster, byte size, and entry location.
  - The upcase discovery record is present and matches the on-disk start cluster, byte size, checksum, and entry location.
  - Unrelated root content does not alter the returned discovery facts.

### Scenario 2: Duplicate root system entries are rejected

- Test intent:
  - Confirm the scanner does not silently pick the first or last duplicate.
- Suggested test shape:
  - Duplicate either the bitmap entry or the upcase entry in an otherwise valid root fixture.
- Assertions:
  - The scan returns an error.
  - The error is surfaced to the caller instead of being normalized away.

### Scenario 3: Missing root system entries are rejected

- Test intent:
  - Confirm the scanner owns the missing-entry boundary.
- Suggested test shape:
  - Remove the bitmap entry, remove the upcase entry, and remove both in separate fixtures if practical.
- Assertions:
  - The scan returns an error when either required discovery fact is absent.
  - The error is surfaced instead of returning a partial result.

### Scenario 4: Malformed root entry metadata is rejected

- Test intent:
  - Confirm the scanner rejects structurally invalid discovery facts before later loaders see them.
- Suggested test shape:
  - Use a root entry with an illegal start cluster, an unrepresentable size, or another malformed payload that the scanner boundary can detect without loading content.
- Assertions:
  - The scan returns an error.
  - The malformed fact is not normalized into a discovery record.

### Scenario 5: Discovery facts stay read-only and loader-shaped

- Test intent:
  - Confirm the returned aggregate is discovery data only.
- Assertions:
  - The result exposes the preserved discovery facts and no loaded bitmap or upcase content.
  - The `UPCASE` checksum is available as discovery data.
  - No page-cache, VFS, mount-sequencing, or async harness is needed.

## Observability

- The tests should stay local to `sysroot.rs`.
- The tests should inspect only the scanner result and the error path.
- They should not require `PageCache`, `PageCacheBackend`, buffered I/O, mount sequencing, directory mutation, or VFS trait coverage.
- They should not introduce a separate helper module unless the local `sysroot.rs` test block becomes unexpectedly crowded.
- No dedicated concurrency tests required.

## Minimal Checker Obligation

The checker must include one regression that explicitly proves the scanner is not a general directory API. The mixed-root discovery test can satisfy that obligation if it shows unrelated root content does not force the caller to perform a second scan or parse a full namespace operation.

## Exit Condition

The ktest plan is complete when a future checker can implement the coverage using only local `#[ktest]` blocks and can verify both of these statements:

1. the scanner returns one read-only discovery aggregate with the bitmap and upcase facts preserved,
2. the scanner rejects duplicate, missing, and malformed root system entries without pulling in mount or async behavior.
