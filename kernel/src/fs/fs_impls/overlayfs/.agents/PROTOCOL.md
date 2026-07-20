<!-- SPDX-License-Identifier: MPL-2.0 -->

# Filesystem Implementation/Refactor Multi-Agent Protocol

This file is the main-agent-owned scheduler protocol for the `kernel/src/fs/fs_impls/overlayfs` workspace.
It defines what the main agent controls: delegation, gates, parallel scheduling, and acceptance.

Use the surrounding documents as follows:
- `README.md`: workspace map and project framing
- `PASS_SLICING.md`: durable main-agent-owned pass boundary decisions
- `$fs-main-agent`: preferred Codex entry point for main-agent tasks
- `$fs-subagent-workflow`: preferred Codex entry point for delegated subagent tasks
- `protocol/`: source-text role rules mirrored by the subagent skills

## 0. Core Terms

- **Final Owner**: The stable finished-system owner (VFS trait carrier, On-disk Structure Owner, daemon process, record type).
- **On-disk Structure Owner**: A Final Owner for one concrete durable filesystem structure or state machine, such as a superblock region, allocation map, block map, case-folding table, directory-entry set, stream or extent descriptor, or volume identity record. Use this full term; do not shorten it to an ambiguous generic phrase.
- **Macro-Owner**: The large-scale architectural entities that belong to the final-owner concept, including VFS trait carriers and On-disk Structure Owners (for example `Filesystem`, `Inode`, `AllocationMap`, `CaseFoldingTable`, `BlockMap`).
- **Meso-Component**: Explicit interfaces and primary structures mapped under a Macro-Owner.
- **Micro-Feature**: The specific functional details derived from prior knowledge.
- **Creator Pass**: A main-agent-defined implementation slice that sits between a Meso-Component and its Micro-Features. Each Creator Pass names exactly one parent meso-component and one explicit covered-micro set.
- **Checker Pass**: A validation slice. For implementation work it MUST mirror the Creator Pass exactly; meso-level integration validation is scheduled as an independent Checker-only pass with its own covered-micro set. New Checker validation MUST use upstream-approved external/system-level methods rather than adding kernel-local tests under `kernel/src/fs/fs_impls/`.
- **Global Lock Topology**: The absolute static hierarchy and holding states of synchronization primitives in the system.
- **Information Funnel & Dispatch Stubs**: Heavy priors are internalized by higher roles (Architect). Lower roles (Creator) receive minimal context via Dispatch Stubs to prevent architectural overreach.

## 1. Scheduler Rules

