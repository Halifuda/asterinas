<!-- SPDX-License-Identifier: MPL-2.0 -->

# Architect Handoff

## Metadata

- Component ID: `EXR-READ-11B`
- Title: Buffered Regular-File Read Execution And Read-Side Zero-Fill
- Status: `Architected`
- Author: `architect`
- Date: `2026-04-05`
- Task packet: [`EXR-READ-11B-ARCH-20260405-1128`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-READ-11B/20260405-1128-architect-packet.md)

## Purpose

This handoff covers the smallest useful execution-side read component for the refactor: buffered `read_at` for already-mounted exFAT regular files, including the read-side zero-fill behavior visible to callers when the logical read extends into unwritten or otherwise uninitialized portions of the file image.

The component consumes mount-owned shared state and the physical-placement boundary from `EXR-READ-11A`, but it does not own logical-to-physical mapping, page-cache backend ownership, allocation growth, truncation, or namespace behavior.

## Why This Comes Now

This split is safe now because the upstream boundaries already exist:

- `EXR-MOUNT-09` owns mount bootstrap and the shared filesystem object.
- `EXR-READ-11A` owns the logical-to-physical placement boundary for existing regular-file contents.
- `EXR-PGCACHE-11B` owns page-cache backend integration.

The remaining pressure is not "where does the file live?" and not "who owns the page-cache backend?" It is "how does a buffered regular-file read turn the accepted placement boundary into user-visible bytes, while preserving the exFAT zero-fill and EOF rules?" That is this component.

Linux `file.c` and `inode.c` make the same separation explicit: block placement, buffered read execution, and page-cache/backend ownership are related but not the same responsibility. The refactor should keep that split intact instead of folding the read policy back into mapping or backend plumbing.

## Dependency Contract

- Depends on:
  - `EXR-MOUNT-09`
  - `EXR-READ-11A`
  - `EXR-PGCACHE-11B`
- Blocks:
  - final exFAT regular-file buffered-read wiring that consumes this contract
- Can run in parallel with:
  - `EXR-READ-11A` creator/checker/reviewer flow once the mount contract is accepted
  - `EXR-PGCACHE-11B` architect work
  - command-free planning or review that only needs the buffered-read contract, not implementation details
- Recommended parallel wave:
  - finish mount-state acceptance first;
  - keep logical-to-physical placement and page-cache backend ownership in their own lanes;
  - let `EXR-READ-11B` define the buffered-read policy on top of those frozen contracts rather than trying to discover either dependency itself.
- Stable pre-existing interfaces used:
  - mount-owned `ExfatFs` shared state from `EXR-MOUNT-09`
  - the read-placement result from `EXR-READ-11A`
  - `InodeIo::read_at` from `kernel/src/fs/vfs/fs_apis/inode.rs`
  - `PageCache` and `PageCacheBackend` from `kernel/src/fs/vfs/page_cache.rs`
  - the inode and file-data helpers already present in the refactor and legacy exFAT implementation
- Prior sources or prior slices that materially shaped the split:
  - `Microsoft-exFAT-spec.md` for `NoFatChain`, valid-data-length, and physical placement rules.
  - `linux-exFAT-implementation-summary.md` plus Linux `file.c` and `inode.c` for the separation between buffered reads, zero-fill behavior, and lower-layer mapping/backend ownership.
  - `EXR-MOUNT-09` for the mount-owned shared-state boundary.
  - `EXR-READ-11A` for the mapping boundary this component must consume rather than re-own.
  - `EXR-PGCACHE-11B` for the cache-backend ownership boundary this component must not absorb.
  - `ASTERINAS_ARCHITECT_PRIORS.md` and `ASTERINAS_CODE_QUALITY_PRIORS.md` for the local rules about narrow ownership, top-down readability, and keeping write-side growth out of the read path.

## exFAT Concepts Covered

- Buffered reads of existing regular-file data.
- Read-side zero-fill for bytes that are not backed by initialized file contents.
- EOF behavior driven by the accepted file-size / valid-data boundary, not by invented placement.
- Consumption of mount-owned state plus the mapping boundary from `EXR-READ-11A`.
- Exclusion of logical-to-physical mapping, page-cache backend ownership, directory lookup, namespace mutation, allocation growth, and truncation.

## Target Files

- Existing files likely to change:
  - `kernel/src/fs/fs_impls/exfat_refactor/read.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- New files expected:
  - `kernel/src/fs/fs_impls/exfat_refactor/read.rs`

## Code Budget

- Target new or heavily rewritten code size:
  - `220-320` lines
- Reason if the budget might exceed 500 lines:
  - It should not if the component stays on buffered read execution and zero-fill policy only. If it starts absorbing page-cache backend wiring, direct I/O, allocation growth, or truncate bookkeeping, the split is too wide and should be cut again.

## Exit Condition

Design work may start when there is exactly one buffered-read entry point that:

1. accepts mount-owned shared filesystem state and the `EXR-READ-11A` placement result,
2. serves regular-file buffered `read_at`,
3. zero-fills unread or partially initialized bytes visible to readers,
4. delegates backend I/O to the page-cache contract owned by `EXR-PGCACHE-11B`,
5. does not implement logical-to-physical mapping, page-cache backend ownership, allocation growth, truncation, or namespace behavior.

## Risks

- Buffered read execution can quietly re-own mapping if the design starts choosing clusters instead of consuming the placement boundary from `EXR-READ-11A`.
- Zero-fill can drift into write-side valid-size management if the design tries to repair unwritten ranges by extending the file instead of only shaping the read result.
- Page-cache backend hooks can become mixed with read policy if the component defines its own backend facade instead of consuming the one owned by `EXR-PGCACHE-11B`.
- EOF behavior can be wrong if the design uses allocated size where it should use the accepted valid-data boundary to decide what is readable versus what must be zero-filled.
- The slice can become too large if it starts absorbing direct I/O, truncate, growth, or namespace work.
