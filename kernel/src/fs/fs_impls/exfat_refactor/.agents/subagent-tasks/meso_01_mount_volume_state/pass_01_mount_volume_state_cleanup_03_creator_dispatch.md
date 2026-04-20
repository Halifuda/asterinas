<!-- SPDX-License-Identifier: MPL-2.0 -->

# Subagent Dispatch Stub (Strict Funnel)

**Role ID:** CREATOR
**Pass Kind:** Creator-Synced Pass
**Component/Task Group:** `pass_01_mount_volume_state_cleanup_03`
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
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/meso_01_mount_volume_state/pass_01_mount_volume_state_cleanup_02_creator.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/meso_01_mount_volume_state/pass_01_mount_volume_state_cleanup_02_reviewer.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/meso_01_mount_volume_state/pass_01_mount_volume_state_cleanup_02_checker.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/priors/ASTERINAS_CODE_QUALITY_PRIORS.md`
- `kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/boot.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/fat.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/upcase.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/diagnostics.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/mod.rs`

## 2. Output Requirement
*The exact template the Subagent MUST fill out. Do not deviate from its structure.*
- **Required Template:** `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/templates/pass_[XX]_[component]_creator_TEMPLATE.md`
- **Output Destination:** `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/meso_01_mount_volume_state/pass_01_mount_volume_state_cleanup_03_creator.md`

## 3. Specific Overrides & Commands (Keep Minimal)
*Only list execution-specific overrides (like testing filters) or repair paths. NO architectural summaries, design hints, or "how-to" tutorials here.*
- This is a user-requested structural cleanup pass for the already accepted `pass_01_mount_volume_state` implementation surface only; do not implement unfinished `meso_01_mount_volume_state` work and do not widen into downstream mesos.
- The primary target is the surviving production free-helper families in `boot.rs`, `fat.rs`, `bitmap.rs`, and `upcase.rs`.
- For every surviving production free helper in the assigned write-set, either:
  - absorb it into a clearer owner-local type / impl / narrower boundary, or
  - keep it only if the report gives a concrete Rule A/B/C-style justification for why it should remain a naked helper in the final pass-01 structure.
- Do not treat “this helper predates the pass” as sufficient justification. This pass is a live re-judgment of surviving helper families under user direction.
- Keep the pass bounded. You do **not** need to redesign every `*Record` / `*State` carrier globally, but if a naked helper family only exists to prop up a weak carrier boundary, you may tighten that local owner shape inside the assigned write-set.
- Preserve the accepted pass-scoped behavior. No new features, public interfaces, or meso expansion.
- If the checker repair batch from `pass_01_mount_volume_state_cleanup_02_checker.md` can be cleanly fixed inside the same touched files without widening scope, include those compile fixes as incidental support; otherwise record why they were left for a later repair-only pass.
- Production write-set is restricted to Rust files under `kernel/src/fs/fs_impls/exfat_refactor/`.
- Do not mine `kernel/src/fs/fs_impls/exfat/` as an oracle, scaffold, or structure template.
- The Creator report MUST include a complete generated-entity census for every introduced production entity and must also include an explicit subsection auditing the surviving production free-helper families in the touched write-set: kept, absorbed, moved, or deleted, with rationale.
- Do not modify Architect, Designer, Checker, Reviewer, or `SYSTEM_BLUEPRINT.md` artifacts.
- Do not write tests.
- Do not run build or test commands.
