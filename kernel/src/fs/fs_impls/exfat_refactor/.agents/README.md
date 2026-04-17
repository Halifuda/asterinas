<!-- SPDX-License-Identifier: MPL-2.0 -->

# exFAT Refactor Multi-Agent Workspace

This directory stores the operating protocol for the parallel exFAT refactor module.

## Project Framing

This project is not just "implement exFAT."
It has two equally important goals:

1. Refactor the exFAT implementation into clearer, better-specified components.
2. Explore the practical automation boundary of LLM agents when building filesystem code without losing engineering control.

The main question is whether agents can do filesystem engineering without losing control of specification coverage, implementation detail, style consistency, and bug rate.

The implementation strategy keeps the `exfat` module intact while building the refactored implementation in parallel under `exfat_refactor`. Microsoft exFAT and the Linux exFAT implementation remain the primary authorities.

**Top-Down Strict Protocol**: Concurrency, locks, and system states are static and dynamic laws determined upfront by the Architect and Designer before the Creator writes a single line of code. Architect and Designer artifacts stay at the Meso level; the main agent later slices that Meso contract into one or more implementation passes for the Creator and synchronized Checker.

## Codex Skills

Two reusable Codex skills mirror the stable workflow rules for this workspace:

- `$exfat-main-agent`
  Use when acting as the scheduler for `exfat_refactor`: resuming the board, shaping waves, curating packets, updating `SYSTEM_BLUEPRINT.md`, or writing main-agent handoffs.
- `$exfat-subagent-workflow`
  Use for ordinary delegated architect, designer, creator, checker, and reviewer work.

## Role Model

- **Main agent:**
  Owns scheduling, protocol enforcement, acceptance, continuity, and the 4-part task board (`SYSTEM_BLUEPRINT.md`).
- **Architect (The Planner & System Mapper):**
  Defines the system by internalizing heavy priors. Produces the Global Static Lock Topology, the Bi-Directional Traceability Matrix (mapping all features/specs to the macro-meso-micro hierarchy), and establishes static lock boundaries.
- **Designer (The Dynamic Path & Lock Orchestrator):**
  Takes the Architect's static boundaries and dictates the dynamic execution process. Sets ironclad Lock Interaction Contracts and Path Boundary Restraints (e.g., non-blocking mandates), while also emitting both unit-test obligations and meso-level integration-test obligations.
- **Creator (The Unconditional Executor):**
  Blindly follows the Designer's blueprints inside a main-agent-defined Creator Pass. Each pass names one parent meso-component and an explicit covered-micro set; the Creator implements only that slice and records it in the pass report.
- **Checker (The Validator & Condenser):**
  Validates behavior, owns targeted test writing, evaluates `qemu-serial.log` for execution evidence, and records actionable repair batches. Creator-synced Checker passes must mirror the Creator pass exactly; meso-level integration testing is scheduled as a separate Checker-owned pass. *Owns lock-guarded execution.*
- **Reviewer (The Quality Gate):**
  Performs static code-quality reviews on stabilized implementation passes and may directly edit in-scope code to enforce formatting, naming, and style priors before final acceptance.

## Directory Map

- `ASTERINAS_ARCHITECT_PRIORS.md`: Asterinas-local architectural context.
- `ASTERINAS_CODE_QUALITY_PRIORS.md`: Reusable code-quality guidance.
- `linux-exFAT-implementation-summary.md`: Linux-side implementation map. 
- *Note on Priors*: We use an **Information Funnel**. Heavy priors (Microsoft specs, Linux source) are internalized by the Architect. Designers internalize Architect outputs + Linux references. Creators only see the Designer's contract, the main-agent-selected pass coverage, and Code Quality priors. Checkers see the Designer test contract plus the relevant Creator pass receipts.
- `TESTING_GUIDE.md`: How exFAT ktests should be written and executed.
- `PROTOCOL.md`: Main-agent-owned normative workflow.
- `protocol/`: Scoped documents forwarded to subagents (`ARCHITECT.md`, `DESIGNER.md`, `CREATOR.md`, `CHECKER.md`, `REVIEWER.md`).
- `subagent-tasks/`: Task packets grouped by `<component-id>`. Packets are lightweight **Dispatch Stubs** rather than heavy prose, avoiding context bloat and preventing Creator overreach.
- `components/`: Subagent artifact outputs. Specs, evaluations, constraints are placed under exact `<component-id>` folders.
- `tools/`: Workflow scripts, including the checker execution-lock helper (`checker_lock.sh`).
- `SYSTEM_BLUEPRINT.md`: The scheduler-owned active global blueprint and traceability matrix.
- `priors/`: Prior knowledge layer containing heavy context separated for the strict information funnel.
- `protocol/templates/`: Required handoff formats. Component artifacts use a prefix scheme based on the architectural mapping level (e.g. `pass_03_write_at_creator.md` for an implementation pass under a meso component).
- `main-agent/`: Main-agent checkpoint notes follow the `YYYYMMDD-HHMM-nickname-summary_main_agent_handoff_TEMPLATE.md` convention. A single main-agent tenure should maintain one live handoff file and update it in place rather than creating a new file for every short session.
