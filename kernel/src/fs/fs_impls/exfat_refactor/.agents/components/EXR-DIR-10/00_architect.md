<!-- SPDX-License-Identifier: MPL-2.0 -->

# Architect Handoff

## Metadata

- Component ID: `EXR-DIR-10`
- Title: Directory Iteration And Lookup Over Shared Filesystem State
- Status: `Architected`
- Author: `architect`
- Date: `2026-04-05`
- Task packet: [`EXR-DIR-10-ARCH-20260405-1047`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-DIR-10/20260405-1047-architect-packet.md)

## Purpose

This handoff covers the smallest useful directory component for the refactor: iteration over already-mounted exFAT directory state and lookup against validated directory records.

The component owns the directory-facing policy that walks validated file-record sets, consumes the mount-owned shared filesystem state, and compares candidate names through the canonical upcase-backed name-hash service. It does not own mount bootstrap, root discovery, page-cache-backed regular-file reads, or namespace mutation.

## Why This Comes Now

This split is safe now because the prerequisites already exist:

- `EXR-MOUNT-09` seeds the mount-owned shared filesystem state that directory work should consume instead of rediscovering.
- `EXR-FILESET-04B` provides the validated `FILE -> STREAM -> NAME* -> benign secondary*` file-record boundary that lookup should consume.
- `EXR-INODE-05B` provides the read-only inode metadata shell that directory work can read without taking over live inode behavior.
- `EXR-UPCASE-07B` provides the canonical table-backed fold-and-hash service for name matching.
- `EXR-DENTRY-04A` provides the typed dentry decoding boundary that directory traversal should not reimplement inline.

Linux makes the same split explicit: `namei.c` owns case-insensitive lookup and namespace resolution policy, while `dir.c` owns directory-entry iteration and file-record scanning. Mount/bootstrap lives elsewhere in `super.c`, not inside lookup.

## Dependency Contract

- Depends on:
  - `EXR-DENTRY-04A`
  - `EXR-FILESET-04B`
  - `EXR-INODE-05B`
  - `EXR-UPCASE-07B`
  - `EXR-MOUNT-09`
- Blocks:
  - `EXR-CREATE-12A`
  - `EXR-CREATE-12B`
  - `EXR-RENAME-13D`
- Can run in parallel with:
  - `EXR-READ-11A` once `EXR-MOUNT-09` has frozen the shared-state contract
  - command-free planning and review for later namespace work that only needs the lookup contract, not mutation details
- Recommended parallel wave:
  - keep mount bootstrap separate;
  - then let directory lookup/iteration planning run alongside read-path planning, while create/rename remain blocked behind the finished directory contract.
- Stable pre-existing interfaces used:
  - `ExfatDentrySet::new(...)` and `ExfatDentrySet::raw_name_units()` from `fileset.rs`
  - `ExfatUpcaseTable::name_hash(...)` from `upcase_table.rs`
  - `ExfatInodeMeta` from `inode.rs`
  - `ExfatChain` from `fat.rs`
  - typed dentry decoding from `dentry.rs`
- Prior sources or prior slices that materially shaped the split:
  - `Microsoft-exFAT-spec.md` for directory-entry ordering, lookup identity, and name-hash rules.
  - `linux-exFAT-implementation-summary.md` plus Linux `namei.c` and `dir.c` for the separation between lookup policy, directory iteration, and mount bootstrap.
  - `EXR-MOUNT-09` for the mount-owned shared-state contract this component must consume.
  - `EXR-FILESET-04B` for the validated file-record boundary lookup should trust.
  - `EXR-INODE-05B` for the read-only inode shell boundary.
  - `EXR-UPCASE-07B` for the canonical fold-and-hash service.
  - `ASTERINAS_ARCHITECT_PRIORS.md` and `ASTERINAS_CODE_QUALITY_PRIORS.md` for the local rules about narrow ownership, top-down readability, and keeping mutation separate from read-only trust boundaries.

## exFAT Concepts Covered

- Directory entry iteration over an accepted directory chain.
- Lookup against validated file-record sets.
- Case-insensitive matching through the canonical upcase-backed name-hash service.
- Directory cursor or search-hint state needed to walk a directory efficiently.
- Root-directory and subdirectory lookup behavior as read-only consumers of mounted shared state.
- Rejection of any drift into mount bootstrap, file-data reads, allocation search, or namespace mutation.

## Target Files

- Existing files likely to change:
  - `kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/fileset.rs`
- New files expected:
  - `kernel/src/fs/fs_impls/exfat_refactor/dir.rs`

## Code Budget

- Target new or heavily rewritten code size:
  - `240-340` lines
- Reason if the budget might exceed 500 lines:
  - It should not if the component stays focused on iteration and lookup. If it starts absorbing create, unlink, mkdir, rmdir, rename, or page-cache-backed file reads, the boundary is wrong and the work should be split instead of widened.

## Exit Condition

Design work may start when there is exactly one directory-owned entry point that:

1. consumes already-mounted shared filesystem state,
2. iterates directory entries from validated directory/file-record surfaces,
3. resolves names through the canonical upcase-backed hash and comparison path,
4. exposes lookup and iteration behavior without rediscovering mount resources,
5. does not implement create, unlink, mkdir, rmdir, rename, or regular-file read behavior.

## Risks

- Directory lookup can become a catch-all if it starts owning namespace mutation or writeback helpers.
- The lookup path could quietly reintroduce mount bootstrap if it tries to rediscover the upcase table or root tables instead of consuming mount-owned shared state.
- The component could drift into raw dentry parsing if it stops trusting the validated file-record boundary from `EXR-FILESET-04B`.
- Page-cache-backed regular-file reads belong to `EXR-READ-11A` and later components; if directory code starts needing them, the split is too wide.
- The component needs to stay explicit about lookup versus mutation because later create and rename work will depend on this contract without being allowed to redefine it.
