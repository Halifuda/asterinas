<!-- SPDX-License-Identifier: MPL-2.0 -->

# Subagent Dispatch Stub (Strict Funnel)

**Role ID:** CREATOR
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
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/CREATOR.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/templates/pass_[XX]_[component]_creator_TEMPLATE.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/meso_01_mount_volume_state/pass_01_mount_volume_state_cleanup_04_reviewer.md`
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
- **Required Template:** `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/templates/pass_[XX]_[component]_creator_TEMPLATE.md`
- **Output Destination:** `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/meso_01_mount_volume_state/pass_01_mount_volume_state_cleanup_04_creator.md`

## 3. Specific Overrides & Commands (Keep Minimal)
*Only list execution-specific overrides (like testing filters) or repair paths. NO architectural summaries, design hints, or "how-to" tutorials here.*
- Execute the structural cleanup requested by `pass_01_mount_volume_state_cleanup_04_reviewer.md`; do not reopen unfinished later `meso_01_mount_volume_state` work.
- Remove the duplicated `read_le_u16` / `read_le_u32` / `read_le_u64` wrapper families from production and ktest-only code. Replace each call site with direct fixed-width `from_le_bytes(...)` decoding. Do not create a generic `utils` dumping ground.
- Replace the flat `#[cfg(ktest)] mod diagnostics;` layout with a dedicated test-only support hierarchy. Preferred shape:
  - `#[cfg(ktest)] mod test_support;` in `mod.rs`
  - `test_support/mod.rs` re-exporting the compatibility entrypoints needed by tests / `ondisk.rs`
  - split the current diagnostic logic into purpose-specific files such as `test_support/mount_diagnostics.rs`, `test_support/boot_region.rs`, `test_support/root_directory.rs`, `test_support/bitmap.rs`, and `test_support/upcase.rs` when that split keeps the files cohesive.
- Update `ondisk.rs` and `fs.rs` ktest-only references to import from the new test-support hierarchy without reintroducing a production catch-all path.
- Preserve the accepted pass-scoped behavior. No new features, public interfaces, or meso expansion.
- Production write-set is restricted to Rust files under `kernel/src/fs/fs_impls/exfat_refactor/`.
- Do not mine `kernel/src/fs/fs_impls/exfat/` as an oracle, scaffold, or structure template.
- The Creator report MUST include:
  - complete production entity census for introduced/removed/moved production entities,
  - separate test-only census for every introduced/moved test-support module and helper,
  - an explicit subsection confirming every old `read_le_*` wrapper was removed or explaining any exception.
- Do not modify Architect, Designer, Checker, Reviewer, or `SYSTEM_BLUEPRINT.md` artifacts.
- Do not write tests.
- Do not run build or test commands.
