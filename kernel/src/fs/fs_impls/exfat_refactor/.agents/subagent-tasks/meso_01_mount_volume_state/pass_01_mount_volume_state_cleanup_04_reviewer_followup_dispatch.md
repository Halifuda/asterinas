<!-- SPDX-License-Identifier: MPL-2.0 -->

# Subagent Dispatch Stub (Strict Funnel)

**Role ID:** REVIEWER
**Pass Kind:** Reviewer Pass
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
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/REVIEWER.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/templates/pass_[XX]_[component]_reviewer_TEMPLATE.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/meso_01_mount_volume_state/pass_01_mount_volume_state_cleanup_04_reviewer.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/meso_01_mount_volume_state/pass_01_mount_volume_state_cleanup_04_creator.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/meso_01_mount_volume_state/pass_01_mount_volume_state_cleanup_03_checker.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/priors/ASTERINAS_CODE_QUALITY_PRIORS.md`
- `kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/boot.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/fat.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/upcase.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/ondisk.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/test_support/mod.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/test_support/mount_diagnostics.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/test_support/boot_region.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/test_support/root_directory.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/test_support/bitmap.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/test_support/upcase.rs`

## 2. Output Requirement
*The exact template the Subagent MUST fill out. Do not deviate from its structure.*
- **Required Template:** `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/templates/pass_[XX]_[component]_reviewer_TEMPLATE.md`
- **Output Destination:** `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/meso_01_mount_volume_state/pass_01_mount_volume_state_cleanup_04_reviewer_followup.md`

## 3. Specific Overrides & Commands (Keep Minimal)
*Only list execution-specific overrides (like testing filters) or repair paths. NO architectural summaries, design hints, or "how-to" tutorials here.*
- This is the follow-up structural quality gate for the Creator response to `pass_01_mount_volume_state_cleanup_04_reviewer.md`.
- Confirm two things only:
  - duplicated `read_le_*` wrappers are actually gone, with direct fixed-width `from_le_bytes(...)` decoding left in place,
  - the old flat `diagnostics.rs` shape was replaced by a dedicated `#[cfg(ktest)]` `test_support/` hierarchy split by concern.
- Also verify that the remaining ktest-only `ondisk.rs` shim is still outside production ownership flow and now re-exports from `test_support` rather than a flat diagnostics file.
- Keep direct Reviewer edits line-level and non-functional only.
- Do not run build or test commands.
