<!-- SPDX-License-Identifier: MPL-2.0 -->

# Subagent Dispatch Stub (Strict Funnel)

**Role ID:** ARCHITECT
**Pass Kind:** Macro Backbone + complete Meso Mapping
**Component/Task Group:** `architect-global-topology`
**Parent Meso-Component:** N/A for the Macro artifact; establish all Meso components in this design wave
**Covered Micro-Features:** The complete formal inventory: `P0-01` through `P0-18`, `P1-01` through `P1-37`, `P2-01` through `P2-17`, and `P3-01` through `P3-09` (81 IDs total). Do not create a `P3-10` feature; the inventory's historical folded-note is not a feature ID.

## 1. Input Context (Read-Only)

Read the packet first, then the assigned role protocol, then the scheduler protocol. Read these staged priors directly and use them as the authoritative design input:

- `kernel/src/fs/fs_impls/overlayfs/.agents/protocol/ARCHITECT.md`
- `kernel/src/fs/fs_impls/overlayfs/.agents/PROTOCOL.md`
- `kernel/src/fs/fs_impls/overlayfs/.agents/README.md`
- `kernel/src/fs/fs_impls/overlayfs/.agents/priors/README.md`
- `kernel/src/fs/fs_impls/overlayfs/.agents/priors/FILESYSTEM_SPEC_SUMMARY.md`
- `kernel/src/fs/fs_impls/overlayfs/.agents/priors/FILESYSTEM_SPEC_INDEX.md`
- `kernel/src/fs/fs_impls/overlayfs/.agents/priors/REFERENCE_IMPLEMENTATION_SUMMARY.md`
- `kernel/src/fs/fs_impls/overlayfs/.agents/priors/MICRO_FEATURE_INVENTORY.md`
- `kernel/src/fs/fs_impls/overlayfs/.agents/priors/ASTERINAS_INTEGRATION_PRIORS.md`
- `kernel/src/fs/fs_impls/overlayfs/.agents/priors/ASTERINAS_CODE_QUALITY_PRIORS.md`
- `.agents/skills/ra-code-nav/SKILL.md`

Do not use `kernel/src/fs/fs_impls/overlayfs/fs.rs` or other legacy implementation files as a design oracle. Do not read unrelated implementation files unless a scoped, read-only symbol lookup is required to verify an Asterinas integration constraint.

## 2. Output Requirement

Produce the Macro artifact and every Meso architecture map required by the final ownership projection. Use the exact supplied templates and do not add Designer, Creator, Checker, Reviewer, or pass-slicing artifacts.

- **Required Macro Template:** `kernel/src/fs/fs_impls/overlayfs/.agents/protocol/templates/macro_00_global_topology_TEMPLATE.md`
- **Required Meso Template:** `kernel/src/fs/fs_impls/overlayfs/.agents/protocol/templates/meso_[XX]_[component]_architecture_TEMPLATE.md`
- **Output Directory:** `kernel/src/fs/fs_impls/overlayfs/.agents/components/architect-global-topology/`
- **Required Macro Output:** `kernel/src/fs/fs_impls/overlayfs/.agents/components/architect-global-topology/macro_00_global_topology.md`
- **Required Meso Outputs:** `meso_XX_<component_name>_architecture.md`, with contiguous stable numeric IDs and one file for each accepted Meso component.

The Macro artifact must identify Macro-Owners, durable/runtime projection, candidate Meso boundaries, the complete static global lock topology, lock holding/BIO constraints, and cross-cutting lifetime/invariant rules. The Meso artifacts must use the Macro artifact's names, assign every one of the 81 formal micro-feature IDs exactly once as the primary owner, state explicit Macro-Owner and static lock inlet/topology placement, and record cross-component structural interactions without dynamic execution paths.

## 3. Specific Overrides & Commands (Keep Minimal)

- Execute both Architect phases in order: establish and internally review `macro_00_global_topology.md` first, then generate all Meso maps against that fixed topology.
- Keep the Macro/Meso/Micro hierarchy independent from xfstests granularity. xfstests is an external many-to-many validation view and is not an ownership tree or unit-test substitute.
- This is an xfstests-only refactor. Do not request, design, create, modify, or imply any `#[ktest]`, `#[cfg(ktest)]`, kernel-mode test module, `test_support/`, memory-disk fixture, or other ktest harness anywhere in the repository. Do not add any test artifact.
- Honor the Asterinas integration priors: there is no caller-held VFS parent-directory lock for inode operations; introduce and place the overlay-owned `DIR` consistency domain; any lock-held path that may cross BIO must use `ostd::sync::Mutex`; account for `INODE`, `CUL`, and `UPPER` as BIO-crossing domains; keep `WL` short/no-BIO; model `IU` as mount-time exclusivity with atomic/waitqueue semantics; and account for no reentrant locks. Resolve these constraints into one coherent static topology, or record a precise integration divergence rather than hand-waving it.
- Do not specify dynamic lock acquisition sequences, rollback mechanics, private helper functions, concrete new Rust signatures, Creator/Checker pass slicing, or implementation order beyond the required static hierarchy.
- The Meso maps must be exhaustive but unsliced. Include a final coverage ledger or equivalent in the Macro artifact that makes the 81-ID ownership count, duplicate ownership, and owner-gap status directly auditable.
- Do not run builds, tests, QEMU, xfstests, or other dynamic execution. This packet is command-free architecture work.

Stop after writing the complete Macro artifact and all Meso architecture maps. In the final response, list the exact artifact paths and report the number of formal micro-feature IDs assigned, any intentional cross-cutting secondary references, and any unresolved architectural ambiguity for the main agent to review.
