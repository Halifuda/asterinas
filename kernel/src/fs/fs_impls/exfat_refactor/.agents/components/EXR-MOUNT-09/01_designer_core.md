<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Core

## Metadata

- Component ID: `EXR-MOUNT-09`
- Title: Mount Bootstrap And Shared Filesystem State
- Status: `Specified`
- Author: `designer`
- Date: `2026-04-04`
- Task packet: `EXR-MOUNT-09-DESIGN-20260404-1511`
- Based on architect artifact: `00_architect.md`

## Scope

- In scope:
  - Mount bootstrap for the refactored exFAT implementation.
  - Consumption of validated superblock facts and validated root-discovery facts.
  - Loading the accepted upcase-table and allocation-bitmap surfaces into mount-owned state.
  - Creating the synthetic root inode shell and seeding it into the shared filesystem object.
  - Publishing one mount-owned shared-state object for later directory, read, and write components.
- Out of scope:
  - Root-directory rescanning or rediscovery of the `BITMAP` and `UPCASE` entries.
  - Inode metadata shaping beyond the synthetic root shell.
  - Directory lookup policy, namespace mutation, rename policy, or create/unlink behavior.
  - Page-cache backend ownership, buffered I/O policy, allocation search, or bitmap mutation.
  - Background work, async coordination, or any second overlapping mount helper surface.

## Module Specification

- Dependencies:
  - `EXR-CHAIN-03B`
  - `EXR-INODE-05B`
  - `EXR-SYSROOT-06`
  - `EXR-UPCASE-07B`
  - `EXR-BITMAP-08A`
- Interfaces provided:
  - One canonical mount/bootstrap constructor in `fs.rs` that assembles the filesystem from validated inputs.
  - One mount-owned filesystem state object that later components can borrow as the shared runtime anchor.
  - One root-seeding path that builds the reserved root inode through `ExfatInodeMeta::new_root(...)`.
