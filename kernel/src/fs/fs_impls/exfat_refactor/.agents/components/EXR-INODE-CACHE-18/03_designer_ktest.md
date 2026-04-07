<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Ktest

## Metadata

- Component ID: EXR-INODE-CACHE-18
- Title: ExfatFs Opened-Inode Table And Validated InodeKey
- Status: `Specified`
- Author: designer
- Date: 2026-04-07
- Based on architect artifact: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INODE-CACHE-18/00_architect.md`

## Purpose

Define the minimal checker-owned regression coverage needed to prove that `ExfatFs` owns the opened-inode table, that `InodeKey` is a validated location-derived value type, and that the root special case stays separate from the ordinary keyspace.

## Test Ownership

- Owner: checker
- Test style: `#[ktest]`
- Placement: local to `fs.rs`
- Helper touch: none expected

## Required Coverage

### Scenario 1: `InodeKey` is derived from validated location facts

- Test intent:
  - Confirm the key boundary depends only on trusted directory-location facts and not on mutable inode metadata.
- Suggested test shape:
  - Build two trusted location facts that point to the same primary-entry location while varying non-identity metadata such as name text, timestamps, or file size.
  - Build a second pair that differs in primary-entry location.
- Assertions:
  - Equivalent validated location facts produce equal keys.
  - Changing only mutable inode metadata does not change the key.
  - Changing the primary-entry location changes the key.

### Scenario 2: The opened-inode table reuses the canonical handle

- Test intent:
  - Confirm repeated publication of the same validated key returns the same `Arc<ExfatInode>` handle.
- Suggested test shape:
  - Publish an inode for one validated key, then request the same key again through the owner table path.
  - Compare the returned handles by identity.
- Assertions:
  - The table returns the canonical handle instead of creating a second inode shell.
  - Removing the exact key drops only that entry.
  - Unrelated keys remain available.

### Scenario 3: The root special case stays out of the ordinary keyspace

- Test intent:
  - Confirm the root handle, once published by the later handoff, is not represented as a synthetic `InodeKey`.
- Suggested test shape:
  - Exercise the dedicated root slot directly on the filesystem owner.
  - Confirm that ordinary keyed lookup and root publication are distinct code paths.
- Assertions:
  - Root is not retrievable through the ordinary opened-inode map as if it were a normal key.
  - The root special case remains an owner-private slot rather than a fake key.

## Observability

- These tests should only inspect key derivation, handle reuse, root separation, and exact-key removal.
- They should not require mount/open sequencing, directory traversal, page-cache behavior, or inode data-path coverage.
- They should not introduce a separate helper module unless the local `fs.rs` test block becomes unexpectedly cluttered, which is not expected for this component.
- No dedicated concurrency tests are required because the serialization contract is owner-private and can be validated through the reuse and root-separation regressions.

## Minimal Checker Obligation

The checker must include regressions that prove:

- `InodeKey` comes from trusted directory-location facts, not from mutable inode metadata.
- The opened-inode table returns one canonical `Arc<ExfatInode>` for repeated publication of the same key.
- The root special case is separate from the ordinary keyspace.
- Exact-key removal does not disturb unrelated entries.

## Exit Condition

The ktest plan is complete when a future checker can validate the owner-owned inode cache contract entirely from `fs.rs` tests and can confirm that root remains a special-case owner slot rather than an ordinary `InodeKey`.
