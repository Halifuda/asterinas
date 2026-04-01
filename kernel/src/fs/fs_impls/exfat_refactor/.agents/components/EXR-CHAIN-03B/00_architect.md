<!-- SPDX-License-Identifier: MPL-2.0 -->

# Architect Handoff

## Metadata

- Component ID: EXR-CHAIN-03B
- Title: Chain State And Read-Only Cluster Walking
- Status: `Architected`
- Author: main-agent
- Date: 2026-04-01

## Purpose

Introduce the smallest chain-layer building block needed by later inode and read-mapping work: a typed chain state plus read-only traversal across contiguous and FAT-backed cluster sequences.

This component stops before cluster allocation, freeing, truncation, or bitmap mutation. It also stops before any filesystem-wide mount object is required.

## Why This Comes Now

`EXR-FATVAL-03A` already isolates raw FAT value decoding and one-step next-cluster interpretation.
`EXR-BOOT-01` and `EXR-IO-02` already provide validated geometry and metadata-byte reads.
`EXR-CHAIN-03B` is the next dependency-safe layer because it turns those facts into reusable chain navigation without pulling in namespace, inode, or write-side policy.

The legacy `exfat/fat.rs` mixes chain walking with allocation and truncation. That is too broad for this component; keep those behaviors out so the chain slice remains reviewable and does not absorb later bitmap work.

## Dependency Contract

- Depends on:
  - `EXR-BOOT-01`
  - `EXR-IO-02`
  - `EXR-FATVAL-03A`
- Blocks:
  - `EXR-INOKEY-05A`
  - `EXR-INODE-05B`
  - `EXR-READ-11A`
  - later write-side chain consumers
- Can run in parallel with:
  - no same-wave sibling inside this chain slice
- Recommended parallel wave:
  - this component is the prerequisite for the next read-oriented wave; once it lands, `EXR-INOKEY-05A` and `EXR-READ-11A` can be planned independently on top of the chain facts
- Stable pre-existing interfaces used:
  - `read_next_fat_value`
  - `read_metadata_bytes`
  - `ExfatSuperBlock`
  - `BlockDevice`
  - existing kernel error conventions

## exFAT Concepts Covered

- Chain state with current cluster, cluster count, and FAT-vs-contiguous mode.
- Read-only traversal over contiguous chains without FAT reads.
- Read-only traversal over FAT-backed chains using validated decoded FAT values.
- Counting clusters from the chain head when the length is not known up front.
- Walking to a cluster at a byte offset within a chain.
- Translating chain position into physical cluster placement when needed by later read mapping.
- Rejecting invalid source clusters, invalid step counts, and invalid next-cluster targets.

## Target Files

- Existing files likely to change:
  - `kernel/src/fs/fs_impls/exfat_refactor/fat.rs`

## Code Budget

- Target new or heavily rewritten code size: `220-320` lines
- Reason if the budget might exceed 500 lines:
  - It should not if allocation, free, and truncation stay out of scope. If those behaviors reappear, the boundary is wrong and the component should be kept read-only rather than expanded.

## Exit Condition

Design work may start once the component is understood as exactly:

1. a chain state object with current cluster, cluster count, and contiguous-vs-FAT mode,
2. one constructor path that can accept known or unknown length and count via FAT only when needed,
3. read-only walking helpers for both contiguous and FAT-backed chains,
4. offset-to-chain-position helpers for later read mapping,
5. checker-owned tests for contiguous traversal, FAT-backed traversal, unknown-length counting, and invalid-step rejection,
6. no allocation, freeing, truncation, or bitmap writes.

## Risks

- The legacy chain file is much larger than this slice because it also owns allocation and truncation. Those behaviors must stay out of `EXR-CHAIN-03B` or the component will cease to be dependency-safe.
- FAT-backed traversal must continue to rely on the validated decoded values from `EXR-FATVAL-03A`; it should not re-interpret raw entries inline.
- Empty-chain semantics need to stay explicit so later write-side work can decide how to represent and extend them without changing the chain API.
- If the implementation pressure starts pushing this past the budget, that is a sign the slice has drifted, not a reason to widen the component here.
