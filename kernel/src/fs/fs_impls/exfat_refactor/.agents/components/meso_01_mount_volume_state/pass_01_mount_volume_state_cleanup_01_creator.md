<!-- SPDX-License-Identifier: MPL-2.0 -->

# Creator Pass Implementation Report: `mount_volume_state_cleanup_01`

*This artifact acts as the Creator's receipt, documenting how the Designer's strict specifications were actually implemented in Rust for one explicit Creator Pass. It serves as context for the synchronized Checker's validation and the Reviewer's static checks.*

## 1. Pass Identity & Write-Set

**Creator Pass ID:** `pass_01_mount_volume_state_cleanup_01`
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
- `kernel/src/fs/fs_impls/exfat_refactor/mod.rs` (declared owner-local top-level modules for the cleanup split)
- `kernel/src/fs/fs_impls/exfat_refactor/fs.rs` (removed dispatcher-only `RootInode` / `SuperBlock` / `Flags` operation and outcome branches)
- `kernel/src/fs/fs_impls/exfat_refactor/ondisk.rs` (reduced the former catch-all to a thin compatibility re-export surface)
- `kernel/src/fs/fs_impls/exfat_refactor/boot.rs` (created boot-region/bootstrap ownership for validated mount loading, anomaly capture, and root-directory scanning)
- `kernel/src/fs/fs_impls/exfat_refactor/fat.rs` (created FAT / cluster-chain traversal ownership)
- `kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs` (created Allocation Bitmap ownership for record parsing and cached cluster counting)
- `kernel/src/fs/fs_impls/exfat_refactor/upcase.rs` (created Up-case ownership for record parsing and table loading)
- `kernel/src/fs/fs_impls/exfat_refactor/diagnostics.rs` (created the test-only diagnostics owner module)
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/meso_01_mount_volume_state/pass_01_mount_volume_state_cleanup_01_creator.md` (this Creator artifact)

## 2. Pass Coverage & Contract Satisfaction

- This cleanup pass preserves the accepted `pass_01_mount_volume_state` behavior while applying only the packet’s three structural buckets.
- Bucket 1 is satisfied by deleting `DirectoryBootstrap` outright and by removing the production dispatcher-only `MountVolumeStateOperation::{RootInode, SuperBlock, Flags}` plus matching `MountVolumeStateOutcome` variants.
- Bucket 2 is satisfied by splitting the former `ondisk.rs` catch-all into owner-local top-level modules: boot/bootstrap logic in `boot.rs`, FAT traversal in `fat.rs`, Allocation Bitmap logic in `bitmap.rs`, Up-case logic in `upcase.rs`, and test-only diagnostics in `diagnostics.rs`.
- Bucket 3 is satisfied by relocating retained transit carriers and helpers to narrower owners: mount bootstrap outputs and anomaly state to `boot.rs`, FAT traversal to `fat.rs`, bitmap parsing/counting to `bitmap.rs`, Up-case parsing/loading to `upcase.rs`, and diagnostics to `diagnostics.rs`.
- `ondisk.rs` now remains only as a thin compatibility surface for existing in-tree references; it no longer owns production parsing, traversal, bootstrap, byte-reading, or diagnostic logic.
- Incidental support was limited to updating the existing in-file ktest caller so repeated root inode and superblock reads use `ExfatFs` directly after the dispatcher-only branches were removed.

## 3. Lock Orchestration & RAII Notes

- This cleanup is structural only; it preserves the accepted lock and publication behavior of the original pass.
- Mount bootstrap still performs boot-region reads, FAT traversal, Allocation Bitmap counting, and Up-case loading before any published `ExfatFs` lock state becomes reachable.
- The module split does not add locks, widen lock scope, or move blocking I/O under published filesystem locks.
- Removing the dispatcher-only root/superblock/flags branches preserves RAII behavior because the direct `ExfatFs` accessors already own the published-state lock scopes.

## 4. Generated Entity Census

### 4.1 Production Entity Census

| Introduced Symbol | Kind | File | Owner / Module Boundary | Real Call Sites or Reuse | Whitelist Rule / Exemption | Final-System Status |
|-------------------|------|------|-------------------------|--------------------------|----------------------------|---------------------|
| `bitmap` | Module | `kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs` | Allocation Bitmap owner-local logic | Used by `boot` and `ondisk` re-exports | Packet bucket 2 mandate: owner-local top-level split of pre-existing accepted logic | Intended final owner-local module |
| `boot` | Module | `kernel/src/fs/fs_impls/exfat_refactor/boot.rs` | Boot-region and mount-bootstrap owner-local logic | Used by `fs`, `bitmap`, `fat`, `upcase`, and `ondisk` re-exports | Packet bucket 2 mandate: owner-local top-level split of pre-existing accepted logic | Intended final owner-local module |
| `fat` | Module | `kernel/src/fs/fs_impls/exfat_refactor/fat.rs` | FAT / cluster-chain traversal owner-local logic | Used by `boot`, `bitmap`, `upcase`, and diagnostics | Packet bucket 2 mandate: owner-local top-level split of pre-existing accepted logic | Intended final owner-local module |
| `upcase` | Module | `kernel/src/fs/fs_impls/exfat_refactor/upcase.rs` | Up-case owner-local logic | Used by `boot` and `ondisk` re-exports | Packet bucket 2 mandate: owner-local top-level split of pre-existing accepted logic | Intended final owner-local module |
| `AllocationBitmapRecord` | Struct | `kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs` | Allocation Bitmap record carrier | Used by `count_used_clusters`, `boot`, `fs`, and diagnostics | Exemption: relocated pre-existing accepted carrier to its owner-local module per bucket 1/2 | Intended final owner-local carrier |
| `count_used_clusters` | Helper fn | `kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs` | Allocation Bitmap accounting | Called by `boot::load_validated_mount` | Exemption: relocated pre-existing accepted helper to owner-local module per bucket 2 | Intended final owner-local helper |
| `parse_bitmap_record` | Helper fn | `kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs` | Allocation Bitmap directory-entry parsing | Called by `boot::scan_root_directory` and diagnostics | Exemption: relocated pre-existing accepted helper to owner-local module per bucket 1/2 | Intended final owner-local helper |
| `read_le_u32` | Helper fn | `kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs` | Allocation Bitmap byte decoding | Called by `parse_bitmap_record` | Exemption: relocated byte-reading helper to narrow owner-local module per bucket 3 | Intended final private helper |
| `read_le_u64` | Helper fn | `kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs` | Allocation Bitmap byte decoding | Called by `parse_bitmap_record` | Exemption: relocated byte-reading helper to narrow owner-local module per bucket 3 | Intended final private helper |
| `BootRegion` | Struct | `kernel/src/fs/fs_impls/exfat_refactor/boot.rs` | Boot-region validated geometry | Used by mount bootstrap, FAT traversal, bitmap/upcase stream validation, superblock projection, and tests | Exemption: relocated pre-existing accepted carrier to owner-local module per bucket 2 | Intended final owner-local carrier |
| `impl BootRegion` | Inherent methods | `kernel/src/fs/fs_impls/exfat_refactor/boot.rs` | Boot-region geometry helpers | `cluster_offset`, `cluster_count_usize`, `data_capacity_bytes`, `is_valid_cluster` reused by FAT, bitmap, upcase, fs, and diagnostics | Exemption: relocated pre-existing accepted methods with `BootRegion` owner per bucket 2/3 | Intended final owner-local methods |
| `VolumeAnomalyState` | Struct | `kernel/src/fs/fs_impls/exfat_refactor/boot.rs` | Boot/VolumeFlags anomaly carrier | Used by `load_validated_mount` and published mount state | Exemption: relocated pre-existing accepted carrier to mount-bootstrap owner per bucket 3 | Intended final owner-local carrier |
| `ValidatedMount` | Struct | `kernel/src/fs/fs_impls/exfat_refactor/boot.rs` | Mount bootstrap output carrier | Returned by `load_validated_mount` and consumed by `mount_candidate` | Exemption: relocated pre-existing accepted transit carrier to mount-bootstrap owner per bucket 3 | Temporary bootstrap carrier until later mount-state integration can narrow it further |
| `load_validated_mount` | Helper fn | `kernel/src/fs/fs_impls/exfat_refactor/boot.rs` | Mount bootstrap orchestration | Called by `fs::mount_candidate` and in-file test helpers | Exemption: relocated pre-existing accepted entry helper to mount-bootstrap owner per bucket 2/3 | Intended final owner-local helper |
| `validate_stream_record` | Helper fn | `kernel/src/fs/fs_impls/exfat_refactor/boot.rs` | Boot-region stream bounds validation | Called by `bitmap::count_used_clusters` and `upcase::load_upcase_table` | Exemption: relocated pre-existing accepted helper to boot-region owner per bucket 3 | Intended final owner-local helper |
| `read_anomaly_state` | Helper fn | `kernel/src/fs/fs_impls/exfat_refactor/boot.rs` | VolumeFlags anomaly read | Called by `load_validated_mount` | Exemption: relocated pre-existing accepted helper to mount-bootstrap owner per bucket 3 | Intended final private helper |
| `read_boot_region` | Helper fn | `kernel/src/fs/fs_impls/exfat_refactor/boot.rs` | Boot-region load and validation | Called by `load_validated_mount` | Exemption: relocated pre-existing accepted helper to boot-region owner per bucket 2 | Intended final private helper |
| `scan_root_directory` | Helper fn | `kernel/src/fs/fs_impls/exfat_refactor/boot.rs` | Root-directory bootstrap scan | Called by `load_validated_mount` | Exemption: relocated pre-existing accepted helper to mount-bootstrap owner; now returns owner-local records instead of `DirectoryBootstrap` | Intended final private helper |
| `finalize_root_records` | Helper fn | `kernel/src/fs/fs_impls/exfat_refactor/boot.rs` | Root-directory bootstrap result validation | Called multiple times inside `scan_root_directory` | **Rule B**: exact missing-record finalization is used at each early-stop and end-of-chain path within this meso | Intended final private helper |
| `validate_boot_checksum` | Helper fn | `kernel/src/fs/fs_impls/exfat_refactor/boot.rs` | Boot-region checksum validation | Called by `read_boot_region` | Exemption: relocated pre-existing accepted helper to boot-region owner per bucket 2 | Intended final private helper |
| `validate_boot_geometry` | Helper fn | `kernel/src/fs/fs_impls/exfat_refactor/boot.rs` | Boot-region geometry validation | Called by `read_boot_region` | Exemption: relocated pre-existing accepted helper to boot-region owner per bucket 2 | Intended final private helper |
| `read_device_bytes` | Helper fn | `kernel/src/fs/fs_impls/exfat_refactor/boot.rs` | Boot-region byte I/O | Called by boot-region and anomaly helpers | Exemption: relocated byte-reading helper to narrow boot owner per bucket 3 | Intended final private helper |
| `read_le_u16` | Helper fn | `kernel/src/fs/fs_impls/exfat_refactor/boot.rs` | Boot-region byte decoding | Called by boot-region/anomaly helpers | Exemption: relocated byte-reading helper to narrow boot owner per bucket 3 | Intended final private helper |
| `read_le_u32` | Helper fn | `kernel/src/fs/fs_impls/exfat_refactor/boot.rs` | Boot-region byte decoding | Called by boot-region helpers | Exemption: relocated byte-reading helper to narrow boot owner per bucket 3 | Intended final private helper |
| `read_le_u64` | Helper fn | `kernel/src/fs/fs_impls/exfat_refactor/boot.rs` | Boot-region byte decoding | Called by boot-region helpers | Exemption: relocated byte-reading helper to narrow boot owner per bucket 3 | Intended final private helper |
| `boot_region_checksum` | Helper fn | `kernel/src/fs/fs_impls/exfat_refactor/boot.rs` | Boot-region checksum math | Called by `validate_boot_checksum` | Exemption: relocated pre-existing accepted helper to boot-region owner per bucket 2 | Intended final private helper |
| `ChainVisitControl` | Enum | `kernel/src/fs/fs_impls/exfat_refactor/fat.rs` | FAT / cluster-chain visitor control | Returned by bitmap, boot, and upcase traversal callbacks | Exemption: relocated pre-existing accepted callback control enum to FAT owner per bucket 2 | Intended final owner-local enum |
| `FatChainStep` | Enum | `kernel/src/fs/fs_impls/exfat_refactor/fat.rs` | FAT chain step result | Returned by `FatReader::next_cluster` and used by `walk_cluster_chain` / diagnostics | Exemption: relocated pre-existing accepted enum to FAT owner per bucket 2 | Intended final owner-local enum |
| `FatReader` | Struct | `kernel/src/fs/fs_impls/exfat_refactor/fat.rs` | FAT sector cache and chain traversal | Used by boot, bitmap, upcase, and diagnostics | Exemption: relocated pre-existing accepted helper struct to FAT owner per bucket 2 | Intended final owner-local helper |
| `impl FatReader` | Inherent methods | `kernel/src/fs/fs_impls/exfat_refactor/fat.rs` | FAT sector cache and next-cluster lookup | `new` and `next_cluster` used by boot traversal and diagnostics | Exemption: relocated pre-existing accepted methods with `FatReader` owner per bucket 2 | Intended final owner-local methods |
| `walk_cluster_chain` | Helper fn | `kernel/src/fs/fs_impls/exfat_refactor/fat.rs` | FAT / cluster-chain traversal | Called by boot root scan, bitmap counting, and Up-case loading | Exemption: relocated pre-existing accepted traversal helper to FAT owner per bucket 2 | Intended final owner-local helper |
| `read_device_bytes` | Helper fn | `kernel/src/fs/fs_impls/exfat_refactor/fat.rs` | FAT byte I/O | Called by `FatReader::next_cluster` and `walk_cluster_chain` | Exemption: relocated byte-reading helper to narrow FAT owner per bucket 3 | Intended final private helper |
| `read_le_u32` | Helper fn | `kernel/src/fs/fs_impls/exfat_refactor/fat.rs` | FAT entry decoding | Called by `FatReader::next_cluster` | Exemption: relocated byte-reading helper to narrow FAT owner per bucket 3 | Intended final private helper |
| `UpcaseTable` | Struct | `kernel/src/fs/fs_impls/exfat_refactor/upcase.rs` | Up-case durable table carrier | Used by `fs` publication and `ondisk` compatibility re-export | Exemption: relocated pre-existing accepted carrier to Up-case owner per bucket 1/2 | Intended final owner-local carrier |
| `UpcaseRecord` | Struct | `kernel/src/fs/fs_impls/exfat_refactor/upcase.rs` | Up-case directory-entry metadata | Returned by `parse_upcase_record` and consumed by `load_upcase_table` | Exemption: relocated pre-existing accepted carrier to Up-case owner per bucket 1/2 | Intended final owner-local carrier |
| `load_upcase_table` | Helper fn | `kernel/src/fs/fs_impls/exfat_refactor/upcase.rs` | Up-case stream loading and checksum validation | Called by `boot::load_validated_mount` | Exemption: relocated pre-existing accepted helper to Up-case owner per bucket 2 | Intended final owner-local helper |
| `parse_upcase_record` | Helper fn | `kernel/src/fs/fs_impls/exfat_refactor/upcase.rs` | Up-case directory-entry parsing | Called by `boot::scan_root_directory` and diagnostics | Exemption: relocated pre-existing accepted helper to Up-case owner per bucket 1/2 | Intended final owner-local helper |
| `read_le_u32` | Helper fn | `kernel/src/fs/fs_impls/exfat_refactor/upcase.rs` | Up-case byte decoding | Called by `parse_upcase_record` | Exemption: relocated byte-reading helper to narrow Up-case owner per bucket 3 | Intended final private helper |
| `read_le_u64` | Helper fn | `kernel/src/fs/fs_impls/exfat_refactor/upcase.rs` | Up-case byte decoding | Called by `parse_upcase_record` | Exemption: relocated byte-reading helper to narrow Up-case owner per bucket 3 | Intended final private helper |
| `stream_checksum` | Helper fn | `kernel/src/fs/fs_impls/exfat_refactor/upcase.rs` | Up-case stream checksum math | Called by `load_upcase_table` | Exemption: relocated pre-existing accepted helper to Up-case owner per bucket 2 | Intended final private helper |

### 4.2 Trait-Required Grouped Methods

| Impl Block | Grouped Methods | Why Exempt from Per-Method Census |
|------------|-----------------|-----------------------------------|
| *(None)* | *(None)* | No new trait impl blocks or trait-required methods were introduced in this cleanup pass. |

### 4.3 Test-Only Entity Census

| Introduced Symbol | Kind | File | Why Test-Only | Notes |
|-------------------|------|------|---------------|-------|
| `diagnostics` | Module | `kernel/src/fs/fs_impls/exfat_refactor/diagnostics.rs` | `#[cfg(ktest)]` invalid-layout diagnosis only | Relocates diagnostic gate helpers out of the former `ondisk.rs` catch-all per bucket 2 |
| `diagnose_invalid_on_disk_layout_gate` | Helper fn | `kernel/src/fs/fs_impls/exfat_refactor/diagnostics.rs` | `#[cfg(ktest)]` diagnostic entry only | Re-exported through `ondisk.rs` only under `#[cfg(ktest)]` |
| `diagnose_boot_region` | Helper fn | `kernel/src/fs/fs_impls/exfat_refactor/diagnostics.rs` | `#[cfg(ktest)]` detailed boot failure gate | Mirrors production boot parsing for failure localization |
| `diagnose_validate_boot_checksum` | Helper fn | `kernel/src/fs/fs_impls/exfat_refactor/diagnostics.rs` | `#[cfg(ktest)]` detailed checksum failure gate | Mirrors production checksum validation for failure localization |
| `diagnose_validate_boot_geometry` | Helper fn | `kernel/src/fs/fs_impls/exfat_refactor/diagnostics.rs` | `#[cfg(ktest)]` detailed geometry failure gate | Mirrors production geometry validation for failure localization |
| `diagnose_scan_root_directory` | Helper fn | `kernel/src/fs/fs_impls/exfat_refactor/diagnostics.rs` | `#[cfg(ktest)]` detailed root-scan failure gate | Replaces the former `DirectoryBootstrap` return with a tuple of owner-local records |
| `diagnose_load_upcase_table` | Helper fn | `kernel/src/fs/fs_impls/exfat_refactor/diagnostics.rs` | `#[cfg(ktest)]` detailed Up-case failure gate | Mirrors Up-case stream validation for failure localization |
| `diagnose_count_used_clusters` | Helper fn | `kernel/src/fs/fs_impls/exfat_refactor/diagnostics.rs` | `#[cfg(ktest)]` detailed bitmap accounting failure gate | Mirrors Allocation Bitmap stream validation for failure localization |
| `finalize_root_records` | Helper fn | `kernel/src/fs/fs_impls/exfat_refactor/diagnostics.rs` | `#[cfg(ktest)]` root-scan diagnostic finalization only | Keeps missing-record gate names consistent across scan exit paths |
| `read_anomaly_state` | Helper fn | `kernel/src/fs/fs_impls/exfat_refactor/diagnostics.rs` | `#[cfg(ktest)]` anomaly-read diagnostic gate | Reads the boot sector only to distinguish device I/O failure |
| `read_device_bytes` | Helper fn | `kernel/src/fs/fs_impls/exfat_refactor/diagnostics.rs` | `#[cfg(ktest)]` diagnostic byte I/O only | Converts block read failure to diagnostic gate decisions |
| `read_le_u16` | Helper fn | `kernel/src/fs/fs_impls/exfat_refactor/diagnostics.rs` | `#[cfg(ktest)]` diagnostic byte decoding only | Local to boot diagnostic parsing |
| `read_le_u32` | Helper fn | `kernel/src/fs/fs_impls/exfat_refactor/diagnostics.rs` | `#[cfg(ktest)]` diagnostic byte decoding only | Local to boot diagnostic parsing |
| `read_le_u64` | Helper fn | `kernel/src/fs/fs_impls/exfat_refactor/diagnostics.rs` | `#[cfg(ktest)]` diagnostic byte decoding only | Local to boot diagnostic parsing |
| `boot_region_checksum` | Helper fn | `kernel/src/fs/fs_impls/exfat_refactor/diagnostics.rs` | `#[cfg(ktest)]` diagnostic checksum math only | Mirrors production checksum math for gate precision |
| `stream_checksum` | Helper fn | `kernel/src/fs/fs_impls/exfat_refactor/diagnostics.rs` | `#[cfg(ktest)]` diagnostic checksum math only | Mirrors Up-case stream checksum for gate precision |

## 5. Contract Deviations & Boundary Notes

- **Incidental Supporting Edits Outside Covered Micro-Features:** Updated the existing `mount_volume_state_root_and_superblock_reads_are_stable` ktest in `kernel/src/fs/fs_impls/exfat_refactor/fs.rs` to use `ExfatFs::root_inode()` and `ExfatFs::sb()` after removing the dispatcher-only production branches. No tests were added.
- **Deviations:** None.
- **Unresolved Ambiguities:** Treated `ondisk.rs` as an allowed thin compatibility surface because the packet explicitly permits it to shrink instead of disappearing, while still requiring owner-local top-level modules.
