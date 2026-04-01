<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Ktest

## Metadata

- Component ID: EXR-SBGEOM-15
- Title: Explicit Data-Cluster Geometry Bounds
- Status: `Specified`
- Author: designer
- Date: 2026-04-01

## Purpose

Define the minimal checker-owned regression coverage needed to prove that the superblock geometry now treats `ClusterCount` as a count, not as a hidden inclusive bound.

## Test Ownership

- Owner: checker
- Test style: `#[ktest]`
- Placement: local to `super_block.rs`
- Helper touch: none expected

## Required Coverage

### Scenario 1: Geometry translation remains correct

- Test intent:
  - Confirm the derived geometry still translates the root cluster and valid data clusters correctly after the naming cleanup.
- Suggested test shape:
  - Reuse the existing cluster-translation scenario or an equivalent local ktest in `super_block.rs`.
- Assertions:
  - The root cluster maps to the expected sector and byte offset.
  - The new explicit data-cluster helpers agree with the boot-sector-derived geometry.

### Scenario 2: Reserved and out-of-range cluster ids are rejected

- Test intent:
  - Confirm `0` and `1` are rejected as non-data clusters.
  - Confirm the exclusive upper bound is enforced explicitly.
- Required regression:
  - Reject `cluster_count + 2` as a valid cluster id.
- Suggested assertions:
  - `is_data_cluster_id(0)` is false.
  - `is_data_cluster_id(1)` is false.
  - `is_data_cluster_id(data_cluster_end_exclusive)` is false.
  - `cluster_to_sector(0)` and `cluster_to_byte_offset(data_cluster_end_exclusive)` return errors.

### Scenario 3: Half-open range validation is explicit

- Test intent:
  - Confirm range checks are readable and use half-open semantics directly.
- Suggested assertions:
  - `is_data_cluster_range(2..data_cluster_end_exclusive)` is true.
  - `is_data_cluster_range(data_cluster_end_exclusive..data_cluster_end_exclusive)` is true.
  - `is_data_cluster_range(0..data_cluster_end_exclusive)` is false.
  - `is_data_cluster_range(2..data_cluster_end_exclusive + 1)` is false.

## Observability

- These tests should only inspect geometry and translation helpers.
- They should not require FAT, inode, bitmap, or mount coverage.
- They should not introduce a separate helper module unless the existing `super_block.rs` test block becomes too cluttered, which is not expected for this repair.

## Minimal Checker Obligation

The checker must include a regression that explicitly rejects `cluster_count + 2`. That regression can live inside the reserved/out-of-range cluster-id scenario, but it must be named and asserted clearly enough that the exclusive upper bound is obvious to future readers.

## Exit Condition

The ktest plan is complete when a future checker can implement the coverage using only `super_block.rs` tests and can verify that the last legal data-cluster id is `ClusterCount + 1` while `ClusterCount + 2` is rejected.
