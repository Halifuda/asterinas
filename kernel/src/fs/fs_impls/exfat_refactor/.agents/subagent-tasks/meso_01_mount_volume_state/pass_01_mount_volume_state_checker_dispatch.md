<!-- SPDX-License-Identifier: MPL-2.0 -->

# Subagent Dispatch Stub (Strict Funnel)

**Role ID:** CHECKER
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
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/CHECKER.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/templates/pass_[XX]_[component]_checker_TEMPLATE.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/meso_01_mount_volume_state/meso_01_mount_volume_state_designer_ktest.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/meso_01_mount_volume_state/pass_01_mount_volume_state_creator.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/tools/checker_lock.sh`
- `kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/ondisk.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- `kernel/src/fs/fs_impls/mod.rs`

## 2. Output Requirement
*The exact template the Subagent MUST fill out. Do not deviate from its structure.*
- **Required Template:** `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/templates/pass_[XX]_[component]_checker_TEMPLATE.md`
- **Output Destination:** `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/meso_01_mount_volume_state/pass_01_mount_volume_state_checker.md`

## 3. Specific Overrides & Commands (Keep Minimal)
*Only list execution-specific overrides (like testing filters) or repair paths. NO architectural summaries, design hints, or "how-to" tutorials here.*
- This is the Creator-synced Checker for `pass_01_mount_volume_state`; do not widen or narrow the covered micro-feature set.
- Test-writing is in scope. Production logic edits are forbidden; if logic fails, emit a repair batch instead of patching the implementation.
- Execution must use `.agents/tools/checker_lock.sh acquire` before any `cargo` or `make` command and `.agents/tools/checker_lock.sh release` afterward.
- Full compile receipt is required with `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas && make kernel'`.
- Runtime verification must use exact-name `cargo osdk test` commands in `codex-asterinas-dev`: `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test <ktest full name>'`.
- Record exact-name proof and inspect `qemu-serial.log`.
- If failure is deeper than a very shallow obvious fix, stop at the repair batch; do not attempt production repair locally.
