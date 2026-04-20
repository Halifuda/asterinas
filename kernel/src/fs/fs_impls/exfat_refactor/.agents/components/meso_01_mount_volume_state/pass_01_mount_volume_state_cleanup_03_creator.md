<!-- SPDX-License-Identifier: MPL-2.0 -->

# Creator Pass Implementation Report: `mount_volume_state_cleanup_03`

*This artifact acts as the Creator's receipt, documenting how the Designer's strict specifications were actually implemented in Rust for one explicit Creator Pass. It serves as context for the synchronized Checker's validation and the Reviewer's static checks.*

## 1. Pass Identity & Write-Set

**Creator Pass ID:** `pass_01_mount_volume_state_cleanup_03`
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
- `kernel/src/fs/fs_impls/exfat_refactor/boot.rs` (absorbed the surviving mount-bootstrap free helpers into `BootRegion`, `VolumeAnomalyState`, and `ValidatedMount`, and folded the checker-requested root-directory visitor fix plus `VmIo` import)
- `kernel/src/fs/fs_impls/exfat_refactor/fat.rs` (moved cluster-chain walking and device-read helpers under `FatReader`, and folded the checker-requested `VmIo` import)
- `kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs` (absorbed bitmap parse/accounting helpers into `AllocationBitmapRecord`)
- `kernel/src/fs/fs_impls/exfat_refactor/upcase.rs` (absorbed Up-case load/parse/checksum helpers into `UpcaseTable` and `UpcaseRecord`)
- `kernel/src/fs/fs_impls/exfat_refactor/fs.rs` (switched production mount bootstrap to `boot::ValidatedMount::load(...)`)
- `kernel/src/fs/fs_impls/exfat_refactor/diagnostics.rs` (updated ktest-only diagnostics parsing call sites to the new owner-local parse methods)
- `kernel/src/fs/fs_impls/exfat_refactor/ondisk.rs` (kept archived ktest compatibility through a test-only wrapper after the production boot helper was absorbed)
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/meso_01_mount_volume_state/pass_01_mount_volume_state_cleanup_03_creator.md` (recorded this cleanup pass)

## 2. Pass Coverage & Contract Satisfaction

- This pass re-judges the surviving helper families in `boot.rs`, `fat.rs`, `bitmap.rs`, and `upcase.rs` under the user-requested cleanup wave and removes their production naked-helper form. The mount bootstrap now routes through owner-local associated functions on `BootRegion`, `VolumeAnomalyState`, `ValidatedMount`, `FatReader`, `AllocationBitmapRecord`, `UpcaseTable`, and `UpcaseRecord`.
- The accepted pass-01 behavior stays intact. `mount_candidate(...)` still performs the same boot validation, anomaly load, root-directory scan, Up-case load, bitmap accounting, filesystem publication, and superblock projection; only the helper ownership boundaries changed.
- The checker-reported compile repair in `boot.rs` is folded into this pass: the root-directory chain visitor now returns only `ChainVisitControl::{Continue, Stop}`, and tuple finalization happens once after the walk completes.
- The checker-reported `VmIo` scope repair is also folded into this pass in `boot.rs` and `fat.rs`, so the owner-local byte readers retain the `read_bytes(...)` extension method without reopening broader mount logic.
- `ondisk.rs` no longer depends on a production free helper in `boot.rs`; instead it provides a ktest-only compatibility wrapper around the absorbed owner-local `ValidatedMount::load(...)` entry point.

## 3. Lock Orchestration & RAII Notes

- No lock order, publication sequencing, or RAII scope changed in this cleanup pass.
- Mount-time device I/O still completes before `ExfatFs` publishes allocator state and root inode visibility, so the existing pre-publication versus published-state boundary remains unchanged.
- The helper cleanup only narrows ownership inside existing mount/bootstrap owners; it does not add a new blocking boundary or extend any guard lifetime.

## 4. Generated Entity Census

### 4.1 Production Entity Census

| Introduced Symbol | Kind | File | Owner / Module Boundary | Real Call Sites or Reuse | Whitelist Rule / Exemption | Final-System Status |
|-------------------|------|------|-------------------------|--------------------------|----------------------------|---------------------|
| `BootRegion::read` | Associated fn | `kernel/src/fs/fs_impls/exfat_refactor/boot.rs` | `BootRegion` boot-region owner | Called by `ValidatedMount::load` | Exemption: absorbs the pre-existing `read_boot_region` logic into the final boot-region owner instead of leaving a naked module helper. | Intended final owner-local entry |
| `BootRegion::validate_stream_data` | Method | `kernel/src/fs/fs_impls/exfat_refactor/boot.rs` | `BootRegion` stream-bound validator | Called by `AllocationBitmapRecord::count_used_clusters` and `UpcaseTable::load` | Exemption: absorbs the pre-existing `validate_stream_record` logic into the final boot-region owner boundary. | Intended final owner-local validator |
| `BootRegion::validate_checksum` | Method | `kernel/src/fs/fs_impls/exfat_refactor/boot.rs` | `BootRegion` boot checksum validator | Called by `BootRegion::read` | Exemption: absorbs the pre-existing `validate_boot_checksum` helper into the boot-region owner. | Intended final owner-local validator |
| `BootRegion::validate_geometry` | Method | `kernel/src/fs/fs_impls/exfat_refactor/boot.rs` | `BootRegion` geometry validator | Called by `BootRegion::read` | Exemption: absorbs the pre-existing `validate_boot_geometry` helper into the boot-region owner. | Intended final owner-local validator |
| `BootRegion::read_device_bytes` | Associated fn | `kernel/src/fs/fs_impls/exfat_refactor/boot.rs` | `BootRegion` private boot-byte reader | Called by `BootRegion::read`, `BootRegion::validate_checksum`, and `VolumeAnomalyState::read` | Exemption: relocation of an existing private helper into the owner impl to eliminate the module-level helper family. | Intended final private owner helper |
| `BootRegion::read_le_u16` | Associated fn | `kernel/src/fs/fs_impls/exfat_refactor/boot.rs` | `BootRegion` private boot parser | Called by `BootRegion::read` and `VolumeAnomalyState::read` | Exemption: relocation of existing private parsing logic into the boot owner impl. | Intended final private owner helper |
| `BootRegion::read_le_u32` | Associated fn | `kernel/src/fs/fs_impls/exfat_refactor/boot.rs` | `BootRegion` private boot parser | Called by `BootRegion::read` and `BootRegion::validate_checksum` | Exemption: relocation of existing private parsing logic into the boot owner impl. | Intended final private owner helper |
| `BootRegion::read_le_u64` | Associated fn | `kernel/src/fs/fs_impls/exfat_refactor/boot.rs` | `BootRegion` private boot parser | Called by `BootRegion::read` | Exemption: relocation of existing private parsing logic into the boot owner impl. | Intended final private owner helper |
| `BootRegion::checksum` | Associated fn | `kernel/src/fs/fs_impls/exfat_refactor/boot.rs` | `BootRegion` private checksum logic | Called by `BootRegion::validate_checksum` | Exemption: relocation of the pre-existing checksum helper into the boot owner impl. | Intended final private owner helper |
| `VolumeAnomalyState::read` | Associated fn | `kernel/src/fs/fs_impls/exfat_refactor/boot.rs` | `VolumeAnomalyState` anomaly-state owner | Called by `ValidatedMount::load` | Exemption: absorbs the pre-existing `read_anomaly_state` helper into the final anomaly-state owner. | Intended final owner-local entry |
| `ValidatedMount::load` | Associated fn | `kernel/src/fs/fs_impls/exfat_refactor/boot.rs` | `ValidatedMount` mount-bootstrap owner | Called by `fs.rs` production mount flow and ktest-only `ondisk.rs` wrapper | Exemption: absorbs the pre-existing `load_validated_mount` helper into the final mount-bootstrap owner. | Intended final owner-local entry |
| `ValidatedMount::scan_root_directory` | Associated fn | `kernel/src/fs/fs_impls/exfat_refactor/boot.rs` | `ValidatedMount` private root-directory bootstrap logic | Called by `ValidatedMount::load` | Exemption: relocation of the pre-existing root scan helper into the mount-bootstrap owner. | Intended final private owner helper |
| `ValidatedMount::finalize_root_records` | Associated fn | `kernel/src/fs/fs_impls/exfat_refactor/boot.rs` | `ValidatedMount` private root-directory bootstrap logic | Called by `ValidatedMount::scan_root_directory` | Exemption: relocation of the pre-existing record finalizer into the mount-bootstrap owner. | Intended final private owner helper |
| `FatReader::walk_cluster_chain` | Method | `kernel/src/fs/fs_impls/exfat_refactor/fat.rs` | `FatReader` chain-traversal owner | Called by `ValidatedMount::scan_root_directory`, `AllocationBitmapRecord::count_used_clusters`, and `UpcaseTable::load` | Exemption: absorbs the pre-existing `walk_cluster_chain` helper into the stateful FAT owner that already carries the device/cache context. | Intended final owner-local traversal method |
| `FatReader::read_device_bytes` | Associated fn | `kernel/src/fs/fs_impls/exfat_refactor/fat.rs` | `FatReader` private I/O helper | Called by `FatReader::walk_cluster_chain` and `FatReader::next_cluster` | Exemption: relocation of an existing private helper into the FAT owner impl. | Intended final private owner helper |
| `FatReader::read_le_u32` | Associated fn | `kernel/src/fs/fs_impls/exfat_refactor/fat.rs` | `FatReader` private parser | Called by `FatReader::next_cluster` | Exemption: relocation of an existing private parser into the FAT owner impl. | Intended final private owner helper |
| `AllocationBitmapRecord::count_used_clusters` | Method | `kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs` | `AllocationBitmapRecord` allocation-bitmap owner | Called by `ValidatedMount::load` | Exemption: absorbs the pre-existing `count_used_clusters` helper into the allocation-bitmap record owner. | Intended final owner-local accounting method |
| `AllocationBitmapRecord::parse` | Associated fn | `kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs` | `AllocationBitmapRecord` allocation-bitmap parser | Called by `ValidatedMount::scan_root_directory` and `diagnostics.rs` | Exemption: absorbs the pre-existing `parse_bitmap_record` helper into the allocation-bitmap record owner. | Intended final owner-local parser |
| `AllocationBitmapRecord::read_le_u32` | Associated fn | `kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs` | `AllocationBitmapRecord` private parser | Called by `AllocationBitmapRecord::parse` | Exemption: relocation of existing private parsing logic into the allocation-bitmap owner impl. | Intended final private owner helper |
| `AllocationBitmapRecord::read_le_u64` | Associated fn | `kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs` | `AllocationBitmapRecord` private parser | Called by `AllocationBitmapRecord::parse` | Exemption: relocation of existing private parsing logic into the allocation-bitmap owner impl. | Intended final private owner helper |
| `UpcaseTable::load` | Associated fn | `kernel/src/fs/fs_impls/exfat_refactor/upcase.rs` | `UpcaseTable` Up-case owner | Called by `ValidatedMount::load` | Exemption: absorbs the pre-existing `load_upcase_table` helper into the final Up-case owner. | Intended final owner-local entry |
| `UpcaseTable::stream_checksum` | Associated fn | `kernel/src/fs/fs_impls/exfat_refactor/upcase.rs` | `UpcaseTable` private checksum logic | Called by `UpcaseTable::load` | Exemption: relocation of the pre-existing checksum helper into the Up-case owner impl. | Intended final private owner helper |
| `UpcaseRecord::parse` | Associated fn | `kernel/src/fs/fs_impls/exfat_refactor/upcase.rs` | `UpcaseRecord` Up-case parser | Called by `ValidatedMount::scan_root_directory` and `diagnostics.rs` | Exemption: absorbs the pre-existing `parse_upcase_record` helper into the Up-case record owner. | Intended final owner-local parser |
| `UpcaseRecord::read_le_u32` | Associated fn | `kernel/src/fs/fs_impls/exfat_refactor/upcase.rs` | `UpcaseRecord` private parser | Called by `UpcaseRecord::parse` | Exemption: relocation of existing private parsing logic into the Up-case record owner impl. | Intended final private owner helper |
| `UpcaseRecord::read_le_u64` | Associated fn | `kernel/src/fs/fs_impls/exfat_refactor/upcase.rs` | `UpcaseRecord` private parser | Called by `UpcaseRecord::parse` | Exemption: relocation of existing private parsing logic into the Up-case record owner impl. | Intended final private owner helper |

### 4.2 Trait-Required Grouped Methods

| Impl Block | Grouped Methods | Why Exempt from Per-Method Census |
|------------|-----------------|-----------------------------------|
| `*(None)*` | `*(None)*` | No new trait impl blocks or trait-required methods were introduced in this cleanup pass. |

### 4.3 Test-Only Entity Census

| Introduced Symbol | Kind | File | Why Test-Only | Notes |
|-------------------|------|------|---------------|-------|
| `ondisk::load_validated_mount` | Private fn | `kernel/src/fs/fs_impls/exfat_refactor/ondisk.rs` | `ondisk.rs` remains declared only under `#[cfg(ktest)]`, so this wrapper is unreachable from the production build. | Preserves archived ktest call sites while keeping the production mount bootstrap surface on `boot::ValidatedMount::load(...)`. |

