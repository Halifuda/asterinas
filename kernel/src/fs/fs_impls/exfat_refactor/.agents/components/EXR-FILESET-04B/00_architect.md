<!-- SPDX-License-Identifier: MPL-2.0 -->

# Architect Handoff

## Metadata

- Component ID: EXR-FILESET-04B
- Title: Validated File-Record Set And Raw Name Aggregation
- Status: `Architected`
- Author: main-agent
- Date: 2026-04-01

## Purpose

Introduce the smallest multi-entry exFAT file-record layer needed by later inode and namespace work: a validated file-record set object that consumes typed dentries, enforces the `File -> Stream -> Name* -> benign secondary*` shape, aggregates raw name data, verifies and updates the record checksum, and can serialize a complete set back to bytes for later write-side consumers.

This component stops before directory iteration, inode identity, FAT-chain mapping, and namespace mutation policy.

## Why This Comes Now

`EXR-DENTRY-04A` already isolates raw 32-byte entry classification. The next dependency-safe step is to consume those typed entries as one validated unit, because the file record is the trust boundary that later inode, sysroot, bitmap, upcase, and create/rename code all need.

Keeping this boundary separate prevents the raw dentry layer from absorbing multi-entry validation and keeps the file-record state machine reviewable.

## Dependency Contract

- Depends on:
  - `EXR-BOOT-01`
  - `EXR-DENTRY-04A`
- Blocks:
  - `EXR-INOKEY-05A`
  - `EXR-INODE-05B`
  - `EXR-SYSROOT-06`
  - `EXR-UPCASE-07A`
  - `EXR-BITMAP-08A`
  - `EXR-DIR-10`
  - `EXR-CREATE-12A`
  - `EXR-CREATE-12B`
- Can run in parallel with:
  - `EXR-FATVAL-03A`
- Recommended parallel wave:
  - `EXR-FATVAL-03A` plus `EXR-FILESET-04B`
- Stable pre-existing interfaces used:
  - typed single-entry dentry decoding from `EXR-DENTRY-04A`
  - existing kernel `Vec`, `Arc`, and page-cache byte-access facilities
  - existing kernel error conventions

## exFAT Concepts Covered

- File primary entries and stream entries.
- Name-entry aggregation across one or more name dentries.
- Benign secondary tails, including generic secondary and vendor extension/allocation entries.
- Multi-entry file-record state machine validation.
- Record checksum coverage across the full set.
- Serialization of a validated file-record set for later write-side consumers.
- Raw name-data preservation without upcase-table policy.

## Target Files

- Existing files likely to change:
  - `kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- New files expected:
  - `kernel/src/fs/fs_impls/exfat_refactor/fileset.rs`

## Code Budget

- Target new or heavily rewritten code size: `220-320` lines
- Reason if the budget might exceed 500 lines:
  - It should stay within budget if the component remains one validated file-record object plus checksum and raw-name helpers. If directory traversal, inode key derivation, chain walking, or upcase policy starts to appear, the boundary is wrong and should be split instead of expanded.

## Exit Condition

Design work may start once the component is understood as exactly:

1. a validated multi-entry file-record type built from typed dentries,
2. `File -> Stream -> Name* -> benign secondary*` ordering validation,
3. checksum calculation and update across the set,
4. raw name aggregation from name entries without upcase policy,
5. byte serialization for later write-side consumers,
6. a narrow construction path for assembling a validated set from already-known file metadata and name data,
7. no directory traversal, inode mapping, or FAT-chain semantics.

## Risks

- Do not let the state machine absorb directory iteration or inode identity.
- Do not move upcase-table loading, case folding, or name lookup here; those belong to `EXR-UPCASE-07A` and `EXR-UPCASE-07B`.
- The set object may expose several small methods, but they all belong to one invariant boundary. Splitting checksum, name reconstruction, and serialization would duplicate the same file-record contract.
- If the component starts needing more than one record shape, it is probably too broad for this wave.
