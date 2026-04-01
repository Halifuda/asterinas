<!-- SPDX-License-Identifier: MPL-2.0 -->

# Architect Handoff

## Metadata

- Component ID: EXR-FATVAL-03A
- Title: FAT Entry Value Model And Single-Step Next-Cluster Decode
- Status: `Architected`
- Author: main-agent
- Date: 2026-04-01

## Purpose

Introduce the smallest FAT-facing building block needed by later chain walking: a typed FAT entry value model plus one-step decoding of a single FAT entry for a validated cluster identifier.

This component intentionally stops before chain traversal, cluster counting, contiguous-chain logic, or inode mapping.

## Why This Comes Now

`EXR-BOOT-01` and `EXR-IO-02` already provide validated geometry, cluster validation, and metadata-byte reads.
Later `EXR-CHAIN-03B` depends on typed FAT entry interpretation, but the value model and one-step decode are small enough to stand on their own and remain reviewable.

## Dependency Contract

- Depends on:
  - `EXR-BOOT-01`
  - `EXR-IO-02`
- Blocks:
  - `EXR-CHAIN-03B`
- Can run in parallel with:
  - `EXR-DENTRY-04A`
- Recommended parallel wave:
  - `EXR-FATVAL-03A` plus `EXR-DENTRY-04A`
- Stable pre-existing interfaces used:
  - `read_metadata_bytes`
  - `ExfatSuperBlock`
  - accepted boot or geometry constants from `boot_sector.rs`
  - existing kernel error conventions

## exFAT Concepts Covered

- FAT entry width and on-disk little-endian decoding.
- Special FAT values:
  - free
  - bad
  - end-of-chain
  - next-cluster
- Mapping a cluster identifier to its FAT entry byte offset.
- Rejecting invalid source clusters and invalid next-cluster targets.

## Target Files

- Existing files likely to change:
  - `kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- New files expected:
  - `kernel/src/fs/fs_impls/exfat_refactor/fat.rs`

## Code Budget

- Target new or heavily rewritten code size: `180-260` lines
- Reason if the budget might exceed 500 lines:
  - It should not. If chain walking, allocation policy, bitmap coupling, or inode offset logic starts to appear, the scope is wrong.

## Exit Condition

Design work may start once the component is understood as exactly:

1. a `FatValue`-style enum and raw-value conversion rules,
2. one helper to decode a FAT entry for a single validated cluster,
3. no cluster-chain traversal, counting, or allocation semantics,
4. checker-owned tests for raw decoding and minimal on-disk reads.

## Risks

- The design must keep raw FAT value decoding separate from chain policy.
- Special marker handling must not accidentally accept reserved or out-of-range next-cluster targets as ordinary successors.
- The helper API should stay narrow enough that later chain work can build on it without inheriting allocation or caching policy.
