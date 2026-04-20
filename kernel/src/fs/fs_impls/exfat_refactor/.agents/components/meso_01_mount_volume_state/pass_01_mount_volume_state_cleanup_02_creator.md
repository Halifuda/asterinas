<!-- SPDX-License-Identifier: MPL-2.0 -->

# Creator Pass Implementation Report: `mount_volume_state_cleanup_02`

*This artifact acts as the Creator's receipt, documenting how the Designer's strict specifications were actually implemented in Rust for one explicit Creator Pass. It serves as context for the synchronized Checker's validation and the Reviewer's static checks.*

## 1. Pass Identity & Write-Set

**Creator Pass ID:** `pass_01_mount_volume_state_cleanup_02`
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
- `kernel/src/fs/fs_impls/exfat_refactor/fs.rs` (retired the production `ondisk.rs` import path and imported mount-state carriers from `boot.rs`, `bitmap.rs`, and `upcase.rs` directly)
- `kernel/src/fs/fs_impls/exfat_refactor/mod.rs` (restricted `ondisk` module exposure to `#[cfg(ktest)]` so it remains test-local only)
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/meso_01_mount_volume_state/pass_01_mount_volume_state_cleanup_02_creator.md` (recorded this cleanup pass)

## 2. Pass Coverage & Contract Satisfaction

- This cleanup applies the Reviewer rejection exactly: production `fs.rs` no longer imports `AllocationBitmapRecord`, `BootRegion`, `VolumeAnomalyState`, `UpcaseTable`, or `load_validated_mount(...)` through `ondisk.rs`.
- The accepted mount-volume-state behavior stays unchanged. `mount_candidate(...)`, publication, superblock projection, and remount handling still consume the same owner-local carriers; only the module boundary on the production path changed.
- `ondisk.rs` is no longer part of the production owner flow. The module remains available only for existing `#[cfg(ktest)]` references after `mod.rs` gates it to the test-only build.
- No new features, public interfaces, tests, or wider `*Record` / `*State` redesigns were introduced in this pass.

## 3. Lock Orchestration & RAII Notes

- No lock order or RAII scopes changed in this cleanup. The pass only rewires module imports and module exposure.
- Pre-publication block I/O still completes before published `ExfatFs` state is reachable, and the published-state read/write lock behavior in `fs.rs` is unchanged.
- Because the cleanup is import-path only on the production side, it introduces no new blocking boundary, lock hold, or guard lifetime.

## 4. Generated Entity Census

### 4.1 Production Entity Census

| Introduced Symbol | Kind | File | Owner / Module Boundary | Real Call Sites or Reuse | Whitelist Rule / Exemption | Final-System Status |
|-------------------|------|------|-------------------------|--------------------------|----------------------------|---------------------|
| `*(None)*` | `*(None)*` | `*(None)*` | `*(None)*` | `*(None)*` | No new production entities were introduced in this cleanup pass. The change only removed the production dependency on the existing `ondisk` compatibility facade. | `*(None)*` |

### 4.2 Trait-Required Grouped Methods

| Impl Block | Grouped Methods | Why Exempt from Per-Method Census |
|------------|-----------------|-----------------------------------|
| `*(None)*` | `*(None)*` | No new trait impl blocks or trait-required methods were introduced in this cleanup pass. |

### 4.3 Test-Only Entity Census

| Introduced Symbol | Kind | File | Why Test-Only | Notes |
|-------------------|------|------|---------------|-------|
| `ondisk` | Module visibility change | `kernel/src/fs/fs_impls/exfat_refactor/ondisk.rs` | Existing `#[cfg(ktest)]` references in `fs.rs` still use the compatibility facade for test-local convenience only. | The module itself is not new, but `kernel/src/fs/fs_impls/exfat_refactor/mod.rs` now declares it under `#[cfg(ktest)]`, removing it from the production path while preserving the archived test call sites. |

## 5. Contract Deviations & Boundary Notes

- **Incidental Supporting Edits Outside Covered Micro-Features:** None.
- **Deviations:** None. This pass only retires the production `ondisk.rs` import path requested by the Reviewer.
- **Unresolved Ambiguities:** None. The packet explicitly allowed `ondisk.rs` to remain for non-production compatibility, so I kept it as a test-only module instead of widening this pass into test rewrites or broader carrier reshaping.
