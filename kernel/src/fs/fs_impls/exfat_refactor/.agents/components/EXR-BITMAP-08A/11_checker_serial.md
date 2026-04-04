<!-- SPDX-License-Identifier: MPL-2.0 -->

# Checker Serial Result

## Metadata

- Packet ID: `EXR-BITMAP-08A-CHECK-20260404-1435`
- Component ID: `EXR-BITMAP-08A`
- Role: `checker`
- Phase: `checker-serial`
- Date: `2026-04-04`

## Verification

- Command run under the execution lock:
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test bitmap::tests'`
- Result:
  - Pass
  - Exit code: `0`
- Observations:
  - QEMU booted in TCG mode, with the expected `TCG doesn't support requested feature` warnings in the guest launch output.
  - The focused `bitmap::tests` ktest filter completed successfully.

## Coverage Added

- Local `#[ktest]` regressions were added in `bitmap.rs` for:
  - valid bitmap loading with occupied and free occupancy queries,
  - undersized bitmap rejection,
  - self-coverage rejection when the bitmap file does not mark its own clusters allocated,
  - out-of-range cluster query rejection,
  - oversized bitmap acceptance.

