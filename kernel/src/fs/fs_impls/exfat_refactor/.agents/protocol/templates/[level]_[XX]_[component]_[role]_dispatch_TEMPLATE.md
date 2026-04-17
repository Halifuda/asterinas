<!-- SPDX-License-Identifier: MPL-2.0 -->

# Subagent Dispatch Stub (Strict Funnel)

**Role ID:** [ARCHITECT | DESIGNER | CREATOR | CHECKER | REVIEWER]
**Pass Kind:** [Macro Backbone | Meso Mapping | Meso Spec | Creator-Synced Pass | Meso Integration Pass | Reviewer Pass]
**Component/Task Group:** [e.g., EXR-WRITEAT-P02]
**Parent Meso-Component:** [e.g., `meso_03_write_at` or `N/A` for macro backbone]
**Covered Micro-Features:** [Exact names, one per bullet if multiple, or `N/A`]

---

## 1. Input Context (Read-Only)
*List exact file paths allowed by the Information Funnel for this Role. DO NOT summarize the contents. The Subagent MUST read these files directly.*
- `[path/to/prior_or_upstream_artifact.md]`
- `[path/to/relevant_code_files.rs]`

## 2. Output Requirement
*The exact template the Subagent MUST fill out. Do not deviate from its structure.*
- **Required Template:** `.agents/protocol/templates/[level]_[XX]_[component]_[role]_TEMPLATE.md`
- **Output Destination:** `[path/to/destination_artifact.md]`

## 3. Specific Overrides & Commands (Keep Minimal)
*Only list execution-specific overrides (like testing filters) or repair paths. NO architectural summaries, design hints, or "how-to" tutorials here.*
- [e.g., "Checker: Lock required before running `cargo osdk test`", or "Integration Pass: implement only the meso-level integration scenarios", or "None"]
