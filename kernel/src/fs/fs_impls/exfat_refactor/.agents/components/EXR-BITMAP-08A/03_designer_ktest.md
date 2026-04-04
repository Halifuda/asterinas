<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Ktest

## Metadata

- Component ID: `EXR-BITMAP-08A`
- Title: Allocation Bitmap Loader And Read-Only Occupancy Queries
- Status: `Specified`
- Author: `designer`
- Date: `2026-04-04`
- Task packet: `EXR-BITMAP-08A-DESIGN-20260404-1414`
- Based on architect artifact: `00_architect.md`

## Purpose

Define the minimal checker-owned regression coverage needed to prove that the bitmap loader accepts a validated discovery record, rejects malformed bitmap payloads, and exposes only read-only occupancy queries.

## Test Ownership

- Owner: checker
- Test style: `#[ktest]`
- Placement: local to `bitmap.rs`
- Helper touch: tests may inspect module-local private fields if the loaded bitmap keeps them private; no production getter surface is required beyond the canonical occupancy API

## Required Coverage

### Scenario 1: Happy-path loading preserves occupancy facts

- Test intent:
  - Confirm the loader can materialize the allocation bitmap from the validated discovery facts and answer occupancy queries correctly.
- Suggested test shape:
  - Load the standard exFAT test image, obtain the validated bitmap discovery record from `EXR-SYSROOT-06`, and load the bitmap surface.
- Assertions:
  - The load succeeds.
  - At least one known allocated data cluster reports occupied.
  - At least one known free data cluster reports unoccupied.
  - The returned bitmap surface is read-only and does not expose mutation behavior.

### Scenario 2: Undersized bitmap payloads are rejected

- Test intent:
  - Confirm the loader owns the minimum-size boundary.
- Suggested test shape:
  - Corrupt the bitmap file or its metadata so the discovered byte size is smaller than the minimum needed for the volume's data-cluster count.
- Assertions:
  - The load returns an error.
  - No bitmap surface is exposed for an undersized payload.

### Scenario 3: The bitmap file's own clusters must be marked allocated

- Test intent:
  - Confirm the loader rejects a bitmap that would not even describe its own on-disk footprint correctly.
- Suggested test shape:
  - Clear one bit that corresponds to a cluster occupied by the bitmap file itself, then attempt to load the bitmap surface.
- Assertions:
  - The load returns an error.
  - The malformed payload is not normalized into a usable surface.

### Scenario 4: Out-of-range occupancy queries are rejected

- Test intent:
  - Confirm the query surface does not treat reserved ids or tail padding as real volume space.
- Suggested test shape:
  - Query cluster ids `0`, `1`, and `ClusterCount + 2` after a successful load.
- Assertions:
  - Each out-of-range query returns an error.
  - The query surface does not silently coerce reserved ids into ordinary data clusters.

### Scenario 5: Oversized payloads are acceptable when the geometry is covered

- Test intent:
  - Confirm the loader accepts a bitmap that is larger than the minimum required size.
- Suggested test shape:
  - Extend the bitmap payload with extra bytes beyond the minimum while keeping the covered data-cluster bits valid.
- Assertions:
  - The load succeeds.
  - Occupancy queries within the legal data-cluster range still behave correctly.
  - The extra tail bytes do not become additional real clusters.

## Observability

- The tests should stay local to `bitmap.rs`.
- The tests should inspect only the loader result and the occupancy query path.
- They should not require page cache, mount sequencing, VFS trait coverage, background tasks, or search-policy helpers.
- They should not introduce a separate helper module unless the local `bitmap.rs` test block becomes unexpectedly crowded.
- No dedicated concurrency tests required.

## Minimal Checker Obligation

The checker must include one regression that explicitly proves the bitmap component is not an allocation-policy engine. The happy-path load test can satisfy that obligation if it shows read-only occupancy queries only, with no free-space search or mutation behavior required to validate the result.

## Exit Condition

The ktest plan is complete when a future checker can implement the coverage using only local `#[ktest]` blocks and can verify both of these statements:

1. the loader returns one read-only bitmap surface that preserves valid occupancy facts and rejects undersized or malformed payloads,
2. the query surface rejects out-of-range cluster ids without introducing search, hint, or mutation behavior.
