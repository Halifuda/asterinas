<!-- SPDX-License-Identifier: MPL-2.0 -->

# Subagent Dispatch Stub (Strict Funnel)

**Role ID:** [ARCHITECT | DESIGNER | CREATOR | CHECKER | REVIEWER]
**Component/Task Group:** [e.g., EXR-ALLOC]
**Atomic transaction/Micro-Feature:** [Specific focus if applicable]

---

## 1. Input Context (Read-Only)
*List exact file paths allowed by the Information Funnel for this Role. DO NOT summarize the contents. The Subagent MUST read these files directly.*
- `[path/to/prior_or_upstream_artifact.md]`
- `[path/to/relevant_code_files.rs]`

## 2. Output Requirement
*The exact template the Subagent MUST fill out. Do not deviate from its structure.*
- **Required Template:** `new_protocol/protocol/templates/[level]_[XX]_[component]_[role]_[type]_TEMPLATE.md`
- **Output Destination:** `[path/to/destination_artifact.md]`

## 3. Specific Overrides & Commands (Keep Minimal)
*Only list execution-specific overrides (like testing filters) or repair paths. NO architectural summaries, design hints, or "how-to" tutorials here.*
- [e.g., "Checker: Lock required before running `cargo osdk test`", or "Creator: Repair batch from Checker attached below", or "None"]
