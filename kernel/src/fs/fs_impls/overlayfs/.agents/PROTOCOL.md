<!-- SPDX-License-Identifier: MPL-2.0 -->

# Filesystem Implementation/Refactor Multi-Agent Protocol

This file is the main-agent-owned scheduler protocol for the `kernel/src/fs/fs_impls/overlayfs` workspace.
It defines what the main agent controls: delegation, gates, parallel scheduling, and acceptance.

Use the surrounding documents as follows:
- `README.md`: workspace map and project framing
- `PASS_SLICING.md`: durable main-agent-owned pass boundary decisions
- `$ovfs-main`: preferred Codex entry point for main-agent tasks
- `$ovfs-subagent`: preferred Codex entry point for delegated Architect, Designer, Creator, Checker, and Reviewer tasks
- `$ovfs-checker`: preferred Codex entry point for authorized overlayfs xfstests validation
- `protocol/`: source-text role rules mirrored by the subagent skills

## 0. Core Terms

- **Final Owner**: The stable finished-system owner (VFS trait carrier, On-disk Structure Owner, daemon process, record type).
- **On-disk Structure Owner**: A Final Owner for one concrete durable filesystem structure or state machine, such as a superblock region, allocation map, block map, case-folding table, directory-entry set, stream or extent descriptor, or volume identity record. Use this full term; do not shorten it to an ambiguous generic phrase.
- **Macro-Owner**: The large-scale architectural entities that belong to the final-owner concept, including VFS trait carriers and On-disk Structure Owners (for example `Filesystem`, `Inode`, `AllocationMap`, `CaseFoldingTable`, `BlockMap`).
- **Meso-Component**: Explicit interfaces and primary structures mapped under a Macro-Owner.
- **Micro-Feature**: The specific functional details derived from prior knowledge.
- **External Validation Mapping**: A many-to-many mapping from micro-features to
  upstream test cases or groups. It records what xfstests exercises, but does
  not claim that one black-box test isolates one micro-feature.
- **Creator Pass**: A main-agent-defined implementation slice that sits between a Meso-Component and its Micro-Features. Each Creator Pass names exactly one parent meso-component and one explicit covered-micro set.
- **Checker Pass**: A validation slice. For implementation work it MUST mirror the Creator Pass exactly; meso-level integration validation is scheduled as an independent Checker-only pass with its own covered-micro set. This refactor uses xfstests as its only validation method and must not create, modify, or grow any ktest-based validation surface.
- **Global Lock Topology**: The absolute static hierarchy and holding states of synchronization primitives in the system.
- **Information Funnel & Dispatch Stubs**: Heavy priors and the workspace design documents are internalized by the higher design roles (Architect, Designer). Lower roles (Creator) receive minimal context via Dispatch Stubs to prevent architectural overreach.
- **Task**: A durable work boundary with a stable `task_id`, role, task kind,
  risk tier, scope, write-set, acceptance rule, and escalation rule.
- **Task Kind**: An operational classification orthogonal to role: `design`,
  `implementation`, `diagnosis`, `validation`, or `review`. It does not replace
  the Architect/Designer/Creator/Checker/Reviewer role model.
- **Continuation Event**: A recorded repair, revision, rerun decision, or
  follow-up under an existing task. It may reuse the task boundary when the
  write-set, contract, owner/lock boundary, and objective remain stable.
- **Validation Run**: One isolated compile, runtime, or upstream-suite
  execution under a validation task. Each run has a unique `run_id`, exact
  command, selected tests, status, and preserved evidence.
- **Risk Tier**: `Low`, `Normal`, or `High` classification selected by the
  main agent. It controls receipt depth, but never removes the mandatory
  scope, evidence, lock, or xfstests-only floors below.
- **Receipt**: The task's durable summary and pointer to its evidence. A
  receipt may aggregate several continuation events or validation runs without
  copying every event log into the current status file.

## 0.5 Adopted Workflow: Design Documents as the Design Root

