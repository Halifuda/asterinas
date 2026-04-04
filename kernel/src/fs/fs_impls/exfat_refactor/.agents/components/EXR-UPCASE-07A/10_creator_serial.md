<!-- SPDX-License-Identifier: MPL-2.0 -->

# Creator Serial Handoff

## Metadata

- Component ID: `EXR-UPCASE-07A`
- Title: On-Disk Upcase Table Loader And Validator
- Status: `SerialImplementing`
- Author: `creator`
- Date: `2026-04-04`
- Task packet: `EXR-UPCASE-07A-CREATE-20260404-1420`
- Implemented spec:
  - `00_architect.md`
  - `01_designer_core.md`
  - `03_designer_ktest.md`
- Pass kind: `serial implementation`

## Planned File Ownership

- Files to edit:
  - `kernel/src/fs/fs_impls/exfat_refactor/upcase_table.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07A/10_creator_serial.md`
- Files intentionally left untouched:
  - `kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
  - all checker-owned test files and artifacts
  - all other component artifacts under `EXR-UPCASE-07A`

## Implementation Notes

Implemented the staged upcase-table loader and canonical read-only table surface in `upcase_table.rs`.

The pass now provides:

- a single loader entry point that consumes `ExfatSysRootUpcaseDiscovery`,
- a read-only `ExfatUpcaseTable` value that owns the validated UTF-16 payload,
- structural size validation for the discovered table byte count,
- checksum validation before the table becomes visible to later code,
- preservation of the full discovered payload rather than a legacy prefix.

The implementation stays within the specified boundary:

- no case-folding API,
- no name hashing,
- no fallback/default-table policy,
- no root rediscovery,
- no tests,
- no command execution,
- no edits to `mod.rs`.

## Approved Deviations

None.

## Optional Self-Checks

- Commands run, if any:
  - None.
- Compile checks run, if any:
  - None.
- Manual reasoning checks:
  - The loader rejects undersized, odd-sized, checksum-mismatched, and range-invalid discoveries.
  - The exposed table surface is read-only and only returns the validated words, byte size, and checksum fact.

## Remaining Risks

- Module wiring is still intentionally outside this write set, so the new loader remains staged until the main agent connects it.
- Checker coverage still needs to prove the loader preserves bytes beyond the legacy 128-entry boundary and rejects truncated payloads without introducing fallback behavior.
