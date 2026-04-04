<!-- SPDX-License-Identifier: MPL-2.0 -->

# Creator Serial Handoff

## Metadata

- Component ID: `EXR-BITMAP-08A`
- Title: Allocation Bitmap Loading And Read-Only Occupancy Queries
- Status: `SerialImplemented`
- Author: `creator`
- Date: `2026-04-04`
- Task packet: `EXR-BITMAP-08A-CREATE-20260404-1420`
- Implemented spec:
  - `00_architect.md`
  - `01_designer_core.md`
  - `03_designer_ktest.md`
- Pass kind: `serial implementation`

## Planned File Ownership

- Files edited:
  - `kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BITMAP-08A/10_creator_serial.md`
- Files intentionally left untouched:
  - `kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
  - all other `EXR-BITMAP-08A` artifact files
  - all checker-owned test files and artifacts

## Implementation Notes

Implemented the read-only allocation-bitmap surface in `bitmap.rs`.

- Added `ExfatAllocationBitmap` as the canonical read-only bitmap value.
- Added `ExfatAllocationBitmap::load()` to:
  - consume the validated root-discovery bitmap record,
  - validate the bitmap against the volume geometry,
  - read the discovered bitmap payload through the existing metadata I/O helper,
  - reject undersized payloads,
  - reject payloads whose own clusters are not marked allocated.
- The load path validates the bitmap as one contiguous cluster span before
  exposing the read-only surface.
- Added read-only occupancy queries for:
  - a single cluster,
  - a bounded half-open cluster range.
- Kept the module free of mutation, hinting, free-space search, and dirty tracking.

## Verification

- No compile, cargo, docker, or test commands were run, per the task packet.
- I stayed within the assigned write set and did not touch `mod.rs`.

## Residual Risks

- Checker-owned coverage still needs to prove the loader accepts a valid bitmap and rejects undersized or self-inconsistent payloads.
- A later pass would need broader chain traversal support if a future volume
  turns out to store the bitmap file non-contiguously.
- The module remains intentionally read-only; allocation policy and any future write path still belong to later passes.
