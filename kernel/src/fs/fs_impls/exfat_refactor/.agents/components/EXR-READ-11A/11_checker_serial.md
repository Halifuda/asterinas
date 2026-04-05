<!-- SPDX-License-Identifier: MPL-2.0 -->

# Checker Report

## Metadata

- Component ID: `EXR-READ-11A`
- Title: Logical-To-Physical Mapping For Existing Regular-File Reads
- Status: `SerialChecked`
- Author: `main-agent`
- Date: `2026-04-05`
- Task packet: locally executed by `main-agent` in the `sable-lattice` wave; no delegated checker packet
- Checked implementation:
  - `10_creator_serial.md`
- Pass kind: `serial`

## Scope of Review

Checked the new `read.rs` mapping boundary plus its narrow supporting helpers in `inode.rs`, `fat.rs`, and `mod.rs` against the accepted `EXR-READ-11A` architect and designer artifacts.

## Test Changes

Added local `#[ktest]` coverage in `read.rs` for:

- contiguous placement without FAT reads,
- FAT-backed placement through accepted chain walking,
- EOF behavior at and beyond `valid_data_length`,
- rejection of directory shells before they cross the read-mapping boundary.

Each ktest has a short scenario comment and stays local to `read.rs`.

## Findings

No remaining blocking findings.

The first checker execution found one local test-harness defect:

- `RejectReadBlockDevice` needed `Debug` to satisfy the `BlockDevice` trait bound.

That fix stayed inside the `read.rs` checker-owned test fixture and did not widen the production read-mapping boundary.

## Verified Properties

- `map_logical_read_offset(...)` returns placement facts only and does not drift into buffered read policy.
- The mapper rejects directory shells through the narrow inode read-view boundary.
- Contiguous placement succeeds without requiring FAT reads.
- FAT-backed placement still follows accepted chain walking instead of arithmetic remapping.
- Offsets at or beyond `valid_data_length` return `None`.
- Focused exact-name `cargo osdk test` runs passed under `.agents/tools/checker_lock.sh` in the TCG-backed container environment.
- The recorded filters are the exact local ktest function names:
  - `contiguous_offset_maps_without_fat_reads`
  - `fat_backed_offset_maps_through_chain`
  - `offset_at_valid_data_end_returns_none`
  - `non_regular_file_is_rejected`
- Those filters were treated as valid coverage because `cargo osdk test` matches test-path suffixes, so each exact function name maps to the intended `read.rs` ktest by source inspection.

## Unverified Properties

- No reviewer pass had run when this serial checker completed.
- Buffered `read_at`, page-cache backend ownership, and zero-fill behavior remain intentionally deferred to later components.

## Recommendation

- Next owner: `main-agent`
- Reason: run the bounded reviewer pass and then the normal post-review final checker.
- Blocking or non-blocking: non-blocking
