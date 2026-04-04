<!-- SPDX-License-Identifier: MPL-2.0 -->

# Architect Handoff

## Metadata

- Component ID: `EXR-UPCASE-07A`
- Title: On-Disk Upcase Table Loader And Validator
- Status: `Architected`
- Author: `architect`
- Date: `2026-04-04`
- Task packet: [`20260404-1410-architect-packet.md`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-UPCASE-07A/20260404-1410-architect-packet.md)

## Purpose

This handoff covers the smallest useful exFAT component that loads the upcase-table payload identified by `EXR-SYSROOT-06`, validates its on-disk shape and checksum, and materializes one canonical loaded-table surface for later case-folding code. It stops before case folding, name hashing, mount policy, NLS policy, or general filename conversion.

The component owns only the on-disk table loading and validation boundary. It does not rescan the root directory, it does not decide whether a missing table should fall back to a built-in default, and it does not expose any name-normalization behavior.

## Why This Comes Now

`EXR-SYSROOT-06` already isolates the root-directory discovery fact that identifies the `0x82` upcase entry. `EXR-IO-02` already provides aligned metadata reads, and the accepted superblock geometry already gives the loader the exact byte-translation rules it needs. That makes this a dependency-safe read-side boundary instead of a mount-time or name-layer concern.

Microsoft exFAT and Linux both split discovery from use: first locate the root-entry metadata, then load and validate the table, then consume that table for case-insensitive name work. This component keeps the same split but narrows it further so only the loader and validator live here.

## Dependency Contract

- Depends on:
  - `EXR-IO-02`
  - `EXR-CHAIN-03B`
  - `EXR-FILESET-04B`
  - `EXR-SYSROOT-06`
- Blocks:
  - `EXR-UPCASE-07B`
- Can run in parallel with:
  - `EXR-BITMAP-08A` architect or designer work once the `EXR-SYSROOT-06` discovery contract is fixed
  - other command-free planning that consumes the same root-entry discovery facts but writes a disjoint component
- Recommended parallel wave:
  - keep `EXR-UPCASE-07A` and `EXR-BITMAP-08A` as sibling loaders after `EXR-SYSROOT-06`, with `EXR-UPCASE-07B` held back until the canonical loaded-table surface is accepted
- Stable pre-existing interfaces used:
  - the validated root-entry discovery result from `EXR-SYSROOT-06`
  - `ExfatSuperBlock` geometry helpers from `super_block.rs`
  - aligned metadata reads from `io.rs`
  - the legacy upcase-loading split in `kernel/src/fs/fs_impls/exfat/upcase_table.rs` as integration pressure only
- Prior sources or prior slices that materially shaped the split:
  - `Microsoft-exFAT-spec.md` for the `0x82` upcase-entry semantics, table checksum rules, and strict case-insensitive model
  - `linux-exFAT-implementation-summary.md` plus Linux `fs/exfat/nls.c` and `fs/exfat/super.c` for the discovery-then-load control flow and checksum behavior
  - `EXR-SYSROOT-06` for the local rule that root-entry discovery is owned elsewhere
  - `ASTERINAS_ARCHITECT_PRIORS.md` for the local boundary between discovery, loading, and later case-folding work

## exFAT Concepts Covered

- Root-directory discovery of the `0x82` upcase-table entry.
- On-disk upcase-table size validation.
- On-disk upcase-table checksum validation.
- Loading the table bytes into one canonical in-memory surface.
- Rejecting missing, malformed, or checksum-mismatched table data.
- Preserving the split between loading and later case folding.
- Excluding mount policy, default-table fallback policy, name hashing, and UTF-8 or NLS conversion.

## Target Files

- Existing files likely to change:
  - `kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- New files expected:
  - `kernel/src/fs/fs_impls/exfat_refactor/upcase_table.rs`

## Code Budget

- Target new or heavily rewritten code size:
  - `180-260` lines
- Reason if the budget might exceed 500 lines:
  - It should not. If the slice grows much beyond this, it is probably absorbing case folding, default-table fallback policy, or name-hash plumbing that belong in `EXR-UPCASE-07B` or later mount/name work.

## Exit Condition

Design work may start once the component is defined as exactly this and nothing more:

1. one loader entry point that accepts the validated upcase-entry facts from `EXR-SYSROOT-06`,
2. one canonical loaded-table type or equivalent surface for later consumers,
3. explicit rejection of missing-entry, size, geometry, or checksum failures,
4. no case-folding, no name-hash API, no mount bootstrap, and no built-in fallback-table policy.

## Risks

- The loader could drift into the legacy fallback-default-table path. That policy belongs outside this component.
- The loader could truncate the table to the legacy 128-entry prefix and accidentally starve `EXR-UPCASE-07B` of the full canonical table surface.
- The loader could start rediscovering the root entry instead of trusting `EXR-SYSROOT-06`.
- The loader could accrete case-folding, hashing, or charset helpers. Those belong in later components.
- The loader could become a hidden mount-state object if it starts owning filesystem-wide policy instead of a narrow validated table value.
