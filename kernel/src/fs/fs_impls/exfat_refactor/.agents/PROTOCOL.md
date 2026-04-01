<!-- SPDX-License-Identifier: MPL-2.0 -->

# exFAT Refactor Multi-Agent Protocol

This file defines how the main agent and subagents collaborate on the exFAT multi-agent project.
The project has two explicit goals:

1. Refactor the exFAT implementation into clearer, better-specified, dependency-safe components.
2. Explore how far LLM agents can reliably automate filesystem engineering work without losing control of correctness, scope, and implementation detail.

The workflow is intentionally process-heavy because both goals matter. The process is part of the experiment, not overhead to be ignored.

This file is the main-agent-owned scheduler protocol.
It is the authoritative workflow reference, but it is not the default document that should be forwarded wholesale to ordinary subagents.
Ordinary subagent dispatch should instead use the scoped files under `.agents/protocol/` plus a task-specific packet that limits read scope, write scope, and stop conditions.

## 1. Global Rules

1. Every agent must obey the repository-level `AGENTS.md`.
2. Any code written by a creator or reviewer must fully comply with the coding guidelines in the repository-level `AGENTS.md`. This is a hard requirement, not a best-effort preference.
3. `kernel/` code must remain safe Rust. No agent may introduce `unsafe` into `kernel/src/fs/fs_impls/exfat_refactor/`.
4. The main agent is the only scheduler. Subagents do not self-assign work or widen scope.
5. The main agent is the only agent allowed to modify `COMPONENT_INDEX.md` or to change a component's official state or owner.
6. No component may enter implementation before its architect handoff and required designer artifacts both exist. For new components, that normally means the full designer set: `01_designer_core.md`, `02_designer_async.md`, and `03_designer_ktest.md`. Legacy single-file `01_designer_spec.md` artifacts remain valid only for components already created before this split.
7. Each component should normally produce about `150-300` lines of initial implementation and should stay comfortably below `400` lines. A component that still fits under `500` lines may nevertheless be too large if it bundles several independent behaviors, policy decisions, or non-trivial methods into one pass. Exceeding `500` lines requires an explicit main-agent decision that records why a smaller split would be worse.
8. A component must depend only on already-accepted components or on stable pre-existing kernel interfaces.
9. The legacy `kernel/src/fs/fs_impls/exfat/` implementation remains the active registered filesystem and regression baseline until the main agent explicitly schedules a takeover.
10. `exfat_refactor` should be compiled in-tree but should not register itself as the `exfat` filesystem type during the exploratory refactor phase.
11. All code and artifact edits must stay inside the workspace at `/home/halifuda/asterinas` and inside the write scope assigned by the main agent. No agent may modify external dependencies, toolchain sources, container image contents, home-directory tools, caches, or any code outside the workspace, even if the external bug appears obvious. If an external bug is suspected, the agent must collect evidence and hand it to the main agent for user review instead of patching it.
12. The designer must still state test obligations, but `#[ktest]` authoring and other test-writing are checker-owned by default. Creators must ignore test-authoring obligations in the spec unless the main agent explicitly overrides this rule. For new components, those obligations should live in `03_designer_ktest.md` rather than inside the creator-facing designer files.
13. Kernel build or run authority is role-restricted for efficiency:
    - the main agent, architect, designer, advisor, and reviewer must not execute kernel build, test, or QEMU run commands;
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
18. Every active workflow step writes its own handoff artifact. No role should append new step results into an older role's file or into a file from a previous phase. File boundaries are role boundaries.
19. Subagent protocol delivery must be role-scoped and task-scoped:
    - the main agent should normally send ordinary subagents only the relevant files from `.agents/protocol/` plus the current component artifacts and task packet;
    - the main agent should not forward this full scheduler protocol to ordinary subagents unless the delegated task is itself a main-agent continuity or protocol-maintenance task;
    - every task packet must define the subagent's read set, write set, forbidden files, and stop condition.
