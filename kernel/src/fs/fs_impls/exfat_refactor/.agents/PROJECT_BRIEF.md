<!-- SPDX-License-Identifier: MPL-2.0 -->

# exFAT Multi-Agent Project Brief

This note captures the project framing that predates the formal protocol.
It explains what this workspace is trying to do and why the workflow is intentionally strict.

## 1. What This Project Is Actually About

This project is not "just implement exFAT."

It has two equally important goals:

1. Refactor and improve the exFAT implementation.
2. Use exFAT as the validation target for a multi-agent workflow, in order to explore the automation boundary of LLM agents when implementing filesystem code.

The main question is not only whether agents can produce code, but whether they can do filesystem engineering without losing control of specification coverage, implementation detail, style consistency, and bug rate.

The current implementation strategy is to keep the legacy `exfat` module intact while building the refactored implementation in parallel under `exfat_refactor`.
That lets the project use the old code as a stable baseline while the new design is developed with cleaner boundaries.

## 2. Why The Workflow Starts From The Very Beginning

The work must start from the first step of the first step rather than jumping directly into implementation.
Subagents are expected to handle different kinds of work, each with a sharply bounded role.

This constraint exists because a filesystem, even one as comparatively small as exFAT, contains too many interacting details for current LLM agents to safely handle as one large implementation task.
If a single agent tries to design and implement large swaths of the filesystem at once, it is likely to miss details, drift in code style, and accumulate hidden mistakes that later turn into a mess.

The workflow therefore depends on a closed loop:

1. Split the work into small components.
2. Specify each component before implementation.
3. Implement only against the specification.
4. Check the result.
5. Feed defects back as bounded repair work.

## 3. Role Model

### Main Agent

The main agent is the coordinator.

It is responsible for:

- orchestrating subagents,
- defining and enforcing the working protocol,
- controlling stage transitions,
- assigning and tracking work,
- maintaining environment continuity knowledge so the work can resume cleanly after a machine switch or a new thread,
- deciding component ownership and handoff timing,
- checking that architect and designer outputs are complete enough before implementation begins,
- deciding when checker and advisor passes are needed,
- accepting, rejecting, or sending back artifacts and implementations,
- maintaining the global task board and dependency order,
- producing checkpoint handoff notes that summarize the current environment, decisions, blockers, and next actions,
- collecting results and resolving conflicts.

The main agent does not hand scheduling authority to subagents.
The main agent is also the process owner: it is responsible for keeping the whole workflow specification-first, dependency-safe, and small-step rather than allowing ad hoc implementation sprawl.
Because this is a long-running effort, the main agent must optimize for resumability, not only for local progress.
That means future sessions should be able to recover the environment assumptions and project state from artifacts rather than from memory.

### Architect

The architect starts from prior knowledge, which will often include exFAT documentation and other external knowledge about exFAT.

The architect is responsible for:

1. Understanding the major components of exFAT.
2. Splitting exFAT into components with a dependency-safe implementation order.
3. Keeping each component small enough that an initial implementation remains reviewable and repairable.
4. Passing those bounded components to the next stage.

The expected ordering should be operationally sensible and dependency-safe.
One expected progression is:

```text
mount -> lookup/read family -> create family -> write family
```

Component size must stay tightly controlled.
As a rule, the initial implementation for one component should not exceed 500 lines.
It must not exceed 1000 lines without explicit approval.

The reason for this limit is to avoid large first-pass implementations with too many bugs and too much unstructured code.

### Designer

The designer receives a component task from the architect and produces a complete design using:

- the architect's handoff,
- prior knowledge about exFAT,
- the repository codebase.

The design must be detailed enough that the creator does not need to invent policy.
At minimum, it must contain three kinds of specification.

1. Modular specification.
   This defines what the component depends on, what interfaces it provides, and where its boundaries are.
2. Functional specification.
   This describes, in a Hoare-logic-like style, what inputs the component accepts, what outputs it produces, and what changes it makes to the world model or system state.
3. Concurrency specification.
   This defines concurrency, serialization, locking, and atomicity constraints that apply on top of the modular and functional behavior.

### Creator

The creator receives the designer's specification and implements the component.

The creator must:

- follow the specification strictly,
- **fully comply with the coding guidelines in the repository-root `AGENTS.md`, not merely approximate the existing local style,**
- follow repository code style and repository-wide constraints in every affected file,
- avoid silently widening scope or redesigning the component.

For creator work, conformance to the root `AGENTS.md` coding guidelines is a hard requirement.
This includes the safe-Rust boundary, naming and module conventions, error-handling rules, concurrency rules, documentation expectations, testing expectations, and the general requirement to keep code clear, bounded, and reviewable.

### Checker

The checker is introduced at stages chosen by the main agent or architect.

The checker is responsible for:

- testing implemented components,
- writing or updating targeted unit tests when existing coverage is insufficient,
- checking whether the current machine or container appears to have KVM available before interpreting slow or acceleration-sensitive test runs,
- checking for defects and regressions,
- summarizing discovered problems.

There may be more than one checker, and one checker does not need to own all components.
For this workflow, the checker should also be treated as the default owner of verification-oriented test writing.
That keeps executable checks close to the verification pass instead of introducing a separate test-writer role with duplicated context and weaker accountability.

### Advisor

The advisor may be the same agent as the checker, but serves a different role.

The advisor is responsible for:

- using the specification and checker findings,
- telling the creator what must be changed,
- turning problems into bounded repair instructions.

The creator then uses those instructions to fix the implementation.

## 4. Global Constraints

- The repository-level `AGENTS.md` is binding on every agent in the workflow.
- **Any code produced by a creator must satisfy the coding guidelines in the repository-root `AGENTS.md`.**
- The workflow must remain specification-first rather than "think while coding."
- Components must be small, ordered, and dependency-safe.
- The process is intentionally designed to reduce omission, uncontrolled scope growth, and first-pass code collapse.

## 5. Why exFAT Was Chosen

exFAT is the working target because it is complex enough to expose the real difficulties of filesystem implementation, while still being small enough to make componentized, specification-first experimentation practical.

That makes it suitable both for engineering improvement and for studying where multi-agent automation works, where it breaks, and what process structure is needed to keep it reliable.
