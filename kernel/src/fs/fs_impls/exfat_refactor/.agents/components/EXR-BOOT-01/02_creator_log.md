<!-- SPDX-License-Identifier: MPL-2.0 -->

# Creator Log

## Metadata

- Component ID: EXR-BOOT-01
- Title: Boot Region Parsing And Normalized Runtime Geometry
- Status: `Implemented`
- Author: creator
- Date: 2026-03-31
- Implemented spec: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BOOT-01/01_designer_spec.md`

## Planned File Ownership

- Files to edit:
  - `kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/boot_sector.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/super_block.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BOOT-01/02_creator_log.md`
- Files intentionally left untouched:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BOOT-01/00_architect.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BOOT-01/01_designer_spec.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`

## Implementation Notes

Implemented the `EXR-BOOT-01` read-only boot bootstrap in three layers.

- `boot_sector.rs` now defines `ExfatBootSector`, reads sector `0`, validates the primary boot sector, verifies the primary boot-region checksum, and exposes the top-level `read_primary_super_block()` bootstrap entry.
- `super_block.rs` now defines `ExfatSuperBlock` and performs the normalized runtime-geometry conversion from a validated `ExfatBootSector`.
- `mod.rs` now wires the staged modules and adds targeted `#[ktest]` coverage for the success path plus malformed signature, malformed fs name, nonzero reserved bytes, corrupted checksum region, corrupted checksum sector, and invalid region layout.

The implementation kept the component strictly read-only, preserved `ExfatBootSector` and `ExfatSuperBlock`, and did not widen scope into `ExfatFs`, inode, bitmap, upcase, or filesystem registration work.

## Approved Deviations

None.

## Self-Checks

- Commands run:
  - `cargo fmt --all -- kernel/src/fs/fs_impls/exfat_refactor/mod.rs kernel/src/fs/fs_impls/exfat_refactor/boot_sector.rs kernel/src/fs/fs_impls/exfat_refactor/super_block.rs`
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas && make kernel'`
  - `docker exec codex-asterinas-dev bash -lc 'test -e /dev/kvm && ls -l /dev/kvm || echo no-kvm'`
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test boot_region_loads_super_block'`
- Tests run:
  - `make kernel`: succeeded.
  - Filtered ktest `boot_region_loads_super_block`: compiled, launched QEMU, and exited with status `1` after UEFI boot output. The observed environment was `no-kvm`, so the run used TCG semantics. No in-kernel test assertion output was captured before exit.
- Manual reasoning checks:
  - The validation rules match the accepted designer spec for primary boot signature, fs name, reserved bytes, FAT count, sector-size bounds, cluster-size bound, FAT capacity, root-cluster range, data-region placement, and volume length.
  - The checksum implementation reads the primary Main Boot Region only, skips bytes `106`, `107`, and `112`, and compares every checksum entry in sector `11`.
  - The normalization logic preserves the legacy-comparable `ExfatBootSector` and `ExfatSuperBlock` naming and keeps the component free of mount-policy or writeback behavior.

## Remaining Risks

- The filtered ktest currently fails at runtime with only UEFI boot output visible, so checker work is needed to determine whether the failure is in the new test harness, the test runner path, or the boot/bootstrap logic itself.
- The new boot and superblock helpers are intentionally staged before mount integration, so non-ktest builds still rely on `#[expect(dead_code)]` until later components wire these APIs into the refactored mount path.
