<!-- SPDX-License-Identifier: MPL-2.0 -->

# exFAT Refactor Multi-Agent Protocol

This file defines how the main agent and subagents collaborate on the exFAT multi-agent project.
The project has two explicit goals:

1. Refactor the exFAT implementation into clearer, better-specified, dependency-safe components.
2. Explore how far LLM agents can reliably automate filesystem engineering work without losing control of correctness, scope, and implementation detail.

The workflow is intentionally process-heavy because both goals matter. The process is part of the experiment, not overhead to be ignored.

## 1. Global Rules

1. Every agent must obey the repository-level `AGENTS.md`.
2. Any code written by a creator must fully comply with the coding guidelines in the repository-level `AGENTS.md`. This is a hard requirement, not a best-effort preference.
3. `kernel/` code must remain safe Rust. No agent may introduce `unsafe` into `kernel/src/fs/fs_impls/exfat_refactor/`.
4. The main agent is the only scheduler. Subagents do not self-assign work or widen scope.
5. The main agent is the only agent allowed to modify `COMPONENT_INDEX.md` or to change a component's official state or owner.
6. No component may enter implementation before its architect handoff and designer specification both exist.
7. Each component should normally produce less than 500 lines of initial implementation. Exceeding 1000 lines requires explicit approval from the main agent.
8. A component must depend only on already-accepted components or on stable pre-existing kernel interfaces.
9. The legacy `kernel/src/fs/fs_impls/exfat/` implementation remains the active registered filesystem and regression baseline until the main agent explicitly schedules a takeover.
10. `exfat_refactor` should be compiled in-tree but should not register itself as the `exfat` filesystem type during the exploratory refactor phase.
11. All code and artifact edits must stay inside the workspace at `/home/halifuda/asterinas` and inside the write scope assigned by the main agent. No agent may modify external dependencies, toolchain sources, container image contents, home-directory tools, caches, or any code outside the workspace, even if the external bug appears obvious. If an external bug is suspected, the agent must collect evidence and hand it to the main agent for user review instead of patching it.
12. The designer must still state test obligations, but `#[ktest]` authoring and other test-writing are checker-owned by default. Creators must ignore test-authoring obligations in the spec unless the main agent explicitly overrides this rule.
13. Kernel build or run authority is role-restricted for efficiency:
    - the main agent, architect, designer, and advisor must not execute kernel build, test, or QEMU run commands;
    - the creator may execute compile-only kernel commands when needed for owned code, but must not execute kernel runtime or test commands such as `cargo osdk test`, `make ktest`, or direct QEMU runs;
    - the checker owns kernel test execution and other runtime verification commands.
    Ordinary read, search, and artifact-inspection commands remain allowed for every role.
14. Kernel verification commands must be run sequentially in this workflow. Do not run multiple `cargo osdk test`, `make ktest`, or other QEMU-producing commands in parallel, because tooling-level concurrency failures can obscure component results.
15. Checker-owned `#[ktest]` code should normally be colocated with the module it validates. Use `#[cfg(ktest)] mod tests` in the nearest relevant module by default. A shared `test_support.rs` or similar test-only module is allowed for reusable fixtures, but unrelated tests should not be dumped into `mod.rs` unless the main agent records a reason.
16. Comment discipline is mandatory but selective. Follow the repository coding guidelines rather than a blanket quota:
    - add comments when the intent, invariant, boundary, or tradeoff is not obvious from the code itself;
    - do not add comments that merely paraphrase the code;
    - prefer comments that explain why the code is structured this way, what assumption is being protected, or what subtle case it handles;
    - when a doc comment is warranted, it must follow the repository style rules in `AGENTS.md` and the coding-guidelines book.
17. Every checker-owned `#[ktest]` should carry a short comment that states the scenario or property being validated, because tests are read as executable specification and their purpose should be obvious without reverse-engineering the setup.

## 2. Roles

### Main Agent

- Owns scheduling, delegation, and acceptance decisions.
- Owns the workflow itself: protocol enforcement, gate enforcement, and rollback or rework decisions.
- Maintains [`COMPONENT_INDEX.md`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md).
- Maintains environment continuity information so the project can survive machine switches, new threads, or interrupted sessions with minimal rediscovery work.
- Assigns component ownership and decides when a handoff is mature enough to advance.
- Decides when to spawn checker/advisor passes.
- Accepts or rejects architect, designer, creator, checker, and advisor artifacts.
- Enforces workspace-only edits, pass boundaries, and role-specific command authority.
- Resolves conflicts between specification, implementation, and existing code constraints.
- Produces periodic main-agent handoff notes when the project reaches a meaningful checkpoint or when environment assumptions change.

### Architect

- Starts from exFAT prior knowledge and external documentation.
- Identifies exFAT components and their dependency graph.
- Splits components into an implementation order that is dependency-safe and operationally sensible.
- Emits one architect handoff per component, small enough for a designer and creator to execute without hidden scope explosion.

### Designer

