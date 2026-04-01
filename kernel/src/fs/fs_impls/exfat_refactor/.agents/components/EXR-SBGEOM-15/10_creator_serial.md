<!-- SPDX-License-Identifier: MPL-2.0 -->

# Creator Serial Pass

## Metadata

- Component ID: EXR-SBGEOM-15
- Title: Explicit Data-Cluster Geometry Bounds
- Status: `Implemented`
- Author: creator
- Date: 2026-04-01

## Summary

Implemented the geometry cleanup in `super_block.rs` so the stored cluster count is now the raw BPB data-cluster count, while the legal-cluster bounds are exposed through explicit helpers.

## Changes

- Kept `num_clusters` as the raw data-cluster count from `ClusterCount`.
- Added explicit helpers for:
  - `data_cluster_count`
  - `data_cluster_last_id`
  - `data_cluster_end_exclusive`
  - `is_data_cluster_id`
  - `is_data_cluster_range`
- Moved the internal geometry users in `super_block.rs` and the chain call sites in `fat.rs` to the explicit helpers.
- Removed the old generic cluster-predicate aliases so the explicit names are now canonical at the local call sites.
- Updated the in-file `#[ktest]` call sites in `super_block.rs` to use the explicit bound helpers where the old `num_clusters + 1` assumption was previously embedded.

## Notes

- The change stayed inside `super_block.rs` as requested.
- I did not add new tests.
- The repair batch also updated `fat.rs` call sites to the explicit predicates, but did not widen beyond the geometry and chain files.
