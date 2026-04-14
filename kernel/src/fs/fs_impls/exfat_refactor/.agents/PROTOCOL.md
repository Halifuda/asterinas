<!-- SPDX-License-Identifier: MPL-2.0 -->

# exFAT Refactor Multi-Agent Protocol

This file is the main-agent-owned scheduler protocol for the `exfat_refactor` workspace.
It defines what the main agent controls: delegation, gates, parallel scheduling, and acceptance.

Use the surrounding documents as follows:
- `README.md`: workspace map and project framing
- `$exfat-main-agent`: preferred Codex entry point for main-agent tasks
- `$exfat-subagent-workflow`: preferred Codex entry point for delegated subagent tasks
- `protocol/`: source-text role rules mirrored by the subagent skill

## 0. Core Terms

- **Final Owner**: The stable finished-system owner (VFS trait carrier, structure owner, daemon process, record type).
- **Macro-Owner**: The large-scale architectural entities that belong to the final-owner concept (e.g., `ExfatFs`, `ExfatInode`, `FatChain`).
- **Meso-Component**: Explicit interfaces and primary structures mapped under a Macro-Owner (e.g., `write_at`, `resize`).
- **Micro-Feature**: The specific functional details derived from prior knowledge (e.g., file write zero-fill gaps, allocation cluster counting, timestamp updates).
- **Global Lock Topology**: The absolute static hierarchy and holding states of synchronization primitives in the system.
- **Information Funnel & Dispatch Stubs**: Heavy priors are internalized by higher roles (Architect). Lower roles (Creator) receive minimal context via Dispatch Stubs to prevent architectural overreach.

## 1. Scheduler Rules

1. Every agent must obey the repository-level `AGENTS.md`. No `unsafe` in `kernel/src/fs/fs_impls/exfat_refactor/`.
2. The main agent is the only scheduler. Only the main agent changes official component state or the `SYSTEM_BLUEPRINT.md`.
3. No component enters implementation before its Architect handoff and Designer artifacts exist. Designer artifacts merge the logic specification and dynamic lock contracts into a single comprehensive spec file (`_designer_spec.md`), alongside `_ktest.md`.
4. **Strict Acceptance via Templates**: Main-Agent acceptance is purely structural, not logical. Subagent artifacts MUST fully populate their corresponding `protocols/templates/...` structures. If a template section (e.g., Designer Rely-Guarantee locks, or Creator Whitelist Rules) is omitted or conceptually empty, the Main-Agent must reject it outright for protocol violation.
5. Components may depend only on accepted components or stable pre-existing kernel interfaces.
6. The `exfat` module remains the active registered filesystem until explicitly scheduled for takeover.
7. Test authoring is Checker-owned by default. Creators should ignore test-writing unless the packet overrides this.
8. Role command authority: Main agent, Architect, Designer, Reviewer must not run kernel build/test commands. Creators are command-free. Checkers own runtime verification commands.
9. **Checker execution is lock-guarded**:
   - Before execution (build/test/QEMU), Checker MUST use `tools/checker_lock.sh acquire`.
   - If locked, Checker waits quietly and retries (minimum `60` seconds).
   - After completion, Checker MUST release the lock via `tools/checker_lock.sh release`.
   - Only the main agent may clear a stale lock.
10. **Exact-Name Proof Obligation**: Filtered verification commands must prove they targeted intended tests. A green exit status alone is insufficient when `cargo osdk test <filter>` is used. The Checker must record an exact, uniquely justified test-path suffix or output that explicitly names executed tests.
11. **Evaluating qemu-serial.log**: Checkers MUST examine `qemu-serial.log` (or other execution traces) for panics, TCG errors, or deadlocks. Exit codes are not enough. 
12. **Integrated Repair & Blind Passthrough**: Checkers analyze failures directly and condense them into actionable repair batches for Creators. The main agent acts strictly as a router here; it MUST NOT interpret tests or instruct the Creator. The main agent passes the Checker's batch directly to the Creator.
13. **Escalation Path (Max Retries = 5)**: If the Creator-Checker repair loop cycles 5 times without producing a passing exact-name test receipt, the main agent MUST halt the loop and package the impasse to escalate upwards (e.g., sending the deadlock/crash history back to DESIGNER to re-evaluate lock constraints).
14. Command-free work (Architect, Designer, Reviewer, Creator passes) should fill parallel lanes whenever dependency graphs and write-sets allow, even while the checker execution lock is held.
15. If a delegated command-free lane stalls, the main agent should repair and re-delegate instead of absorbing the work into the main thread.
16. **Strict Information Funnel**: Packets MUST be saved in `subagent-tasks/<component-id>/` and MUST use the `protocol/templates/[level]_[XX]_[component]_[role]_dispatch_TEMPLATE.md`. The main agent MUST NOT write design summaries or architectural hints in the packet. The packet is purely a pointer route (File Paths) to the input files and the output template.
    - **To Architect**: Inputs = `priors/Microsoft-exFAT-spec.md`, `priors/linux-exFAT-implementation-summary.md` and `priors/ASTERINAS_ARCHITECT_PRIORS.md`.
    - **To Designer**: Inputs = Architect topology and local component context.
    - **To Creator**: Inputs = Designer's contract spec and `priors/ASTERINAS_CODE_QUALITY_PRIORS.md`. NEVER supply heavy exFAT specs or Linux code to Creator.
    - **To Checker**: Inputs = Designer spec (for test obligations) and Creator's implementation map.
