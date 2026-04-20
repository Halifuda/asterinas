<!-- SPDX-License-Identifier: MPL-2.0 -->

# Subagent Dispatch Stub (Strict Funnel)

**Role ID:** CREATOR
**Pass Kind:** Creator-Synced Pass
**Component/Task Group:** `pass_01_mount_volume_state_cleanup_02`
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
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/meso_01_mount_volume_state/pass_01_mount_volume_state_creator.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/meso_01_mount_volume_state/pass_01_mount_volume_state_checker.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/meso_01_mount_volume_state/pass_01_mount_volume_state_reviewer.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/meso_01_mount_volume_state/pass_01_mount_volume_state_cleanup_01_creator.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/meso_01_mount_volume_state/pass_01_mount_volume_state_cleanup_01_reviewer.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/priors/ASTERINAS_CODE_QUALITY_PRIORS.md`
- `kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/ondisk.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/boot.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/fat.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/upcase.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/diagnostics.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- `kernel/src/fs/fs_impls/mod.rs`

## 2. Output Requirement
*The exact template the Subagent MUST fill out. Do not deviate from its structure.*
- **Required Template:** `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/templates/pass_[XX]_[component]_creator_TEMPLATE.md`
- **Output Destination:** `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/meso_01_mount_volume_state/pass_01_mount_volume_state_cleanup_02_creator.md`

## 3. Specific Overrides & Commands (Keep Minimal)
*Only list execution-specific overrides (like testing filters) or repair paths. NO architectural summaries, design hints, or "how-to" tutorials here.*
- This is a follow-up structural cleanup pass for the already accepted `pass_01_mount_volume_state` implementation surface only; do not implement unfinished `meso_01_mount_volume_state` work and do not widen into downstream mesos.
- Preserve the pass-scoped behavior already accepted by `pass_01_mount_volume_state`; do not add new features, tests, or public interfaces.
- Apply Reviewer rejection `pass_01_mount_volume_state_cleanup_01_reviewer.md` exactly: retire the live production `ondisk.rs` compatibility boundary from the `fs.rs` production path.
- Required outcome for this pass: production `fs.rs` must import boot / bitmap / upcase / mount-bootstrap carriers directly from their owner-local modules instead of routing through `ondisk.rs`.
- `ondisk.rs` may remain only for non-production compatibility or test-local convenience if no production owner path depends on it; if it is no longer needed, remove it.
- Keep this pass narrow. Do **not** widen into a fresh re-judgment of all `*Record` / `*State` carriers; that doubt remains recorded but deferred.
- Production write-set is restricted to Rust files under `kernel/src/fs/fs_impls/exfat_refactor/`, plus `kernel/src/fs/fs_impls/mod.rs` only if module exposure is required.
- Do not mine `kernel/src/fs/fs_impls/exfat/` as an oracle, scaffold, or structure template.
- The Creator report MUST include a complete generated-entity census for every introduced production entity and a separate test-only census, including any removals or visibility changes relevant to this cleanup.
- Do not modify Architect, Designer, Checker, Reviewer, or `SYSTEM_BLUEPRINT.md` artifacts.
- Do not write tests.
- Do not run build or test commands.