20. A subagent does not acquire scheduler authority just because it can see what the next sensible workflow step might be. Even when a later step appears obvious, the subagent must stop at its assigned role boundary and leave state transitions, acceptance, and task-board edits to the main agent.
21. Any delegated step that runs project commands must receive an explicit execution environment in its task packet. The main agent must not assume the subagent will remember to enter Docker, choose the correct working directory, or infer the approved command prefix on its own.
22. In the current shared-worktree and shared-container workflow, command-producing delegated work is not parallel-safe by default. If a delegated step may create or mutate build artifacts, OSDK state, Cargo state, boot images, or QEMU runtime state, the main agent should schedule it serially unless a genuinely isolated worktree and execution environment were prepared first.
23. For new components, designer context must be role-scoped as well as task-scoped:
    - serial creators should normally receive only `01_designer_core.md`,
    - concurrency creators should normally receive `01_designer_core.md` plus `02_designer_async.md`,
    - serial or final checkers should normally receive `01_designer_core.md` plus `03_designer_ktest.md`,
    - concurrency checkers should normally receive the full designer set.
    The main agent may deviate only when the packet records why the extra context is necessary.

## 2. Roles

### Main Agent

- Owns scheduling, delegation, and acceptance decisions.
- Owns the workflow itself: protocol enforcement, gate enforcement, and rollback or rework decisions.
- Maintains [`COMPONENT_INDEX.md`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md).
- Owns the role-scoped protocol packets under `.agents/protocol/` and decides which subset is forwarded to each subagent.
- Maintains environment continuity information so the project can survive machine switches, new threads, or interrupted sessions with minimal rediscovery work.
- Assigns component ownership and decides when a handoff is mature enough to advance.
- Accepts or rejects architect, designer, creator, checker, advisor, and reviewer artifacts.
- Must reject architect outputs that are only line-budget compliant but still too wide in responsibility, method count, or policy surface.
- Must treat subagent overreach as a process defect even when the underlying edit looks reasonable. Scheduler-owned state changes only become official after the main agent reviews and records them.
- Must specify the execution environment explicitly for any subagent that may run commands, including whether commands must be run through Docker and what repository path inside the container should be used.
- Should assume that command-running subagents are mutually interfering unless their workspaces and runtime state are explicitly isolated.
- Enforces workspace-only edits, pass boundaries, and role-specific command authority.
- Resolves conflicts between specification, implementation, and existing code constraints.
- Produces periodic main-agent handoff notes when the project reaches a meaningful checkpoint or when environment assumptions change.
- Should prefer scheduling whole dependency-safe parallel waves when architect artifacts expose them, instead of advancing only one ready component by default.

### Architect

- Starts from exFAT prior knowledge and external documentation.
- Identifies exFAT components and their dependency graph.
- Splits components into an implementation order that is dependency-safe and operationally sensible.
- Must make parallelism visible instead of leaving the plan as a single linear chain whenever independent components exist.
- Must optimize for narrow implementation units, not merely for superficial compliance with a line budget.
- Must prefer splitting one area into several components when the work would otherwise introduce multiple unrelated methods, multiple policy decisions, or mixed trust boundaries in one pass.
- Must treat labels such as `chain`, `dentry`, or `inode` as umbrella areas rather than automatically valid component boundaries. If such an area contains separable concerns, the architect must emit smaller subcomponents instead of one broad handoff.
- Must explicitly justify any component that is expected to:
  - add more than roughly `3-4` non-trivial production methods,
  - touch more than one primary behavior family,
  - or span more than one meaningful trust or validation boundary.
- Emits one architect handoff per component, small enough for a designer and creator to execute without hidden scope explosion.
- Must name the component's ready-now parallel siblings or recommended parallel wave so the main agent can schedule them deliberately.

### Designer

