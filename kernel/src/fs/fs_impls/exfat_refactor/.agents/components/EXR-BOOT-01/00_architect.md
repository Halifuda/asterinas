<!-- SPDX-License-Identifier: MPL-2.0 -->

# Architect Handoff

## Metadata

- Component ID: EXR-BOOT-01
- Title: Boot Region Parsing And Normalized Runtime Geometry
- Status: `Architected`
- Author: architect
- Date: 2026-03-31

## Purpose

Establish the first trusted runtime facts for `exfat_refactor` by reading the exFAT boot region, validating mandatory on-disk boot metadata, and converting it into a normalized in-memory geometry or state object that later components can depend on.

This component stops at validated boot-region interpretation. It does not create `ExfatFs`, does not build the root inode, does not load the upcase table or allocation bitmap, and does not register a filesystem type.

## Why This Comes Now

Every later component depends on the geometry and validity rules established here:

- cluster and sector sizing,
- FAT and data-region offsets,
- root-directory start cluster,
- volume flags that must be preserved or cleared,
- physical cluster addressing constraints,
- whether the boot region is trustworthy enough to continue mounting.

This ordering is dependency-safe because no later exFAT algorithm can be implemented correctly without these facts, while boot parsing itself depends only on stable pre-existing block-device and byte-parsing interfaces.

## Dependency Contract

- Depends on: none
- Blocks:
  - `EXR-IO-02`
  - `EXR-CHAIN-03`
  - `EXR-DENTRY-04`
  - `EXR-INODE-05`
  - `EXR-MOUNT-09`
- Stable pre-existing interfaces used:
  - Asterinas block-device sector reads from the kernel block layer.
  - Safe byte parsing through existing Rust or kernel utilities already used by the legacy exFAT code.
  - Repository error or result conventions under `kernel/`.
  - Existing kernel test infrastructure for `#[ktest]`.

## exFAT Concepts Covered

- Main Boot Region layout.
- Backup Boot Region layout as a parsed concept only, not fallback policy.
- Boot-sector signature checks.
- Boot checksum algorithm over the first 11 sectors.
- BPB field validation rules.
- Sector-size and cluster-size derivation.
- FAT-region and data-region placement.
- Root-directory first-cluster extraction.
- Volume flags and persistent-flag subset handling.
- Cluster-count and geometry sanity limits.

## Target Files

- Existing files likely to change:
  - `kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- New files expected:
  - `kernel/src/fs/fs_impls/exfat_refactor/boot_sector.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/super_block.rs`

## Code Budget

- Target new or heavily rewritten code size: `250-400` lines
- Reason if the budget might exceed 500 lines:
  - It should stay below 500 lines if the scope remains limited to parsing, validation, normalized geometry, and targeted ktests.
  - If backup-boot fallback, volume-flag mutation, or mount-option policy gets pulled in, the component boundary is wrong and must be split rather than expanded.

## Exit Condition

Design work may start once the component boundary is accepted as all of the following and nothing more:

1. Read and validate the exFAT boot region from disk.
2. Expose a normalized runtime geometry or state structure for later components.
3. Define the exact error conditions for malformed boot metadata.
4. Keep backup-superblock recovery policy, filesystem-object construction, root-inode creation, bitmap loading, and upcase loading out of scope.

Observable readiness means the designer can point to exactly these touched modules and produce a complete spec without inventing inode, page-cache, or namespace behavior.

## Risks

- Backup boot region handling is ambiguous in the current legacy code, so the main agent must decide whether this component only validates the primary boot region or also specifies read-time fallback or compare behavior.
- TexFAT-style two-FAT semantics should not silently widen this component beyond geometry normalization and basic FAT placement facts.
- Volume-flag persistence rules can accidentally expand into writeback policy; this component should parse and normalize flags, not own mutation policy.
- The output API must stay narrow. If this component starts exposing `ExfatFs` or root-inode construction hooks, it has absorbed later mount work and should be re-scoped.
