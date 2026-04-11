<!-- SPDX-License-Identifier: MPL-2.0 -->

# Creator Log

## Metadata

- Component ID: `EXR-FS-OPEN-22`
- Title: `ExfatFs` Mount/Open Sequencing And Root Publication
- Status: `SerialImplementing`
- Author: Codex
- Date: `2026-04-11`
- Task packet: checker-driven local repair after `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-OPEN-22/11_checker_serial.md`
- Implemented spec: `EXR-FS-OPEN-22` checker repair for root-directory prerequisite discovery and mount-ready test scaffolding
- Pass kind: `serial repair`

## Planned File Ownership

- Files to edit:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/dentry.rs`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- Files intentionally left untouched:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fileset.rs`

## Implementation Notes

The first real 22 checker rerun showed that `open_root_inode()` was not blocked by the QEMU harness anymore; it was blocked by concrete prerequisite discovery failures inside the mount-ready scenario.

The repair stayed narrow:

- Added `ExfatDentry::is_volume_label()` in `dentry.rs` so the directory stream can recognize the root volume-label metadata entry without widening into generic singleton policy.
- Updated `DirectoryEngine::next_record()` in `directory.rs` to skip that volume-label metadata entry while still surfacing only `Bitmap` and `Upcase` singleton candidates and still rejecting every other unexpected top-level dentry.
- Added `directory_engine_skips_volume_label_and_keeps_system_singletons()` so the directory-side narrowing is locked in by local regression coverage.
- Tightened the `new_mount_ready_exfat_fs()` helper path in `fs.rs` so the test fixture reuses the image's existing upcase slot and cluster instead of writing a synthetic payload into an arbitrary cluster and corrupting sibling mount prerequisites.

## Approved Deviations

- This repair touched the consumed `DirectoryEngine` dependency even though 22 primarily lands in `fs.rs`.
  The deviation was accepted because the checker showed 22 could not reach its own sequencing assertions until the root-directory metadata path handled the real volume-label entry and until the mount-ready fixture stopped clobbering prerequisite data.

## Optional Self-Checks

- Commands run, if any: None in this repair artifact. Runtime verification is recorded separately in `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-FS-OPEN-22/14_checker_serial_retry.md`.
- Compile checks run, if any: None.
- Manual reasoning checks:
  - Confirmed the directory repair does not widen singleton policy beyond `Bitmap` / `Upcase`.
  - Confirmed the `fs.rs` test-helper repair stays test-only and does not change production mount sequencing.

## Remaining Risks

- The mount-ready helper now explicitly depends on the bundled exFAT image continuing to provide an existing upcase slot.
- No executable verification is claimed from this repair artifact by itself; the checker retry must remain the acceptance gate.
