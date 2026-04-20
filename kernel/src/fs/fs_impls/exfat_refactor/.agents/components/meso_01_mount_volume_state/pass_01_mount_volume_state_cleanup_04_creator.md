<!-- SPDX-License-Identifier: MPL-2.0 -->

# Creator Pass Implementation Report: `mount_volume_state_cleanup_04`

*This artifact acts as the Creator's receipt, documenting how the Designer's strict specifications were actually implemented in Rust for one explicit Creator Pass. It serves as context for the synchronized Checker's validation and the Reviewer's static checks.*

## 1. Pass Identity & Write-Set

**Creator Pass ID:** `pass_01_mount_volume_state_cleanup_04`
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
**Source Files Modified/Created:**
- `kernel/src/fs/fs_impls/exfat_refactor/mod.rs` (replaced the flat `#[cfg(ktest)] mod diagnostics;` entry with `#[cfg(ktest)] mod test_support;`)
- `kernel/src/fs/fs_impls/exfat_refactor/boot.rs` (deleted `BootRegion::read_le_u16` / `read_le_u32` / `read_le_u64` and decoded little-endian fields directly at call sites)
- `kernel/src/fs/fs_impls/exfat_refactor/fat.rs` (deleted `FatReader::read_le_u32` and decoded the FAT entry directly in `next_cluster`)
- `kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs` (deleted `AllocationBitmapRecord::read_le_u32` / `read_le_u64` and decoded record fields directly in `parse`)
- `kernel/src/fs/fs_impls/exfat_refactor/upcase.rs` (deleted `UpcaseRecord::read_le_u32` / `read_le_u64` and decoded record fields directly in `parse`)
- `kernel/src/fs/fs_impls/exfat_refactor/fs.rs` (retargeted ktest-only helpers to `test_support`, replaced local `read_le_*` test wrappers with direct `from_le_bytes(...)`, and left production behavior unchanged)
- `kernel/src/fs/fs_impls/exfat_refactor/ondisk.rs` (retargeted the ktest compatibility surface to re-export from `test_support`)
- `kernel/src/fs/fs_impls/exfat_refactor/diagnostics.rs` (deleted; split into the dedicated `test_support/` hierarchy)
- `kernel/src/fs/fs_impls/exfat_refactor/test_support/mod.rs` (created the dedicated ktest support root and the `load_validated_mount` compatibility entrypoint)
- `kernel/src/fs/fs_impls/exfat_refactor/test_support/mount_diagnostics.rs` (created the diagnostic entrypoint module)
- `kernel/src/fs/fs_impls/exfat_refactor/test_support/boot_region.rs` (moved boot-region diagnostic helpers out of the flat file)
- `kernel/src/fs/fs_impls/exfat_refactor/test_support/root_directory.rs` (moved root-directory diagnostic helpers out of the flat file)
- `kernel/src/fs/fs_impls/exfat_refactor/test_support/bitmap.rs` (moved allocation-bitmap diagnostic helpers out of the flat file)
- `kernel/src/fs/fs_impls/exfat_refactor/test_support/upcase.rs` (moved Up-case diagnostic helpers out of the flat file)

## 2. Pass Coverage & Contract Satisfaction

- This cleanup pass removes the duplicated little-endian wrappers in the scoped production readers (`boot.rs`, `fat.rs`, `bitmap.rs`, `upcase.rs`) and in the scoped ktest helper code (`fs.rs`, former `diagnostics.rs`) by decoding with direct fixed-width `from_le_bytes(...)` expressions at each call site.
- This pass replaces the flat `diagnostics.rs` test-only namespace with a dedicated `#[cfg(ktest)] mod test_support;` hierarchy split by concern (`mount_diagnostics`, `boot_region`, `root_directory`, `bitmap`, `upcase`), which matches the Reviewer's requested layout without changing production mount behavior.
- This pass keeps the accepted mount pipeline intact: production mount validation still flows through `boot::ValidatedMount::load`, while ktest-only compatibility entrypoints (`load_validated_mount`, `diagnose_invalid_on_disk_layout_gate`) now live under `test_support` and are re-exported through `ondisk.rs` for compatibility.
- Incidental support outside the covered micro-features is limited to ktest-only import retargeting in `fs.rs` and `ondisk.rs` so the moved support surface remains reachable without reintroducing a catch-all module.

