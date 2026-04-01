<!-- SPDX-License-Identifier: MPL-2.0 -->

# Asterinas Architect Priors For exFAT

This note records the Asterinas-local knowledge that an architect should treat as prior context before splitting or sequencing the exFAT refactor.

It complements:

- `Microsoft-exFAT-spec.md`, which provides the on-disk rules.
- `linux-exFAT-implementation-summary.md`, which provides a mature implementation reference model.

This file focuses on the current Asterinas codebase, its FS/VFS contracts, and the practical testing environment that shapes implementable component boundaries.
It is intentionally not the normative source for exFAT semantics. Unless a task packet records a justified local exception, Microsoft exFAT rules take precedence, Linux exFAT remains the preferred implementation reference, and this file only constrains local integration, code organization, and testing reality.

## 1. Why This File Exists

The Microsoft specification and the Linux summary are necessary, but they are not sufficient on their own.

An architect for this project must also understand:

1. how exFAT is currently split inside Asterinas,
2. which VFS traits and page-cache interfaces the implementation must satisfy,
3. which global state and locking decisions already exist,
4. which parts of the current implementation are clearly incomplete or provisional,
5. how filesystem behavior is actually tested in this repository.

Without this local context, a component plan can easily become structurally correct in theory but misaligned with the real Asterinas integration surface.
The inverse failure mode is also forbidden: this file must not be used to pull the refactor back toward reproducing legacy Asterinas exFAT semantics when Microsoft and Linux priors support a cleaner or more correct design.

## 2. Repository-Level Constraints That Matter To The Architect

- The repository-root `AGENTS.md` is binding on all agents.
- `kernel/` code must remain safe Rust. No `unsafe` may be introduced into exFAT code under `kernel/src/fs/fs_impls/exfat_refactor/`.
- Creator output must fully comply with the coding guidelines in the repository-root `AGENTS.md`.
- Component plans should favor narrow visibility, small focused functions, explicit invariants, and specification-first behavior.

These constraints are not only creator constraints. They directly affect how the architect should cut components.

## 3. Current Asterinas exFAT Source Map

The legacy Asterinas exFAT implementation is split into these files:

- `bitmap.rs`: allocation bitmap loading, free-cluster search, bit updates, dirty-byte tracking.
- `constants.rs`: exFAT constants used throughout the implementation.
- `dentry.rs`: raw and typed directory entry parsing, dentry-set validation, name entries, checksum handling.
- `fat.rs`: cluster-chain abstraction, FAT entry interpretation, chain walking, allocation and freeing logic.
- `fs.rs`: filesystem instance object, mount/open path, caches, global lock, block-device and page-cache integration.
- `inode.rs`: inode representation, metadata state, page-cache backend, VFS inode operations, directory and file behavior.
- `super_block.rs`: boot-sector structs and translation into the in-memory superblock.
- `upcase_table.rs`: upcase-table loading and case-folding support.
- `utils.rs`: checksums, timestamps, hash helpers, and small exFAT-specific utility logic.
- `mod.rs`: filesystem registration plus exFAT ktests.

Architecturally, this means the current legacy implementation is already organized by major exFAT concerns, but the boundaries are still coarse.
Refactor components in `exfat_refactor` should usually treat these legacy files as a baseline to learn from, not as files that must be edited in place or as the semantic target to preserve by default.

## 4. Asterinas FS/VFS Contracts exFAT Must Satisfy

### FileSystem contract

Every concrete filesystem instance implements the `FileSystem` trait.
The key requirements are:

- return the filesystem name,
- provide `sync()`,
- return the root inode,
- expose a `SuperBlock`,
- maintain filesystem event subscriber statistics.

This means exFAT work cannot be planned as a pure parser project.
Mount/open, root inode availability, and superblock reporting are first-class integration requirements.

### Inode contract

The VFS-facing inode contract is wide.
The `Inode` trait includes:

