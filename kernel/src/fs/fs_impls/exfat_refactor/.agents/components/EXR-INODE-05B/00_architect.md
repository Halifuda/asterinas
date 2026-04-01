<!-- SPDX-License-Identifier: MPL-2.0 -->

# Architect Handoff

## Metadata

- Component ID: EXR-INODE-05B
- Title: Read-Only Inode Metadata Shell
- Status: `Architected`
- Author: main-agent
- Date: 2026-04-01

## Purpose

Introduce the smallest dependency-safe inode slice that turns validated file-record facts, validated chain facts, and accepted inode identity into a read-only inode metadata shell.

This component stops at metadata state and pure accessors. It does not own `PageCacheBackend`, buffered I/O, page-cache size coordination, directory iteration, mount sequencing, VFS-facing inode behavior, or any write/update path.

The root inode is a synthetic special case: it uses the reserved inode identity and validated root-chain facts, but it does not come from a parsed file record.

## Why This Comes Now

`EXR-FILESET-04B` already validates the file-record boundary and preserves raw name data.
`EXR-CHAIN-03B` already supplies read-only chain state and traversal facts.
`EXR-INOKEY-05A` already supplies the accepted inode identity key and exact lookup contract.

The legacy `exfat/inode.rs` shows why this component must stay narrow: it mixes metadata, page cache, buffered I/O, directory enumeration, rename propagation, and writeback state. The refactor should peel off only the metadata shell now so later work can own page-cache integration and VFS behavior separately.

`EXR-PGCACHE-11B` should stay planning-coupled with this work, but its implementation must remain blocked until this shell exists.

## Dependency Contract

- Depends on:
  - `EXR-CHAIN-03B`
  - `EXR-FILESET-04B`
  - `EXR-INOKEY-05A`
- Blocks:
  - `EXR-READ-11A`
  - `EXR-PGCACHE-11B`
  - `EXR-DIR-10`
  - `EXR-MOUNT-09`
  - `EXR-CREATE-12A`
  - `EXR-CREATE-12B`
- Can run in parallel with:
  - none at implementation time
- Stable prior sources that materially shaped the split:
  - `Microsoft-exFAT-spec.md` for file and stream metadata, checksum coverage, name length/hash, and `NoFatChain`.
  - `linux-exFAT-implementation-summary.md` for the separation between inode metadata, mapping, and page-cache behavior.
  - `ASTERINAS_ARCHITECT_PRIORS.md` for safe-Rust kernel code, VFS/page-cache constraints, and testing reality.
  - Legacy `exfat/inode.rs` and `exfat/fs.rs` as integration-pressure references only.

## exFAT Concepts Covered

- Accepted inode identity, including the reserved root special case.
- File-record metadata facts: attributes, timestamps, raw name payload, and the logical-versus-allocated size pair from the validated file record.
- Chain facts: current cluster, cluster count, and contiguous versus FAT-backed mode.
- Stream flags that matter to inode metadata, including the `NoFatChain` branch already normalized by chain facts.
- Read-only metadata accessors for identity, kind, mode, size, timestamps, raw name, and chain placement.
- The shell does not store or maintain directory-derived child counts, parent propagation state, lookup policy, or mount ownership.

This component intentionally consumes trusted parsed objects instead of raw on-disk bytes. That follows the local Rust boundary established by `EXR-FILESET-04B` and keeps dentry parsing out of the inode layer.

## Target Files

- Existing files likely to change:
  - `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`

## Code Budget

- Target new or heavily rewritten code size: `160-240` lines
- Reason if the budget might exceed 400 lines:
  - It should not if the component remains a plain metadata shell plus constructors and accessors. If buffered I/O, page-cache sizing, directory walking, or VFS methods appear, the boundary has drifted and must be split instead of widened.

## Exit Condition

Design work may start once the component is understood as exactly:

1. a read-only inode shell that can be constructed from validated file-record facts, validated chain facts, and accepted inode identity,
2. pure accessors for identity, metadata, chain state, raw name, and size/timestamp/attribute facts,
3. a root special case that is synthetic and does not require a parsed file record,
4. no `PageCacheBackend`, buffered I/O, cache-size coordination, directory iteration, mount sequencing, or VFS inode operations,
5. no directory-derived child accounting or parent-propagation logic.

## Risks

- The legacy `ExfatInode` still suggests one narrower follow-up slice after this component: a live inode adapter that owns page-cache state, buffered reads and writes, and VFS behavior. That work belongs in later components, not in this metadata shell.
- If the shell starts needing directory traversal, child counts, or parent tracking, it has crossed into `EXR-DIR-10` or namespace work.
- If the constructor starts reading raw on-disk bytes or re-parsing dentry layouts, it has drifted back into `EXR-FILESET-04B`.
- If any helper reaches for `PageCache`, `PageCacheBackend`, or inode writeback, the boundary is wrong and must be split instead of widened.
- The root shell path is the only local divergence from the "parsed file-record facts" default; keep that special case explicit rather than allowing mount sequencing to leak in.
