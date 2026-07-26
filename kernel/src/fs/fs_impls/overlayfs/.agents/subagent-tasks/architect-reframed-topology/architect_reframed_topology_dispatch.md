<!-- SPDX-License-Identifier: MPL-2.0 -->

# Subagent Dispatch Stub

**Role ID:** ARCHITECT  
**Pass Kind:** Macro Backbone + complete Meso Mapping  
**Task ID:** `ovfs-architect-reframed-topology-20260730`  
**Task Kind:** design  
**Risk Tier:** High  
**Workspace Root:** `/home/ayd/asterinas`  
**Component/Task Group:** `architect-reframed-topology`  
**Parent Meso-Component:** `N/A` (fresh global decomposition)  
**Covered Micro-Features:** All 81 formal Micro IDs in `MICRO_FEATURE_INVENTORY.md`  
**Continuation / Parent Task:** `N/A`  
**Write-Set:**

- `kernel/src/fs/fs_impls/overlayfs/.agents/components/architect-reframed-topology/macro_00_global_topology.md`
- `kernel/src/fs/fs_impls/overlayfs/.agents/components/architect-reframed-topology/meso_XX_<new_component>_architecture.md`
- This dispatch stub, if a bounded continuation is needed

**Capabilities:** `can_read_priors`, `can_read_design_docs`, `can_edit_assigned_artifacts`; no production code, build, test, runtime, or Designer-artifact capability

## 1. Input Context (Read-Only)

Read these files directly. The repository-local `ARCHITECT.md` and `PROTOCOL.md`
remain normative. The old Architect maps, old Meso maps, old Designer specs,
old Designer validation contracts, and their old dispatch packets are deleted
and must not be reconstructed or used as design input.

- `AGENTS.md`
- `kernel/src/fs/fs_impls/overlayfs/.agents/README.md`
- `kernel/src/fs/fs_impls/overlayfs/.agents/PROTOCOL.md`
- `kernel/src/fs/fs_impls/overlayfs/.agents/protocol/ARCHITECT.md`
- `kernel/src/fs/fs_impls/overlayfs/.agents/protocol/templates/macro_00_global_topology_TEMPLATE.md`
- `kernel/src/fs/fs_impls/overlayfs/.agents/protocol/templates/meso_[XX]_[component]_architecture_TEMPLATE.md`
- `kernel/src/fs/fs_impls/overlayfs/.agents/priors/MICRO_FEATURE_INVENTORY.md`
- `kernel/src/fs/fs_impls/overlayfs/.agents/priors/FILESYSTEM_SPEC_SUMMARY.md`
- `kernel/src/fs/fs_impls/overlayfs/.agents/priors/FILESYSTEM_SPEC_INDEX.md`
- `kernel/src/fs/fs_impls/overlayfs/.agents/priors/REFERENCE_IMPLEMENTATION_SUMMARY.md`
- `kernel/src/fs/fs_impls/overlayfs/.agents/priors/ASTERINAS_INTEGRATION_PRIORS.md`
- `kernel/src/fs/fs_impls/overlayfs/.agents/designdoc/stageAdraft.md`
- `kernel/src/fs/fs_impls/overlayfs/.agents/designdoc/stageBCdraft/README.md`
- `kernel/src/fs/fs_impls/overlayfs/.agents/designdoc/stageBCdraft/BC-1-mount-layer-upper-workdir.md`
- `kernel/src/fs/fs_impls/overlayfs/.agents/designdoc/stageBCdraft/BC-2-projection-identity-lookup.md`
- `kernel/src/fs/fs_impls/overlayfs/.agents/designdoc/stageBCdraft/BC-3-merged-directory-readdir.md`
- `kernel/src/fs/fs_impls/overlayfs/.agents/designdoc/stageBCdraft/BC-4-copy-up-file-io-page-cache.md`
- `kernel/src/fs/fs_impls/overlayfs/.agents/designdoc/stageBCdraft/BC-5-metadata-permission-xattr.md`
- `kernel/src/fs/fs_impls/overlayfs/.agents/designdoc/stageBCdraft/BC-6-directory-mutation-whiteout.md`
- `kernel/src/fs/fs_impls/overlayfs/.agents/designdoc/stageBCdraft/BC-7-advanced-identity-export-data.md`
- `kernel/src/fs/fs_impls/overlayfs/.agents/designdoc/stageBCdraft/BC-8-cross-module-reconciliation.md`
- `kernel/src/fs/fs_impls/overlayfs/.agents/designdoc/stageDdraft.md`
- `kernel/src/fs/fs_impls/overlayfs/.agents/designdoc/stageEdraft.md`
- `kernel/src/fs/fs_impls/overlayfs/.agents/designdoc/stageFdraft.md`
- `kernel/src/fs/fs_impls/overlayfs/.agents/main-agent/20260724-p0-p1-design-tracking_main_agent_handoff.md`

