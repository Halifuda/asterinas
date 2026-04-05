<!-- SPDX-License-Identifier: MPL-2.0 -->

# Architect Handoff

## Metadata

- Component ID: `EXR-READ-11A`
- Title: Logical-To-Physical Mapping For Existing Regular-File Reads
- Status: `Architected`
- Author: `architect`
- Date: `2026-04-05`
- Task packet: [`EXR-READ-11A-ARCH-20260405-1048`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-READ-11A/20260405-1048-architect-packet.md)

## Purpose

This handoff covers the smallest useful read component for the refactor: mapping an existing regular file's logical data offset to the on-disk cluster and byte location that already exists on the mounted volume.

The component owns the read-side translation boundary only. It consumes mount-owned filesystem state plus accepted chain facts and turns them into physical placement decisions for existing file contents.

It does not own directory lookup policy, buffered `read_at`, page-cache backend ownership, allocation growth, or any write-side extension path.

## Why This Comes Now

This split is safe now because the prerequisites already exist:

- `EXR-MOUNT-09` owns mount bootstrap and the shared filesystem state object.
- `EXR-CHAIN-03B` owns read-only cluster walking and offset-to-chain translation.
- `EXR-INODE-05B` owns the read-only inode metadata shell, including the chain and size facts this mapper must consume.

The remaining pressure is no longer "how do we validate the file?" or "how do we bootstrap the filesystem?" It is "how do we translate a validated file's existing data range into a physical read location without pulling in page-cache policy or buffered I/O?" That is this component.

Linux `inode.c` and `file.c` show the same split clearly: logical block mapping is separate from buffered read execution and page-cache policy. The legacy implementation mixes those concerns more tightly than the refactor should.

## Dependency Contract

- Depends on:
  - `EXR-MOUNT-09`
  - `EXR-CHAIN-03B`
  - `EXR-INODE-05B`
- Blocks:
  - `EXR-PGCACHE-11B`
  - `EXR-READ-11B`
- Can run in parallel with:
  - `EXR-DIR-10` once `EXR-MOUNT-09` is accepted
- Recommended parallel wave:
  - finish mount-state acceptance first;
  - then let `EXR-READ-11A` and `EXR-DIR-10` proceed in parallel as separate read-side and lookup-side branches;
  - keep `EXR-PGCACHE-11B` and `EXR-READ-11B` blocked until this mapping boundary is in place.
- Stable pre-existing interfaces used:
  - mount-owned shared filesystem state from `EXR-MOUNT-09`
  - read-only cluster walking and offset helpers from `EXR-CHAIN-03B`
  - validated inode metadata facts from `EXR-INODE-05B`
  - `ExfatSuperBlock::cluster_to_byte_offset(...)` and related geometry helpers
- Prior sources or prior slices that materially shaped the split:
  - `Microsoft-exFAT-spec.md` for `NoFatChain`, valid-data-length, and physical placement rules.
  - `linux-exFAT-implementation-summary.md` plus Linux `inode.c` and `file.c` for the separation between logical mapping, buffered reads, and page-cache behavior.
  - `EXR-MOUNT-09` for the mount-owned shared-state boundary.
  - `EXR-CHAIN-03B` for chain walking and offset translation.
  - `EXR-INODE-05B` for the metadata shell boundary that keeps inode facts read-only.
  - `ASTERINAS_ARCHITECT_PRIORS.md` and `ASTERINAS_CODE_QUALITY_PRIORS.md` for the local rule that read mapping must stay separate from mount, lookup, and page-cache ownership.

## exFAT Concepts Covered

- Logical-to-physical mapping for existing regular-file contents.
- Contiguous versus FAT-backed placement for read-side traversal.
- File-size and valid-data-length boundaries as read limits.
- Mounted shared state as the source of block-device and geometry facts.
- Exclusion of directory lookup policy, namespace mutation, and write-side allocation growth.

## Target Files

- Existing files likely to change:
  - `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- New files expected:
  - `kernel/src/fs/fs_impls/exfat_refactor/read.rs`

## Code Budget

- Target new or heavily rewritten code size:
  - `180-260` lines
- Reason if the budget might exceed 500 lines:
  - It should not if the component stays at the mapping boundary. If it starts absorbing buffered reads, page-cache wiring, or write-side extension, the split is wrong and the work should be split again.

## Exit Condition

Design work may start when there is exactly one read-mapping entry point that:

1. consumes mounted filesystem state and validated inode chain facts,
2. translates logical offsets for existing regular files into cluster and byte placement,
3. respects contiguous and FAT-backed chains without re-owning chain traversal policy,
4. returns a physical mapping boundary that later page-cache and buffered-read components can consume,
5. does not implement buffered `read_at`, page-cache backend ownership, directory lookup, or allocation growth.

## Risks

- The read boundary can easily drift into buffered `read_at` if the design tries to choose how bytes are copied instead of only where they live.
- The component can become a second mount path if it starts reopening volume state instead of borrowing the accepted mount-owned object.
- The inode metadata shell can get pulled into a generic accessor layer if the mapping design asks for too many stored facts at once.
- Directory lookup policy, name resolution, and namespace behavior must stay outside this component even though they also read from mounted filesystem state.
- If the mapping design starts needing write-side extension or allocation hints, the split has crossed into `EXR-WRITE-13A`.