- basic metadata and timestamps,
- file I/O through `read_at` and `write_at`,
- size management,
- directory operations such as `create`, `lookup`, `readdir_at`, `unlink`, `rmdir`, and `rename`,
- synchronization hooks,
- optional xattr support,
- access to the owning filesystem.

For the architect, this means component boundaries should be aligned to user-visible inode behavior, not only to on-disk record types.

### Page cache contract

The Asterinas page cache expects a `PageCacheBackend` that can:

- read a page asynchronously,
- write a page asynchronously,
- report backend page count.

The current exFAT inode already implements this contract.
Any refactor of mapping, allocation, truncation, or data I/O must preserve this page-cache integration surface.

## 5. Current exFAT Runtime Objects And What They Imply

### `ExfatFs`

`ExfatFs` is the filesystem-wide state object.
It currently owns:

- the block device,
- the in-memory `ExfatSuperBlock`,
- the allocation bitmap,
- the upcase table,
- mount options,
- inode numbering state,
- the opened-inode table,
- a FAT LRU cache,
- a metadata page cache,
- a global mutex used to avoid deadlocks,
- FS event subscriber stats.

This is one of the most important local facts for the architect.
The current implementation explicitly states that bitmap and inode access must respect a global locking rule.
That means concurrency-sensitive refactor components cannot be planned independently of lock order and shared-state ownership.

### Mount/open sequence

The current `ExfatFs::open()` sequence is:

1. read the superblock,
2. verify the boot region,
3. create the root chain,
4. build the root inode,
5. load the upcase table from the root directory,
6. load the allocation bitmap from the root directory,
7. insert the root inode into the opened-inode table.

This sequence gives a strong hint for dependency ordering.
Mount validation, superblock interpretation, chain walking, root inode construction, upcase loading, and bitmap loading are not independent.

### Explicit incompletenesses in current code

The current exFAT implementation visibly contains unfinished areas.
Examples include:

- backup-superblock fallback is still a TODO,
- UTF-8 handling is still a TODO,
- NLS initialization is still a TODO,
- some helper functions and paths still carry `FIXME` or provisional behavior.

Architecturally, these should not be hidden inside unrelated components.
They are natural candidates for explicit non-goals or for dedicated follow-up components.

### `ExfatSuperBlock`

The Asterinas in-memory superblock already normalizes:

- sector size,
- cluster size,
- FAT offsets,
- data-region offset,
- root cluster,
- directory-entry density,
- volume flags and persistent flag subsets,
- cluster search pointer,
- used-cluster accounting.

This means that the architect should separate:

1. on-disk boot/BPB parsing rules, and
2. the normalized runtime geometry/state used by the rest of the FS.

That is a meaningful component boundary in Asterinas.

### `ExfatChain`

`ExfatChain` is the key cluster-walking abstraction.
It combines:

- current cluster ID,
- cluster count,
- FAT-contiguous-vs-FAT-chain flags,
- a back-reference to the filesystem.

Its behavior already encodes the main exFAT allocation branch:

- contiguous mode when FAT chain is not in use,
- explicit FAT walking when FAT chain is in use.

The architect should treat chain semantics as a foundational component, because bitmap management, directory walking, inode mapping, and allocation all depend on it.

### `ExfatBitmap`

The bitmap code already makes a design choice that matters:

- the bitmap is loaded by discovering the bitmap dentry in the root directory,
- the bitmap is kept in memory as a bitvec,
- updates may be tracked as dirty-byte ranges,
- free-space search logic is encapsulated here.

That makes the bitmap manager a real component, not just an implementation detail of FAT allocation.

### `ExfatDentry` and `ExfatDentrySet`

The current dentry layer already distinguishes:

- typed directory entries,
- deleted and unused entries,
- validation of multi-entry file records,
- checksums and name-entry expansion.

It also contains a local state machine for validating file-entry sets.
This is a strong signal that directory-entry parsing/validation should remain an explicit component in the refactor plan.

### `ExfatInode`

`ExfatInode` is where exFAT metadata, VFS behavior, and page-cache integration meet.
Its state includes:

