<!-- SPDX-License-Identifier: MPL-2.0 -->

# Architect Handoff

## Metadata

- Component ID: `EXR-FS-CORE-16`
- Title: `ExfatFs` filesystem owner boundary
- Status: `Architected`
- Author: architect
- Date: `2026-04-07`
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-FS-CORE-16/20260407-1021-architect-packet.md`

## Functional Unit Definition

- Functional goal: introduce `ExfatFs` as the stable VFS `FileSystem` carrier and filesystem-wide runtime-state root, without absorbing mount/open sequencing or inode ownership.
- Final architectural owner: `ExfatFs`
- Expected landing form: trait-carrier type plus owner state
- Boundary kind: stable architectural boundary
- Why this boundary is architecturally real: `ExfatFs` is the single filesystem-wide owner that must own the block device, normalized superblock, mount policy, and filesystem statistics while presenting the VFS `FileSystem` face. Later services such as inode cache, directory engine, upcase, bitmap, allocator, and sync ordering all hang off that owner; they are not separate long-lived filesystem roots.

## Purpose

This unit is the smallest coherent filesystem-wide owner that can exist before inode ownership is ready.
It should establish `ExfatFs` as the type that carries the mount instance, stable geometry, and filesystem identity, but it should not try to solve directory discovery, root inode creation, or cache/state synchronization beyond explicit seams.

## Why This Comes Now

The boot and superblock foundations are already accepted, so the next real owner boundary is the filesystem-wide carrier that consumes them.
The VFS `FileSystem` contract also requires a stable filesystem owner, not just validated on-disk values.
`EXR-FS-CORE-16` can land before `EXR-INODE-CORE-17` because the root owner does not need inode behavior yet; it only needs a named, explicit seam for root exposure until the inode carrier exists.

## Owner And Integration Convergence

- Interfaces, traits, services, or higher-level functions this unit ultimately serves: VFS `FileSystem`; later `EXR-FS-OPEN-22`; later `EXR-SYNC-31`; handshake surface for `EXR-INODE-CORE-17`.
- If the unit is internal-only, why that internal ownership is still stable in the finished system: it is not internal-only. `ExfatFs` is the public integration owner that VFS talks to directly. Its internal state is stable because every later exFAT subsystem needs a single filesystem root to reference.
- Known non-goals or nearby logic that must remain in the parent owner: inode cache, root inode construction, mount/open sequencing, directory services, upcase loading, bitmap loading, allocator policy, and write ordering stay out of this unit.

Trait-method scope for this unit:

- Land now: `name()`, `sb()`, and `fs_event_subscriber_stats()`.
- Keep as explicit temporary seams: `root_inode()` and `sync()`.
- Leave as inherited defaults for now: `source()`, `flags()`, and `set_fs_flags()`.

## Temporary Seam

The only seam that should use a hard placeholder in this unit is `FileSystem::root_inode()`.
It may use `todo!()` or `unimplemented!()` only if the seam is named, commented, and left unreachable from the registered legacy filesystem.

Recommended exact comment for the seam:

```rust
// Temporary seam: EXR-FS-OPEN-22 will install the real root inode after EXR-INODE-CORE-17 lands.
```

Why that is acceptable:

- `exfat_refactor` is compiled in-tree but is not the registered filesystem yet.
- The active filesystem remains the legacy `exfat` module, so this seam cannot be reached by normal mounts until the refactor is intentionally switched over.
- `EXR-FS-OPEN-22` is the later component that absorbs the real root-inode construction once `EXR-INODE-CORE-17` exists.

`sync()` should not become a hidden owner boundary.
In this unit it can remain a trivial no-op or equivalent explicit placeholder, but the real flush ordering belongs to `EXR-SYNC-31`.

## Dependency Contract

- Depends on: `EXR-BOOT-01`, `EXR-SBGEOM-15`, the VFS `FileSystem` contract, and the normalized `ExfatSuperBlock`.
- Blocks: the finished `FileSystem` root path, `EXR-FS-OPEN-22`, and any consumer that needs a real `Arc<dyn Inode>` root.
- Can run in parallel with: `EXR-INODE-CORE-17` architect work; later creator work in `kernel/src/fs/fs_impls/exfat_refactor/inode.rs` because the write sets are disjoint.
- Recommended parallel wave: Wave A trait-carrier convergence, with `EXR-FS-CORE-16` and `EXR-INODE-CORE-17` as sibling lanes.
- Stable pre-existing interfaces used: `FileSystem`, `SuperBlock`, `FsEventSubscriberStats`, `ExfatSuperBlock`, and the accepted boot/superblock foundation.
- Prior sources or prior slices that materially shaped the split: `EXR-BOOT-01`, `EXR-SBGEOM-15`, `ASTERINAS_ARCHITECT_PRIORS.md` profile `I-ARCH`, `components/WORKSPACE-ARCH-RESET/00_architect.md`, the VFS `FileSystem` trait surface, the legacy `kernel/src/fs/fs_impls/exfat/fs.rs` carrier shape, and `linux-exFAT-implementation-summary.md` as orientation only.

## Recommended Work Slices

These are candidate creator slices for scheduler consideration, not the active global plan.

| Slice ID | Parent Unit Scope | Goal | Likely Write Set | Depends On | May Overlap With | Lane Class | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `WS-FS-CORE-16-A` | `EXR-FS-CORE-16` | Define `ExfatFs` owner state and the shallow `FileSystem` identity surface, with the named temporary `root_inode()` seam kept explicit. | `kernel/src/fs/fs_impls/exfat_refactor/fs.rs` | `EXR-BOOT-01`, `EXR-SBGEOM-15` | `WS-INODE-CORE-17-A` only at the architect/scheduler level; the production write sets stay disjoint | creator | Keep the slice narrowly about the filesystem owner. Do not add inode cache, directory services, or mount sequencing here. |

## exFAT Concepts Covered

- Filesystem-wide owner state.
- VFS `FileSystem` identity and superblock reporting.
- Filesystem event subscriber statistics.
- Root inode exposure as a temporary seam.
- Sync as a later filesystem-wide flush-order concern.
- Block-device ownership and normalized superblock geometry.

## Boundary Rejections

- Splitting `ExfatFs` into helper-only fragments such as a separate root-handle owner, stats wrapper, or mount shell was rejected. Those would be packet-convenience surfaces, not stable owners.
- Folding inode cache, root inode construction, or mount/open sequencing into this unit was rejected. Those belong to `EXR-INODE-CORE-17`, `EXR-INODE-CACHE-18`, and `EXR-FS-OPEN-22`.
- Hiding `root_inode()` behind a fake inode carrier or anonymous staging type was rejected. The unit should use an explicit temporary seam instead.
- Treating `sync()` as the real flush owner was rejected. The real ordering and dirty-state traversal belong to `EXR-SYNC-31`.

## Target Files

- Existing files likely to change: none yet
- New files expected: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`

## Code Budget

- Target creator work-slice size: `180-260` lines
- Expected number of creator slices: `1`
- Reason if any single slice might exceed 500 lines: not expected; the unit is intentionally narrow and should stay within the owner skeleton plus the explicit temporary seam.

## Exit Condition

This design is ready for implementation when `ExfatFs` exists as a standalone filesystem owner with stable state fields, the `FileSystem` identity methods are in place, and the only unresolved root-path behavior is the named temporary `root_inode()` seam for `EXR-FS-OPEN-22`.

## Risks

- The `root_inode()` seam must not become a permanent ownerless stub.
- `sync()` must not start pulling in inode cache or allocation logic before `EXR-SYNC-31`.
- The owner-state layout must remain compatible with `EXR-INODE-CORE-17`, which will need to point back to the filesystem owner without forcing a separate filesystem shell.
- Later locking and dirty-state work must continue to treat `ExfatFs` as the single coordination root, not as a collection of unrelated helpers.