- Produces the full specification for one component.
- Must supply three specification layers:
  - Modular specification: dependencies, provided interfaces, hidden internals, touched files.
  - Functional specification: accepted inputs, produced outputs, state transitions, invariants, and error cases.
  - Concurrency specification: lock ordering, atomicity, serialization assumptions, and concurrent access rules.
- Must separate what belongs to the first creator pass from what belongs to the later concurrency pass whenever concurrency work is non-trivial.
- Must state test obligations for the checker, not as creator-owned work.
- Must make the component implementable without creator guesswork.

### Creator

- Implements exactly one specified component or one advisor-defined fix batch.
- Must fully comply with the coding guidelines in the repository-root `AGENTS.md`.
- Follows the spec, repository style, and all safety constraints.
- Must add comments where the implemented intent or boundary would otherwise be non-obvious, but must not pad the code with paraphrasing comments.
- Must treat comments as part of correctness documentation: add them when they protect a hidden assumption, a layout invariant, or a non-obvious boundary, and omit them when the code already reads plainly.
- Must implement in staged passes. The normal first pass covers modular and functional obligations only; the later pass covers concurrency obligations incrementally on top of the earlier checked code, unless the main agent explicitly records that the concurrency obligations are empty or trivial.
- Must ignore spec sections that require writing or updating ktests, because test authoring is checker-owned by default.
- Should ask the main agent to split work further when the designer spec still spans too many modules or behaviors for one bounded pass.
- May run compile-only kernel commands, but must not run kernel tests or QEMU-backed runtime commands.
- Should add concise comments only where the implemented code is not self-explanatory. Do not add comments that merely paraphrase straightforward code.
- Must not silently extend scope, redesign interfaces, or postpone important details with vague TODOs.

### Checker

- Verifies code against the designer specification, existing tests, and observable behavior.
- Owns test authoring for verification: may add, refine, or repair ktests and other test-only coverage needed to validate the component or capture a regression.
- May use code review, targeted tests, focused fault hunting, and test-writing to turn missing coverage into executable checks.
- Must keep checker-authored ktests understandable. Each ktest should carry a short comment describing the scenario and expected behavior unless that intent is already unmistakable from the test name and body.
- When the main agent explicitly assigns comment-only cleanup, the checker may adjust comments in nearby production and test code, but must not change observable behavior in the same pass.
- Reports concrete failures, missing tests, spec mismatches, regression risks, and any test additions or updates it made.
- Must check and record whether the current environment appears to have KVM acceleration available before relying on performance-sensitive test expectations.
- Owns kernel runtime verification commands for this workflow.
- Must keep checker-authored tests readable. Each `#[ktest]` should include a brief comment that says what scenario or contract it is checking.
- Should default to test-only code changes. It must not modify production implementation except when the main agent explicitly permits a checker-owned fix.

### Advisor

- Converts checker findings and spec obligations into an actionable repair plan.
- Tells the creator what to change, why it is required, and what counts as done.
- May be the same agent as the checker, but the output artifact must still be advisory rather than exploratory.

## 3. Artifact Layout

All planning and handoff artifacts live under:

```text
kernel/src/fs/fs_impls/exfat_refactor/.agents/
```

The main agent owns the index:

```text
.agents/COMPONENT_INDEX.md
```

The main agent may also maintain continuity notes under:

```text
.agents/main-agent/
  YYYYMMDD-HHMM-handoff.md
```

Each component gets its own directory once the architect creates it:

```text
.agents/components/<component-id>/
  00_architect.md
  01_designer_spec.md
  02_creator_log.md
  03_checker_report.md
  04_advisor_actions.md
  10_creator_log.md
  11_checker_report.md
  12_advisor_actions.md
  ...
```

Rules:

1. The main agent creates the component directory and assigns ownership.
2. Artifact prefixes are two-digit chronological sequence numbers chosen by the main agent.
3. The first cycle should normally use `00_architect.md`, `01_designer_spec.md`, `02_creator_log.md`, `03_checker_report.md`, and `04_advisor_actions.md`.
4. Later implementation or repair cycles should continue with the next available decade so filesystem sort order matches chronology. For example: `10_creator_log.md`, `11_checker_report.md`, `12_advisor_actions.md`, then `20_*` for the next cycle.
5. Each subagent writes only its own artifact unless the main agent explicitly instructs otherwise.
6. Later passes create new numbered artifacts instead of appending to closed artifacts from earlier cycles. During one active pass, the owning agent may append to its current artifact while work is in progress.
7. Creators edit implementation files plus the currently assigned `*_creator_log.md`, but do not overwrite design or review artifacts.
8. Checkers may edit test code and the currently assigned `*_checker_report.md`, but should not edit production implementation unless the main agent explicitly instructs otherwise.
9. No subagent may modify artifacts owned by another role, and no subagent may update `COMPONENT_INDEX.md`.
10. Main-agent handoff notes should summarize environment facts, validated commands, current component states, open blockers, and the next recommended action so a future session can resume with minimal rediscovery.

## 4. Workflow States

Each component moves through these states:

1. `Planned`
   The component is listed in `COMPONENT_INDEX.md` with dependencies and a size budget.
2. `Architected`
   `00_architect.md` exists and defines scope, order, and readiness for design.