- dentry-set position and size,
- inode type and FAT attributes,
- start chain,
- size and allocated size,
- timestamps,
- name,
- deletion state,
- parent relationship,
- page cache.

Architecturally, this means inode work should likely be split into more than one concern:

- metadata representation and persistence,
- directory semantics,
- file data mapping and page-cache behavior.

## 6. Current Asterinas-Specific Behavior Gaps And Special Cases

The architect should explicitly account for the following local realities:

- The current upcase logic does not yet fully model the on-disk upcase table for all strings. Some paths still fall back to generic uppercase behavior.
- Time handling contains ktest-specific fallback behavior because the time subsystem is not always initialized during tests.
- The current implementation uses the root directory to discover both the upcase table and the allocation bitmap during mount.
- exFAT tests currently rely on an embedded disk image and an in-memory block-device shim instead of only unit-testing small pure functions.

These are not merely implementation details.
They affect which components are safe to isolate and how testable those components are.

## 7. Testing Reality In Asterinas

### Two test worlds

Asterinas uses two distinct styles of tests:

1. Standard `#[test]` for ordinary Rust crates that can use `cargo test`.
2. Kernel-mode `#[ktest]` tests for kernel/OSTD code, executed through `cargo osdk test` and the repository `make ktest` flow.

For exFAT work under `kernel/`, the second style is the relevant one.

### Recommended ktest structure

The standard pattern is:

```rust
#[cfg(ktest)]
mod tests {
    use ostd::prelude::*;

    #[ktest]
    fn test_name() {
        // assertions
    }
}
```

This is the default shape used by the OSTD test framework and by generated templates.

### How ktests are run

The ktest framework gives a `cargo test`-like experience for `no_std` kernel code.
At a high level:

- `cargo osdk test` enables `cfg(ktest)` during compilation,
- tests marked with `#[ktest]` are registered into the test kernel,
- the test kernel runs them in QEMU,
- failures are reported with source file and line information.

At the repository level, `make ktest` is the aggregated entry point for kernel-mode unit tests.

### What filesystem tests look like in practice

The existing filesystem tests show several important patterns:

- exFAT tests build a memory-backed block device, load an embedded exFAT image, open the filesystem, and then assert VFS-visible behavior.
- configfs tests explicitly initialize the time subsystem and subsystem-specific test setup before exercising inode operations.
- overlayfs tests explicitly initialize time and VFS before building multi-layer test state.

The architect should assume that many meaningful filesystem tests in Asterinas are integration-heavy even when they are called unit tests.

## 8. What This Means For Architect Component Planning

The architect should use the following local heuristics when planning exFAT components in Asterinas:

1. Split along Asterinas-facing boundaries, not only Linux source-file boundaries.
2. Treat mount/bootstrap, chain semantics, dentry-set parsing, bitmap management, inode metadata, and file-data mapping as distinct concerns.
3. Keep VFS-facing behavior in scope when splitting components. A component that changes inode semantics is not just an internal helper refactor.
4. Make lock ordering and shared-state ownership explicit in any component that touches `ExfatFs`, bitmap state, or inode tables.
5. Treat clearly unfinished areas such as UTF-8/NLS support, backup-boot handling, and some time behavior as explicit work items or explicit non-goals.
6. Require ktest obligations in specifications for behavior-changing components, especially mount, lookup, readdir, create, truncate, allocation, and rename paths.

## 9. Minimal Local Knowledge Checklist For The Architect

Before producing `00_architect.md`, the architect should be able to answer:

1. What are the real Asterinas module boundaries in the current exFAT implementation?
2. Which parts of exFAT are pure on-disk logic, and which parts are Asterinas VFS integration logic?
3. Which runtime objects are shared and concurrency-sensitive?
4. Which current TODOs or incomplete behaviors must become explicit component work or explicit deferrals?
5. Which ktests already exist, and which future components will need new ktests to keep behavior controlled?

If these questions are not yet answered, the architect does not yet have enough Asterinas-local prior knowledge.
