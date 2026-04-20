<!-- SPDX-License-Identifier: MPL-2.0 -->

# Subagent Dispatch Stub (Strict Funnel)

**Role ID:** CREATOR
**Pass Kind:** Creator-Synced Pass
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
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/CREATOR.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/templates/pass_[XX]_[component]_creator_TEMPLATE.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/meso_01_mount_volume_state/meso_01_mount_volume_state_designer_spec.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/priors/ASTERINAS_CODE_QUALITY_PRIORS.md`
- `kernel/src/fs/fs_impls/exfat_refactor/`
- `kernel/src/fs/fs_impls/mod.rs`

## 2. Output Requirement
*The exact template the Subagent MUST fill out. Do not deviate from its structure.*
- **Required Template:** `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/templates/pass_[XX]_[component]_creator_TEMPLATE.md`
- **Output Destination:** `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/meso_01_mount_volume_state/pass_01_mount_volume_state_creator.md`

## 3. Specific Overrides & Commands (Keep Minimal)
*Only list execution-specific overrides (like testing filters) or repair paths. NO architectural summaries, design hints, or "how-to" tutorials here.*
- Implement this pass as one complete `meso_01_mount_volume_state` Creator slice covering exactly the micro-features listed above; do not split the pass further.
- Production write-set is restricted to Rust files under `kernel/src/fs/fs_impls/exfat_refactor/`, plus `kernel/src/fs/fs_impls/mod.rs` only if module exposure is required.
- Do not modify Architect, Designer, Checker, Reviewer, or `SYSTEM_BLUEPRINT.md` artifacts.
- Do not write tests.
- Do not run build or test commands.