- Produces the full specification for one component.
- Must reject or send back an architected component that is still too coarse to specify without creator guesswork or that would bundle several independent behavior families into one creator pass.
- Must produce three designer artifacts for new components:
  - `01_designer_core.md`
    - modular specification: dependencies, provided interfaces, hidden internals, touched files;
    - functional specification: accepted inputs, produced outputs, state transitions, invariants, error cases, and explicit serial-pass non-goals.
  - `02_designer_async.md`
    - concurrency specification: lock ordering, atomicity, serialization assumptions, concurrent access rules, and whether a dedicated concurrency creator pass is required or explicitly empty.
  - `03_designer_ktest.md`
    - serial-phase checker-owned test obligations,
    - concurrency-phase checker-owned test obligations, or an explicit statement that no dedicated concurrency tests are required,
    - the smallest expected final-checker rerun surface.
- Must keep creator-facing context narrow:
  - serial creator obligations belong in `01_designer_core.md`,
  - concurrency creator obligations belong in `02_designer_async.md`,
  - checker-owned test obligations belong in `03_designer_ktest.md`.
- Must state test obligations for the checker, not as creator-owned work.
- Must make the component implementable without creator guesswork.

### Creator

- Implements exactly one specified component pass or one advisor-defined repair batch.
- Must fully comply with the coding guidelines in the repository-root `AGENTS.md`.
- Follows the spec, repository style, and all safety constraints.
- Must add comments where the implemented intent or boundary would otherwise be non-obvious, but must not pad the code with paraphrasing comments.
- Must treat comments as part of correctness documentation: add them when they protect a hidden assumption, a layout invariant, or a non-obvious boundary, and omit them when the code already reads plainly.
- Must implement in staged passes:
  - serial pass first,
  - serial repair batches as needed,
  - concurrency pass second,
  - concurrency repair batches as needed.
- Must ignore spec sections that require writing or updating ktests, because test authoring is checker-owned by default.
- Should ask the main agent to split work further when the designer spec still spans too many modules or behaviors for one bounded pass.
- May run compile-only kernel commands, but must not run kernel tests or QEMU-backed runtime commands.
- Must not silently extend scope, redesign interfaces, or postpone important details with vague TODOs.

### Checker

- Verifies code against the designer specification, existing tests, and observable behavior.
- Owns test authoring for verification: may add, refine, or repair ktests and other test-only coverage needed to validate the component or capture a regression.
- May use code review, targeted tests, focused fault hunting, and test-writing to turn missing coverage into executable checks.
- Must keep checker-authored ktests understandable. Each ktest should carry a short comment describing the scenario and expected behavior unless that intent is already unmistakable from the test name and body.
- Reports concrete failures, missing tests, spec mismatches, regression risks, and any test additions or updates it made.
- Must check and record whether the current environment appears to have KVM acceleration available before relying on performance-sensitive test expectations.
- Owns kernel runtime verification commands for this workflow.
- Should default to test-only code changes. It must not modify production implementation except when the main agent explicitly permits a checker-owned fix.
- Serves three distinct checkpoints:
  - serial-pass validation,
  - concurrency-pass validation when the designer requires it,
  - final post-review validation after the reviewer finishes quality edits.

### Advisor

- Converts checker findings into an actionable repair plan for the creator.
- Operates only after checker phases in the serial or concurrency loops.
- Tells the creator what to change, why it is required, and what counts as done.
- Does not own reviewer findings, because reviewer edits are direct.
- May be the same agent as the checker, but the output artifact must still be advisory rather than exploratory.

### Reviewer

- Performs a code-quality review after the implementation and checker or advisor loops are complete.
- Checks the code against repository coding guidelines, Rust engineering style, readability, API boundaries, type-level invariant expression, visibility hygiene, and comment quality.
- May directly modify production or test code to fix code-quality issues inside the owned scope.
- Must not widen component scope, redesign the feature, or replace checker-owned behavioral verification.
- Must not run kernel build, test, or QEMU commands.
- Must leave a reviewer report that explains findings, edits made, and any remaining concerns that need a final checker pass.

## 3. Required Step Sequence

Every component follows the same ordered workflow unless the main agent explicitly records a justified deviation:

1. Basic serial implementation by the creator.
2. Serial implementation test writing and verification by the checker.
3. Advisor repair plan based on checker findings, followed by repeated `creator -> checker -> advisor` serial repair cycles until the checker passes the serial phase.
4. Concurrency implementation by the creator against the designer's concurrency specification.
5. Concurrency test writing and verification by the checker, but only when the designer explicitly requires concurrency-specific testing. If the designer records that no dedicated concurrency tests are required, the workflow may move directly from step 4 to step 7. A simple big-lock implementation still belongs to the concurrency phase if that is what the spec calls for.
6. Advisor repair plan based on concurrency-phase checker findings, followed by repeated `creator -> checker -> advisor` concurrency repair cycles until the checker passes the concurrency phase or the phase is explicitly waived.
7. Code-quality review by the reviewer, including direct quality edits when needed.
8. Final checker pass after the reviewer finishes.

Consequences:

- Advisor only reacts to checker findings from the serial or concurrency loops.
- Reviewer does not produce advisor work; reviewer fixes are followed directly by a checker pass.
- A component is not ready for acceptance until the final checker pass after review is complete.
- Architect and main-agent should still seek parallelism at the component-selection level whenever dependencies allow it.

## 4. Artifact Layout

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
  <fancy-nickname>-YYYYMMDD-HHMM-<summary>.md
```

Each component gets its own directory once the architect creates it:

```text
.agents/components/<component-id>/
  00_architect.md
  01_designer_core.md
  02_designer_async.md
  03_designer_ktest.md
  10_creator_serial.md
  11_checker_serial.md
  12_advisor_serial.md
  13_creator_serial_repair.md
  14_checker_serial_repair.md
  15_advisor_serial_repair.md
  20_creator_concurrency.md
  21_checker_concurrency.md
  22_advisor_concurrency.md
  23_creator_concurrency_repair.md
  24_checker_concurrency_repair.md
  25_advisor_concurrency_repair.md
  30_reviewer_report.md
  31_checker_final.md
```

Legacy note:

- Existing components that already use `01_designer_spec.md` remain valid historical records and do not need to be renamed retroactively.
- New components should use the split designer artifact set above.

Rules:

1. The main agent creates the component directory and assigns ownership.
2. Artifact prefixes are two-digit chronological sequence numbers chosen by the main agent.
3. Each workflow step owns one file. The file name must reveal both the role and the phase.
4. The baseline phase slots are:
   - `00` architect,
   - `01` designer core,
   - `02` designer async,
   - `03` designer ktest,
   - `10` serial creator,
   - `11` serial checker,
   - `12` serial advisor,
   - `20` concurrency creator,
   - `21` concurrency checker,
   - `22` concurrency advisor,
   - `30` reviewer,
   - `31` final checker.
5. Repair cycles within a phase continue numerically inside that decade, for example `13`, `14`, `15` for one serial repair loop and `23`, `24`, `25` for one concurrency repair loop. If more than one repair loop is needed in the same phase, continue the numbering monotonically within the same decade.
6. Later passes create new numbered artifacts instead of appending to closed artifacts from earlier steps. During one active step, the owning agent may append only to its current artifact while work is in progress.
7. Each subagent writes only its own artifact unless the main agent explicitly instructs otherwise.
8. Creators edit implementation files plus the currently assigned `creator` artifact, but do not overwrite design, checker, advisor, or reviewer artifacts.
9. Checkers may edit test code and the currently assigned `checker` artifact, but should not edit production implementation unless the main agent explicitly instructs otherwise.
10. Advisors edit only their own advisory artifact.
11. Reviewers may edit code plus the currently assigned reviewer artifact, but do not edit checker or advisor artifacts.
12. No subagent may modify artifacts owned by another role, and no subagent may update `COMPONENT_INDEX.md`.
13. Main-agent handoff notes should summarize environment facts, validated commands, current component states, open blockers, and the next recommended action so a future session can resume with minimal rediscovery.
14. Main-agent handoff file names must start with a memorable fancy nickname so later sessions are easy to spot in filesystem listings. The recommended pattern is `<fancy-nickname>-YYYYMMDD-HHMM-<summary>.md`.

## 5. Workflow States

Each component moves through these high-level states:

1. `Planned`
   The component is listed in `COMPONENT_INDEX.md` with dependencies and a size budget.
2. `Architected`
   `00_architect.md` exists and defines scope, order, and readiness for design.
3. `Specified`
   The required designer artifact set exists and is complete.
   For new components, this means `01_designer_core.md`, `02_designer_async.md`, and `03_designer_ktest.md`.
   For legacy components, the older `01_designer_spec.md` remains acceptable.
4. `SerialImplementing`
   A creator is actively implementing the first serial pass or a serial repair batch.
5. `SerialChecked`
   The current serial checker artifact exists and the serial phase is either passing or awaiting an advisor decision.
6. `ConcurrencyImplementing`
   A creator is actively implementing the concurrency pass or a concurrency repair batch.
7. `ConcurrencyChecked`
   The current concurrency checker artifact exists and the concurrency phase is either passing or awaiting an advisor decision.
8. `Reviewing`
   The reviewer is actively checking or editing code quality.
9. `FinalChecked`
   The post-review checker artifact exists.
10. `Accepted`
   The main agent judges the component complete enough to become a dependency of later work.
11. `Blocked`
   Progress is paused because an unresolved dependency, protocol violation, or environment issue prevents safe advancement.

Normal flow is:

```text
Planned -> Architected -> Specified
  -> SerialImplementing -> SerialChecked
  -> SerialImplementing -> SerialChecked (repeat as needed)
  -> ConcurrencyImplementing -> ConcurrencyChecked
  -> ConcurrencyImplementing -> ConcurrencyChecked (repeat as needed)
  -> Reviewing -> FinalChecked -> Accepted