17. If local Asterinas interfaces force a divergence from Microsoft or Linux behavior, the Architect or Designer artifact must record that explicitly.
18. Reviewer owns broad static code-quality. The Reviewer may directly edit in-scope code to enforce formatting, naming, and style constraints. Reviewer runs after the implementation/checker loops stabilize.
19. Post-review final checker is conditional. The main agent may skip it only if the Reviewer explicitly records that edits were non-functional only.
20. **Continuous Main-Agent Handoff**: The active main-agent thread MUST maintain a continuous handoff record in `.agents/main-agent/` using `protocol/templates/YYYYMMDD-HHMM-nickname-summary_main_agent_handoff_TEMPLATE.md`. Every material scheduling action, template acceptance/rejection, and escalation MUST be reflected here during the active wave. Before closing a session, the handoff MUST conclude with explicit next-main-agent tasks.
21. **Artifact Directory Structure**: Subagent produced artifacts (specs, code analysis, checker reports) MUST be placed under `.agents/components/<component-id>/`. Mixed directories are not allowed.

## 2. Role Ownership

- **Main agent**: Owns scheduling, acceptance, packet curation (Dispatch Stubs), task-board (`SYSTEM_BLUEPRINT.md`), lock-stale decisions.
- **Architect**: Generates the Bi-Directional Traceability Matrix across the Macro/Meso/Micro hierarchy, defines Global Lock Topology, and establishes Static Boundaries for micro-features.
- **Designer**: Translates static boundaries into dynamic execution paths. Merges logic spec and Lock Interaction Contracts into unified documents.
- **Creator**: Translates the Designer's blueprints into Rust implementations.
- **Checker**: Validates behavior, writes targeted ktests, evaluates `qemu-serial.log` for runtime panics, and directly reports actionable repair instructions to the Creator. *Holds the strict lock-guarded execution lane.*
- **Reviewer**: Performs static code-quality review, directly edits code for style compliance, but does not own runtime verification.

## 3. Workflow Gates

The normal component path is:

```text
Planned -> Architected -> Specified
  -> Implementation creator/checker loop
  -> Reviewer
  -> Optional final checker
  -> Accepted
```

Gate rules:
1. `Architected` means the Traceability Matrix and Static Lock Topology are established.
2. `Specified` means ALL Designer artifacts exist, explicitly covering dynamic lock behavior and non-blocking rules.
3. Safety constraints are implemented synchronously in the main Implementation loop.
4. Reviewer evaluates static code quality before final integration.
5. `Accepted` requires no blocking logic/quality findings and verified exact-name ktest execution.

## 4. Parallel Scheduling Model

Think in terms of one serialized command lane (Checker execution) plus as many safe command-free lanes as dependencies allow.

1. Keep the command lane narrow. Only actual execution logic requires the lock.
2. Keep others moving. While Checker holds the lock, Architect, Designer, Reviewer, and Creator lanes proceed if write-sets are disjoint.
3. The task assignment structure controls context size naturally.
4. Compile-only Creator exceptions are rare and consume the shared command environment.

### Conceptual Best-Effort Wave Example

Target workflow (concurrently active):

```text
command lane:
  A checker execution (holding lock)

command-free lanes:
  A checker pre-execution test scripting
  B creator (disjoint transaction)
  C designer (writing lock interaction contracts)
  D architect (mapping heavy specs to topology)
```
The workflow has one serialized execution lane but enables massive multi-phase concurrency.
