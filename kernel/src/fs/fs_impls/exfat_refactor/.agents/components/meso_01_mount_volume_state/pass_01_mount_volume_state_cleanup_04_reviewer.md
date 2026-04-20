<!-- SPDX-License-Identifier: MPL-2.0 -->

# Reviewer Pass Quality Sign-Off: `mount_volume_state_cleanup_04`

*This artifact forms the final Reviewer quality gate for one implementation pass. It checks whether the Creator's `pass_XX_creator.md` structural declarations aligned legally with `ASTERINAS_CODE_QUALITY_PRIORS.md` and the Creator Helper constraints.*

## 1. Pass Identity & Line-Level Quality Enforcement Log

**Reviewer Pass ID:** `pass_01_mount_volume_state_cleanup_04`
**Parent Meso-Component:** `meso_01_mount_volume_state`
**Covered Micro-Features:**
- `Boot region validation and parameter load at mount`
- `Allocation bitmap is the free-space truth source`
- `VolumeDirty marks in-flight versus quiesced global state`
- `VolumeFlags also carries media-failure and clear-before-modify state`
- `Up-case Table is the durable case-folding truth source`
- `Mount option defaults and remount mutability boundary`
- `Superblock counters and statfs reflect cached cluster accounting`
- `Asterinas mount lifecycle must eagerly expose root inode and global sync state`
- `Mount-time accounting may fall back to recount under corruption-recovery conditions`

- **Naming Conventions:** Rejected structurally. `diagnostics.rs` is now a flat test-only namespace containing boot, FAT, bitmap, Up-case, and byte-decoding helpers rather than a dedicated `test_support` hierarchy keyed to those concerns.
- **Imports:** Accepted as-is for this bounded review; no additional line-level import cleanup is needed to make the structural verdict.
- **Formatting:** Accepted as-is; no reviewer formatting edits were required.
- **Doc Comments:** Accepted for the touched production seams; the blocking issues are helper duplication and test-only module topology, not missing line-level documentation.

| Quality Prior Area | Evidence / Line-Level Finding | Action |
|--------------------|-------------------------------|--------|
| `Error handling` | The scoped production mount path continues to propagate `MountVolumeStateError` through owner-local methods; Reviewer did not find a new production `.unwrap()` / `.expect()` added by cleanup_03. | Accepted |
| `Visibility` | Production helper visibility stays narrow, but the new test-only `diagnostics.rs` sibling namespace in `kernel/src/fs/fs_impls/exfat_refactor/mod.rs` broadens a catch-all surface instead of using a dedicated `#[cfg(ktest)]` support subtree. | Rejected |
| `Arithmetic / overflow` | Mount-critical arithmetic remains checked or saturating in the scoped production files; the rejection is not about overflow handling. | Accepted |
| `Lock / RAII readability` | No new publication-order or lock-scope regression was found in the scoped production files; the rejection is structural only. | Accepted |

## 2. Independent Entity Census & Helper Legality Sign-Off

*You must independently inspect the code and compare it against the Creator census. Evaluate every introduced production entity against the `CREATOR.md` Entity Generation Whitelist rules (Rule A, Rule B, Rule C) and against its claimed owner/module boundary.*

| Handled Symbol | Found By Reviewer? | Listed By Creator? | Claimed Owner / Boundary | Whitelist Judgment | Action Taken |
|----------------|--------------------|--------------------|--------------------------|--------------------|--------------|
| `BootRegion::read_le_u16` / `BootRegion::read_le_u32` / `BootRegion::read_le_u64` | Yes | Yes | `BootRegion` private parser helpers in `kernel/src/fs/fs_impls/exfat_refactor/boot.rs` | Rejected for this cleanup wave. These wrappers are one-line aliases over `from_le_bytes(...)` and do not carry a Boot-region-specific invariant, error translation, or ownership boundary. | REJECT back to Creator: inline direct `u16` / `u32` / `u64::from_le_bytes(...)` at call sites, then delete the wrappers. |
| `FatReader::read_le_u32` | Yes | Yes | `FatReader` private parser helper in `kernel/src/fs/fs_impls/exfat_refactor/fat.rs` | Rejected for the same reason: thin byte-order alias only, no FAT-specific behavior beyond fixed-width decode. | REJECT back to Creator: inline direct `u32::from_le_bytes(...)`, then delete the wrapper. |
| `AllocationBitmapRecord::read_le_u32` / `AllocationBitmapRecord::read_le_u64` | Yes | Yes | `AllocationBitmapRecord` private parser helpers in `kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs` | Rejected for the same reason: the wrappers add no record-local semantic contract beyond fixed-width decode. | REJECT back to Creator: inline direct `from_le_bytes(...)`, then delete the wrappers. |
| `UpcaseRecord::read_le_u32` / `UpcaseRecord::read_le_u64` | Yes | Yes | `UpcaseRecord` private parser helpers in `kernel/src/fs/fs_impls/exfat_refactor/upcase.rs` | Rejected for the same reason: thin aliases only. | REJECT back to Creator: inline direct `from_le_bytes(...)`, then delete the wrappers. |
| `diagnose_invalid_on_disk_layout_gate` plus the flat `diagnose_*`, `read_device_bytes`, `read_le_*`, and checksum helper family in `kernel/src/fs/fs_impls/exfat_refactor/diagnostics.rs` | Yes | No | New test-only sibling module declared by `kernel/src/fs/fs_impls/exfat_refactor/mod.rs` and reexported through `kernel/src/fs/fs_impls/exfat_refactor/ondisk.rs` | Rejected. The file is a catch-all test-only ownerless namespace, and the Creator census does not enumerate the introduced test-only symbols even though the cleanup wave created that support surface. | REJECT back to Creator: move this surface under a dedicated `#[cfg(ktest)]` support subtree and enumerate the introduced test-only symbols explicitly. |
| `ondisk::load_validated_mount` | Yes | Yes | ktest-only compatibility wrapper in `kernel/src/fs/fs_impls/exfat_refactor/ondisk.rs` | Accepted as a temporary compatibility seam; it is not the blocking issue by itself. | Accepted, but it must import from the new test-support hierarchy after cleanup. |