```

The main agent may skip the dedicated concurrency checker phase only if the designer explicitly records that no concurrency-specific tests are required for the component.
The main agent may skip the dedicated concurrency creator phase only if the designer explicitly records that the concurrency obligations are empty, trivial, or intentionally deferred to a later accepted component.

## 6. Gate Conditions

### Architect -> Designer

The architect handoff is valid only if it states:

- the component goal,
- the dependency set,
- any components that can be scheduled in parallel with this one once the same prerequisite set is satisfied,
- the recommended parallel wave or an explicit statement that no useful same-wave parallelism exists,
- the reason this order is safe,
- the code budget,
- the concrete files or modules expected to change,
- the exit condition that marks the component ready for implementation.

### Designer -> Creator

The designer handoff is valid only if it states:

- in `01_designer_core.md`:
  - module boundaries and interface surface,
  - functional behavior including failure cases,
  - state changes and maintained invariants,
  - which obligations belong to the serial creator pass,
  - explicit non-goals for this component;
- in `02_designer_async.md`:
  - concurrency and atomicity rules,
  - which obligations belong to the concurrency creator pass,
  - whether the concurrency creator pass is required, empty, or intentionally deferred;
- in `03_designer_ktest.md`:
  - which checker-owned tests belong to the serial phase,
  - whether the concurrency phase requires checker-owned concurrent tests or explicitly does not,
  - the smallest representative final-checker rerun surface.

### Creator -> Checker

The creator handoff is valid only if it states:

- which files changed,
- which spec revision it implemented,
- which pass it completed, for example serial implementation, serial repair batch, concurrency implementation, or concurrency repair batch,
- any approved deviation,
- which self-checks were run,
- which obligations remain intentionally deferred to a later pass,
- known limitations that remain in scope.

### Checker -> Advisor or Main Agent

The checker handoff is valid only if it states:

- confirmed behaviors,
- failing behaviors,
- tests added, updated, or still missing,
- whether the check covered the serial phase, the concurrency phase, or the final post-review phase,
- whether KVM appeared available and whether the observed run used KVM or fell back to TCG when that mattered to interpretation,
- spec clauses that were violated or left unverified,
- regression risk,
- recommended next owner.

If the current checker pass is a serial or concurrency pass and it still has blocking findings, the next owner should normally be `advisor`.
If the current checker pass is the final post-review pass and it has no blocking findings, the next owner should normally be `main-agent`.

### Advisor -> Creator

The advisor handoff is valid only if it states:

- a numbered repair list,
- the reason each repair is needed,
- which checker finding it addresses,
- what evidence will mark the repair complete,
- whether the repair belongs to the serial loop or the concurrency loop.

### Reviewer -> Checker

The reviewer handoff is valid only if it states:

- what code-quality issues were reviewed,
- what edits were made directly,
- which guidelines or style principles those edits address,
- any residual quality concerns left for later components,
- the exact files touched,
- that the next owner is the final checker.

## 7. Specification Quality Bar

Every designer specification must be detailed enough that a creator can implement without inventing hidden policy.

Minimum required sections:

1. `01_designer_core.md`
   - scope and non-goals,
   - dependencies and provided interfaces,
   - data/control flow,
   - functional rules in precondition/action/postcondition form,
   - error handling and invariants,
   - pass boundaries for the serial creator.
2. `02_designer_async.md`
   - concurrency and atomicity constraints,
   - whether async or concurrency implementation is required now, empty, or deferred,
   - pass boundaries for the concurrency creator.
3. `03_designer_ktest.md`
   - serial-phase checker-owned test obligations,
   - concurrency-phase checker-owned test obligations, or an explicit statement that no dedicated concurrency tests are required,
   - the smallest expected final-checker rerun surface.

If any of these are missing, the component is not ready for implementation.

## 8. Checker Policy

Checker passes should be inserted:

- after the serial pass first reaches a creator handoff,
- after every serial repair batch that changes behavior,
- after the concurrency pass first reaches a creator handoff when the designer requires concurrency validation,
- after every concurrency repair batch that changes behavior,
- after the reviewer completes quality edits,
- before promoting a component to a dependency for later components.

Checker output should prefer precise findings over narrative summary.
When behavior-changing code lacks adequate coverage, the checker should normally add or request targeted ktests instead of leaving the obligation purely narrative.
When test runtime or machine capability matters, the checker should also record the environment mode explicitly rather than silently assuming KVM.
When checker reruns multiple verification commands, it should execute kernel test commands sequentially instead of in parallel.
When the checker writes or moves `#[ktest]` code, it should prefer the closest relevant module plus a small shared test-support module for fixtures, instead of centralizing every test in `mod.rs`.