## 5. Contract Deviations & Boundary Notes

- **Incidental Supporting Edits Outside Covered Micro-Features:** `fs.rs` now calls `boot::ValidatedMount::load(...)`; `diagnostics.rs` uses the owner-local record parsers; `ondisk.rs` keeps the archived ktest compatibility function outside the production path.
- **Deviations:** None in behavior or mount-state contract. The only observable change is helper ownership: former free helpers now live on their clear final owners, and the checker-requested compile repairs were folded into those same owner-local paths.
- **Unresolved Ambiguities:** None. The packet explicitly bounded this pass to structural cleanup plus local compile support, so I kept the work inside the touched helper families and did not widen into unrelated carrier redesign.

### 5.1 Surviving Production Free-Helper Audit

- **`boot.rs`:** No surviving production free helpers remain. `load_validated_mount`, `validate_stream_record`, `read_anomaly_state`, `read_boot_region`, `scan_root_directory`, `finalize_root_records`, `validate_boot_checksum`, `validate_boot_geometry`, and the local byte-order/checksum readers were absorbed into `ValidatedMount`, `VolumeAnomalyState`, and `BootRegion`.
- **`fat.rs`:** No surviving production free helpers remain. `walk_cluster_chain` and its private read helpers were absorbed into `FatReader`, which already owns the cached FAT sector and device context.
- **`bitmap.rs`:** No surviving production free helpers remain. `count_used_clusters` and `parse_bitmap_record` were absorbed into `AllocationBitmapRecord`, and the endian readers moved with that owner.
- **`upcase.rs`:** No surviving production free helpers remain. `load_upcase_table`, `parse_upcase_record`, and `stream_checksum` were absorbed into `UpcaseTable` / `UpcaseRecord`, with the local endian readers moved under `UpcaseRecord`.
- **Non-production compatibility note:** `kernel/src/fs/fs_impls/exfat_refactor/ondisk.rs` keeps a `load_validated_mount(...)` wrapper only on the `#[cfg(ktest)]` path. It is not part of the production helper audit surface because `mod ondisk` stays test-only in `mod.rs`.
