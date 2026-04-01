<!-- SPDX-License-Identifier: MPL-2.0 -->

# Reviewer Report

## Findings

- [P2] Keep the explicit data-cluster predicates as the canonical API. `super_block.rs` still exposes `is_valid_cluster()` and `is_cluster_range_valid()` as compatibility aliases, and `cluster_data_index()` continues to call `is_valid_cluster()` internally. That preserves the old count-derived naming at the surface and leaves the new `data_cluster_*` helpers optional rather than authoritative, which weakens the cleanup goal of making the legal range obvious at the call site. Consider routing internal users through `is_data_cluster_id()` / `is_data_cluster_range()` and dropping the aliases once the call sites have moved.

## Residual Risks

- I did not run kernel build or ktest commands in this role, per the reviewer packet.
- The range arithmetic itself looks correct: `data_cluster_last_id()` and `data_cluster_end_exclusive()` line up with the exFAT `ClusterCount + 1` / `+ 2` semantics.

## Assessment

The geometry change is directionally correct, but the compatibility layer still leaves some of the old invariant naming in place. I would not call the naming cleanup fully complete until the explicit helpers become the sole local convention.