## 9. Reviewer Policy

Reviewer passes should focus on static code quality rather than re-running behavioral verification.

The reviewer should check at least:

1. checked arithmetic and boundary validation,
2. whether invariants are expressed through types or interfaces where practical,
3. visibility hygiene and module boundaries,
4. comment and doc-comment quality,
5. Rust readability and control-flow clarity,
6. whether test fixtures hide mistakes instead of surfacing them.

The reviewer may directly edit code, but those edits must stay within the accepted component scope.
Behavior-changing reviewer edits are allowed only when they are clearly in service of code-quality correctness and the final checker can re-validate them.

## 10. Refactor Policy

The default refactor strategy is a parallel in-tree implementation:

1. The legacy `exfat` module stays intact as the active implementation.
2. New work lands under `kernel/src/fs/fs_impls/exfat_refactor/`.
3. The new module is compiled, but it does not become the registered `exfat` filesystem by default.
4. Validation for the refactor should rely first on targeted ktests and dedicated integration tests, not on replacing the legacy default mount path early.
5. Switching the registered filesystem type from legacy `exfat` to `exfat_refactor` is a deliberate project milestone, not an incidental side effect of ongoing work.

## 11. Acceptance Policy

A component may be marked `Accepted` only when:

1. Its artifacts exist and are internally consistent.
2. Its implementation matches the latest accepted designer spec or approved advisor change set.
3. Blocking findings from checker and reviewer work are resolved or consciously deferred by the main agent.
4. The final post-review checker pass reports no blocking findings.
5. The component is stable enough to become a dependency for later components.

Acceptance does not mean the whole filesystem is done. It means later components may build on it without reopening its core contract by default.
