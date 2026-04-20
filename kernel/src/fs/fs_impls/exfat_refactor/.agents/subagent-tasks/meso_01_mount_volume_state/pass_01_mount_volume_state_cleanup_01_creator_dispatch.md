<!-- SPDX-License-Identifier: MPL-2.0 -->

# Subagent Dispatch Stub (Strict Funnel)

**Role ID:** CREATOR
**Pass Kind:** Creator-Synced Pass
**Component/Task Group:** `pass_01_mount_volume_state_cleanup_01`
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
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/priors/ASTERINAS_CODE_QUALITY_PRIORS.md`
- `kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/ondisk.rs`
- `kernel/src/fs/fs_impls/mod.rs`

## 2. Output Requirement
*The exact template the Subagent MUST fill out. Do not deviate from its structure.*
- **Required Template:** `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/templates/pass_[XX]_[component]_creator_TEMPLATE.md`
- **Output Destination:** `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/meso_01_mount_volume_state/pass_01_mount_volume_state_cleanup_01_creator.md`

## 3. Specific Overrides & Commands (Keep Minimal)
*Only list execution-specific overrides (like testing filters) or repair paths. NO architectural summaries, design hints, or "how-to" tutorials here.*
- This is a structural cleanup pass for the already accepted `pass_01_mount_volume_state` implementation surface only; do not implement unfinished `meso_01_mount_volume_state` work and do not widen into downstream mesos.
- Preserve the pass-scoped behavior already accepted by `pass_01_mount_volume_state`; do not add new features, tests, or public interfaces.
- Production write-set is restricted to Rust files under `kernel/src/fs/fs_impls/exfat_refactor/`, plus `kernel/src/fs/fs_impls/mod.rs` only if module exposure is required.
- Prefer owner-local top-level Rust modules under `kernel/src/fs/fs_impls/exfat_refactor/`; do not introduce a deep `ondisk/` submodule tree unless strictly required by Rust module mechanics.
- Apply the cleanup using the three explicit buckets already decided by main-agent + user:
  - **Bucket 1 — should not exist at all:** remove `DirectoryBootstrap`; do not preserve, rename, or reintroduce it. The grouped Allocation Bitmap metadata and Up-case metadata belong under their respective owner-local types/helpers. Also remove production dispatcher-only `RootInode` / `SuperBlock` / `Flags` variants if they remain without real production necessity.
  - **Bucket 2 — should exist but in their own file:** split the current `ondisk.rs` catch-all into owner-local top-level modules as needed (boot-region logic, FAT/cluster-chain traversal, Allocation Bitmap logic, Up-case logic, and test-only diagnostics).
  - **Bucket 3 — may exist but are in the wrong place:** relocate retained transit helpers/carriers such as mount bootstrap outputs, anomaly/flag carriers, root-scan helpers, and byte-reading helpers to the narrowest owner-local module instead of leaving them in a neutral catch-all.
- `ondisk.rs` is not a preservation target. It may shrink to a thin compatibility surface or disappear entirely if the resulting owner-local modules no longer need it.
- Keep the current wave bounded: do **not** widen this pass into a full re-judgment of every `*Record` / `*State` carrier. Some of those carriers remain structurally suspect, but this pass should only eliminate or relocate the ones already covered by the agreed cleanup buckets above.
- Do not mine `kernel/src/fs/fs_impls/exfat/` as an oracle, scaffold, or structure template.
- The Creator report MUST include a complete generated-entity census for every introduced production entity and a separate test-only census.
- Do not modify Architect, Designer, Checker, Reviewer, or `SYSTEM_BLUEPRINT.md` artifacts.
- Do not write tests.
- Do not run build or test commands.
