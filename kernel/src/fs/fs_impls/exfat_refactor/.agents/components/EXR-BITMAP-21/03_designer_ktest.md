<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Ktest

## Metadata

- Component ID: `EXR-BITMAP-21`
- Title: `ExfatFs` Allocation Bitmap Owner State And Read-Only Accounting
- Status: `Specified`
- Author: designer
- Date: 2026-04-10
- Based on architect artifact: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BITMAP-21/00_architect.md`

## Purpose

Define the minimal checker-owned regression coverage needed to prove that the bitmap owner loads one validated immutable image, answers occupancy queries correctly, and derives free-space accounting from the same snapshot.

## Test Ownership

- Owner: checker
- Test style: `#[ktest]`
- Placement: local to `bitmap.rs` or the nearest owner-visible bitmap test block
- Helper touch: none expected

## Required Coverage

### Scenario 1: Invalid bitmap images are rejected

- Test intent:
  - Confirm the loader rejects malformed bitmap candidates before publication.
- Suggested test shape:
  - Feed a bitmap candidate with a malformed length or impossible geometry into the owner loader.
  - Exercise at least one case where the bitmap cannot cover the validated cluster range.
- Assertions:
  - The load fails.
  - No partially loaded bitmap becomes visible to later queries.

### Scenario 2: Occupancy queries match the validated bitmap image

- Test intent:
  - Confirm `cluster_is_allocated()` follows the validated bitmap bytes rather than a separate free-space hint.
- Suggested test shape:
  - Build a small bitmap fixture with both free and allocated clusters.
  - Query the first valid cluster, a middle cluster, and the last valid cluster.
- Assertions:
  - Cluster `2` maps to bit `0`.
  - Allocated clusters report allocated.
  - Free clusters report free.
  - Out-of-range cluster numbers are rejected.

### Scenario 3: Derived accounting matches occupancy

- Test intent:
  - Confirm `used_cluster_count()` and `free_cluster_count()` are derived from the same snapshot that answers occupancy queries.
- Suggested test shape:
  - Use the same bitmap fixture as Scenario 2.
  - Compare the returned counts with the expected number of set and clear bits in the valid cluster range.
- Assertions:
  - Used-cluster count matches the number of allocated bits in range.
  - Free-cluster count is the complement of the used-cluster count within the valid cluster range.
  - Padding bits beyond the valid cluster range do not affect the result.

## Observability

- These tests should inspect only the validated bitmap snapshot, occupancy queries, and derived accounting.
- They should not require allocation search, FAT mutation, directory traversal, or page-cache coverage.
- They should not introduce a separate helper module unless the bitmap test block becomes cluttered, which is not expected for this component.
- No dedicated concurrency tests are required because the only serialization concern is one-time publication of an immutable snapshot, which is already covered by the async artifact.

## Minimal Checker Obligation

The checker must include regressions that prove the bitmap owner is still a single owner-owned snapshot and not a split of separate occupancy and accounting helpers. The same regression set should show that validation happens before publication and that later reads derive from the published image.

## Exit Condition

The ktest plan is complete when a future checker can verify bitmap validation, occupancy queries, and derived accounting entirely from the bitmap owner boundary without needing allocation mutation or FAT-side behavior.
