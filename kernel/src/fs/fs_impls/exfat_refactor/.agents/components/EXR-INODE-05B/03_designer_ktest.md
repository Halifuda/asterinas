<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Ktest

## Metadata

- Component ID: EXR-INODE-05B
- Title: Read-Only Inode Metadata Shell
- Status: `Specified`
- Author: designer
- Date: 2026-04-01

## Purpose

Define the minimal checker-owned regression coverage needed to prove that the inode shell preserves validated metadata facts, keeps the root case explicit, and stays free of page-cache or VFS behavior.

## Test Ownership

- Owner: checker
- Test style: `#[ktest]`
- Placement: local to `inode.rs`
- Helper touch: tests may inspect module-local private fields; no production getter surface is required in this component

## Required Coverage

### Scenario 1: Ordinary shell construction preserves validated facts

- Test intent:
  - Confirm an ordinary inode shell can be constructed from an accepted inode key, a validated file-record boundary, and validated chain facts.
- Suggested test shape:
  - Build a small synthetic validated file-record set and a small validated chain using the existing refactor helpers.
- Assertions:
  - The constructor succeeds.
  - The stored inode identity matches the accepted key.
  - The stored file attributes still reflect a non-root regular-file shell.
  - The stored `valid_data_length` and `data_length` match the stream facts.
  - The stored chain facts still match the validated chain.
  - The stored raw name units preserve the logical name units exactly as provided by the validated file-record boundary.

### Scenario 2: The root special case stays explicit

- Test intent:
  - Confirm root construction uses a dedicated synthetic path and does not rely on the ordinary file-record constructor.
- Assertions:
  - `new_root(...)` succeeds for the reserved root key.
  - `new_root(...)` rejects a non-root key.
  - The stored root key remains the reserved root identity.
  - The stored file attributes still encode directory semantics.
  - The root shell does not require a parsed file-record boundary.
  - The ordinary constructor rejects the reserved root key.

### Scenario 3: Directory size equality is enforced

- Test intent:
  - Confirm the shell rejects a directory payload whose valid-data length and data length disagree.
- Suggested test shape:
  - Start from an otherwise valid directory record or root payload and perturb one of the two length fields.
- Assertions:
  - The constructor returns an error.
  - The error is surfaced to the caller instead of being normalized silently.

### Scenario 4: Synthetic root length equality is enforced

- Test intent:
  - Confirm the synthetic root constructor rejects mismatched logical and allocated lengths.
- Suggested assertions:
  - `new_root(...)` returns an error when `valid_data_length != data_length`.
  - The mismatch is surfaced to the caller instead of being normalized silently.

## Observability

- These tests should only inspect shell construction and the stored construction results.
- They should not require `PageCache`, `PageCacheBackend`, buffered I/O, directory traversal, mount sequencing, or VFS trait coverage.
- They should not introduce a separate helper module unless the local `inode.rs` test block becomes unexpectedly crowded, which is not expected for this component.

## Minimal Checker Obligation

The checker must include a regression that explicitly proves the root special case is not produced by the ordinary constructor. That regression can live inside the root scenario, but it must be named and asserted clearly enough that future readers can see that root is reserved.

## Exit Condition

The ktest plan is complete when a future checker can implement the coverage using only local `#[ktest]` blocks and can verify both of these statements:

1. ordinary inode shells come from the canonical accepted-key plus validated-facts constructor,
2. root uses a distinct explicit constructor and the shell still remains read-only.
