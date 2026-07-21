<!-- SPDX-License-Identifier: MPL-2.0 -->

# Overlayfs Refactor Multi-Agent Workspace

This directory stores the operating protocol for the `overlayfs` refactor workspace at `kernel/src/fs/fs_impls/overlayfs/`.

The legacy single-file implementation lives at `kernel/src/fs/fs_impls/overlayfs/fs.rs` and remains the active registered filesystem until the refactor explicitly schedules a takeover. Per `PROTOCOL.md`, the legacy implementation MUST NOT be used as a reference for the new design; the Architect internalizes the staged priors instead.

This directory was bootstrapped from the generic filesystem protocol bundle; the bundle's generic adoption notes have been superseded by the overlayfs-specific protocol text below.

---

# Filesystem Implementation/Refactor Multi-Agent Workspace

This directory stores the operating protocol for a filesystem implementation or refactor workspace.

## Project Framing

This project is not just "implement a filesystem."
It has two equally important goals:

1. Implement or refactor the filesystem into clearer, better-specified components.
2. Explore the practical automation boundary of LLM agents when building filesystem code without losing engineering control.

The main question is whether agents can do filesystem engineering without losing control of specification coverage, implementation detail, style consistency, and bug rate.

The adopting workspace may keep a legacy filesystem module active while building a new implementation or a major refactor in parallel. Stage the authoritative filesystem specification and one or more reference implementations as priors before opening Architect work.

**Top-Down Strict Protocol**: Concurrency, locks, and system states are static and dynamic laws determined upfront by the Architect and Designer before the Creator writes a single line of code. Macro/Meso/Micro are architecture, ownership, traceability, and scheduling levels, not test granularity levels. The global lock topology is a macro-level artifact; per-component Architect and Designer artifacts are meso-scoped; the main agent later slices each meso contract into implementation passes for the Creator and synchronized Checker. External xfstests provide many-to-many behavioral evidence. This refactor is xfstests-only and must not create, modify, or grow any ktest-based validation.

## Why Macro and Meso Both Exist

- **Macro-Owner / Macro topology** answers who owns durable state, lifetime, and
  cross-component invariants, and establishes the global lock ordering. It
  prevents independently designed components from acquiring locks in
  incompatible orders or creating competing owners for the same state.
- **Meso-Component** answers where one coherent semantic operation lives: its
  boundary, preconditions and postconditions, static lock inlet/outlet state,
  blocking hazards, and integration obligations. It is the stable parent scope
  for Creator and Checker passes.
- **Micro-Feature** answers whether each requirement from the staged priors is
  assigned and eventually implemented. It is a traceability unit, not a test
  unit.

The xfstests matrix is deliberately separate from this hierarchy. A black-box
case may exercise several micro-features, and a micro-feature may need several
cases. Removing ktests therefore removes a validation mechanism, not the need
for ownership, lock, semantic-boundary, or traceability structure.

## Codex Skills

The reusable Codex entry points live at the repository root:

- `$ovfs-main`
  Use when acting as the scheduler for `overlayfs`: resuming the board, shaping waves, curating packets, updating `SYSTEM_BLUEPRINT.md`, or writing main-agent handoffs.
- `$ovfs-subagent`
  Use for bounded Architect, Designer, Creator, Checker, and Reviewer packets. Pass the role protocol explicitly; Architect packets own heavy-prior intake and static topology.
- `$ovfs-checker`
  Use for authorized overlayfs xfstests validation in `codex-asterinas-dev`, including artifact preservation and result classification.

## Role Model

- **Main agent:**
  Owns scheduling, protocol enforcement, acceptance, continuity, and the task board (`SYSTEM_BLUEPRINT.md`).
- **Architect (The Planner & System Mapper):**
  Defines the system by internalizing heavy priors. Produces the Global Static Lock Topology, the Bi-Directional Traceability Matrix (mapping all features/specs to the macro-meso-micro hierarchy), and establishes static lock boundaries.
- **Designer (The Dynamic Path & Lock Orchestrator):**
  Takes the Architect's static boundaries and dictates the dynamic execution process. Sets lock interaction contracts and path boundary restraints while also emitting pass-scoped xfstests mappings and meso-level integration obligations for upstream-approved validation lanes.
- **Creator (The Unconditional Executor):**
  Blindly follows the Designer's blueprints inside a main-agent-defined Creator Pass. Each pass names one parent meso-component and an explicit covered-micro set; the Creator implements only that slice and records it in the pass report.
- **Checker (The Validator & Condenser):**
  Validates behavior through upstream-approved external/system-level validation, evaluates preserved guest logs and suite results for execution evidence, and records actionable repair batches. Creator-synced Checker passes must mirror the Creator pass exactly for scope and report many-to-many xfstests coverage; meso-level integration validation is scheduled as a separate Checker-owned pass. This refactor must not create, modify, or grow any `#[ktest]`, `#[cfg(ktest)]`, kernel-mode test module, `test_support/`, memory-disk fixture, or other ktest harness anywhere in the repository. *Owns lock-guarded execution.*
- **Reviewer (The Quality Gate):**
  Performs static code-quality reviews on stabilized implementation passes and may directly edit in-scope code to enforce formatting, naming, and style priors before final acceptance.

## Directory Map

- `priors/ASTERINAS_INTEGRATION_PRIORS.md`: Asterinas-local architectural context.
- `priors/ASTERINAS_CODE_QUALITY_PRIORS.md`: Reusable code-quality guidance.
- `priors/FILESYSTEM_SPEC_SUMMARY.md`: Workspace-staged authoritative filesystem spec summary.
- `priors/REFERENCE_IMPLEMENTATION_SUMMARY.md`: Workspace-staged reference implementation summary.
- `priors/MICRO_FEATURE_INVENTORY.md`: Workspace-staged micro-feature decomposition.
- `PROTOCOL.md`: Main-agent-owned normative workflow.
- `PASS_SLICING.md`: Main-agent-owned pass-slicing ledger that records how meso-level Designer contracts are split into Creator, Checker, Reviewer, and integration passes.
- `protocol/`: Scoped documents forwarded to subagents (`ARCHITECT.md`, `DESIGNER.md`, `CREATOR.md`, `CHECKER.md`, `REVIEWER.md`).
- `subagent-tasks/`: Task packets grouped by `<component-id>`. Packets are lightweight dispatch stubs rather than heavy prose, avoiding context bloat and preventing Creator overreach.
- `components/<component-id>/`: Subagent artifacts and Checker execution receipts grouped by parent meso-component.
- Runtime helper scripts are intentionally not vendored in this workspace; use the top-level `ovfs-checker` command lane and record its evidence under the matching component directory.
- `SYSTEM_BLUEPRINT.md`: The scheduler-owned active global blueprint and traceability matrix.
- `protocol/templates/`: Required handoff formats.
- `main-agent/`: Main-agent checkpoint notes. Maintain one live handoff file per active tenure and update it in place.
