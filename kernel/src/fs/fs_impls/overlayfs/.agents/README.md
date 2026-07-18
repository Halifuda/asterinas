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

**Top-Down Strict Protocol**: Concurrency, locks, and system states are static and dynamic laws determined upfront by the Architect and Designer before the Creator writes a single line of code. Architect and Designer artifacts stay at the Meso level; the main agent later slices that Meso contract into one or more implementation passes for the Creator and synchronized Checker.

## Codex Skills

Three reusable Codex skills mirror the stable workflow rules for this workspace:

- `$fs-main-agent`
  Use when acting as the scheduler for `overlayfs`: resuming the board, shaping waves, curating packets, updating `SYSTEM_BLUEPRINT.md`, or writing main-agent handoffs.
- `$fs-subagent-workflow`
  Use for ordinary delegated architect, designer, creator, checker, and reviewer work.
- `$fs-architect-agent`
  Use for Architect packets that own the heavy-prior intake and the static topology.

## Role Model

- **Main agent:**
  Owns scheduling, protocol enforcement, acceptance, continuity, and the task board (`SYSTEM_BLUEPRINT.md`).
- **Architect (The Planner & System Mapper):**
  Defines the system by internalizing heavy priors. Produces the Global Static Lock Topology, the Bi-Directional Traceability Matrix (mapping all features/specs to the macro-meso-micro hierarchy), and establishes static lock boundaries.
- **Designer (The Dynamic Path & Lock Orchestrator):**
  Takes the Architect's static boundaries and dictates the dynamic execution process. Sets lock interaction contracts and path boundary restraints while also emitting Creator-synced and meso-level integration validation obligations for upstream-approved validation lanes.
- **Creator (The Unconditional Executor):**
  Blindly follows the Designer's blueprints inside a main-agent-defined Creator Pass. Each pass names one parent meso-component and an explicit covered-micro set; the Creator implements only that slice and records it in the pass report.
- **Checker (The Validator & Condenser):**
  Validates behavior through upstream-approved external/system-level validation, evaluates preserved guest logs and suite results for execution evidence, and records actionable repair batches. Creator-synced Checker passes must mirror the Creator pass exactly; meso-level integration validation is scheduled as a separate Checker-owned pass. New Checker work must not add filesystem-local ktests or `test_support/` under `kernel/src/fs/fs_impls/`. *Owns lock-guarded execution.*
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
- `components/`: Subagent artifact outputs. Specs, evaluations, constraints are placed under exact `<component-id>` folders.
- `checker-runs/`: Checker execution receipts grouped by parent meso-component.
- `tools/`: Workflow helpers expected by the protocol; see `tools/README.md`.
- `SYSTEM_BLUEPRINT.md`: The scheduler-owned active global blueprint and traceability matrix.
- `protocol/templates/`: Required handoff formats.
- `main-agent/`: Main-agent checkpoint notes. Maintain one live handoff file per active tenure and update it in place.
