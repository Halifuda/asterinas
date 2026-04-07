<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Ktest

## Metadata

- Component ID: `EXR-DIR-ENGINE-19`
- Title: `DirectoryEngine` Read-Only Record Stream
- Status: `Specified`
- Author: designer
- Date: 2026-04-07
- Based on architect artifact: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-ENGINE-19/00_architect.md`

## Purpose

Define the minimal checker-owned regression coverage needed to prove that `DirectoryEngine` is a real read-only directory record-stream owner and not a bag of raw-byte helpers.

## Test Ownership

- Owner: checker
- Test style: `#[ktest]`
- Placement: local to `directory.rs`
- Helper touch: none expected

## Required Coverage

### Scenario 1: Record emission preserves directory boundaries

- Test intent:
  - Confirm the scan cursor advances correctly across block and cluster boundaries while preserving on-disk record order.
- Suggested test shape:
  - Use the embedded exFAT image and a validated root-directory chain, then step the stream across a boundary where the next record begins in a different read window.
- Assertions:
  - The first emitted record matches the expected on-disk ordering.
  - The next emitted record begins exactly where the previous record ended.
  - No record is duplicated, skipped, or reordered when the scan crosses a boundary.

### Scenario 2: Valid file records become validated `ExfatDentrySet` values

- Test intent:
  - Confirm the engine groups a `File -> Stream -> Name+ -> benign secondary*` sequence into one validated record and does not leak partial state.
- Suggested test shape:
  - Build or reuse a directory image that contains one valid file record with at least one name dentry and optional benign tail entries.
- Assertions:
  - The emitted value is a validated `ExfatDentrySet`.
  - The set preserves the original dentry order.
  - The set remains checksum-valid and name-hash-valid through the existing fileset boundary.

### Scenario 3: Singleton system entries are surfaced without policy

- Test intent:
  - Confirm `Bitmap` and `Upcase` entries are surfaced as raw typed candidates only.
- Suggested test shape:
  - Use a directory image or constructed directory sequence containing one `Bitmap` dentry and one `Upcase` dentry.
- Assertions:
  - The candidate preserves the raw typed entry kind.
  - The candidate preserves the raw payload fields such as start cluster and size.
  - No name folding, bitmap interpretation, or hidden normalization is performed.

### Scenario 4: Tombstones and end markers are handled explicitly

- Test intent:
  - Confirm deleted entries are skipped and `Unused` ends the stream.
- Suggested test shape:
  - Place a deleted dentry before a valid record and an unused dentry after the final used record.
- Assertions:
  - Deleted entries do not become emitted records.
  - Scanning stops at the first `Unused` entry.
  - No record after `Unused` is observed.

## Observability

- These tests should only inspect directory-stream behavior, record grouping, and the raw singleton system-entry candidates.
- They should not require inode-cache, mount-open, or VFS directory coverage.
- They should not introduce a separate helper module unless the local `directory.rs` test block becomes unexpectedly cluttered, which is not expected for this component.
- No dedicated concurrency tests required.

## Minimal Checker Obligation

The checker must include a regression that proves the stream is boundary-aware:

- a record that crosses a read window is still emitted as one logical record,
- the next record begins at the correct cursor position,
- and singleton `Bitmap` / `Upcase` candidates are preserved without policy interpretation.

## Exit Condition

The ktest plan is complete when a future checker can implement the coverage using only `directory.rs` tests and can verify that the read-only directory stream preserves record boundaries, yields validated file records, and leaves system-entry policy to later owners.
