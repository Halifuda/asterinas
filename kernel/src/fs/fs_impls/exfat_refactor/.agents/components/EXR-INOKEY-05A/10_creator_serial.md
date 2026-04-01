<!-- SPDX-License-Identifier: MPL-2.0 -->

# Creator Log

## Metadata

- Component ID: EXR-INOKEY-05A
- Title: Inode Identity Key And Opened-Inode Lookup
- Status: `SerialImplemented`
- Author: creator
- Date: 2026-04-01
- Implemented spec: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INOKEY-05A/01_designer_core.md`

## Planned File Ownership

- Files edited:
  - `kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-INOKEY-05A/10_creator_serial.md`
- Files intentionally left untouched:
  - `kernel/src/fs/fs_impls/exfat_refactor/fat.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/fileset.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/boot_sector.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/dentry.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/io.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/super_block.rs`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/`

## Implementation Notes

Implemented the serial creator pass for inode identity.

- Added `ExfatInodeKey` in `inode.rs` as the canonical location-derived identity value.
- Added `ExfatInodeKey::from_cluster_and_offset()` as the only ordinary constructor surface.
- Added `ExfatInodeKey::root()` as the explicit reserved root-key constructor.
- Preserved the legacy packed `(cluster << 32) | offset` layout.
- Rejected offsets that do not fit in the packed low 32-bit field instead of truncating them.
- Wired the new module files through `mod.rs`.
- A later helper-sweep removed the standalone `fs.rs` lookup wrapper because no production caller existed yet; exact opened-inode lookup is now deferred to `EXR-MOUNT-09`.

## Verification

- I did not run tests.
- I did not run `cargo osdk test`, `make ktest`, or any QEMU-producing command.
- I attempted the allowed compile-only check with `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo check -p aster-kernel --lib'`.
- That check failed in pre-existing `ostd` dependency resolution, with unresolved external crates such as `acpi`, `x86_64`, `x86`, `tdx_guest`, `multiboot2`, and `unwinding`, before it could provide signal on this component's new files.

## Residual Risks

- Checker-owned tests still need to cover ordinary key packing, root-key construction, and offset overflow rejection.
- The component intentionally stops before inode lifecycle, mount ownership, registry mutation, page-cache behavior, directory traversal, and VFS integration.
