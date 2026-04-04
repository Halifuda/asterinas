<!-- SPDX-License-Identifier: MPL-2.0 -->

# Creator Serial Handoff

## Metadata

- Component ID: `EXR-SYSROOT-06`
- Title: Root-Directory System-Entry Scanner
- Status: `SerialImplementing`
- Author: `creator`
- Date: `2026-04-04`
- Task packet: `EXR-SYSROOT-06-CREATE-20260404-1412`
- Implemented spec:
  - `00_architect.md`
  - `01_designer_core.md`
  - `03_designer_ktest.md`
- Pass kind: `serial implementation`

## Planned File Ownership

- Files to edit:
  - `kernel/src/fs/fs_impls/exfat_refactor/sysroot.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-SYSROOT-06/10_creator_serial.md`
- Files intentionally left untouched:
  - `kernel/src/fs/fs_impls/exfat_refactor/dentry.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
  - all checker-owned test files and artifacts

## Implementation Notes

Implemented the synchronous root-directory system-entry scanner in `sysroot.rs` and wired the new module into `mod.rs`.

The pass now provides:

- a read-only discovery aggregate plus typed bitmap and upcase discovery records,
- an opaque root-entry location token,
- duplicate, missing, malformed, wrong-kind, and truncated root-entry rejection at the scanner boundary,
- preservation of the root entry location token, start cluster, byte size, and `UPCASE` checksum facts for later loaders.

The implementation stays within the specified boundary:

- no mount bootstrap state,
- no general directory API,
- no bitmap or upcase payload loading,
- no tests or command execution,
- no edits to `dentry.rs` or `inode.rs`.

## Approved Deviations

None.

## Optional Self-Checks

- Commands run, if any:
  - None.
- Compile checks run, if any:
  - None.
- Manual reasoning checks:
  - The scanner remains synchronous and read-only.
  - The result surface is discovery-only and stays loader-shaped for later `UPCASE-07A` and `BITMAP-08A` work.

## Remaining Risks

- Checker coverage still needs to prove that malformed chain termination is surfaced as an error rather than being normalized into end-of-directory.
- Checker coverage still needs to prove that mixed root content does not widen this component into a general directory API.
