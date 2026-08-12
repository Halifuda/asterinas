<!-- SPDX-License-Identifier: MPL-2.0 -->

# Subagent Dispatch Stub (Strict Funnel)

**Role ID:** [ARCHITECT | DESIGNER | CREATOR | CHECKER | REVIEWER]
**Pass Kind:** [Macro Backbone | Meso Mapping | Meso Spec | Creator-Synced Pass | Meso Integration Pass | Reviewer Pass]
**Task ID:** [Stable task identifier]
**Task Kind:** [design | implementation | diagnosis | validation | review]
**Risk Tier:** [Low | Normal | High]
**Workspace Root:** [Workspace-relative alias or canonical root]
**Component/Task Group:** [e.g., EXR-WRITEAT-P02]
**Parent Meso-Component:** [e.g., `meso_03_write_at` or `N/A` for macro backbone]
**Covered Micro-Features:** [Exact names, one per bullet if multiple, or `N/A`]
**Continuation / Parent Task:** [Existing task/event ID, or `N/A`]
**Write-Set:** [Exact files/directories the role may modify, or `Read-only`]
**Capabilities:** [Explicitly granted capabilities, e.g. `can_edit`, `can_compile`, `can_runtime_test`, `can_create_continuation`]

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
- [For structural cleanup passes, enumerate each targeted structural objective as a separate bullet instead of bundling them into one generic "cleanup" line.]
- [For full-surface Reviewer structural-audit passes, explicitly say that every production `struct`, `enum`, return carrier, operation/outcome carrier, and non-trait helper in the named files is in scope; generic "review `fs.rs`" wording is insufficient.]
- [For user-named repair waves, copy every named symbol, helper family, legacy file-local test module, and legacy test-support path into the packet checklist; downstream artifacts must disposition each one.]
- [For carrier/helper cleanup, state the default rejection rule: temporary carriers, top-level helper families, and thin helpers must be removed, inlined, moved, or strongly proven.]
- [For full-surface legacy test audits, explicitly enumerate every in-scope pre-existing filesystem-local test module, test helper family, and test-support file; do not rely on generic wording like "check the tests".]
- [For Checker packets, state the approved validation lane and required receipts, e.g., NixOS xfstests config, exact generic test IDs/groups, filesystem type proof, result/notrun/fail files, and preserved guest logs.]
- [e.g., "Checker: Lock required before running NixOS xfstests", or "Integration Pass: validate only the meso-level integration scenarios", or "None"]

## 4. Manifest Contract

- **Acceptance:** [Exact acceptance conditions for this task]
- **Escalation:** [Exact escalation trigger and recipient]
- **Expected Outputs:** [Artifact paths, validation run IDs, or `None`]
- **Run Policy:** [If validation, state how reruns/suffixes/compile preflights
  reuse this task boundary and receive distinct `run_id` values.]
