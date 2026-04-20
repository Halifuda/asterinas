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
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/meso_01_mount_volume_state/pass_01_mount_volume_state_cleanup_03_creator.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/meso_01_mount_volume_state/pass_01_mount_volume_state_cleanup_03_reviewer.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/meso_01_mount_volume_state/pass_01_mount_volume_state_cleanup_03_checker.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/priors/ASTERINAS_CODE_QUALITY_PRIORS.md`
- `kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/boot.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/fat.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/upcase.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/diagnostics.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/ondisk.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/mod.rs`

## 2. Output Requirement
*The exact template the Subagent MUST fill out. Do not deviate from its structure.*
- **Required Template:** `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/templates/pass_[XX]_[component]_reviewer_TEMPLATE.md`
- **Output Destination:** `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/meso_01_mount_volume_state/pass_01_mount_volume_state_cleanup_04_reviewer.md`

## 3. Specific Overrides & Commands (Keep Minimal)
*Only list execution-specific overrides (like testing filters) or repair paths. NO architectural summaries, design hints, or "how-to" tutorials here.*
- This is a user-requested structural review pass for two remaining cleanup concerns after `pass_01_mount_volume_state_cleanup_03` passed Checker. Do not reopen unfinished later `meso_01_mount_volume_state` work.
- Review concern 1: duplicated `read_le_u16` / `read_le_u32` / `read_le_u64` helper families across production and ktest-only files. Determine whether these wrappers should be deleted in favor of direct `from_le_bytes(...)` calls, or consolidated into one narrowly named byte-decoding module. If a shared module is recommended, it must be narrow and purpose-specific, not a generic `utils` dumping ground.
- Review concern 2: `diagnostics.rs` is a poor long-term test-only namespace name and will not scale if future ktests and support helpers are all placed in one flat file. Determine the preferred test-only module layout, such as a dedicated `test_support` / `tests` submodule hierarchy with file splits, while keeping production ownership flow clean.
- Produce a Reviewer artifact that either approves the current structure with concrete rationale or rejects with a Creator-facing structural cleanup plan. If rejecting, make the required Creator actions explicit enough to execute blindly.
- Treat direct edits as line-level only. Do not perform module splitting, helper relocation, or production/test topology rewrites inside Reviewer.
- Do not run build or test commands.
