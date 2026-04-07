<!-- SPDX-License-Identifier: MPL-2.0 -->

# Architect Handoff

## Metadata

- Component ID: EXR-INODE-CORE-17
- Title: Inode Carrier And Metadata Owner
- Status: `Architected`
- Author: architect
- Date: 2026-04-07
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-INODE-CORE-17/20260407-1021-architect-packet.md`

## Functional Unit Definition

- Functional goal: introduce the smallest coherent `ExfatInode` owner that carries inode identity, metadata, timestamps, and the weak filesystem back-reference needed for future VFS behavior, while keeping inode cache, lookup, readdir, file mapping, page cache, read/write, and namespace mutation out of this unit.
- Final architectural owner: `ExfatInode`
- Expected landing form: trait-carrier type plus owner state, owner methods, and owner-private helpers; no standalone metadata-shell component.
- Boundary kind: stable architectural boundary
- Why this boundary is architecturally real: the inode is the stable VFS-visible owner for per-file identity and metadata. It is not a parser helper or a cache key. Its lifecycle, timestamps, and filesystem back-reference belong together, while cache, directory, and data-path behavior have separate owners.

## Purpose

This component defines the inode carrier that later VFS-visible behavior can attach to without dragging in inode caching or data-path logic. The owner should store only the validated facts needed to describe one exFAT inode and to hand it back to `ExfatFs` and future inode-table logic.

The unit should consume validated `ExfatDentrySet` and `ExfatChain` inputs at construction time, copy the needed metadata into `ExfatInode`, and then stop. It should not retain a file-record set object as a surrogate owner.

## Why This Comes Now

`EXR-FS-CORE-16` is the stable filesystem-owner sibling, `EXR-FILESET-04B` already supplies the validated file-record boundary, and `EXR-CHAIN-03B` already supplies read-only chain state. The next smallest coherent step is the inode owner itself, because later cache, directory, read/write, and namespace units all hang off a real inode carrier.

This placement also prevents the old drift pattern from returning: a metadata shell that pretends to be the inode boundary but never becomes the VFS carrier.

## Owner And Integration Convergence

- Interfaces, traits, services, or higher-level functions this unit ultimately serves:
  - VFS `Inode`
  - VFS `InodeIo`
  - future `EXR-INODE-CACHE-18` opened-inode table
  - future `EXR-DIR-OPS-23`, `EXR-FILE-MAP-24`, `EXR-READ-OPS-25`, `EXR-PGCACHE-26`, `EXR-NAMESPACE-29`, and `EXR-WRITE-30`
  - the sibling `EXR-FS-CORE-16` root-inode contract
- If the unit is internal-only, why this internal ownership is still stable in the finished system:
  - it is not internal-only; it is the stable VFS carrier for exFAT inodes
- Known non-goals or nearby logic that must remain in the parent owner:
  - inode cache and `InodeKey`
  - lookup and readdir
  - file mapping and page-cache backend
  - buffered read and write behavior
  - namespace mutation
  - sync ordering and allocation policy

`ExfatInode` should hold a `Weak<ExfatFs>` back-reference, not a strong filesystem cycle. `ExfatFs` stays the runtime owner, and later cache logic can hand out strong inode handles without introducing a separate ownerless layer.

The inode should keep validated location facts only as owner-private state: dentry-set position, dentry entry index, parent linkage, start chain, and file sizes. Those facts are inputs to future persistence and later `InodeKey` derivation, but the cache key itself belongs to `EXR-INODE-CACHE-18`.

## Dependency Contract

- Depends on:
  - `EXR-FS-CORE-16`
  - `EXR-FILESET-04B`
  - `EXR-CHAIN-03B`
- Blocks:
  - `EXR-INODE-CACHE-18`
  - `EXR-DIR-OPS-23`
  - `EXR-FILE-MAP-24`
  - `EXR-READ-OPS-25`
  - `EXR-PGCACHE-26`
  - `EXR-NAMESPACE-29`
  - `EXR-WRITE-30`
  - `EXR-SYNC-31`
- Can run in parallel with:
  - `EXR-FS-CORE-16` at architect/spec time
  - creator work only if the file landing zones are kept disjoint and the shared `mod.rs` declaration edit is serialized
- Recommended parallel wave:
  - Wave A: finalize `EXR-FS-CORE-16` root-handoff assumptions and then land `EXR-INODE-CORE-17` as the inode carrier in `inode.rs`
- Stable pre-existing interfaces used:
  - `kernel/src/fs/vfs/fs_apis/file_system.rs`
  - `kernel/src/fs/vfs/fs_apis/inode.rs`
  - `kernel/src/fs/vfs/fs_apis/inode_ext.rs`
  - `EXR-FILESET-04B` and `EXR-CHAIN-03B` validated value boundaries
- Prior sources or prior slices that materially shaped the split:
  - `WORKSPACE-ARCH-RESET/00_architect.md`
  - `river-anvil-20260407-1010-resume-board-validation.md`
  - `ASTERINAS_ARCHITECT_PRIORS.md` profile `I-ARCH`
  - `ASTERINAS_CODE_QUALITY_PRIORS.md` profile `Q-ARCH`
  - `linux-exFAT-implementation-summary.md`
  - the legacy `exfat` inode and filesystem carriers as integration context only

## Recommended Work Slices

These are candidate slices for scheduler consideration, not the globally active plan.

| Slice ID | Parent Unit Scope | Goal | Likely Write Set | Depends On | May Overlap With | Lane Class | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `WS-INODE-CORE-17-STRUCT` | `EXR-INODE-CORE-17` | Define `ExfatInode` state, the weak `ExfatFs` back-reference, and owner-private constructors from trusted metadata and chain inputs. | `kernel/src/fs/fs_impls/exfat_refactor/inode.rs` | `EXR-FS-CORE-16`, `EXR-FILESET-04B`, `EXR-CHAIN-03B` | `WS-FS-CORE-16-ARCH` only as a command-free sibling with a disjoint write set | command-free | Keep the constructor surface narrow and crate-private; do not add cache or data-path helpers here. |
| `WS-INODE-CORE-17-META` | `EXR-INODE-CORE-17` | Implement the VFS identity and metadata methods that are meaningful before read/write and cache work exist. | `kernel/src/fs/fs_impls/exfat_refactor/inode.rs` | `WS-INODE-CORE-17-STRUCT` | `WS-FS-CORE-16-ARCH` only after the root-handoff seam is settled | command-free | Cover `ino`, `size`, `metadata`, `type_`, `mode`, owner/group/timestamp accessors, and `fs()`. Keep `page_cache()` deferred. |
| `WS-INODE-CORE-17-SEAM` | `EXR-INODE-CORE-17` | Carry the explicit temporary `InodeIo` seam until the read/write/page-cache owners land. | `kernel/src/fs/fs_impls/exfat_refactor/inode.rs` | `WS-INODE-CORE-17-STRUCT` | none beyond the inode lane | command-free | `read_at` and `write_at` should be explicit temporary seams, not hidden helpers. The seam should be marked with `// TEMPORARY: absorbed by EXR-READ-OPS-25 / EXR-WRITE-30 / EXR-PGCACHE-26.` |