## 3. Lock Orchestration & RAII Notes

- This cleanup pass does not change the production lock topology, guard lifetime, or mount publication RAII structure.
- The production edits are local decode substitutions only; they preserve the existing borrow scopes in `BootRegion::read`, `FatReader::next_cluster`, `AllocationBitmapRecord::parse`, and `UpcaseRecord::parse`.
- The ktest support split is purely module-topology work. It preserves the existing diagnostic execution order and FAT-reader ownership inside the test-only entrypoint while moving helpers into concern-specific files.

## 4. Generated Entity Census

*You MUST list every introduced production entity in the assigned write-set. This includes each new `struct`, `enum`, local type alias, module, and non-trait helper function. Trait-required methods may be grouped explicitly under an impl block instead of listed one-by-one. Test-only entities MUST appear in a separate subsection and are not exempt from reporting.*

### 4.1 Production Entity Census

| Introduced Symbol | Kind | File | Owner / Module Boundary | Real Call Sites or Reuse | Whitelist Rule / Exemption | Final-System Status |
|-------------------|------|------|-------------------------|--------------------------|----------------------------|---------------------|
| `None` | — | — | No new production entities were introduced in this cleanup pass. | The production work is limited to deleting duplicate `read_le_*` wrappers and inlining direct `from_le_bytes(...)` decoding at existing call sites. | Exempt: deletion-only production cleanup. | Existing production owners remain in place. |

### 4.2 Trait-Required Grouped Methods

| Impl Block | Grouped Methods | Why Exempt from Per-Method Census |
|------------|-----------------|-----------------------------------|
| `None` | `None` | This pass introduces no new trait-required methods or impl blocks. |

### 4.3 Test-Only Entity Census

