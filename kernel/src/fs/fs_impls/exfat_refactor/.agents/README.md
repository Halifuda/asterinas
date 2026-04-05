<!-- SPDX-License-Identifier: MPL-2.0 -->

# exFAT Refactor Multi-Agent Workspace

This directory stores the operating protocol for the parallel exFAT refactor module.

## Project Framing

This project is not just "implement exFAT."
It has two equally important goals:

1. Refactor the exFAT implementation into clearer, better-specified components.
2. Explore the practical automation boundary of LLM agents when building filesystem code without losing engineering control.

The main question is not only whether agents can produce code, but whether they can do filesystem engineering without losing control of specification coverage, implementation detail, style consistency, and bug rate.

The implementation strategy is to keep the legacy `exfat` module intact while building the refactored implementation in parallel under `exfat_refactor`.
That legacy baseline is a migration and regression reference, not the semantic target of the refactor.
For exFAT rules and design choices, Microsoft exFAT and the Linux exFAT implementation remain the primary authorities.
Asterinas-local precedent matters mainly for Rust interfaces, repository constraints, and integration boundaries.

The workflow is intentionally strict because a filesystem contains too many interacting details for current LLM agents to safely design and implement in one pass.
The process therefore depends on a closed loop:

1. Split the work into small, functionally coherent units.
2. Specify each component before implementation.
3. Implement only against the specification.
4. Check the result.
5. Feed defects back as bounded repair work.

The scheduler is intentionally loop-based rather than purely linear:

- one main-agent loop may launch one creator round, and that round may include multiple disjoint sibling creators in parallel,
- after that creator round starts, the remaining parallel budget in the same loop should go to architect, designer, reviewer, packet-preparation, or checker-preparation work,
- if a command-free delegated lane stalls, the preferred fix is to repair and continue delegation rather than collapsing that work back into the main thread.

## Role Model

- Main agent:
  - owns scheduling, protocol enforcement, acceptance, continuity, and the global task board.
- Architect:
  - defines functionally coherent architectural units, names their final owners, and proposes safe work-slice and parallel-wave boundaries.
- Designer:
  - turns one architected component into a bounded modular, functional, and concurrency-aware specification.
- Creator:
  - implements exactly one specified pass and must fully comply with the repository coding guidelines.
- Checker:
  - validates behavior, owns targeted test writing, and records executable evidence.
- Advisor:
  - turns checker findings into bounded repair work.

## Directory Map

- [`ASTERINAS_ARCHITECT_PRIORS.md`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_ARCHITECT_PRIORS.md) records the Asterinas-local architectural context that packets may slice by role.
- [`ASTERINAS_CODE_QUALITY_PRIORS.md`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_CODE_QUALITY_PRIORS.md) records the reusable code-quality guidance distilled from the repository `AGENTS.md` and the coding-guidelines book.
- [`linux-exFAT-implementation-summary.md`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/linux-exFAT-implementation-summary.md) is the Linux-side implementation map. It is intentionally a source index plus high-level summary, not a replacement for reading `/home/halifuda/linux/fs/exfat/` when exact behavior matters.
- When exact Linux behavior, sequencing, or boundary shape matters, packets may explicitly authorize direct reads from `/home/halifuda/linux/fs/exfat/`. The summary is a map and orientation aid, not a substitute for those packet-scoped source reads.
- [`TESTING_GUIDE.md`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/TESTING_GUIDE.md) records how exFAT ktests should be written, selected, and executed in the validated container workflow.
- [`PROTOCOL.md`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/PROTOCOL.md) is the main-agent-owned normative workflow and should not normally be forwarded to ordinary subagents.
- `protocol/` contains the scoped documents that should actually be forwarded to ordinary subagents:
  - `COMMON_SUBAGENT.md`
  - role-specific files such as `ARCHITECT.md`, `CREATOR.md`, and `CHECKER.md`
  - `TASK_PACKET_TEMPLATE.md` for per-task read or write scopes and stop conditions
  - these role-scoped files should restate any term or boundary rule that an ordinary subagent must understand; do not assume the subagent also received scheduler-only terminology from `PROTOCOL.md`
- `subagent-tasks/` stores the archived task packets that were actually sent to delegated subagents.
- `tools/` stores small workflow scripts owned by the main agent, including the checker execution-lock helper.
- [`COMPONENT_INDEX.md`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md) is the scheduler-owned task board.
- `templates/` contains the required handoff formats for each agent role.
- Component artifacts use chronological two-digit prefixes grouped by phase:
  - `00` architect
  - `01`-`03` designer split (`core`, `async`, `ktest`)
  - `10`-series serial implementation loop
  - `20`-series concurrency loop
  - `30`-series reviewer and optional final checker
- `templates/MAIN_AGENT_HANDOFF_TEMPLATE.md` is the checkpoint handoff format for cross-thread or cross-machine continuity.
- Main-agent handoff notes are living wave records, not end-only summaries. They should be updated throughout each wave and compacted as needed before handoff.
- Architect artifacts may recommend candidate work slices and overlap opportunities, but the active main-agent handoff is the scheduler-owned record of the currently active global work-slice matrix.
- `templates/REVIEWER_REPORT_TEMPLATE.md` is the dedicated code-quality review handoff format.
- `EXR-BOOT-01` and `EXR-IO-02` were created before the step-by-step handoff redesign. Their historical artifact names remain valid as legacy records, but new components should follow the phase-grouped naming scheme listed above in this README.
- Components that already use a single `01_designer_spec.md` remain valid legacy records. New components should use the split designer artifact set described in this README and the role templates.
- Main-agent checkpoint notes should use a memorable fancy nickname in the filename, following the pattern `<fancy-nickname>-YYYYMMDD-HHMM-<summary>.md`.

All agents working on `exfat_refactor` must follow both the repository-level `AGENTS.md` and this directory-local protocol.