## exFAT Concepts Covered

- Inode identity and per-inode metadata.
- Weak filesystem back-reference from inode to `ExfatFs`.
- Dentry-set derived construction inputs.
- Chain-derived start cluster and allocated length facts.
- File and directory mode, ownership, and timestamps as inode-owned state.
- Root inode as a distinguished inode carrier, not as a cache key.
- VFS `Inode` and `InodeIo` carrier obligations.

## Boundary Rejections

- Splits considered but rejected:
  - a standalone metadata shell separate from `ExfatInode`
  - an `InodeKey` helper component inside this unit
  - inode cache or opened-inode-table behavior
  - lookup, readdir, or directory-engine logic
  - file mapping, read/write, or page-cache backend logic
  - namespace mutation or sync ordering
- Why those rejected splits would be packet convenience, not real architecture:
  - they would create another staging layer between the filesystem owner and the VFS inode carrier instead of converging on the stable owner the rest of the system needs
  - the inode identity cache belongs to `EXR-INODE-CACHE-18`, not to this owner boundary
  - page cache and data-path behavior belong to later read/write units and should not be folded into metadata ownership

## Target Files

- Existing files likely to change:
  - `kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- New files expected:
  - `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`

## Code Budget

- Target creator work-slice size: `150-300` lines
- Expected number of creator slices: 2 to 3
- Reason if any single slice might exceed 500 lines:
  - it should not. If a slice grows that large, read/write or namespace behavior has leaked into the inode carrier and the unit must be re-sliced instead of expanded.

## Exit Condition

Design work may start once the component is understood as exactly:

1. an `ExfatInode` carrier with stable owner-private state,
2. a weak back-reference to `ExfatFs`,
3. metadata and identity methods that do not require cache or data-path ownership,
4. explicit temporary seams for `InodeIo` read/write behavior,
5. no inode cache, page-cache backend, directory lookup, or namespace mutation logic.

Observable readiness means the designer can point to a single `inode.rs` owner and produce a complete spec without reopening filesystem-global cache or read/write behavior.

## Risks

- `EXR-FS-CORE-16` still needs a clean root-handoff seam. If that seam turns into a fake owner or an inode cache shortcut, this unit should be re-sliced with the sibling lane instead of widened.
- `mod.rs` is a likely collision point between the two Wave A lanes. If both lanes need that file, serialize it rather than pretending the declaration edit is file-parallel.
- `read_at` and `write_at` must stay explicit temporary seams until `EXR-READ-OPS-25`, `EXR-WRITE-30`, and `EXR-PGCACHE-26` land.
- Do not let `set_mode`, `set_owner`, or `set_group` drift into hidden writeback policy. If they remain present before write-side ownership exists, they should be treated as temporary no-op or rejection seams, not as durable mutation behavior.
