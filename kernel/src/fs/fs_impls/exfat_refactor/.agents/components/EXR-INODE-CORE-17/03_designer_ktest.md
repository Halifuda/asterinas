<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Ktest

## Metadata

- Component ID: EXR-INODE-CORE-17
- Title: Inode Carrier And Metadata Owner
- Status: `Specified`
- Author: designer
- Date: 2026-04-07
- Based on architect artifact: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CORE-17/00_architect.md`

## Purpose

Define the minimal checker-owned regression coverage needed to prove that `ExfatInode` is a real metadata carrier, not a hidden `ExfatDentrySet`/`ExfatChain` wrapper or a cache placeholder.

## Test Ownership

- Owner: checker
- Test style: `#[ktest]`
- Placement: local to `inode.rs`
- Helper touch: none expected

## Required Coverage

### Scenario 1: Trusted inputs are snapped into inode metadata

- Test intent:
  - Confirm the inode copies the scalar facts needed for VFS metadata from trusted construction inputs.
- Suggested test shape:
  - Build a trusted `ExfatDentrySet` and `ExfatChain` through the existing local helpers.
  - Construct an `ExfatInode` from those trusted inputs and then inspect the inode through VFS accessors.
- Assertions:
  - `ino()` returns the expected stable inode number.
  - `size()` matches the trusted file size snapshot.
  - `type_()`, `mode()`, `owner()`, and `group()` match the constructor snapshot.
  - `atime()`, `mtime()`, and `ctime()` match the constructor snapshot.
  - `metadata()` agrees with the dedicated accessors for the same inode.

### Scenario 2: The filesystem back-reference is live

- Test intent:
  - Confirm the inode keeps only a weak filesystem reference and can still recover the owning filesystem while it is live.
- Suggested test shape:
  - Build the inode from an `Arc<ExfatFs>` owner and then call `fs()`.
- Assertions:
  - `fs()` returns the owning filesystem object.
  - The returned filesystem is pointer-equal to the one used to build the inode, or otherwise clearly the same owner.
  - The inode does not require a strong filesystem cycle to answer the question.

### Scenario 3: Temporary seams are explicit rejections

- Test intent:
  - Confirm the unimplemented data-path and mutation methods are visibly temporary rather than silently durable.
- Suggested assertions:
  - `read_at()` returns the named temporary rejection.
  - `write_at()` returns the named temporary rejection.
  - `resize()`, `set_mode()`, `set_owner()`, and `set_group()` reject rather than mutating hidden state.
- Observability:
  - The rejection path should be clear enough that a future reader can see the work belongs to `EXR-READ-OPS-25`, `EXR-WRITE-30`, `EXR-PGCACHE-26`, or the later write-side ownership units.

## Observability

- These tests should only inspect inode metadata, owner recovery, and explicit temporary seams.
- They should not require inode-cache, directory, page-cache, or sync coverage.
- They should not introduce a separate helper module unless the local `inode.rs` test block becomes unexpectedly cluttered, which is not expected for this carrier.
- No dedicated concurrency tests required.

## Minimal Checker Obligation

The checker must include a regression that demonstrates the inode is a snapshot carrier:

- trusted input values are copied into the inode,
- `metadata()` remains coherent with the dedicated accessors,
- `fs()` still returns the live owner through the weak reference,
- the temporary seams are explicit rejections, not silent stubs.

## Exit Condition

The ktest plan is complete when a future checker can implement the coverage using only `inode.rs` tests and can verify that `ExfatInode` is the stable VFS metadata owner while read/write and mutation behavior remain intentionally deferred.
