<!-- SPDX-License-Identifier: MPL-2.0 -->

# Subagent Dispatch Stub (Strict Funnel)

**Role ID:** ARCHITECT
**Pass Kind:** Meso Mapping
**Component/Task Group:** `meso_02_free_space_accounting_and_discard`
**Parent Meso-Component:** `meso_02_free_space_accounting_and_discard`
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
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/20260417-1956-architect_review_repair_main_agent_handoff.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/priors/ASTERINAS_MICRO_FEATURE_INVENTORY.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/priors/Microsoft-exFAT-spec.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/priors/Microsoft-exFAT-spec-index.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/priors/linux-exFAT-implementation-summary.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/priors/ASTERINAS_INTEGRATION_PRIORS.md`

## 2. Output Requirement
*The exact template the Subagent MUST fill out. Do not deviate from its structure.*
- **Required Template:** `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/templates/meso_[XX]_[component]_architecture_TEMPLATE.md`
- **Output Destination:** `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/meso_02_free_space_accounting_and_discard/meso_02_free_space_accounting_and_discard_architecture.md`

## 3. Specific Overrides & Commands (Keep Minimal)
*Only list execution-specific overrides (like testing filters) or repair paths. NO architectural summaries, design hints, or "how-to" tutorials here.*
- Map only `meso_02_free_space_accounting_and_discard` from the accepted macro topology.
- Pull all relevant micro-features from the authorized inventory for allocator accounting, `statfs`, online discard, `FITRIM`, and recount fallback.
- Keep mount/bootstrap seeding as an upstream interaction with `meso_01_mount_volume_state`; do not remap `meso_01` ownership.
- Keep the micro-feature rows exhaustive and unsliced; the main agent will decide later Creator/Checker pass boundaries.
- Tie static lock boundaries strictly to `macro_00_global_topology.md`; do not revise the macro lock hierarchy.
- Do not edit `kernel/src/fs/fs_impls/exfat_refactor/.agents/SYSTEM_BLUEPRINT.md`.
- Do not write production Rust files.
- Do not create Designer, Creator, Checker, Reviewer, or other meso artifacts in this pass.
- Do not run build or test commands.
