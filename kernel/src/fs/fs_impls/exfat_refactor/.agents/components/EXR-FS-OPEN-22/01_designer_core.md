<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Core

## Metadata

- Component ID: `EXR-FS-OPEN-22`
- Title: `ExfatFs` Mount/Open Sequencing And Root Publication
- Status: `Specified`
- Author: designer
- Date: `2026-04-10`
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-FS-OPEN-22/20260410-1300-designer-packet.md`
- Based on architect artifact: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-OPEN-22/00_architect.md`

## Scope

- In scope:
  - Define the minimal `ExfatFs` owner-method sequence that turns trusted boot facts into a mounted filesystem with a published root inode.
  - Absorb the current `root_inode()` seam into `ExfatFs`-owned root publication behavior.
  - Specify the mount-time consumption order for opened-inode cache, `DirectoryEngine`, `UpcaseTable`, and `AllocationBitmap`.
  - State the serialization and critical-section expectations needed to avoid duplicate root publication or half-installed mount state.
  - Keep the root-directory system-entry handoff explicit so later creator work has a named exit path from the placeholder seam.
- Out of scope:
  - Later directory lookup/readdir, namespace mutation, allocator mutation policy, buffered read/write, page-cache behavior, and sync ordering.
  - Any separate mount object, root-scanner owner, or fake root carrier.
  - Reopening inode cache semantics or directory policy beyond the mount-time discovery path.
  - Production-code edits outside the future `fs.rs` owner file and any module declaration needed to wire it in.

## Module Specification

- Dependencies:
  - The accepted `ExfatFs` filesystem owner boundary.
  - The accepted `ExfatInode` carrier and opened-inode cache ownership.
  - `DirectoryEngine` as the read-only directory record stream.
  - `UpcaseTable` as the mount-time canonicalization service.
  - `AllocationBitmap` as the mount-time read-only occupancy snapshot.
  - The validated boot and superblock foundations from `EXR-BOOT-01` and `EXR-SBGEOM-15`.
- Interfaces provided:
  - A creator-ready `ExfatFs` mount/open sequence that can be implemented as `open(...)` or an equivalent owner method set.
  - A named root-publication handoff that replaces the temporary `root_inode()` seam.
  - A mount-time discovery order that later roles can preserve without guessing.
