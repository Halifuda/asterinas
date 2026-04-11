<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Async

## Metadata

- Component ID: `EXR-FS-OPEN-22`
- Title: `ExfatFs` Mount/Open Sequencing Serialization
- Status: `Specified`
- Author: designer
- Date: `2026-04-10`
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-FS-OPEN-22/20260410-1300-designer-packet.md`
- Based on architect artifact: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-OPEN-22/00_architect.md`

## Scope

- In scope:
  - State the serialization boundary for mount/open sequencing under `ExfatFs`.
  - Define the order in which mount-time discovery, publication, and root readiness are linearized.
  - Make the root-publication handoff race-free with respect to the opened-inode cache.
  - Keep the sequencing contract narrow enough that creator work can implement it without inventing extra locks or background tasks.
- Out of scope:
  - Fine-grained concurrency policy for later directory mutation, allocator mutation, read/write, or sync.
  - Any lock-free cache design.
  - Any background mount worker or async publication protocol.
  - Any public helper whose only purpose would be to expose a lock or field.

## Serialization Contract

- Shared serialization boundary:
  - `ExfatFs` owns the mount/open publication boundary.
- Linearization rule:
  - Mount-time discovery, root construction or reuse, and root publication must be serialized so a caller sees either the pre-mount state or a fully published root state.
- Publication order:
  - First, establish the filesystem-owner critical section.
  - Second, install or confirm `UpcaseTable` if mount-time discovery requires it.
  - Third, install or confirm `AllocationBitmap` if the root discovery path needs occupancy state.
  - Fourth, drive `DirectoryEngine` to obtain the trusted root-directory facts.
  - Fifth, construct or reuse the root `ExfatInode`.
  - Sixth, publish the root handle into the owner-private root slot or equivalent owner-owned publication path.
- Reuse rule:
  - If the root already exists in published owner state, return the canonical published handle instead of creating a second one.
- Cache rule:
  - The opened-inode cache remains the owner-owned reuse boundary for the root special case and later inode handles.

## Lock-Order Expectations

- Owner lock before discovery:
  - The filesystem-owner serialization boundary may be entered before mutating publication state, but blocking disk I/O and directory traversal should be kept conceptually outside the lock if the implementation can do so safely.
- Discovery before publication:
  - Directory scanning and raw candidate discovery happen before root publication is finalized.
- Publication before exposure:
  - No caller should observe a ready root handle until the publication step is complete.
- Root before ordinary reuse only as a special case:
  - The root special case may be published through the cache owner, but it must stay distinct from ordinary non-root `InodeKey` insertions.

## Forbidden Interleavings

- Do not let two concurrent creators publish two distinct root handles.
- Do not let `root_inode()` return a temporary seam result after the root has been published.
- Do not allow root publication to race with ordinary opened-inode lookup in a way that makes root appear as a synthetic ordinary key.
- Do not hold publication state while performing any later mutation work that belongs to directory ops or allocator policy.

## Implementation Notes For Creator Work

- A temporary big lock is acceptable if it keeps the mount/open sequence simple and clearly linearized.
- The sequencing helper, if any, must remain owner-local to `ExfatFs`.
- `DirectoryEngine`, `UpcaseTable`, and `AllocationBitmap` should be treated as prerequisites, not as peers that can publish mount state independently.
- The root-publication handoff should be written so later creator work can delete the temporary `root_inode()` seam without changing the concurrency model.

## Reviewer/Checker Expectations

- The reviewer should confirm that the serialization boundary is explicit and remains on `ExfatFs`.
- The checker should confirm that root publication cannot be observed half-complete.
- The checker should confirm that the mount path does not quietly introduce a second publication mechanism outside the owner boundary.
