<!-- SPDX-License-Identifier: MPL-2.0 -->

# Creator Serial Handoff

## Metadata

- Component ID: `EXR-UPCASE-07B`
- Title: Canonical Upcase-Backed Case Folding And Name Hashing
- Status: `SerialImplementing`
- Author: `creator`
- Date: `2026-04-04`
- Task packet: `EXR-UPCASE-07B-CREATE-20260404-1508`
- Implemented spec:
  - `00_architect.md`
  - `01_designer_core.md`
  - `03_designer_ktest.md`
- Pass kind: `serial implementation`

## Planned File Ownership

- Files to edit:
  - `kernel/src/fs/fs_impls/exfat_refactor/upcase_table.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/fileset.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07B/10_creator_serial.md`
- Files intentionally left untouched:
  - `kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
  - all checker-owned test files and artifacts
  - all other component artifacts under `EXR-UPCASE-07B`

## Implementation Notes

Implemented the canonical upcase-backed name-hash service on the loaded table boundary and added a fileset-side construction path that can consume it directly.

- Added `ExfatUpcaseTable::name_hash()` in `upcase_table.rs` to fold logical UTF-16 units through the loaded table and derive the exFAT `NameHash` from the folded UTF-16 bytes.
- Kept the folding primitive private so the canonical hash service remains the only external contract on the table surface.
- Refactored `fileset.rs` to share file-record construction through one internal builder.
- Added a `from_trusted_metadata_with_upcase()` constructor that uses the canonical table-backed hash service instead of the provisional raw UTF-16 checksum path.
- Preserved the existing legacy constructor for compatibility with call sites outside this write set.

## Approved Deviations

None.

## Optional Self-Checks

- Commands run, if any:
  - None.
- Compile checks run, if any:
  - None.
- Manual reasoning checks:
  - The hash service now folds each UTF-16 unit through the loaded table before hashing.
  - The loaded table remains read-only and the folding helper stays private to the module.
  - The fileset module now has a canonical table-backed constructor path, but existing call sites outside this write set still need to be moved over in a later pass.

## Remaining Risks

- The consumer-side redirection is only partially staged because the current `inode.rs` ktest call site is outside the write set for this pass.
- Checker-owned regressions still need to prove the fold-before-hash behavior and the consumer-side use of the canonical constructor path.