The original protocol intent was an agent-fully-responsible pipeline that
minimizes human intervention from Architect to Reviewer. This overlayfs wave
deliberately departs from that intent: it is **design-document-driven**. The
workspace design documents under `.agents/designdoc/` (the Stage A-F drafts and
any future synthesis) are the authoritative design source, produced and
confirmed interactively with the user. The design documents are already
exhaustive at the semantic level, so the Designer's value-add is the **Rust
code-form mapping**: turning the design documents and the accepted Architect
topology into concrete meso-level Rust structure — module layout, structs,
enums, carrier types, and helper signatures. This amends every clause below
that treated Designer output as purely semantic or forbade signature design.

Because the Designer now designs signatures, all signature/type/helper output
MUST follow the Asterinas coding guidelines:
`priors/ASTERINAS_CODE_QUALITY_PRIORS.md` (naming, visibility, error handling,
types/functions, helper and owner-placement rules) and
`book/src/to-contribute/coding-guidelines/` (five personas; the
for-maintainability and for-development indexes are the primary checklists for
signature shape, naming, layout, and concurrency).

## 1. Scheduler Rules


### 1.1 Task Lifecycle and Evidence Model

The role pipeline remains authoritative, while task kinds provide a smaller
operational vocabulary for dispatch and receipts:

```text
task
  -> continuation events
  -> validation runs
  -> bounded review
  -> accepted
```

- A `design` task may cover an Architect handoff or a bounded Designer
  revision. A Designer revision may substantially rewrite its two Meso
  artifacts, but it must preserve the parent Meso, covered Micro set, and
  accepted static topology. A static owner or lock-topology defect is routed
  back to a bounded Architect repair instead of being silently changed by the
  Designer.
- An `implementation` task is a Creator Pass. A `validation` task is a
  Creator-synced or Meso-Integration Checker Pass. A `diagnosis` task is
  lightweight localization or evidence work and need not mirror a Creator
  Micro set. A `review` task is a post-Checker Reviewer Pass or an explicitly
  packeted bounded review wave.
- A rerun, suffix run, compile preflight, or same-Creator repair is normally a
  continuation event or Validation Run, not a new formal pass. Create a new
  task or reopen the implementation boundary when the write-set, design
  contract, owner/lock/persistence boundary, validation objective, or risk
  tier materially changes.
- Every task has one compact manifest containing at least:
  `task_id`, `role`, `task_kind`, `risk`, `workspace_root`, `scope`,
  `write_set`, `allowed_inputs`, `capabilities`, `acceptance`, `escalation`,
  and `expected_outputs`. Dispatch stubs may expose role-specific views of
  this manifest, but they must not duplicate full design artifacts or run
  evidence.
- Risk tiers are advisory depth selectors. `Low` is for bounded work with no
  new production entities, owner/lock/persistence change, or ambiguous
  behavior; `Normal` is ordinary implementation or repair; `High` covers new
  locks, copy-up, whiteout/opaque visibility, cross-owner operations,
  PageCache, persistence order, rollback, recovery, or permission/credential
  propagation. All implementation and validation tasks still retain their
  required scope and evidence floors.
- `SYSTEM_BLUEPRINT.md`, `PASS_SLICING.md`, and the current handoff retain
  current state, durable decisions, and pointers. Continuation events and raw
  validation runs belong in the task/component evidence area rather than being
  copied in full into those status files.