1. **Global Repository Boundary**: Every agent must obey the repository-level `AGENTS.md`; no `unsafe` is allowed in `kernel/src/fs/fs_impls/overlayfs/`. Components may depend only on accepted components or stable pre-existing kernel interfaces, and any staged pre-refactor filesystem module remains the active registered filesystem until explicitly scheduled for takeover. Any local Asterinas interface divergence from the authoritative specification or reference implementations must be recorded by the Architect or Designer artifact.
2. **Main-Agent Authority & Continuity**: The main agent is the only scheduler and the only role that changes official component state, `SYSTEM_BLUEPRINT.md`, or `PASS_SLICING.md`. The active main-agent thread must maintain exactly one live handoff record under `.agents/main-agent/`, update it for every material scheduling action / acceptance / rejection / escalation, and end each session with explicit next-main-agent tasks.
3. **Subagent Instantiation & Context Policy**: The main agent selects the model and reasoning effort for each delegated Architect, Designer, Creator, Checker, Reviewer, or lightweight triage task according to cost, risk, and required judgment. Subagents MUST NOT be spawned with forked main-thread context; dispatch packets carry the authorized context boundary. Results are protocol-valid only when the dispatch packet names the role, scope, and expected artifact authority clearly enough for the selected agent class.
4. **Pipeline Gates & Pass Slicing**: No component enters implementation before its Architect handoff and meso-scoped Designer artifacts (`_designer_spec.md` and a validation contract) exist. New Designer validation contracts MUST describe upstream-approved external/system-level validation obligations and MUST NOT request new `#[ktest]` coverage, `test_support/` modules, or other tests under `kernel/src/fs/fs_impls/`. The main agent decides every Creator Pass boundary, records it in `PASS_SLICING.md` before or with dispatch, and requires every Creator / Checker / Reviewer packet to name exactly one parent meso-component plus explicit covered micro-features. Architects and Designers stay exhaustive at the meso level and must not pre-slice implementation passes.
5. **Checker Synchronization & Integration Separation**: Every Creator Pass must have a matching Creator-synced Checker Pass with the same parent meso-component and covered micro-feature set; the main agent must not widen or narrow it. Meso-level integration validation from the Designer validation contract is never folded into Creator-synced Checker passes; it is scheduled as a separate Checker-owned pass only after the relevant implementation passes exist.
6. **Strict Information Funnel & Artifact Layout**: Packets MUST be saved under `subagent-tasks/<component-id>/`, use `protocol/templates/[level]_[XX]_[component]_[role]_dispatch_TEMPLATE.md`, and remain pointer routes rather than design summaries. Subagent artifacts MUST be written under `.agents/components/<component-id>/`; mixed artifact directories are forbidden. Allowed context by role:
   - **Architect**: workspace-staged filesystem spec summaries, reference implementation summaries, micro-feature inventories, and relevant Asterinas priors.
   - **Designer**: accepted Architect topology plus local component context.
   - **Creator**: Designer contract, main-agent-selected covered micro set, `priors/ASTERINAS_CODE_QUALITY_PRIORS.md`, and only stable pre-existing kernel interfaces required to typecheck. NEVER provide heavy filesystem specs, reference implementation source dumps, or legacy implementation files to Creator.
   - **Checker (Creator-Synced)**: Designer validation contract, the matching Creator Pass report, and pass write-set / code paths.
   - **Checker (Meso Integration)**: Designer validation contract, accepted Creator reports covering the target micro-features, and pass write-set / code paths.
7. **Template Acceptance Is Structural**: Main-agent acceptance is structural, not logical. Subagent artifacts MUST fully populate their required templates; omitted, conceptually empty, or wrong-destination sections are protocol violations and must be rejected.
8. **Read-Only LSIF Navigation Tooling**: All roles should use the `ra-code-nav` skill (see `.agents/skills/ra-code-nav/SKILL.md` at the repository root, or the workspace-level `.agents/skills/ra-code-nav/`) for packet-scoped read-only Rust navigation whenever they need symbol-aware lookup. The skill queries a pre-generated rust-analyzer LSIF index with shell + `jq`; no Python LSP client is involved. Dispatch packets should remind agents that this is the preferred rust-analyzer tool for scoped code navigation. It does not authorize reading outside packet scope and does not replace required role artifacts.
9. **Command Lane & Checker Evidence**: Main agent, Architect, Designer, and Reviewer must not run kernel build/test commands; Creators are command-free unless explicitly overridden; Checkers own runtime verification by default. New Checker work MUST NOT add kernel-local `#[ktest]` tests, `test_support/` trees, or other test code under `kernel/src/fs/fs_impls/`. Checker execution is lock-guarded and must preserve guest logs plus validation result files before they can be overwritten.
10. **Repair Loop & Escalation**: Checkers analyze failures directly and condense them into actionable repair batches. The main agent is a blind router: it must preserve the Checker reproduce command, failed test, and evidence verbatim; it must not reinterpret diagnostics. Creator-synced failures route to the same Creator Pass, integration failures route to reopened Creator Passes or upward escalation, and any loop that fails five times without a passing upstream-approved validation receipt must be halted and escalated upward.
11. **Parallel Scheduling Discipline**: Command-free Architect / Designer / Reviewer / Creator lanes should proceed in parallel whenever dependencies and write-sets allow, even while the checker execution lock is held. If a delegated command-free lane stalls, the main agent should repair and re-delegate instead of absorbing the work locally.
12. **Cleanup Scope & User-Named Surfaces**: Structural cleanup packets must enumerate each targeted objective separately; Creator and Reviewer artifacts must disposition each objective and the main agent must not infer closure while any named objective remains open.
13. **Entity Census & Full-Surface Audits**: Every Creator artifact MUST census all newly introduced production entities in its write-set: `struct`, `enum`, local type alias, module, and non-trait helper function. Trait-required methods may be grouped under their impl block; test-only entities must appear separately.
14. **Carrier, Seam, And Error Defaults**: Temporary or helper-local carriers are rejected by default unless strongly proven. Owner-seams and temporary error seams must either be promoted / localized or carry a precise exit plan naming the future owner, trigger, and seam to remove.
15. **Helper & Thin-Wrapper Defaults**: Top-level helpers and helper families are rejected by default when their parameters or body revolve around one owner; they must become owner-private methods or be inlined unless they are stable meso entries, forced by a trait / registration API, or genuinely cross multiple owners.
16. **Reviewer Authority & Final Checker**: Reviewer normally runs after Checker; an extra pre-checker structural audit is allowed only when explicitly requested by the user or main agent and does not replace the ordinary post-checker Reviewer gate. Reviewer direct edits are limited to line-level non-functional changes that preserve behavior and topology; structural findings reopen Creator cleanup rather than being rewritten by Reviewer.
17. **Test-Code Boundary**: New validation must not add or grow test code under `kernel/src/fs/fs_impls/`. If upstream-approved validation requires new test harness code, it belongs outside the filesystem implementation tree, preferably in the xfstests lane or another upstream-standard location.

