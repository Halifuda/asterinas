<!-- SPDX-License-Identifier: MPL-2.0 -->

# Architect Handoff

## Metadata

- Component ID: EXR-DENTRY-04A
- Title: Raw Dentry Layout And Typed Single-Entry Decode
- Status: `Architected`
- Author: main-agent
- Date: 2026-04-01

## Purpose

Introduce the smallest dentry-layer building block needed by later file-record parsing: the raw 32-byte dentry layout plus one-entry decoding into typed variants.

This component stops before multi-entry file-record validation, checksum matching, or name aggregation.

## Why This Comes Now

The legacy exFAT implementation mixes raw dentry decoding with the later file-record state machine.
Splitting the raw single-entry layer first keeps the entry taxonomy reviewable and gives `EXR-FILESET-04B` a clean dependency.

## Dependency Contract

- Depends on:
  - `EXR-BOOT-01`
- Blocks:
  - `EXR-FILESET-04B`
- Can run in parallel with:
  - `EXR-FATVAL-03A`
- Recommended parallel wave:
  - `EXR-FATVAL-03A` plus `EXR-DENTRY-04A`
- Stable pre-existing interfaces used:
  - basic byte reinterpretation utilities already accepted in the kernel,
  - exFAT constants from the legacy implementation as a semantic reference,
  - existing kernel error conventions

## exFAT Concepts Covered

- 32-byte directory-entry layout.
- Entry-type byte classification.
- Typed decoding of:
  - file
  - stream
  - name
  - bitmap
  - upcase
  - vendor extension
  - vendor allocation
  - generic primary
  - generic secondary
  - deleted
  - unused

## Target Files

- Existing files likely to change:
  - `kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- New files expected:
  - `kernel/src/fs/fs_impls/exfat_refactor/dentry.rs`

## Code Budget

- Target new or heavily rewritten code size: `200-280` lines
- Reason if the budget might exceed 500 lines:
  - It should not if checksum logic and multi-entry validation stay out of scope. If the file-record state machine starts appearing here, the boundary is wrong.

## Exit Condition

Design work may start once the component is understood as exactly:

1. the raw 32-byte dentry representation,
2. one-entry typed decoding based on the type byte,
3. no multi-entry state machine,
4. checker-owned tests for entry-kind classification and representative typed decodes.

## Risks

- The design must not let this component absorb `ExfatDentrySet` validation.
- Type-byte classification order matters because some ranges overlap special concrete entry kinds.
- The API should make later file-record parsing build on typed entries instead of redoing raw byte classification.