1. **Global Repository Boundary**: Every agent must obey the repository-level `AGENTS.md`; no `unsafe` is allowed in `kernel/src/fs/fs_impls/overlayfs/`. Components may depend only on accepted components or stable pre-existing kernel interfaces, and any staged pre-refactor filesystem module remains the active registered filesystem until explicitly scheduled for takeover. Any local Asterinas interface divergence from the authoritative specification or reference implementations must be recorded by the Architect or Designer artifact.
2. **Main-Agent Authority & Continuity**: The main agent is the only scheduler and the only role that changes official component state, `SYSTEM_BLUEPRINT.md`, or `PASS_SLICING.md`. The active main-agent thread must maintain exactly one live handoff record under `.agents/main-agent/`, update it for every material scheduling action / acceptance / rejection / escalation, and end each session with explicit next-main-agent tasks. Handoffs are compact snapshots; continuation events and raw validation runs are referenced rather than duplicated in full.
3. **Subagent Instantiation & Context Policy**: The main agent selects the model and reasoning effort for each delegated Architect, Designer, Creator, Checker, Reviewer, or lightweight triage task according to cost, risk, and required judgment. Subagents MUST NOT be spawned with forked main-thread context; dispatch packets carry the authorized context boundary and explicit capabilities. Results are protocol-valid only when the dispatch packet names the task ID, role, task kind, risk, scope, write-set, and expected artifact authority clearly enough for the selected agent class.
4. **Pipeline Gates & Pass Slicing**: No Meso enters implementation before its Architect handoff and applicable Designer artifacts (`_designer_spec.md` and a validation contract) exist. Designer contracts may be revised or substantially rewritten in bounded Meso waves; a later Meso's Designer work need not wait for every other Meso if dependencies and write-sets allow. A revision must preserve the parent Meso, covered Micro set, and accepted static topology; static owner or lock-topology defects require a bounded Architect repair. New Designer validation contracts MUST describe an external validation mapping using xfstests and MUST NOT request or imply any ktest-based validation. No packet may create, modify, or grow `#[ktest]`, `#[cfg(ktest)]`, kernel-mode test modules, `test_support/`, memory-disk fixtures, or other ktest harnesses anywhere in the repository as part of this refactor. The main agent decides every Creator Pass boundary, records it in `PASS_SLICING.md` before or with dispatch, and requires every Creator / Checker / Reviewer packet to name exactly one parent meso-component plus explicit covered micro-features. Architects and Designers stay exhaustive at the Meso level and must not pre-slice implementation passes.
5. **Checker Synchronization & Integration Separation**: Every Creator Pass must have a matching Creator-synced Checker Pass with the same parent meso-component and covered micro-feature set; this is a scope and failure-attribution boundary, not a requirement that xfstests isolate each micro-feature. A selected upstream test may cover several related micro-features, and the Checker must report the mapped and actually observed coverage. A validation task may contain multiple isolated runs under that unchanged scope, including compile preflight, rerun, or suffix runs. Meso-level integration validation from the Designer validation contract is never folded into Creator-synced Checker passes; it is scheduled as a separate Checker-owned pass only after the relevant implementation passes exist.
6. **Strict Information Funnel & Artifact Layout**: Packets MUST be saved under `subagent-tasks/<component-id>/`, use `protocol/templates/[level]_[XX]_[component]_[role]_dispatch_TEMPLATE.md`, and remain pointer routes rather than design summaries. Subagent artifacts MUST be written under `.agents/components/<component-id>/`; mixed artifact directories are forbidden. Allowed context by role:
   - **Architect**: workspace-staged filesystem spec summaries, reference implementation summaries, micro-feature inventories, and relevant Asterinas priors.
   - **Designer**: the authoritative design documents under `designdoc/` (Stage A-F drafts and future synthesis), the accepted Architect topology, the coding guidelines (`priors/ASTERINAS_CODE_QUALITY_PRIORS.md` and `book/src/to-contribute/coding-guidelines/`), and local component context.
   - **Creator**: Designer contract, main-agent-selected covered micro set, `priors/ASTERINAS_CODE_QUALITY_PRIORS.md`, and only stable pre-existing kernel interfaces required to typecheck. NEVER provide heavy filesystem specs, reference implementation source dumps, or legacy implementation files to Creator.
   - **Checker (Creator-Synced)**: Designer external-validation mapping, the matching Creator Pass report, and pass write-set / code paths.
   - **Checker (Meso Integration)**: Designer validation contract, accepted Creator reports covering the target micro-features, and pass write-set / code paths.
