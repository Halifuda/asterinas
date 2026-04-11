<!-- SPDX-License-Identifier: MPL-2.0 -->

# Creator Log

## Metadata

- Component ID: `EXR-UPCASE-20`
- Title: `ExfatFs` Upcase Table Ownership And Canonicalization Services
- Status: `SerialImplementing`
- Author: Codex
- Date: `2026-04-10`
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-UPCASE-20/20260410-1210-creator-serial-packet.md`
- Implemented spec: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-20/01_designer_core.md`
- Pass kind: `serial implementation`

## Planned File Ownership

- Files edited:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-20/10_creator_serial.md`
- Files intentionally left untouched:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/dentry.rs`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/io.rs`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/super_block.rs`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-20/` sibling artifacts

## Implementation Notes

Implemented owner-private upcase-table state inside `ExfatFs` in `fs.rs`.
The owner now keeps a single validated table behind the existing filesystem-owner mutex boundary and rejects any second publication attempt.

Added validated upcase-table publication from a raw `ExfatUpcaseDentry` plus the raw on-disk table bytes.
Publication checks the advertised size, validates the Microsoft exFAT checksum over the raw bytes, decodes compressed identity runs into the full Unicode range, and verifies the mandatory first-128 mappings before publishing the immutable table.

Added owner-local UTF-16 folding and name-hash services:

- `fold_utf16()` maps each code unit through the installed table.
- `name_hash_from_folded_utf16()` computes the exFAT hash over folded UTF-16 bytes.
- `name_hash()` folds first and then hashes for callers that only have the original name units.

## Approved Deviations

None.

## Optional Self-Checks

- Commands run, if any: None.
- Compile checks run, if any: None.
- Manual reasoning checks:
  - Confirmed the new state stays owner-local to `fs.rs`.
  - Confirmed publication is atomic behind the existing mutex boundary.
  - Confirmed no directory traversal, mount sequencing, or generic helper module was introduced.

## Remaining Risks

- The code was not compile-verified in this lane by design.
- Checker-owned regressions still need to prove the install, fold, and hash paths against synthetic fixtures.