| Introduced Symbol | Kind | File | Why Test-Only | Notes |
|-------------------|------|------|---------------|-------|
| `test_support` | Module | `kernel/src/fs/fs_impls/exfat_refactor/test_support/mod.rs` | `#[cfg(ktest)]` support root only | New dedicated hierarchy replacing the flat `diagnostics.rs` namespace. |
| `test_support::mount_diagnostics` | Module | `kernel/src/fs/fs_impls/exfat_refactor/test_support/mount_diagnostics.rs` | `#[cfg(ktest)]` diagnostic entrypoint only | Holds the top-level gate dispatcher moved out of `diagnostics.rs`. |
| `test_support::boot_region` | Module | `kernel/src/fs/fs_impls/exfat_refactor/test_support/boot_region.rs` | `#[cfg(ktest)]` boot-region diagnostics only | Groups boot-sector and checksum diagnostics by concern. |
| `test_support::root_directory` | Module | `kernel/src/fs/fs_impls/exfat_refactor/test_support/root_directory.rs` | `#[cfg(ktest)]` root-directory diagnostics only | Groups root scan diagnostics by concern. |
| `test_support::bitmap` | Module | `kernel/src/fs/fs_impls/exfat_refactor/test_support/bitmap.rs` | `#[cfg(ktest)]` allocation-bitmap diagnostics only | Groups bitmap-count diagnostics by concern. |
| `test_support::upcase` | Module | `kernel/src/fs/fs_impls/exfat_refactor/test_support/upcase.rs` | `#[cfg(ktest)]` Up-case diagnostics only | Groups Up-case table diagnostics by concern. |
| `test_support::load_validated_mount` | Private fn | `kernel/src/fs/fs_impls/exfat_refactor/test_support/mod.rs` | `#[cfg(ktest)]` compatibility entrypoint only | Moved the ktest compatibility wrapper out of `ondisk.rs`; `ondisk.rs` now re-exports it. |
| `test_support::mount_diagnostics::diagnose_invalid_on_disk_layout_gate` | Private fn | `kernel/src/fs/fs_impls/exfat_refactor/test_support/mount_diagnostics.rs` | `#[cfg(ktest)]` mount-fixture diagnostics only | Moved unchanged entrypoint logic out of the deleted flat file. |
| `test_support::boot_region::diagnose_boot_region` | Private fn | `kernel/src/fs/fs_impls/exfat_refactor/test_support/boot_region.rs` | `#[cfg(ktest)]` boot-region diagnostics only | Moved from `diagnostics.rs`; now scoped to boot-region support. |
| `test_support::boot_region::read_anomaly_state` | Private fn | `kernel/src/fs/fs_impls/exfat_refactor/test_support/boot_region.rs` | `#[cfg(ktest)]` anomaly-state I/O gate only | Moved from `diagnostics.rs`; still only checks diagnostic-read reachability. |
| `test_support::boot_region::diagnose_validate_boot_checksum` | Private fn | `kernel/src/fs/fs_impls/exfat_refactor/test_support/boot_region.rs` | `#[cfg(ktest)]` checksum diagnostics only | Moved from `diagnostics.rs`; unchanged diagnostic behavior. |
| `test_support::boot_region::diagnose_validate_boot_geometry` | Private fn | `kernel/src/fs/fs_impls/exfat_refactor/test_support/boot_region.rs` | `#[cfg(ktest)]` geometry diagnostics only | Moved from `diagnostics.rs`; unchanged diagnostic behavior. |
| `test_support::boot_region::read_device_bytes` | Private fn | `kernel/src/fs/fs_impls/exfat_refactor/test_support/boot_region.rs` | `#[cfg(ktest)]` boot-diagnostic device-read helper only | Moved from `diagnostics.rs`; kept local to boot diagnostics instead of a generic shared module. |
| `test_support::boot_region::boot_region_checksum` | Private fn | `kernel/src/fs/fs_impls/exfat_refactor/test_support/boot_region.rs` | `#[cfg(ktest)]` boot-diagnostic checksum helper only | Moved from `diagnostics.rs`; stays boot-specific. |
| `test_support::root_directory::diagnose_scan_root_directory` | Private fn | `kernel/src/fs/fs_impls/exfat_refactor/test_support/root_directory.rs` | `#[cfg(ktest)]` root-directory diagnostics only | Moved from `diagnostics.rs`; now isolated to directory-scan logic. |
| `test_support::root_directory::finalize_root_records` | Private fn | `kernel/src/fs/fs_impls/exfat_refactor/test_support/root_directory.rs` | `#[cfg(ktest)]` root-directory diagnostics only | Moved from `diagnostics.rs`; remains private to the root-directory support file. |
| `test_support::upcase::diagnose_load_upcase_table` | Private fn | `kernel/src/fs/fs_impls/exfat_refactor/test_support/upcase.rs` | `#[cfg(ktest)]` Up-case diagnostics only | Moved from `diagnostics.rs`; now isolated to Up-case loading checks. |
| `test_support::upcase::stream_checksum` | Private fn | `kernel/src/fs/fs_impls/exfat_refactor/test_support/upcase.rs` | `#[cfg(ktest)]` Up-case diagnostics only | Moved from `diagnostics.rs`; remains local to Up-case support. |
| `test_support::bitmap::diagnose_count_used_clusters` | Private fn | `kernel/src/fs/fs_impls/exfat_refactor/test_support/bitmap.rs` | `#[cfg(ktest)]` allocation-bitmap diagnostics only | Moved from `diagnostics.rs`; now isolated to bitmap-accounting checks. |

### 4.4 Fate of Every Old `read_le_*` Wrapper