The Stage-F responsibility table and any old component names appearing in the
design documents are historical proposals for this task. Treat them as
semantic evidence to reassess, not as an accepted component boundary or naming
source. Derive the new decomposition from the top-down design and the staged
priors. Do not inspect the legacy implementation `kernel/src/fs/fs_impls/overlayfs/fs.rs`.

## 2. Output Requirement

Generate exactly these classes of Architect artifacts and no others:

1. One complete Macro artifact:
   `kernel/src/fs/fs_impls/overlayfs/.agents/components/architect-reframed-topology/macro_00_global_topology.md`
2. One architecture map for every newly defined Meso-Component, all under:
   `kernel/src/fs/fs_impls/overlayfs/.agents/components/architect-reframed-topology/`
   using the pattern `meso_XX_<new_component>_architecture.md`.

Every Meso map must use the complete
`meso_[XX]_[component]_architecture_TEMPLATE.md` structure. Each Micro row
must include the exact formal Micro ID (`P0-01` ... `P3-09`, with the inventory's
`P3-10` historical note already folded into `P2-03`) and a precise description.
The 81 formal IDs must occur exactly once as a primary owner across all Meso
maps. Secondary interactions may be recorded in the structural-interaction
section, but they must not create a second primary owner or an unowned feature.

The Macro artifact must use the complete
`macro_00_global_topology_TEMPLATE.md` structure and must contain:

- final-owner / runtime-owner identification;
- the durable-to-runtime projection;
- the fresh candidate Meso index;
- a complete static global lock hierarchy and holding constraints;
- structural lifetime, publication, persistence, and cross-owner invariants.

The new Meso boundaries must be semantic and large enough for one later
Designer contract. Do not produce one Meso per Micro ID, do not pre-slice
Creator or Checker passes, and do not simply reproduce the deleted 13-Meso
split. Reuse a conceptual name only when the fresh ownership analysis proves
that it remains the right boundary; the output must still be a new topology,
not a rename-only copy.

## 3. Static-Architecture Requirements

- Re-derive Macro-Owners and Meso boundaries from the design documents and
  priors. The accepted old topology is intentionally invalid for this task.
- Cover all 81 formal Micro IDs, including deferred/optional P2/P3 behavior;
  record scope status and dependencies without dropping any ID.
- Give each Meso an explicit Macro-Owner, responsibility, static inlet lock
  state, highest lock level it may acquire, prohibited higher-level
  dependencies, and strict external structural interactions.
- Define one global lock topology that is compatible with the Asterinas priors:
  sleep-capable BIO paths cannot use spin-based critical sections, Asterinas
  VFS does not provide a parent-directory lock around inode operations, and
  possible same-level or re-entry hazards must have an explicit static rule.
  Do not copy the old topology without independently justifying it.
- Keep ownership, authority, lifecycle, persistence, and publication sources
  of truth unambiguous. A collaborator may consume another owner's result but
  must not silently become a second owner.
- Record Asterinas divergences and unresolved interface assumptions as
  architectural constraints or explicit open risks. Do not freeze new Rust
  signatures or helper layouts.
- The xfstests mapping remains a separate many-to-many validation concern; do
  not turn tests into Meso boundaries and do not invent validation artifacts.

## 4. Forbidden Outputs and Actions

- Do not create or modify any `*_designer_spec.md` file.
- Do not create or modify any `*_designer_validation.md` file.
- Do not create Designer, Creator, Checker, Reviewer, or implementation-pass
  dispatch packets.
- Do not update `SYSTEM_BLUEPRINT.md`, `PASS_SLICING.md`, any main-agent
  handoff, or any design document; the main agent owns those files.
- Do not write production Rust, tests, ktests, xfstests harness changes, or
  runtime/build artifacts.
- Do not read the deleted old Architect maps or Designer artifacts as a design
  source, even if they are available through git history.
- Do not use the legacy Overlay implementation as a design source.

## 5. Acceptance and Escalation

**Acceptance:** The main agent can mechanically audit the output and find one
Macro artifact, a coherent fresh Meso set, complete template sections, exactly
81 unique primary Micro-ID owners, no owner gaps, no duplicate primary owners,
and a complete static lock topology consistent with the role protocol and
Asterinas priors. No forbidden artifact or file change exists.

**Escalation:** If the design documents and staged priors leave a genuine
ownership or lock conflict unresolved, record the conflict and the competing
constraints in the Macro artifact and stop after the Architect outputs. Do not
solve it by creating Designer artifacts, implementation plans, or production
code; report the exact file/section conflict to the main agent.

**Expected Outputs:**

- `components/architect-reframed-topology/macro_00_global_topology.md`
- `components/architect-reframed-topology/meso_XX_<new_component>_architecture.md` for every fresh Meso

**Run Policy:** This is a command-free design task. Do not run build, test,
runtime, xfstests, or ktest commands. A bounded continuation may revise only
the same Macro/Meso artifact set and must not expand into Designer work.
