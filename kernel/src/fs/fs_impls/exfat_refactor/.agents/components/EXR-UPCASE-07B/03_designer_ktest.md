<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Ktest

## Metadata

- Component ID: `EXR-UPCASE-07B`
- Title: Canonical Upcase-Backed Case Folding And Name Hashing
- Status: `Specified`
- Author: `designer`
- Date: `2026-04-04`
- Task packet: `EXR-UPCASE-07B-DESIGN-20260404-1501`
- Based on architect artifact: `00_architect.md`

## Purpose

Define the checker-owned regression coverage needed to prove that the canonical upcase-backed service folds UTF-16 through the loaded table before hashing, that `fileset.rs` consumes that service, and that the provisional raw-UTF-16 hash path no longer defines the component boundary.

## Test Ownership

- Owner: checker
- Test style: `#[ktest]`
- Placement: local to `upcase_table.rs` and, if needed, the `fileset.rs` test block for the consumer-side regression
- Helper touch: tests may inspect module-private state when needed to prove the canonical surface stays read-only; no public accessor expansion is required for this component

## Required Coverage

### Scenario 1: Canonical fold-then-hash matches the loaded upcase table

- Test intent:
  - Confirm the canonical service uppercases logical UTF-16 units through the loaded table before deriving `NameHash`.
- Suggested test shape:
  - Build a small loaded-table fixture with at least one non-identity case-fold mapping.
  - Feed a name containing the mapped code unit and compare the produced hash with a manually folded expected value.
- Assertions:
  - The service returns the expected exFAT hash for the folded bytes.
  - The result changes when a code unit folds to a different value, proving the hash is not a raw UTF-16 checksum.

### Scenario 2: The consumer path in `fileset.rs` uses the canonical service

- Test intent:
  - Confirm the stream-entry `name_hash` validation or synthesis path no longer depends on the provisional raw-UTF-16 helper.
- Suggested test shape:
  - Construct a file-record fixture whose name units require folding.
  - Compare the stream-entry hash against the canonical service result, not the raw checksum path.
- Assertions:
  - The file-record validates only when the canonical table-backed hash is used.
  - The old raw-UTF-16 checksum result is not accepted as the canonical answer when folding changes the bytes.

### Scenario 3: The loaded table remains a read-only normalization source

- Test intent:
  - Confirm the canonical service reads from the loaded table without mutating it or synthesizing an alternate table.
- Suggested test shape:
  - Call the service multiple times on the same loaded table and compare outputs.
- Assertions:
  - Repeated calls are stable.
  - The loaded table contents remain unchanged.

### Scenario 4: Full-table coverage survives beyond the legacy prefix boundary

- Test intent:
  - Confirm the canonical service uses the full loaded table, not a compatibility prefix.
- Suggested test shape:
  - Use a fixture whose interesting mapping lies beyond the old 128-entry assumption.
- Assertions:
  - The canonical service still folds the later mapping correctly.
  - The test would fail if only a truncated prefix were available.

### Scenario 5: The component does not expose a separate public fold-only API unless justified

- Test intent:
  - Confirm the checker can prove the contract using the canonical service alone.
- Suggested test shape:
  - Keep the regression local to the canonical service and its consumer.
- Assertions:
  - No additional public helper is required for the checked behavior.
  - If a fold-only helper exists, it remains an internal implementation detail and is not needed by the checker.

## Observability

- The tests should stay local to the component files named in the task packet.
- The tests should inspect only the folding and hashing contract, plus the consumer-side wiring in `fileset.rs` if needed.
- They should not require `PageCache`, `PageCacheBackend`, mount sequencing, directory mutation, lookup orchestration, or async harnesses.
- They should not introduce a separate helper module unless the local `upcase_table.rs` or `fileset.rs` test block becomes unexpectedly crowded.
- No dedicated concurrency tests required.

## Minimal Checker Obligation

The checker must include one regression that explicitly proves the component is table-backed and not a raw-UTF-16 hash helper. The fold-then-hash regression can satisfy that obligation if it shows a folded code unit changes the hash result.

## Exit Condition

The ktest plan is complete when a future checker can implement the coverage using only local `#[ktest]` blocks and verify both of these statements:

1. the canonical service folds logical UTF-16 through the loaded upcase table before hashing,
2. `fileset.rs` consumes that canonical service instead of preserving a raw-UTF-16 name-hash path.