| Old Wrapper | Fate in This Pass | Replacement |
|-------------|-------------------|-------------|
| `BootRegion::read_le_u16` | Removed from `kernel/src/fs/fs_impls/exfat_refactor/boot.rs` | `BootRegion::read` and `VolumeAnomalyState::read` now decode inline with `u16::from_le_bytes([...])`. |
| `BootRegion::read_le_u32` | Removed from `kernel/src/fs/fs_impls/exfat_refactor/boot.rs` | `BootRegion::read` and `BootRegion::validate_checksum` now decode inline with `u32::from_le_bytes([...])`. |
| `BootRegion::read_le_u64` | Removed from `kernel/src/fs/fs_impls/exfat_refactor/boot.rs` | `BootRegion::read` now decodes inline with `u64::from_le_bytes([...])`. |
| `FatReader::read_le_u32` | Removed from `kernel/src/fs/fs_impls/exfat_refactor/fat.rs` | `FatReader::next_cluster` now decodes the FAT entry inline with `u32::from_le_bytes([...])`. |
| `AllocationBitmapRecord::read_le_u32` | Removed from `kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs` | `AllocationBitmapRecord::parse` now decodes `first_cluster` inline with `u32::from_le_bytes([...])`. |
| `AllocationBitmapRecord::read_le_u64` | Removed from `kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs` | `AllocationBitmapRecord::parse` now decodes `data_length` inline with `u64::from_le_bytes([...])`. |
| `UpcaseRecord::read_le_u32` | Removed from `kernel/src/fs/fs_impls/exfat_refactor/upcase.rs` | `UpcaseRecord::parse` now decodes `checksum` and `first_cluster` inline with `u32::from_le_bytes([...])`. |
| `UpcaseRecord::read_le_u64` | Removed from `kernel/src/fs/fs_impls/exfat_refactor/upcase.rs` | `UpcaseRecord::parse` now decodes `data_length` inline with `u64::from_le_bytes([...])`. |
| `diagnostics::read_le_u16` | Removed with deletion of `kernel/src/fs/fs_impls/exfat_refactor/diagnostics.rs` | `test_support::boot_region::diagnose_boot_region` now decodes the signature inline with `u16::from_le_bytes([...])`. |
| `diagnostics::read_le_u32` | Removed with deletion of `kernel/src/fs/fs_impls/exfat_refactor/diagnostics.rs` | `test_support::boot_region::diagnose_boot_region` and `diagnose_validate_boot_checksum` now decode inline with `u32::from_le_bytes([...])`. |
| `diagnostics::read_le_u64` | Removed with deletion of `kernel/src/fs/fs_impls/exfat_refactor/diagnostics.rs` | `test_support::boot_region::diagnose_boot_region` now decodes inline with `u64::from_le_bytes([...])`. |
| `fs::tests::read_le_u16` | Removed from `kernel/src/fs/fs_impls/exfat_refactor/fs.rs` | No call sites remained; the unused local wrapper was deleted outright. |
| `fs::tests::read_le_u32` | Removed from `kernel/src/fs/fs_impls/exfat_refactor/fs.rs` | `next_cluster`, `upcase_data_offset`, and `allocation_bitmap_data_offset` now decode inline with `u32::from_le_bytes([...])`. |

## 5. Contract Deviations & Boundary Notes

- **Incidental Supporting Edits Outside Covered Micro-Features:** Retargeted ktest-only imports in `kernel/src/fs/fs_impls/exfat_refactor/fs.rs` and `kernel/src/fs/fs_impls/exfat_refactor/ondisk.rs` so the moved support surface remains reachable after deleting `diagnostics.rs`.
- **Deviations:** None. The pass stays within the Reviewer's requested structural cleanup: no new production features, no public interface widening, no later `meso_01` expansion, no new tests, and no build/test commands.
- **Unresolved Ambiguities:** None. The packet explicitly requested direct `from_le_bytes(...)` decoding and a dedicated `test_support` hierarchy, so the implementation followed that shape without introducing a generic helper module.
