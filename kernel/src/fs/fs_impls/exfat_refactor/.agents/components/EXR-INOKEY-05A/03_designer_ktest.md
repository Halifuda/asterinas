<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Ktest

## Metadata

- Component ID: EXR-INOKEY-05A
- Title: Inode Identity Key And Opened-Inode Lookup
- Status: `Specified`
- Author: designer
- Date: 2026-04-01

## Purpose

Define the minimal checker-owned regression coverage needed to prove that inode identity is stable and root stays explicit.

## Test Ownership

- Owner: checker
- Test style: `#[ktest]`
- Placement: local to `inode.rs`
- Helper touch: none expected

## Required Coverage

### Scenario 1: Ordinary key packing is stable

- Test intent:
  - Confirm the canonical location-based key helper packs the same key for the same validated cluster and byte offset.
- Suggested test shape:
  - Use a representative data-cluster id and a representative in-cluster byte offset.
- Assertions:
  - The helper returns `Ok`.
  - Repeating the same inputs returns the same key.
  - The key preserves the legacy packed layout rather than inventing a new identity scheme.

### Scenario 2: Root stays explicit

- Test intent:
  - Confirm the root inode has a dedicated key path and does not rely on a packed location-derived key.
- Assertions:
  - `ExfatInodeKey::root()` returns the reserved root key.
  - The root key is distinct from at least one ordinary packed key derived from a valid data-cluster location.
  - The root constructor does not require cluster or offset inputs.

### Scenario 3: Offset overflow or truncation is rejected

- Test intent:
  - Confirm the key helper does not silently truncate the packed offset field.
- Suggested assertions:
  - An offset that does not fit the packed low 32-bit field returns an error.
  - The error path is visible to the caller rather than being normalized into a different key.

## Observability

- These tests should only inspect identity-key packing behavior.
- They should not require directory traversal, mount sequencing, page-cache behavior, FAT walking, or payload construction.
- They should not introduce a separate helper module unless the local `inode.rs` test block becomes unexpectedly crowded, which is not expected for this component.

## Minimal Checker Obligation

The checker must include a regression that explicitly proves the root special case is not produced by the ordinary packed helper. That regression can live inside the root scenario, but it must be named and asserted clearly enough that future readers can see that root is reserved.

## Exit Condition

The ktest plan is complete when a future checker can implement the coverage using only local `#[ktest]` blocks and can verify both of these statements:

1. ordinary inode keys come from the canonical location-based helper,
2. root uses a distinct explicit constructor.
