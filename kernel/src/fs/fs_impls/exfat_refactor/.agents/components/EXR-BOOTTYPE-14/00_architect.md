<!-- SPDX-License-Identifier: MPL-2.0 -->

# Architect Handoff

## Metadata

- Component ID: EXR-BOOTTYPE-14
- Title: Validated Boot Sector Typing Boundary
- Status: `Architected`
- Author: main-agent
- Date: 2026-04-01

## Purpose

Replace the implicit "boot sector must already be validated before superblock normalization" convention with an explicit validated type boundary that later code can rely on without reading call-order assumptions out of the caller.

This is a narrow follow-up cleanup on the accepted bootstrap slice. It should not widen into new mount logic, backup-boot policy, or unrelated parser changes.

## Why This Comes Now

The current code already has a reviewer-identified hidden precondition: `ExfatSuperBlock::from(ExfatBootSector)` is only correct after `validate_primary_boot_sector` succeeds. That boundary is small, self-contained, and safe to tighten now before more downstream components begin depending on the current loose conversion shape.

Doing this now reduces future cleanup cost in `CHAIN`, `DENTRY`, `INODE`, and mount-path work, because later code can depend on an explicit validated bootstrap state instead of a convention.

## Dependency Contract

- Depends on:
  - `EXR-BOOT-01`
- Blocks:
  - none strictly, but it should land before substantial new bootstrap consumers accumulate.
- Can run in parallel with:
  - the current architect replanning of umbrella components in `COMPONENT_INDEX.md`
- Recommended parallel wave:
  - bootstrap-boundary cleanup plus next-wave architecture replanning
- Stable pre-existing interfaces used:
  - `ExfatBootSector`
  - `read_primary_boot_sector`
  - `validate_primary_boot_sector`
  - `verify_primary_boot_region_checksum`
  - `ExfatSuperBlock`

## exFAT Concepts Covered

- Boot-sector structural validation.
- Main Boot Region checksum validation.
- Normalized superblock construction after validation.
- Type-level expression of "trusted boot metadata" versus raw on-disk bytes.

## Target Files

- Existing files likely to change:
  - `kernel/src/fs/fs_impls/exfat_refactor/boot_sector.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/super_block.rs`
- New files expected:
  - none

## Code Budget

- Target new or heavily rewritten code size: `120-220` lines
- Reason if the budget might exceed 500 lines:
  - It should not. If the work starts pulling in mount-object creation, broader error taxonomy changes, or backup-boot policy, the boundary is wrong and the component must be cut back.

## Exit Condition

Design work may start once the component is understood as exactly:

1. introducing an explicit validated boot-sector representation or equivalent type boundary,
2. making superblock normalization consume that validated representation instead of a raw unchecked `ExfatBootSector`,
3. preserving the current read-only bootstrap behavior and current validation semantics,
4. keeping all later mount, FAT, inode, and namespace work out of scope.

## Risks

- The type boundary must not accidentally duplicate large amounts of boot-sector storage or create awkward ownership churn without reason.
- The cleanup must not silently weaken existing validation coverage or reorder checksum validation.
- The public or module-visible API should become stricter, not more sprawling.
- Tests should prove the typed boundary is actually exercised, not merely renamed.