3. `Specified`
   `01_designer_spec.md` exists and is complete in modular, functional, and concurrency dimensions.
4. `Implementing`
   A creator is actively modifying code for the currently assigned implementation pass and maintaining the current `*_creator_log.md`.
5. `Implemented`
   The code change and creator self-check are complete.
6. `Checked`
   The current `*_checker_report.md` exists for the current implementation pass.
7. `Advised`
   The current `*_advisor_actions.md` turns findings into a bounded repair set.
8. `Accepted`
   The main agent judges the component complete enough to become a dependency of later work.

Normal flow is:

```text
Planned -> Architected -> Specified
  -> Implementing (modular/functional pass) -> Implemented -> Checked
  -> Advised or ready for concurrency pass
  -> Implementing (concurrency pass) -> Implemented -> Checked
  -> Advised as needed -> Implementing -> Implemented -> Checked -> Accepted
```

The main agent may skip the dedicated concurrency pass only if it explicitly records that the component's concurrency obligations are empty, trivial, or intentionally deferred to a later accepted component.
If checker finds no actionable issue in the final required pass, the main agent may move directly from `Checked` to `Accepted`.

## 5. Gate Conditions

### Architect -> Designer

The architect handoff is valid only if it states:

- the component goal,
- the dependency set,
- the reason this order is safe,
- the code budget,
- the concrete files or modules expected to change,
- the exit condition that marks the component ready for implementation.

### Designer -> Creator

The designer handoff is valid only if it states:

- module boundaries and interface surface,
- functional behavior including failure cases,
- state changes and maintained invariants,
- concurrency and atomicity rules,
- which obligations belong to the first modular or functional implementation pass and which belong to the later concurrency pass,
- checker-owned tests or observable checks that should pass,
- explicit non-goals for this component.

### Creator -> Checker

The creator handoff is valid only if it states:

- which files changed,
- which spec revision it implemented,
- which implementation pass it completed, for example modular or functional pass, concurrency pass, or advisor-defined repair batch,
- any approved deviation,
- which self-checks were run,
- which spec obligations remain intentionally deferred to a later pass,
- known limitations that remain in scope.

### Checker -> Advisor

The checker handoff is valid only if it states:

- confirmed behaviors,
- failing behaviors,
- tests added, updated, or still missing,
- whether the check covered the modular or functional pass, the concurrency pass, or a repair batch,
- whether KVM appeared available and whether the observed run used KVM or fell back to TCG when that mattered to interpretation,
- spec clauses that were violated or left unverified,
- regression risk,
- recommended next owner.

### Advisor -> Creator

The advisor handoff is valid only if it states:

- a numbered repair list,
- the reason each repair is needed,
- which spec clause or checker finding it addresses,
- what evidence will mark the repair complete.

## 6. Specification Quality Bar

Every designer specification must be detailed enough that a creator can implement without inventing hidden policy.

Minimum required sections:

1. Scope and non-goals.
2. Dependencies and provided interfaces.
3. Data/control flow.
4. Functional rules in precondition/action/postcondition form.
5. Error handling and invariants.
6. Concurrency and atomicity constraints.
7. Checker-owned test obligations.
8. Pass boundaries when modular or functional work and concurrency work should not land in the same creator pass.

If any of these are missing, the component is not ready for implementation.

## 7. Checker Policy

Checker passes should be inserted:

- after the modular or functional pass first reaches `Implemented`,
- after the concurrency pass reaches `Implemented`,
- after any fix batch that changes behavior,
- before promoting a component to a dependency for later components,
- when architect or main agent suspects the component boundary is wrong.

Checker output should prefer precise findings over narrative summary.
When behavior-changing code lacks adequate coverage, the checker should normally add or request targeted ktests instead of leaving the obligation purely narrative.
When test runtime or machine capability matters, the checker should also record the environment mode explicitly rather than silently assuming KVM.
When checker reruns multiple verification commands, it should execute kernel test commands sequentially instead of in parallel.
When the checker writes or moves `#[ktest]` code, it should prefer the closest relevant module plus a small shared test-support module for fixtures, instead of centralizing every test in `mod.rs`.

## 8. Refactor Policy

The default refactor strategy is a parallel in-tree implementation:

1. The legacy `exfat` module stays intact as the active implementation.
2. New work lands under `kernel/src/fs/fs_impls/exfat_refactor/`.
3. The new module is compiled, but it does not become the registered `exfat` filesystem by default.
4. Validation for the refactor should rely first on targeted ktests and dedicated integration tests, not on replacing the legacy default mount path early.
5. Switching the registered filesystem type from legacy `exfat` to `exfat_refactor` is a deliberate project milestone, not an incidental side effect of ongoing work.

## 9. Acceptance Policy

A component may be marked `Accepted` only when:

1. Its artifacts exist and are internally consistent.
2. Its implementation matches the latest accepted designer spec or approved advisor change set.
3. Blocking findings are resolved or consciously deferred by the main agent.
4. The component is stable enough to become a dependency for later components.

Acceptance does not mean the whole filesystem is done. It means later components may build on it without reopening its core contract by default.
