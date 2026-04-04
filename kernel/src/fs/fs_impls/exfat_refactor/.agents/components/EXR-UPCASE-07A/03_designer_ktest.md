<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Ktest

## Metadata

- Component ID: `EXR-UPCASE-07A`
- Title: On-Disk Upcase Table Loader And Validator
- Status: `Specified`
- Author: `designer`
- Date: `2026-04-04`
- Task packet: `EXR-UPCASE-07A-DESIGN-20260404-1414`
- Based on architect artifact: `00_architect.md`

## Purpose

Define the checker-owned regression coverage needed to prove that the upcase-table loader accepts validated discovery facts, loads the full on-disk table, validates checksum and structure, and stays out of case folding, hashing, fallback policy, and mount behavior.

## Test Ownership

- Owner: checker
- Test style: `#[ktest]`
- Placement: local to `upcase_table.rs`
- Helper touch: tests may inspect module-private fields if the canonical loaded-table value keeps them private; no production getter expansion is required for this component

## Required Coverage

### Scenario 1: Valid discovery facts load the full canonical table

- Test intent:
  - Confirm the loader accepts the validated upcase discovery record and returns the canonical loaded-table surface.
- Suggested test shape:
  - Build a small synthetic upcase-table fixture with a valid start cluster, a valid byte size, and a matching checksum.
  - Include enough payload bytes to prove the loader does not truncate to a legacy prefix.
- Assertions:
  - The load succeeds.
  - The returned table preserves the full discovered payload.
  - The returned table exposes a read-only surface only, not a folding or hashing API.
  - The preserved checksum fact is still available to later code or equivalent metadata on the returned value.

### Scenario 2: Checksum mismatch is rejected

- Test intent:
  - Confirm the loader owns checksum validation and refuses to synthesize a table when the on-disk bytes do not match the discovery checksum.
- Suggested test shape:
  - Reuse the valid payload from Scenario 1 but change the preserved checksum fact.
- Assertions:
  - The load returns an error.
  - The error is surfaced instead of being normalized into a fallback table.

### Scenario 3: Malformed discovery facts are rejected

- Test intent:
  - Confirm the loader still validates the discovery facts at the load boundary instead of trusting malformed caller input.
- Suggested test shape:
  - Use at least one fixture with an illegal start cluster.
  - Use at least one fixture with an invalid size, such as an odd byte count or another structurally invalid table size.
- Assertions:
  - The load returns an error for each malformed fixture.
  - The malformed fact does not become a canonical table value.

### Scenario 4: Truncated payloads are rejected

- Test intent:
  - Confirm the loader rejects on-disk payloads that cannot supply the full discovered table bytes.
- Suggested test shape:
  - Point the discovery facts at a chain that ends too early or otherwise cannot cover the discovered size.
- Assertions:
  - The load returns an error.
  - The error is surfaced instead of partially filling the canonical table.

### Scenario 5: The canonical table keeps the full table, not a compatibility prefix

- Test intent:
  - Confirm the loader does not silently preserve only the legacy 128-entry subset.
- Suggested test shape:
  - Build a payload whose meaningful bytes extend past the legacy prefix boundary.
- Assertions:
  - The returned table length matches the discovered byte size.
  - Bytes beyond the legacy prefix remain visible in the returned surface.

## Observability

- The tests should stay local to `upcase_table.rs`.
- The tests should inspect only the loader result and the error path.
- They should not require `PageCache`, `PageCacheBackend`, mount sequencing, directory mutation, case folding, name hashing, or async harnesses.
- They should not introduce a separate helper module unless the local `upcase_table.rs` test block becomes unexpectedly crowded.
- No dedicated concurrency tests required.

## Minimal Checker Obligation

The checker must include one regression that explicitly proves the component is not a case-folding or name-hash API. The valid-load regression can satisfy that obligation if it shows the returned surface is just the loaded table value and nothing more.

## Exit Condition

The ktest plan is complete when a future checker can implement the coverage using only local `#[ktest]` blocks and can verify both of these statements:

1. the loader returns one canonical read-only loaded-table value with the full validated payload preserved,
2. the loader rejects malformed, truncated, and checksum-mismatched upcase-table inputs without introducing fallback policy or case-folding behavior.