7. **Template Acceptance Is Structural**: Main-agent acceptance is structural, not logical. Subagent artifacts MUST fully populate their required templates; omitted, conceptually empty, or wrong-destination sections are protocol violations and must be rejected. Risk tiers may select a compact receipt only where the applicable template and packet explicitly allow it; they do not waive scope, evidence, or forbidden-surface rules.
8. **Read-Only LSIF Navigation Tooling**: All roles should use the `ra-code-nav` skill (see `.agents/skills/ra-code-nav/SKILL.md` at the repository root, or the workspace-level `.agents/skills/ra-code-nav/`) for packet-scoped read-only Rust navigation whenever they need symbol-aware lookup. The skill queries a pre-generated rust-analyzer LSIF index with shell + `jq`; no Python LSP client is involved. Dispatch packets should remind agents that this is the preferred rust-analyzer tool for scoped code navigation. It does not authorize reading outside packet scope and does not replace required role artifacts.
9. **Command Lane & Checker Evidence**: Main agent, Architect, Designer, and Reviewer must not run kernel build/test commands. Creators are command-free unless the packet explicitly grants `compile_preflight`; an authorized Creator compile must run inside `codex-asterinas-dev` through `make check`, `make kernel`, or the packet's exact target-specific smoke command and must be recorded in the Creator report. Checkers own runtime verification by default and must use the CI/Makefile forms (`make check`, `make kernel`, and `make run_kernel`) or the packet-authorized `$ovfs-checker` wrapper rather than ad hoc host/root cargo commands. This refactor's validation is xfstests-only: no Checker work may create, modify, or grow any ktest-based validation surface anywhere in the repository. Approved xfstests configuration or harness work remains outside `kernel/src/fs/fs_impls/` and must be explicitly packeted. Checker execution is lock-guarded and must preserve guest logs plus validation result files before they can be overwritten.
10. **Repair Loop & Escalation**: Checkers analyze failures directly and condense them into actionable repair batches. The main agent is a blind router: it must preserve the Checker reproduce command, failed test, and evidence verbatim; it must not reinterpret diagnostics. Same-Creator repair, rerun, and suffix work is recorded as a continuation event or Validation Run while the formal scope remains stable. Creator-synced failures route to the same Creator Pass, integration failures route to reopened Creator Passes or upward escalation, and any loop that fails five times without a passing upstream-approved validation receipt must be halted and escalated upward. A changed write-set, design contract, owner/lock/persistence boundary, validation objective, or risk tier requires a new or explicitly reopened task boundary.
11. **Parallel Scheduling Discipline**: Command-free Architect / Designer / Reviewer / Creator lanes should proceed in parallel whenever dependencies and write-sets allow, even while the checker execution lock is held. If a delegated command-free lane stalls, the main agent should repair and re-delegate instead of absorbing the work locally.
12. **Cleanup Scope & User-Named Surfaces**: Structural cleanup packets must enumerate each targeted objective separately; Creator and Reviewer artifacts must disposition each objective and the main agent must not infer closure while any named objective remains open.
13. **Entity Census & Full-Surface Audits**: Every Creator artifact MUST disclose whether its write-set introduces production entities. A complete census is mandatory whenever any `struct`, `enum`, local type alias, module, or non-trait helper is introduced, or when the packet is High risk, a structural cleanup, a full-surface audit, or a user-named repair wave. Trait-required methods may be grouped under their impl block; test-only entities must appear separately. A Low-risk implementation with no new production entities may use a compact explicit `No new production entities` receipt, but it still requires exact owner, scope, write-set, contract, and deviation accounting.
14. **Carrier, Seam, And Error Defaults**: Temporary or helper-local carriers are rejected by default unless strongly proven. A stable invariant carrier may be retained when it represents a durable owner or lock/persistence invariant, has multiple real call paths or a clear lifetime, does not carry an easily stale snapshot, has explicit guard boundaries, and is more coherent than a parameter bag. Owner-seams and temporary error seams must either be promoted / localized or carry a precise exit plan naming the future owner, trigger, and seam to remove.
15. **Helper & Thin-Wrapper Defaults**: Top-level helpers and helper families are rejected by default when their parameters or body revolve around one owner; they must become owner-private methods or be inlined unless they are stable meso entries, forced by a trait / registration API, genuinely cross multiple owners, or preserve a proven invariant boundary. A growing state-parameter list is a signal to re-evaluate a stable carrier, not an automatic carrier ban. Small modules are merged only when they have one caller, no independent owner or lock/phase boundary, and no clear extension boundary.
16. **Reviewer Authority & Final Checker**: Reviewer normally runs after Checker; an extra pre-checker structural audit is allowed only when explicitly requested by the user or main agent and does not replace the ordinary post-checker Reviewer gate. The main agent may group stabilized passes into a bounded review wave for one parent Meso when the packet enumerates every pass and the exact covered Micro union. Reviewer direct edits are limited to line-level non-functional changes that preserve behavior and topology; structural findings reopen Creator cleanup rather than being rewritten by Reviewer.
17. **Refactor Validation Boundary**: This overlayfs refactor is validated only through the upstream xfstests lane. No role may create, modify, or grow `#[ktest]`, `#[cfg(ktest)]`, kernel-mode test modules, `test_support/`, memory-disk fixtures, or any other ktest harness anywhere in the repository. Any required validation harness or configuration change must be outside `kernel/src/fs/fs_impls/`, explicitly packeted, and part of the xfstests lane.