### 2.1 Reviewer Structural Checks

- **Creator Census Completeness:** Rejected. `pass_01_mount_volume_state_cleanup_03_creator.md` lists only `ondisk::load_validated_mount` in the test-only census, but Reviewer found an introduced test-only helper family in `kernel/src/fs/fs_impls/exfat_refactor/diagnostics.rs` (`diagnose_invalid_on_disk_layout_gate`, `diagnose_boot_region`, `diagnose_validate_boot_checksum`, `diagnose_validate_boot_geometry`, `diagnose_scan_root_directory`, `diagnose_load_upcase_table`, `diagnose_count_used_clusters`, `finalize_root_records`, `read_anomaly_state`, `read_device_bytes`, `read_le_u16`, `read_le_u32`, `read_le_u64`, `boot_region_checksum`, `stream_checksum`).
- **Owner / Module Placement:** Rejected. The preferred cleanup is not a shared generic byte-decoding module. Creator must delete the duplicated `read_le_u16` / `read_le_u32` / `read_le_u64` helper family in `kernel/src/fs/fs_impls/exfat_refactor/boot.rs`, `kernel/src/fs/fs_impls/exfat_refactor/fat.rs`, `kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs`, `kernel/src/fs/fs_impls/exfat_refactor/upcase.rs`, and `kernel/src/fs/fs_impls/exfat_refactor/diagnostics.rs`, and replace each call site with direct `u16` / `u32` / `u64::from_le_bytes(...)` decoding. Creator must also replace the flat `#[cfg(ktest)] mod diagnostics;` entry in `kernel/src/fs/fs_impls/exfat_refactor/mod.rs` with a dedicated test-only hierarchy such as `#[cfg(ktest)] mod test_support;`, move the current diagnostic entrypoint to `test_support/mount_diagnostics.rs`, re-export it from `test_support/mod.rs`, and split the remaining helpers by concern (`boot_region`, `root_directory`, `bitmap`, `upcase`) instead of keeping one catch-all file.
- **Temporary Facades / Dead Variants:** Accepted. Reviewer did not find a new dead production facade or variant in scope. The rejection is about duplicated decode helpers and the scaling-poor test-only namespace.

## 3. Temporary Seam & Exit Plan Verification

*Verify that any structural seams, facades, or work-in-progress abstractions have explicit, documented `.rs` code comments defining their final removal or absorption conditions.*

- **Verification:** No new blocking production seam issue was found in this scoped review. The surviving ktest-only `ondisk::load_validated_mount` wrapper remains a compatibility shim, but after the required cleanup it should re-export from the new `test_support` hierarchy rather than from a flat `diagnostics.rs` sibling module.
- **Edits Made:** None. Reviewer did not perform line-level `.rs` edits because the blocking issues require Creator-owned structural cleanup.

## 4. Edit Scope Classification

- **Reviewer Edit Scope:** `Rejected without edits due structural issues`
- **Why This Scope Is Safe:** The packet forbids module splitting and helper relocation in Reviewer. Both blocking concerns require Creator-owned structural cleanup rather than line-level non-functional edits.

## 5. Final Verdict

- **REJECTED (STRUCTURAL QUALITY CLEANUP REQUIRED)**: The current tree still carries duplicated little-endian helper wrappers and a scaling-poor flat `diagnostics.rs` test-only namespace. Route back to a Creator cleanup pass with the concrete actions in Section 2.1, then return through Checker only if that structural cleanup changes compilation-relevant module wiring.
