<!-- SPDX-License-Identifier: MPL-2.0 -->

# Subagent Dispatch Stub (Strict Funnel)

**Role ID:** DESIGNER
**Pass Kind:** Meso Spec
**Component/Task Group:** `meso_01_mount_volume_state`
**Parent Meso-Component:** `meso_01_mount_volume_state`
**Covered Micro-Features:** All micro-features listed in `meso_01_mount_volume_state_architecture.md`

---

## 1. Input Context (Read-Only)
*List exact file paths allowed by the Information Funnel for this Role. DO NOT summarize the contents. The Subagent MUST read these files directly.*
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/PROTOCOL.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/DESIGNER.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/templates/meso_[XX]_[component]_designer_spec_TEMPLATE.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/templates/meso_[XX]_[component]_designer_ktest_TEMPLATE.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/macro_00_global_topology/macro_00_global_topology.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/meso_01_mount_volume_state/meso_01_mount_volume_state_architecture.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/meso_01_mount_volume_state/meso_01_mount_volume_state_designer_spec.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/meso_01_mount_volume_state/meso_01_mount_volume_state_designer_ktest.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/meso_02_free_space_accounting_and_discard/meso_02_free_space_accounting_and_discard_architecture.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/meso_03_directory_lookup_and_identity/meso_03_directory_lookup_and_identity_architecture.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/meso_08_filesystem_sync_and_volume_state/meso_08_filesystem_sync_and_volume_state_architecture.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/priors/ASTERINAS_INTEGRATION_PRIORS.md`

## 2. Output Requirement
*The exact template the Subagent MUST fill out. Do not deviate from its structure.*
- **Required Template:** `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/templates/meso_[XX]_[component]_designer_spec_TEMPLATE.md`
- **Output Destination:** `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/meso_01_mount_volume_state/meso_01_mount_volume_state_designer_spec.md`
- **Required Template:** `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/templates/meso_[XX]_[component]_designer_ktest_TEMPLATE.md`
- **Output Destination:** `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/meso_01_mount_volume_state/meso_01_mount_volume_state_designer_ktest.md`

## 3. Specific Overrides & Commands (Keep Minimal)
*Only list execution-specific overrides (like testing filters) or repair paths. NO architectural summaries, design hints, or "how-to" tutorials here.*
- Focus this repair on main-agent review findings only.
- Fix the `[GUARANTEE] Meso-Level Interface` so it is an exact, single crate-visible Rust signature with exact type names and exact result / error type. Do not say the Creator may choose concrete type names.
- If the interface relies on meso-local request / outcome enums or structs, name those exact types in the signature and define their required variants / fields in the Designer spec so the Creator has zero architecture guesswork.
- Ensure post-condition error variants use the same exact error type / variants named by the interface contract.
- Keep the artifact meso-scoped and exhaustive across all micro-features named in the Architect map; do not slice Creator passes.
- Do not invent helper APIs or revise the accepted macro / Architect topology.
- Do not write production Rust files.
- Do not run build or test commands.
