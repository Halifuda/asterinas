<!-- SPDX-License-Identifier: MPL-2.0 -->

# Subagent Dispatch Stub (Strict Funnel)

**Role ID:** CHECKER
**Pass Kind:** Creator-Synced Pass
**Component/Task Group:** `pass_01_mount_volume_state_cleanup_04`
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

---

## 1. Input Context (Read-Only)
*List exact file paths allowed by the Information Funnel for this Role. DO NOT summarize the contents. The Subagent MUST read these files directly.*
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/PROTOCOL.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/CHECKER.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/templates/pass_[XX]_[component]_checker_TEMPLATE.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/meso_01_mount_volume_state/meso_01_mount_volume_state_designer_ktest.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/meso_01_mount_volume_state/pass_01_mount_volume_state_cleanup_04_creator.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/meso_01_mount_volume_state/pass_01_mount_volume_state_cleanup_04_reviewer.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/meso_01_mount_volume_state/pass_01_mount_volume_state_cleanup_04_reviewer_followup.md`
- `kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/boot.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/fat.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/upcase.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/test_support/mod.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/test_support/mount_diagnostics.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/test_support/boot_region.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/test_support/root_directory.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/test_support/bitmap.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/test_support/upcase.rs`

## 2. Output Requirement
*The exact template the Subagent MUST fill out. Do not deviate from its structure.*
- **Required Template:** `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/templates/pass_[XX]_[component]_checker_TEMPLATE.md`
- **Output Destination:** `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/meso_01_mount_volume_state/pass_01_mount_volume_state_cleanup_04_checker.md`

## 3. Specific Overrides & Commands (Keep Minimal)
*Only list execution-specific overrides (like testing filters) or repair paths. NO architectural summaries, design hints, or "how-to" tutorials here.*
- Use `.agents/tools/checker_run.sh` for both compile and exact-name ktest execution. If wrapper permission still fails, use the protocol's manual checker-lock fallback and record both the failure and fallback receipts.
- Required full compile receipt:
  - `.agents/tools/checker_run.sh make-kernel --component pass_01_mount_volume_state_cleanup_04 --phase checker`
- Required exact-name ktests:
  - `aster_kernel::fs::fs_impls::exfat_refactor::fs::tests::mount_volume_state_mount_publishes_root_inode_superblock_and_defaults`
  - `aster_kernel::fs::fs_impls::exfat_refactor::fs::tests::mount_volume_state_root_and_superblock_reads_are_stable`
  - `aster_kernel::fs::fs_impls::exfat_refactor::fs::tests::mount_volume_state_recount_fallback_marks_cached_accounting`
  - `aster_kernel::fs::fs_impls::exfat_refactor::fs::tests::mount_volume_state_preserves_volume_anomaly_flags`
  - `aster_kernel::fs::fs_impls::exfat_refactor::fs::tests::mount_volume_state_rejects_invalid_boot_region`
  - `aster_kernel::fs::fs_impls::exfat_refactor::fs::tests::mount_volume_state_rejects_boot_region_device_io`
  - `aster_kernel::fs::fs_impls::exfat_refactor::fs::tests::mount_volume_state_rejects_inconsistent_allocation_bitmap`
  - `aster_kernel::fs::fs_impls::exfat_refactor::fs::tests::mount_volume_state_rejects_allocation_bitmap_device_io`
  - `aster_kernel::fs::fs_impls::exfat_refactor::fs::tests::mount_volume_state_rejects_invalid_upcase_table`
  - `aster_kernel::fs::fs_impls::exfat_refactor::fs::tests::mount_volume_state_remount_allows_discard_and_rejects_immutable_delta`
- Before running commands, inspect `kernel/src/fs/fs_impls/exfat_refactor/test_support/` for any remaining utility-bucket surface. If additional splitting is clearly warranted and can be completed entirely inside checker-owned `#[cfg(ktest)]` support or ktest-only call sites, perform that cleanup and report it explicitly.
- If current code fails after the main-agent removal of `ondisk.rs`, repair only checker-owned `#[cfg(ktest)]` surfaces and continue the pass.
- Do not edit production logic. If a failure requires production changes, emit an actionable repair batch instead.
