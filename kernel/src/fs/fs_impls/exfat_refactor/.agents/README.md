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

**Top-Down Strict Protocol**: Concurrency, locks, and system states are static and dynamic laws determined upfront by the Architect and Designer before the Creator writes a single line of code.

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
  Takes the Architect's static boundaries and dictates the dynamic execution process. Sets ironclad Lock Interaction Contracts and Path Boundary Restraints (e.g., non-blocking mandates).
- **Creator (The Unconditional Executor):**
  Blindly follows the Designer's blueprints. Focuses purely on Rust syntax, `Drop` semantics, and `?` early-returns without inventing unauthorized architectures or helpers.
- **Checker (The Validator & Condenser):**
  Validates behavior, owns targeted test writing, evaluates `qemu-serial.log` for execution evidence, and records actionable repair batches (condensing test failures directly into repair tasks). *Owns lock-guarded execution.*
- **Reviewer (The Quality Gate):**
  Performs static code-quality reviews and may directly edit in-scope code to enforce formatting, naming, and style priors before final acceptance.

## Directory Map

- `ASTERINAS_ARCHITECT_PRIORS.md`: Asterinas-local architectural context.
- `ASTERINAS_CODE_QUALITY_PRIORS.md`: Reusable code-quality guidance.
- `linux-exFAT-implementation-summary.md`: Linux-side implementation map. 
- *Note on Priors*: We use an **Information Funnel**. Heavy priors (Microsoft specs, Linux source) are internalized by the Architect. Designers internalize Architect outputs + Linux references. Creators only see the Designer's contract + Code Quality priors.
- `TESTING_GUIDE.md`: How exFAT ktests should be written and executed.
- `PROTOCOL.md`: Main-agent-owned normative workflow.
- `protocol/`: Scoped documents forwarded to subagents (`ARCHITECT.md`, `DESIGNER.md`, `CREATOR.md`, `CHECKER.md`, `REVIEWER.md`).
- `subagent-tasks/`: Task packets grouped by `<component-id>`. Packets are lightweight **Dispatch Stubs** rather than heavy prose, avoiding context bloat and preventing Creator overreach.
- `components/`: Subagent artifact outputs. Specs, evaluations, constraints are placed under exact `<component-id>` folders.
- `tools/`: Workflow scripts, including the checker execution-lock helper (`checker_lock.sh`).
- `SYSTEM_BLUEPRINT.md`: The scheduler-owned active global blueprint and traceability matrix.
- `priors/`: Prior knowledge layer containing heavy context separated for the strict information funnel.
- `protocol/templates/`: Required handoff formats. Component artifacts use a prefix scheme based on the architectural mapping level (e.g. `micro_03_write_at_creator.md`).
- `main-agent/`: Main-agent checkpoint notes follow the `YYYYMMDD-HHMM-nickname-summary_main_agent_handoff_TEMPLATE.md` convention.
