<!-- SPDX-License-Identifier: MPL-2.0 -->

# Creator Log

## Metadata

- Component ID: EXR-DENTRY-04A
- Title: Raw Dentry Layout And Typed Single-Entry Decode
- Status: `SerialImplementing`
- Author: creator
- Date: 2026-04-01

## Summary

Added the raw 32-byte exFAT directory-entry representation and a typed one-entry decode surface for the refactor module, keeping the work limited to single-entry classification without introducing any multi-entry validation state.

## Code Changes

- Added `RawExfatDentry` in `dentry.rs` as a packed 32-byte `Pod` type with a compile-time size check against `DENTRY_SIZE`.
- Added the typed `ExfatDentry` enum with the required concrete variants:
  - `File`
  - `Stream`
  - `Name`
  - `Bitmap`
  - `Upcase`
  - `VendorExt`
  - `VendorAlloc`
  - `GenericPrimary`
  - `GenericSecondary`
  - `Deleted`
  - `Unused`
- Added packed typed payload structs for the concrete dentry kinds so later file-record parsing can build on typed entries instead of redoing raw byte classification.
- Implemented a one-entry decode path from `RawExfatDentry` into `ExfatDentry` via `From`, with exact special-kind matches handled before the generic primary and secondary fallback ranges.
- Added `ExfatDentry::as_bytes()` to preserve the raw-byte view for later parsing stages that need to re-emit or inspect entry contents.

## Verification

- Per task instructions, I did not run compile, cargo, docker, or tests.
- I kept the work inside the allowed write set and did not touch checker or main-agent artifacts.

## Residual Risks

- Checker-owned tests still need to confirm representative decode behavior, deleted/unused handling, and generic fallback classification.
- The current pass intentionally stops before `ExfatDentrySet`, checksum handling, and any directory-scanning logic.
