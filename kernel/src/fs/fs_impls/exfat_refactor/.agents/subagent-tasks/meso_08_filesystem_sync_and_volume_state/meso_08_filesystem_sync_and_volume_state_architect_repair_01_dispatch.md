<!-- SPDX-License-Identifier: MPL-2.0 -->

# Subagent Dispatch Stub (Strict Funnel)

**Role ID:** ARCHITECT
**Pass Kind:** Meso Mapping
**Component/Task Group:** `meso_08_filesystem_sync_and_volume_state`
**Parent Meso-Component:** `meso_08_filesystem_sync_and_volume_state`
**Covered Micro-Features:** `N/A`

---

## 1. Input Context (Read-Only)
*List exact file paths allowed by the Information Funnel for this Role. DO NOT summarize the contents. The Subagent MUST read these files directly.*
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/PROTOCOL.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/ARCHITECT.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/SYSTEM_BLUEPRINT.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/templates/meso_[XX]_[component]_architecture_TEMPLATE.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/macro_00_global_topology/macro_00_global_topology.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/meso_01_mount_volume_state/meso_01_mount_volume_state_architecture.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/meso_08_filesystem_sync_and_volume_state/meso_08_filesystem_sync_and_volume_state_architecture.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/20260417-1956-architect_review_repair_main_agent_handoff.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/priors/ASTERINAS_MICRO_FEATURE_INVENTORY.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/priors/Microsoft-exFAT-spec.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/priors/Microsoft-exFAT-spec-index.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/priors/linux-exFAT-implementation-summary.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/priors/ASTERINAS_INTEGRATION_PRIORS.md`

## 2. Output Requirement
*The exact template the Subagent MUST fill out. Do not deviate from its structure.*
- **Required Template:** `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/templates/meso_[XX]_[component]_architecture_TEMPLATE.md`
- **Output Destination:** `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/meso_08_filesystem_sync_and_volume_state/meso_08_filesystem_sync_and_volume_state_architecture.md`

## 3. Specific Overrides & Commands (Keep Minimal)
*Only list execution-specific overrides (like testing filters) or repair paths. NO architectural summaries, design hints, or "how-to" tutorials here.*
- Focus this repair on main-agent review findings only.
- Add or explicitly account for `INV-VFS-004` as it relates to filesystem-wide sync semantics; keep root publication and initial mount lifecycle ownership with `meso_01_mount_volume_state`.
- Add or explicitly account for `INV-PHY-010` as a static VolumeDirty write-ordering bracket / overlay obligation; do not turn it into dynamic choreography or steal per-mutation ownership from `meso_04` / `meso_06`.
- Keep `INV-VFS-025` file-scoped `fsync` outside this meso except as an external structural interaction with `meso_07_file_sync_and_persistence`.
- Do not edit `kernel/src/fs/fs_impls/exfat_refactor/.agents/SYSTEM_BLUEPRINT.md`.
- Do not write production Rust files.
- Do not create Designer, Creator, Checker, Reviewer, or other meso artifacts in this pass.
- Do not run build or test commands.
