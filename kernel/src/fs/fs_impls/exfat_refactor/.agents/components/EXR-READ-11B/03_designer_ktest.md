<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Ktest

## Metadata

- Component ID: `EXR-READ-11B`
- Title: Buffered Regular-File Read Execution And Read-Side Zero-Fill
- Status: `Specified`
- Author: `designer`
- Date: `2026-04-05`
- Task packet: `EXR-READ-11B-DESIGN-20260405-1134`
- Based on architect artifact: `00_architect.md`

## Scope

- In scope:
  - Checker-owned serial regressions for buffered regular-file reads.
  - Coverage for initialized data, FAT-backed placement, zero-fill, and EOF behavior.
  - Validation that the read path returns caller-visible bytes only and does not re-own mapping.
- Out of scope:
  - Async tests or separate concurrency harnesses.
  - Directory lookup, root discovery, mount bootstrap, or namespace mutation.
  - Page-cache backend ownership tests beyond the behavior needed to exercise buffered reads.
  - Allocation growth, truncate, writeback, or direct I/O.

## Test Specification

- Test fixtures:
  - One contiguous regular-file fixture whose initialized prefix is placement-backed and whose visible file length is larger than its valid-data length.
  - One FAT-backed regular-file fixture with the same read-visible split between valid data and zero-fill tail.
  - One fixture that places the read offset entirely inside the zero-fill range but still below `data_length`.
  - One EOF fixture that reads at or beyond `data_length`.
- Required checker-owned tests:
  - `contiguous_buffered_read_returns_expected_initialized_bytes`
    - Proves the read path returns the expected initialized bytes for a contiguous file.
    - Proves the returned bytes come from the accepted placement boundary rather than a remapping shortcut.
  - `fat_backed_buffered_read_returns_expected_initialized_bytes`
    - Proves the read path returns the expected initialized bytes for a FAT-backed file.
    - Proves the result is still correct when the initialized prefix depends on the placement boundary from `EXR-READ-11A`.
  - `buffered_read_zero_fills_visible_tail`
    - Proves bytes in the visible file range beyond `valid_data_length` are returned as zeros.
    - Proves the zero-fill path does not expose stale disk contents or trigger write-side growth.
  - `buffered_read_at_or_beyond_data_length_returns_eof`
    - Proves offsets at or beyond `data_length` return zero bytes.
    - Proves EOF is distinguished from the zero-fill range inside the visible file length.

## Observability

- Use assertion macros only.
- Keep each test focused on one visible read behavior.
- Prefer explicit fixtures over inline construction when the split between initialized bytes and zero-fill bytes would otherwise be unclear.
- Do not inspect implementation-private backend state unless it is the minimum needed to show that the buffered-read contract is not re-deriving mapping.

## Acceptance Notes

- The ktests should validate buffered read execution, not the placement helper itself.
- The zero-fill regression must cover the visible-but-uninitialized range between `valid_data_length` and `data_length`.
- No dedicated concurrency tests are required for this component, and no `02_designer_async.md` file is needed.
