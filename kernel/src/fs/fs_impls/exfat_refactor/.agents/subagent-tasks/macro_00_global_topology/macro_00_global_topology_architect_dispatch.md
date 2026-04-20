<!-- SPDX-License-Identifier: MPL-2.0 -->

# Subagent Dispatch Stub (Strict Funnel)

**Role ID:** ARCHITECT
**Pass Kind:** Macro Backbone
**Component/Task Group:** `macro_00_global_topology`
**Parent Meso-Component:** `N/A`
**Covered Micro-Features:** `N/A`

---

## 1. Input Context (Read-Only)
*List exact file paths allowed by the Information Funnel for this Role. DO NOT summarize the contents. The Subagent MUST read these files directly.*
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/PROTOCOL.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/ARCHITECT.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/SYSTEM_BLUEPRINT.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/templates/macro_00_global_topology_TEMPLATE.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/20260417-1956-architect_review_repair_main_agent_handoff.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/priors/ASTERINAS_MICRO_FEATURE_INVENTORY.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/priors/Microsoft-exFAT-spec.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/priors/Microsoft-exFAT-spec-index.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/priors/linux-exFAT-implementation-summary.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/priors/ASTERINAS_INTEGRATION_PRIORS.md`

## 2. Output Requirement
*The exact template the Subagent MUST fill out. Do not deviate from its structure.*
- **Required Template:** `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/templates/macro_00_global_topology_TEMPLATE.md`
- **Output Destination:** `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/macro_00_global_topology/macro_00_global_topology.md`

## 3. Specific Overrides & Commands (Keep Minimal)
*Only list execution-specific overrides (like testing filters) or repair paths. NO architectural summaries, design hints, or "how-to" tutorials here.*
- Treat Section 5 of `kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/20260417-1956-architect_review_repair_main_agent_handoff.md` as non-normative planning context.
- Use the full term `On-disk Structure Owner` for durable exFAT structures; do not shorten it to an ambiguous generic phrase.
- Do not edit `kernel/src/fs/fs_impls/exfat_refactor/.agents/SYSTEM_BLUEPRINT.md`.
- Do not write production Rust files.
- Do not create Phase 2 meso architecture artifacts in this pass.
- Do not run build or test commands.