## 2. Role Ownership

- **Main agent**: Owns scheduling, acceptance, packet curation, task-board updates, and stale-lock decisions.
- **Architect**: Generates the Bi-Directional Traceability Matrix across the Macro/Meso/Micro hierarchy, defines Global Lock Topology, and establishes static boundaries for micro-features.
- **Designer**: Maps the authoritative design documents and the accepted
  Architect topology into dynamic execution paths and a concrete Rust code form
  for its Meso-Component. It MUST define the meso-level Rust surface — module
  layout, structs, enums, carrier types, and helper signatures — following the
  coding guidelines in `priors/ASTERINAS_CODE_QUALITY_PRIORS.md` and
  `book/src/to-contribute/coding-guidelines/` (see §0.5). It emits one meso-level
  spec and one xfstests-only external-validation contract containing pass-scoped
  test mappings and integration obligations. It must not design or imply ktest
  validation.
- **Creator**: Translates the Designer's blueprints into Rust implementations one Creator Pass at a time, as sliced by the main agent.
- **Checker**: Validates behavior in synchronized Creator/Checker passes through xfstests, owns independent meso-level integration passes, evaluates preserved guest logs and upstream-suite results for runtime failures, and directly reports actionable repair instructions. It must not create or modify ktest validation.
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
2. `Specified` means all Designer artifacts exist, explicitly covering dynamic lock behavior, the meso-level Rust code-form signature design (module layout, struct/enum/carrier/helper signatures per the coding guidelines), the xfstests-only external validation mapping, and meso-level integration obligations without any ktest requirement.
3. The main agent must declare Creator Pass coverage before any implementation starts.
4. Creator-synced Checker passes must retain the covered micro set of their matching Creator Pass. Their xfstests evidence may span multiple mapped micro-features and must distinguish mapped, observed, and not-run coverage.
5. Reviewer evaluates static code quality only after implementation and runtime validation stabilize.
6. `Accepted` requires full micro-feature coverage across passes, no blocking logic/quality findings, and verified execution evidence for the required upstream-approved validation batches.

## 4. Parallel Scheduling Model

Think in terms of one serialized command lane (Checker execution) plus as many safe command-free lanes as dependencies allow.

1. Keep the command lane narrow. Only actual execution logic requires the lock.
2. Keep others moving. While Checker holds the lock, Architect, Designer, Reviewer, and Creator lanes proceed if write-sets are disjoint.
3. The task assignment structure controls context size naturally.
4. Compile-only work is a validation mode owned by the Checker and may be one
   `Validation Run` under the matching task; it does not become a separate
   business pass. A Creator may compile only when the packet explicitly grants
   that capability.