- Files or modules touched:
  - `kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- Hidden implementation details:
  - Whether the mount object stores the loaded upcase table and bitmap directly or nests them inside a private shared-state struct.
  - Whether the synthetic root shell is embedded in the mount object or cached alongside the shared state by value.
  - Whether root-chain assembly happens before or after the table loads, provided all dependent loads complete before publication.

The canonical surface must stay narrow. Later code should consume one mount-owned filesystem object instead of a separate lookup helper, a separate table registry, or a second bootstrap path.

## Functional Specification

### Operation

- Name: mount bootstrap and shared-state publication
- Inputs:
  - `block_device: &dyn BlockDevice`
  - `super_block: &ExfatSuperBlock`
  - `root_facts: ExfatSysRootFacts`
- Preconditions:
  - `super_block` already passed the boot-sector and geometry validation boundary.
  - `root_facts` already passed `EXR-SYSROOT-06` and contains the accepted root `BITMAP` and `UPCASE` discovery records.
  - The caller is not asking this component to rediscover root entries or to own namespace policy.
  - The caller is not asking for page-cache behavior, allocation policy, or write-path mutation.
- Actions:
  - Consume the prevalidated root discovery aggregate without rescanning the root directory.
  - Load the upcase table from the discovered upcase facts.
  - Load the allocation bitmap from the discovered bitmap facts.
  - Derive the validated root chain from the superblock root-cluster facts and build the synthetic root inode shell through `ExfatInodeMeta::new_root(...)`.
  - Assemble the mount-owned shared filesystem object only after all dependent loads succeed.
  - Publish one complete filesystem object that later components can borrow as read-only shared state.
- Outputs:
  - `Result<ExfatFs>` or the equivalent mount-owned filesystem object.
- Postconditions:
  - The returned filesystem object owns the bootstrap state needed by later directory, read, and write slices.
  - The root inode is seeded as an explicit synthetic special case.
  - No root rescanning, lookup policy, or namespace mutation is created by the mount step.
  - A failure leaves no partially published filesystem object behind.

## Invariants

- Mount bootstrap consumes validated facts; it does not rediscover them.
- The upcase table and allocation bitmap are loaded once for the mount-owned filesystem object.
- The root inode is always seeded through the synthetic root constructor, not through the ordinary inode constructor.
- The mount object is the canonical ownership boundary for shared runtime state.
- No directory lookup policy, mutation policy, or page-cache policy lives in this component.
- The shared filesystem object is read-only after publication from the mount path's point of view.
- The discovery aggregate may be dropped after bootstrap; it is not the long-lived owner of mount state.

## Concurrency Specification

- Shared state:
  - One mount-owned filesystem object containing the validated superblock facts, loaded upcase table, loaded allocation bitmap, and synthetic root inode shell.
  - The object is private until mount bootstrap completes.
- Lock ordering:
  - No lock ordering is required inside the mount contract itself.
  - Any later mutable locks owned by directory, read, or write components must not be held across mount bootstrap I/O or root assembly.
- Atomicity requirements:
  - Bootstrap is all-or-nothing.
  - No reader may observe a filesystem object that has the superblock but not the tables, or the tables but not the root inode seed.
  - Publication happens only after every dependent load and validation step succeeds.
- Forbidden interleavings:
  - No partial publication of the filesystem object.
  - No visibility of the synthetic root inode before the loaded tables are available.
  - No reuse of a half-built shared-state object after an error.
- Allowed simplifications such as a temporary big lock:
  - A temporary private construction lock or single-threaded assembly is acceptable if needed to make publication atomic.
  - The final contract should still look like a one-shot mount handoff, not a permanent coarse-grained mount lock.

## Pass Split

### Serial Creator Pass

- Required implementation obligations:
  - Add one mount bootstrap entry point in `fs.rs`.
  - Assemble the shared filesystem object from the validated superblock and accepted root discovery facts.
  - Load the accepted upcase and bitmap surfaces exactly once during bootstrap.
  - Seed the root inode through `ExfatInodeMeta::new_root(...)`.
  - Keep the mount boundary narrow enough that later code can borrow the returned filesystem object without rediscovering root metadata.
- Explicit non-goals:
  - No root-directory scanner in the mount component.
  - No directory lookup, rename, create, or unlink policy.
  - No page-cache backend, allocation search, or mutation logic.
  - No async tasks, channels, atomics, or background coordination.

### Serial Checker Pass

- Required checker-owned tests:
  - A happy-path mount regression that proves the bootstrap path accepts prevalidated root facts and returns one complete filesystem object.
  - A missing-fact regression that proves mount fails when either the bitmap or upcase discovery record is absent.
  - A synthetic-root regression that proves the root inode seed is created through the explicit synthetic constructor, not the ordinary inode path.
  - A failure-atomicity regression that proves no partial shared-state object is published when one of the dependent loaders fails.
- Observable properties that must pass before leaving the serial loop:
  - The mount result is one complete shared filesystem object.
  - The result contains the loaded table surfaces and the root seed only after all bootstrap dependencies succeed.
  - The tests do not need lookup orchestration, page cache, or async harnesses.

### Concurrency Creator Pass

- Required implementation obligations:
  - None beyond the publication boundary already recorded above.
- Explicit non-goals:
  - No concurrent mount publication protocol beyond one-shot assembly.
  - No shared mutable cache ownership in this component.

### Concurrency Checker Pass

- Required checker-owned concurrency tests:
  - No dedicated concurrency tests required.
- Observable properties that must pass before leaving the concurrency loop:
  - None; the mount contract is satisfied by the atomic one-shot publication boundary recorded in the core spec and the serial regressions.

## Acceptance Notes

- The mount object should remain smaller than a general filesystem manager. If it starts owning lookup policy, page-cache state, or allocation policy, the boundary has drifted.
- The accepted root discovery aggregate should be consumed, not wrapped in a second discovery layer.
- Root seeding should remain explicit so later `DIR-10` and `READ-11A` work from a clear shared-state anchor.
- If a helper surface is proposed only to expose one stored mount fact, it needs a named downstream caller; otherwise it should not appear.

