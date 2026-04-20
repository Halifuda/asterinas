<!-- SPDX-License-Identifier: MPL-2.0 -->

# Subagent Dispatch Stub (Strict Funnel)

**Role ID:** REVIEWER
**Pass Kind:** Reviewer Pass
**Component/Task Group:** `pass_01_mount_volume_state`
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
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/CREATOR.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/templates/pass_[XX]_[component]_reviewer_TEMPLATE.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/meso_01_mount_volume_state/pass_01_mount_volume_state_creator.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/meso_01_mount_volume_state/pass_01_mount_volume_state_checker.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/priors/ASTERINAS_CODE_QUALITY_PRIORS.md`
- `kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/ondisk.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- `kernel/src/fs/fs_impls/mod.rs`
- `osdk/src/base_crate/mod.rs`

## 2. Output Requirement
*The exact template the Subagent MUST fill out. Do not deviate from its structure.*
- **Required Template:** `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/templates/pass_[XX]_[component]_reviewer_TEMPLATE.md`
- **Output Destination:** `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/meso_01_mount_volume_state/pass_01_mount_volume_state_reviewer.md`

## 3. Specific Overrides & Commands (Keep Minimal)
*Only list execution-specific overrides (like testing filters) or repair paths. NO architectural summaries, design hints, or "how-to" tutorials here.*
- Review only the accepted `pass_01_mount_volume_state` implementation after the green Checker pass.
- Direct edits are allowed only for static style/quality issues in the listed `.rs` files; no functional state-machine, lock-order, or test-obligation changes.
- Treat the checker-only fixture/support code in `fs.rs` as reviewable static code, but do not remove ktest coverage or exact-name test support.
- Do not run build, test, format, or cargo commands.
