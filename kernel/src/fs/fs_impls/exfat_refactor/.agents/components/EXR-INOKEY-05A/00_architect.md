<!-- SPDX-License-Identifier: MPL-2.0 -->

# Architect Handoff

## Metadata

- Component ID: EXR-INOKEY-05A
- Title: Inode Identity Key And Opened-Inode Lookup
- Status: `Architected`
- Author: main-agent
- Date: 2026-04-01

## Purpose

Introduce the smallest dependency-safe identity slice for exFAT inodes: derive the opened-inode key from a validated on-disk primary-dentry location, preserve the root special case, and expose read-only opened-inode lookup helpers that reuse that key.

This component stops before inode metadata shaping, page-cache behavior, directory iteration, mount sequencing, and any VFS-facing inode behavior. If registry maintenance starts to pull in inode creation, eviction, or parent tracking, that is too broad and must be split again; the narrower fallback is just the pure key helper plus read-only lookup accessors.

## Why This Comes Now

`EXR-CHAIN-03B` already gives the validated cluster-walking facts needed to locate a dentry set without reinterpreting FAT state, and `EXR-FILESET-04B` already gives the validated multi-entry file-record boundary that later inode code can trust.

The legacy Asterinas implementation shows the identity rule clearly:

- `utils.rs` packs the key from `(cluster, offset)`.
- `inode.rs` treats the root inode as a reserved special case.
- `fs.rs` stores opened inodes in a hash table indexed by that key.

The Linux implementation summary confirms the same identity shape: inode cache lookup is keyed by the on-disk location of the primary directory entry, not by cluster number alone. That prior art is the direct reason this component should stay narrow and not absorb inode construction or lookup-by-name logic.

## Dependency Contract

- Depends on:
  - `EXR-CHAIN-03B`
  - `EXR-FILESET-04B`
- Blocks:
  - `EXR-INODE-05B`
  - later opened-inode consumers that need the stable keying contract
- Can run in parallel with:
  - none; this is already the narrow keying slice
- Stable prior sources that materially shaped the split:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat/inode.rs`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat/fs.rs`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat/utils.rs`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/linux-exFAT-implementation-summary.md`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_ARCHITECT_PRIORS.md`

## exFAT Concepts Covered

- The inode identity key is derived from the physical location of the primary directory entry.
- The root inode uses a reserved key and must not be packed from a `(cluster, offset)` pair.
- The packed key shape stays compatible with the legacy `(cluster << 32) | offset` convention.
- Opened-inode lookup is a key lookup against the registry, not a search over inode metadata.
- The helper assumes the caller already has a validated on-disk location; it does not discover that location itself.

## Target Files

- Existing files likely to change:
  - `kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- New files expected:
  - `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/fs.rs`

## Code Budget

- Target new or heavily rewritten code size: `120-220` lines
- Reason if the budget might exceed 400 lines:
  - It should not if the component stays limited to key derivation and read-only registry access. If inode construction, parent tracking, or VFS behavior appears, the boundary is wrong and must be split.

## Exit Condition

Design work may start once the component is understood as exactly:

1. a helper that maps a validated on-disk primary-dentry location to a stable opened-inode key,
2. a reserved root special case that bypasses location packing,
3. read-only opened-inode lookup helpers that use the derived key,
4. no inode metadata shaping, no page-cache coordination, no directory iteration, no mount sequencing, and no VFS behavior.

## Risks

- Do not let this component absorb inode creation, eviction, or parent-child propagation; those are lifecycle concerns, not identity concerns.
- Do not move directory scanning or dentry-set discovery here; the key helper must only consume already-validated location data from downstream code.
- Preserve the root special case as an explicit reserved constant, not as an incidental mount-time shortcut.
- If later implementation pressure pushes this beyond the key helper and read-only table lookup, stop and split the identity helper from any registry mutation work.
