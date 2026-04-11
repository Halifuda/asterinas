<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Ktest

## Metadata

- Component ID: `EXR-DIR-OPS-23`
- Title: `ExfatInode` Read-Only Directory Operations Checker Coverage
- Status: `Specified`
- Author: designer
- Date: `2026-04-10`
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-DIR-OPS-23/20260410-1545-designer-packet.md`
- Based on architect artifact: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-OPS-23/00_architect.md`

## Purpose

Define the minimal checker-owned regression coverage needed to prove that `lookup` and `readdir_at` are real read-only `ExfatInode` directory behaviors that consume accepted owners instead of reintroducing a helper boundary.

## Test Ownership

- Owner: checker
- Test style: `#[ktest]`
- Placement: local to `inode.rs`
- Helper touch: owner-private test helpers inside `inode.rs` only if needed to keep the tests readable

## Required Coverage

### Scenario 1: Case-aware lookup resolves a canonical child handle

- Test intent:
  - Confirm directory lookup consumes filesystem-owned canonicalization and filesystem-owned child-handle reuse.
- Suggested test shape:
  - Build a directory inode with a directory record whose visible name differs only by case from the queried name.
- Assertions:
  - Lookup succeeds for the case-equivalent name.
  - Repeated lookup returns the same canonical child handle rather than a duplicate inode shell.
  - The result depends on the installed upcase behavior, not on raw byte equality alone.

### Scenario 2: Lookup miss stays read-only

- Test intent:
  - Confirm a missing name does not mutate directory state or create placeholder child entries.
- Suggested test shape:
  - Query for a name that does not appear in the directory record stream.
- Assertions:
  - Lookup returns the expected miss error.
  - A later successful lookup for an existing entry is unaffected.
  - No synthetic child handle is published for the missing name.

### Scenario 3: Readdir emits visible entries in stable order

- Test intent:
  - Confirm `readdir_at` projects validated file records into user-visible entries and leaves raw system entries hidden.
- Suggested test shape:
  - Use a directory stream that contains ordinary file records plus root-directory singleton metadata entries.
- Assertions:
  - File records are emitted in stable order.
  - Raw bitmap and upcase singleton entries are not emitted as user-visible children.
  - The enumeration remains read-only and does not depend on mutation support.

### Scenario 4: Readdir continuation token is stable

- Test intent:
  - Confirm `readdir_at` can resume from the returned offset without restarting visible enumeration.
- Suggested test shape:
  - Run one `readdir_at` call that stops before the end, record the returned offset, then run a second call from that offset.
- Assertions:
  - The second call continues after the previously emitted entry set.
  - Repeating the first call from the same starting offset reproduces the same prefix.
  - The returned next offset advances monotonically over the same directory snapshot.

## Observability

- These tests should inspect only read-only directory behavior on `ExfatInode`.
- They should consume `DirectoryEngine`, `UpcaseTable`, and filesystem-owned reuse indirectly through the inode methods rather than testing those owners independently again.
- They should not introduce namespace mutation, mount/open sequencing, or file-data coverage.
- No dedicated concurrency tests are required beyond the repeated-call stability covered above.

## Minimal Checker Obligation

The checker must include a regression that proves:

- lookup consumes filesystem-owned canonicalization,
- lookup reuses one canonical child handle for repeated resolution,
- readdir hides system entries and preserves visible ordering,
- and readdir continuation remains stable across repeated calls.

## Exit Condition

The ktest plan is complete when a future checker can implement the coverage entirely in `inode.rs` tests and can verify that `ExfatInode` owns read-only directory lookup and enumeration without reopening mount/open, mutation, or a separate lookup-service boundary.