- Files or modules touched:
  - `kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- Hidden implementation details:
  - Whether the sequencing is expressed as one public `open(...)` method or a small set of owner-private helpers.
  - Whether the mount-time root publication is represented by a dedicated owner-private root slot or by a reuse-first cache path that still preserves the distinguished root case.
  - The exact private field names and helper names, so long as the behavior remains owner-local to `ExfatFs`.

## Functional Specification

### Operation

- Name: `ExfatFs` mount/open sequencing
- Inputs:
  - Trusted boot facts and normalized superblock state.
  - A validated directory stream for the root directory.
  - The owner-owned `UpcaseTable`, `AllocationBitmap`, and opened-inode table states as mount-time consumers.
- Preconditions:
  - Boot geometry has already been validated.
  - `ExfatFs` is the stable filesystem owner.
  - Mount-time discovery is driven only through the owner boundary and not through a separate staging shell.
- Actions:
  - Enter the filesystem-owner serialization boundary before mutating mount-visible owner state.
  - Install or confirm the mount-time canonicalization state needed for later name-sensitive discovery.
  - Discover the root-directory system entries through `DirectoryEngine`.
  - Load and validate the upcase table before any name-dependent root discovery or publication step.
  - Load and validate the allocation bitmap before the root inode is published as ready for use.
  - Construct or reuse the root `ExfatInode` under `ExfatFs` ownership.
  - Publish the root handle through `ExfatFs`-owned state so later `root_inode()` calls return the ready root instead of the temporary seam.
  - Leave later directory ops and mutation policy untouched.
- Outputs:
  - A mounted `ExfatFs` instance with a published root inode.
- Postconditions:
  - The filesystem owner can answer `root_inode()` without a temporary seam.
  - The root handle is stable and publicly reachable only through the `ExfatFs` owner boundary.

### Operation

- Name: Root publication handoff
- Inputs:
  - A fully constructed or reuse-resolved `ExfatInode` for the root directory.
- Preconditions:
  - The root-directory discovery path has already produced the trusted facts needed to identify the root inode.
- Actions:
  - Publish the root inode into the owner-private root slot or the equivalent owner-owned root publication path.
  - Keep the ordinary opened-inode keyspace separate from the root special case.
  - Ensure duplicate root publication resolves to the same canonical handle rather than a second inode shell.
- Outputs:
  - The canonical root inode handle owned by `ExfatFs`.
- Postconditions:
  - `root_inode()` can be implemented as a direct owner lookup of the published root handle.
  - The temporary root seam has a named exit path and does not remain an indefinite placeholder.

### Operation

- Name: Mount-time owner consumption order
- Inputs:
  - `DirectoryEngine`, `UpcaseTable`, `AllocationBitmap`, opened-inode cache, and validated boot facts.
- Preconditions:
  - Each consumed owner already exists as its own stable boundary or validated state.
- Actions:
  - Treat `DirectoryEngine` as the source of root-directory candidates, not as a writable mount object.
  - Treat `UpcaseTable` as a prerequisite for any name-folding or name-hash work during mount.
  - Treat `AllocationBitmap` as a read-only occupancy snapshot during mount.
  - Treat the opened-inode cache as the canonical root publication and reuse point, not as an alternate mount owner.
- Outputs:
  - Sequenced mount/open state ready for root publication.
- Postconditions:
  - Creator work has a fixed dependency order and does not need to infer whether bitmap or upcase discovery comes first.

## Invariants

- `ExfatFs` remains the single owner of mount/open sequencing.
- The temporary `root_inode()` seam from `EXR-FS-CORE-16` is absorbed here rather than preserved as a permanent placeholder.
- The mount sequence must not introduce a separate mount object, scanner owner, or root carrier.
- `DirectoryEngine` remains read-only.
- `UpcaseTable` remains canonicalization-only.
- `AllocationBitmap` remains read-only at this stage.
- The opened-inode cache remains the reuse and publication boundary, not a mount shell.
- Later directory mutation, allocator mutation, data-path behavior, and sync ordering stay out of this component.

## Concurrency Specification

- Shared state:
  - The filesystem-owner mount/open state, including the opened-inode table, root publication slot, and mount-time discovery prerequisites.
- Lock ordering:
  - Acquire the filesystem-owner serialization boundary before mutating mount-visible state.
  - Perform disk I/O and directory traversal outside any critical section that would block progress.
  - Publish the canonical root handle only after the root inode snapshot is fully constructed.
  - Keep root publication, upcase publication, and bitmap publication serialized through the same owner boundary if they are established during the same mount sequence.
- Atomicity requirements:
  - Callers must observe either the pre-open state or the fully published ready-root state.
  - The root inode must not be visible before the mount prerequisites are complete.
  - Duplicate creators for the same root handle must resolve to one canonical published root.
- Forbidden interleavings:
  - Do not perform blocking directory scanning while holding a lock that also guards publication.
  - Do not allow root publication to race with a concurrent lookup into the ordinary opened-inode table in a way that yields two root shells.
  - Do not let mount sequencing mutate allocator policy, namespace state, or sync ordering.
- Allowed simplifications:
  - A single filesystem-owner serialization boundary is acceptable for this component.
  - A temporary big-lock style critical section is acceptable so long as disk I/O and publication are still separated conceptually.

## Pass Split

### Serial Creator Pass

- Required implementation obligations:
  - Define the mount/open sequence in `fs.rs` as `ExfatFs` owner behavior.
  - Replace the temporary `root_inode()` seam with root publication behavior under the same owner boundary.
  - Consume `DirectoryEngine`, `UpcaseTable`, `AllocationBitmap`, and opened-inode cache in a fixed creator-ready order.
  - Preserve the root special case as a distinct owner-owned publication path.
  - Keep the owner-local sequencing explicit enough that later roles do not need to guess about ordering.
- Explicit non-goals:
  - No later directory ops.
  - No namespace mutation.
  - No allocator mutation policy.
  - No read/write or page-cache behavior.
  - No sync ordering.

### Serial Checker Pass

- Required checker-owned tests:
  - A mount-sequencing regression that proves root publication happens only after the mount prerequisites are satisfied.
  - A root-publication regression that proves repeated root access returns the canonical published root handle instead of a temporary seam result.
  - A consumption-order regression that proves the mount path uses `DirectoryEngine`, `UpcaseTable`, and `AllocationBitmap` as owner-local prerequisites rather than as separate owners.
  - A serialization regression that proves duplicate root publication cannot produce two distinct root handles.
- Observable properties that must pass before leaving the serial loop:
  - The root-publication handoff is explicit and owned by `ExfatFs`.
  - The mount sequence does not widen into later mutation or sync behavior.
  - The temporary `root_inode()` seam no longer needs to remain indefinite after this unit lands.

### Concurrency Creator Pass

- Required implementation obligations:
  - No dedicated concurrency implementation beyond the owner serialization boundary described above.
- Explicit non-goals:
  - No lock-free publication.
  - No background mount worker.
  - No async state machine.

### Concurrency Checker Pass

- Required checker-owned concurrency tests:
  - No dedicated concurrency tests required.
- Observable properties that must pass before leaving the concurrency loop:
  - The mount/open sequence remains a single owner boundary with explicit publication ordering.

## Acceptance Notes

- Reviewers should confirm that mount/open sequencing stays on `ExfatFs` and does not become a separate mount owner.
- Reviewers should confirm that the root-publication handoff is explicit and that the ordinary opened-inode keyspace remains distinct from the root special case.
- Reviewers should reject any attempt to fold directory mutation, allocator mutation, read/write behavior, or sync ordering into this component.
- Any creator split should stay serialized on `fs.rs`, because the mount/open path and the root handoff land in the same owner file.