## 2. Role Ownership

- **Main agent**: Owns scheduling, acceptance, packet curation, task-board updates, and stale-lock decisions.
- **Architect**: Generates the Bi-Directional Traceability Matrix across the Macro/Meso/Micro hierarchy, defines Global Lock Topology, and establishes static boundaries for micro-features.
- **Designer**: Translates static boundaries into dynamic execution paths. Emits one meso-level spec and one meso-level validation contract containing Creator-synced and integration validation obligations for upstream-approved validation lanes.
- **Creator**: Translates the Designer's blueprints into Rust implementations one Creator Pass at a time, as sliced by the main agent.
- **Checker**: Validates behavior in synchronized Creator/Checker passes, owns independent meso-level integration passes, evaluates preserved guest logs and upstream-suite results for runtime failures, and directly reports actionable repair instructions.
- **Reviewer**: Performs post-checker static review on stabilized implementation passes, enforcing both line-level code quality and structural helper / owner-placement quality.

## 3. Workflow Gates

The normal component path is:

```text
Planned -> Architected -> Specified
  -> One or more creator/checker pass loops
  -> Independent meso integration checker pass(es)
  -> Reviewer
  -> Optional final checker
  -> Accepted
```

Gate rules:
1. `Architected` means the Traceability Matrix and Static Lock Topology are established.
2. `Specified` means all Designer artifacts exist, explicitly covering dynamic lock behavior, Creator-synced validation obligations, and meso-level integration validation obligations.
3. The main agent must declare Creator Pass coverage before any implementation starts.
4. Creator-synced Checker passes must validate only the covered micro set of their matching Creator Pass.
5. Reviewer evaluates static code quality only after implementation and runtime validation stabilize.
6. `Accepted` requires full micro-feature coverage across passes, no blocking logic/quality findings, and verified execution evidence for the required upstream-approved validation batches.

## 4. Parallel Scheduling Model

Think in terms of one serialized command lane (Checker execution) plus as many safe command-free lanes as dependencies allow.

1. Keep the command lane narrow. Only actual execution logic requires the lock.
2. Keep others moving. While Checker holds the lock, Architect, Designer, Reviewer, and Creator lanes proceed if write-sets are disjoint.
3. The task assignment structure controls context size naturally.
4. Compile-only Creator exceptions are rare and consume the shared command environment.
