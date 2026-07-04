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
  Takes the Architect's static boundaries and dictates the dynamic execution process. Sets ironclad Lock Interaction Contracts and Path Boundary Restraints (e.g., non-blocking mandates), while also emitting Creator-synced and meso-level integration validation obligations for upstream-approved validation lanes.
- **Creator (The Unconditional Executor):**
  Blindly follows the Designer's blueprints inside a main-agent-defined Creator Pass. Each pass names one parent meso-component and an explicit covered-micro set; the Creator implements only that slice and records it in the pass report.
- **Checker (The Validator & Condenser):**
  Validates behavior through upstream-approved external/system-level validation, evaluates preserved guest logs and suite results for execution evidence, and records actionable repair batches. Creator-synced Checker passes must mirror the Creator pass exactly; meso-level integration validation is scheduled as a separate Checker-owned pass. New Checker work must not add filesystem-local ktests or `test_support/` under `kernel/src/fs/fs_impls/`. *Owns lock-guarded execution.*
- **Reviewer (The Quality Gate):**
  Performs static code-quality reviews on stabilized implementation passes and may directly edit in-scope code to enforce formatting, naming, and style priors before final acceptance.

## Directory Map

- `ASTERINAS_ARCHITECT_PRIORS.md`: Asterinas-local architectural context.
- `ASTERINAS_CODE_QUALITY_PRIORS.md`: Reusable code-quality guidance.
- `linux-exFAT-implementation-summary.md`: Linux-side implementation map. 
- *Note on Priors*: We use an **Information Funnel**. Heavy priors (Microsoft specs, Linux source) are internalized by the Architect. Designers internalize Architect outputs + Linux references. Creators only see the Designer's contract, the main-agent-selected pass coverage, and Code Quality priors. Checkers see the Designer validation contract plus the relevant Creator pass receipts.
- `TESTING_GUIDE.md`: Legacy testing note retained only for historical context; new exFAT refactor validation must use upstream-approved external/system-level methods, currently expected to be NixOS xfstests.
- `XFSTEST_GUIDELINES.md`: Current main-agent guide for migrating `exfat_refactor` validation onto upstream's initramfs xfstests conformance lane.
- `PROTOCOL.md`: Main-agent-owned normative workflow.
- `PASS_SLICING.md`: Main-agent-owned pass-slicing ledger that records how meso-level Designer contracts are split into Creator, Checker, Reviewer, and integration passes.
- `protocol/`: Scoped documents forwarded to subagents (`ARCHITECT.md`, `DESIGNER.md`, `CREATOR.md`, `CHECKER.md`, `REVIEWER.md`).
- `protocol/XFSTESTS_LIGHTWEIGHT_TRIAGE.md`: Temporary low-cost xfstests triage layer. It produces evidence receipts only and does not replace formal Checker acceptance.
- `subagent-tasks/`: Task packets grouped by `<component-id>`. Packets are lightweight **Dispatch Stubs** rather than heavy prose, avoiding context bloat and preventing Creator overreach.
- `components/`: Subagent artifact outputs. Specs, evaluations, constraints are placed under exact `<component-id>` folders.
- `checker-runs/`: Checker execution receipts grouped by parent meso-component (`checker-runs/<meso-component>/...`).
- `tools/`: Workflow scripts. `checker_lock.sh` is the low-level checker execution lock, `checker_run.sh` is the existing Checker compile/build runner and may be wrapped or extended for upstream-approved validation lanes; `ra_code_nav.py` is the preferred rust-analyzer code-navigation helper for symbol search, file symbols, definitions, references, implementations, and hover output.
- `SYSTEM_BLUEPRINT.md`: The scheduler-owned active global blueprint and traceability matrix.
- `priors/`: Prior knowledge layer containing heavy context separated for the strict information funnel.
- `protocol/templates/`: Required handoff formats. Component artifacts use a prefix scheme based on the architectural mapping level (e.g. `pass_03_write_at_creator.md` for an implementation pass under a meso component).
- `main-agent/`: Main-agent checkpoint notes follow the `YYYYMMDD-HHMM-nickname-summary_main_agent_handoff_TEMPLATE.md` convention. A single main-agent tenure should maintain one live handoff file and update it in place rather than creating a new file for every short session.
