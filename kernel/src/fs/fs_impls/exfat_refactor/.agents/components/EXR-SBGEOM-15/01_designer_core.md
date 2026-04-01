<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Core

## Metadata

- Component ID: EXR-SBGEOM-15
- Title: Explicit Data-Cluster Geometry Bounds
- Status: `Specified`
- Author: designer
- Date: 2026-04-01

## Purpose

Remove the misleading `num_clusters = cluster_count + 2` semantics from `super_block.rs` and replace it with field and helper names that make the data-cluster range readable at the call site.

This is a narrow repair component. It should not expand into mount logic, FAT walking, allocation, or any other exFAT subsystem.

## Scope

- In scope:
  - Make the stored superblock geometry distinguish between a data-cluster count and derived cluster-bound helpers.
  - Make valid-cluster checks read directly as range checks.
  - Make byte-offset translation rely on explicit cluster-bound helpers instead of on an overloaded `num_clusters` field.
  - Add or adjust checker-owned `#[ktest]` coverage so the `cluster_count + 2` upper bound is rejected explicitly.
- Out of scope:
  - Any production-code change outside `super_block.rs`.
  - Any change to mount, inode, FAT, bitmap, or dentry logic.
  - Any async or concurrency mechanism.
  - Any broader refactor that changes exFAT ownership or call sequencing.

## Preferred Geometry Semantics

The component should use these meanings consistently:

- `data_cluster_count`: the raw BPB `ClusterCount`, meaning the number of usable data clusters.
- `data_cluster_last_id`: the inclusive last legal data-cluster id, computed from the count and the first data cluster id.
- `data_cluster_end_exclusive`: the exclusive upper bound of legal data-cluster ids, equal to `cluster_count + 2` under the current exFAT numbering.

The design should avoid storing `cluster_count + 2` in a field named like a count. If the implementation keeps a derived bound in state, the name must say that it is a bound, not a count.

## Module Specification

- Dependencies:
  - `EXR-BOOT-01` validated boot geometry.
  - The exFAT spec note that valid data-cluster ids begin at `2`.
  - The Asterinas prior note that `ClusterCount` is a count of data clusters, not the last cluster id.
- Files to touch:
  - `kernel/src/fs/fs_impls/exfat_refactor/super_block.rs`
- Preferred API shape:
  - Keep the superblock geometry self-describing.
  - Prefer accessors or private helpers with names like `data_cluster_count`, `data_cluster_last_id`, and `data_cluster_end_exclusive`.
  - Prefer `is_data_cluster_id` and `is_data_cluster_range` over names that hide the cluster numbering rule.
  - Keep cluster-to-byte translation on the same file so the invariant is enforced at one boundary.

## Functional Specification

### Operation

- Name: `ExfatSuperBlock::from`
- Inputs:
  - `ValidatedBootSector`
- Actions:
  - Store the raw data-cluster count as a count.
  - Do not store an overloaded value whose name implies a count while its numeric meaning is actually a last id or upper bound.
  - Preserve the existing normalized geometry fields.
- Outputs:
  - `ExfatSuperBlock`

### Operation

- Name: `ExfatSuperBlock::is_data_cluster_id`
- Inputs:
  - `&self`
  - `cluster: u32`
- Actions:
  - Return whether the cluster id is in the legal data-cluster id range.
- Required semantics:
  - Legal ids start at `2`.
  - The inclusive upper bound is `data_cluster_last_id`.
- Outputs:
  - `bool`

### Operation

- Name: `ExfatSuperBlock::data_cluster_end_exclusive`
- Inputs:
  - `&self`
- Actions:
  - Return the exclusive upper bound of legal data-cluster ids.
- Required semantics:
  - The returned value is the one-past-the-end id for the legal data-cluster range.
  - This is the value that should be rejected when a caller tries to use `cluster_count + 2` as a valid cluster id.
- Outputs:
  - `u32`

### Operation

- Name: `ExfatSuperBlock::is_data_cluster_range`
- Inputs:
  - `&self`
  - `range: Range<u32>`
- Actions:
  - Return whether the half-open range lies wholly within the legal data-cluster range.
- Required semantics:
  - Accept `2..data_cluster_end_exclusive`.
  - Accept the empty range at the exclusive end.
  - Reject any range that starts before `2`.
  - Reject any range that ends after the exclusive bound.
- Outputs:
  - `bool`

### Operation

- Name: `ExfatSuperBlock::cluster_to_sector`
- Inputs:
  - `&self`
  - `cluster: u32`
- Actions:
  - Reject non-data clusters through the explicit data-cluster predicate.
  - Translate from the first data cluster, not from a hidden count-derived sentinel.
- Outputs:
  - `Result<u64>`

### Operation

- Name: `ExfatSuperBlock::cluster_to_byte_offset`
- Inputs:
  - `&self`
  - `cluster: u32`
- Actions:
  - Reuse the explicit data-cluster validation and sector translation.
- Outputs:
  - `Result<usize>`

## Invariants

- `ClusterCount` remains a count of data clusters.
- Data-cluster ids start at `2`.
- The inclusive last legal data-cluster id is `ClusterCount + 1`.
- The exclusive upper bound is `ClusterCount + 2`.
- No code should have to reason about the legal cluster range by mentally undoing a field named like a count.

## Code Budget

- Target new or heavily rewritten code size: `60-120` lines
- Reason if the budget might exceed `180` lines:
  - It should not. If the repair starts pulling in other modules or broader call-site cleanup, the component is no longer a tiny geometry fix.

## Exit Condition

Design work may start once the component is understood as exactly:

1. a `super_block.rs` geometry cleanup,
2. a small set of explicit helpers for count, inclusive last id, and exclusive upper bound,
3. a readably named cluster-range predicate,
4. checker-owned tests that prove `cluster_count + 2` is rejected.

## Risks

- The most likely failure mode is leaving the old count-derived name in place while adding new helpers around it; that would preserve the confusion instead of removing it.
- The repair should not widen into `boot_sector.rs` unless a hard dependency appears. The preferred implementation remains entirely in `super_block.rs`.
- Call sites that currently use the old `num_clusters` name may need to switch to the explicit helpers during the creator pass, but the designer should not pre-plan unrelated cleanup outside the geometry boundary.
