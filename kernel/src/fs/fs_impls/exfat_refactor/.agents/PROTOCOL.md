<!-- SPDX-License-Identifier: MPL-2.0 -->

# exFAT Refactor Multi-Agent Protocol

This file is the main-agent-owned scheduler protocol for the `exfat_refactor` workspace.
It defines what the main agent controls: delegation, gates, parallel scheduling, and acceptance.
It is intentionally narrower than the old all-in-one protocol.

Use the surrounding documents as follows:

- [`README.md`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/README.md):
  workspace map and project framing
- `protocol/`:
  role-scoped rules that ordinary subagents should actually receive
- `templates/`:
  required artifact content and handoff formats
- `subagent-tasks/`:
  archived task packets that were actually sent

Ordinary subagents should normally receive only the relevant `protocol/` files plus a task packet, not this full scheduler document.
Because this file no longer repeats role-level detail, ordinary subagent packets should not forward `PROTOCOL.md` at all unless the delegated task is itself main-agent continuity or protocol-maintenance work.

## 1. Scheduler Rules

1. Every agent must obey the repository-level `AGENTS.md`.
2. `kernel/src/fs/fs_impls/exfat_refactor/` must remain safe Rust. No agent may introduce `unsafe` there.
3. The main agent is the only scheduler. Only the main agent may change official component state, ownership, or [`COMPONENT_INDEX.md`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md).
4. No component may enter implementation before its architect handoff and required designer artifacts exist. For new components, that normally means `01_designer_core.md` plus `03_designer_ktest.md`; `02_designer_async.md` is required only when the component has meaningful concurrency, serialization, atomicity, lock-ordering, or async-facing obligations that later roles cannot safely infer from the core spec alone.
5. Components must stay narrow. A creator pass should normally land about `150-300` lines of initial implementation and stay comfortably below `400`; exceeding `500` lines requires an explicit main-agent decision that records why a smaller split would be worse.
6. A component may depend only on accepted components or stable pre-existing kernel interfaces.
7. The legacy `kernel/src/fs/fs_impls/exfat/` remains the active registered filesystem and regression baseline until the main agent explicitly schedules takeover. `exfat_refactor` may compile in tree, but it must not silently become the registered `exfat` type during exploratory refactor work.
8. All edits must stay inside `/home/halifuda/asterinas` and inside the task packet write scope. No agent may patch external tools, home-directory state, or other workspaces.
9. Test authoring is checker-owned by default. Designers must still state test obligations, but creators should ignore test-writing obligations unless the task packet explicitly overrides that default.
10. Role command authority is strict:
    - main agent, architect, designer, advisor, and reviewer must not run kernel build, test, or QEMU commands;
    - creator passes are command-free by default;
    - creator compile-only commands are rare packet-scoped exceptions, not a default step;
    - checker owns kernel test execution and other runtime verification commands.
11. In the shared worktree and shared container, there is one serialized command lane by default. Unless the main agent prepared a genuinely isolated environment, do not run multiple command-producing verification lanes in parallel.
12. Checker execution is lock-guarded:
    - command-free checker work may happen before execution in the same pass;
    - before any build, `cargo osdk test`, `make ktest`, or QEMU-producing command, the checker must atomically create `.agents/locks/checker-execution.lock/`;
    - after acquiring that directory, the checker must write one metadata file, `.agents/locks/checker-execution.lock/owner.toml`;
    - `owner.toml` should record the component, checker phase, command, pid if available, and start time;
    - if the lock already exists, the checker waits quietly and retries;
    - the retry interval must be at least `60` seconds unless the task packet requires a longer interval;
    - only the main agent may decide a lock is stale and clear it.
13. Command-free work should fill the parallel lanes whenever dependencies and write sets allow it. Architect, designer, reviewer, most advisor work, creator passes, checker preparation work, and packet preparation should not sit idle just because the command lane is busy.
14. Task packets are mandatory for delegated ordinary-subagent work. Each packet must define read scope, write scope, forbidden files, prior inputs, lane classification, stop condition, and command environment if commands are allowed.
15. Every delegated ordinary-subagent packet must also name the role-specific protocol file set that accompanies it. A packet is incomplete if the subagent receives only the packet without the matching role rules under `protocol/`.
16. `PROTOCOL.md` is main-agent-facing by default. Do not forward it to ordinary subagents unless the delegated task is explicitly about main-agent continuity, protocol maintenance, or another scheduler-owned workflow task.
17. The actual packet sent to a subagent must be archived under `.agents/subagent-tasks/<component-id>/`. Reissued packets must be kept as new historical files rather than overwriting old ones.
18. Every delegated role artifact must cite the archived packet it followed. If the main agent performed the step locally, the artifact should say so explicitly.
19. Task packets must state whether the step is `command-free`, an `explicit compile-only exception`, or `runtime/test-producing`, and must name known conflicts that block overlap with sibling lanes.
20. Prior knowledge is also packet-scoped. The main agent must curate which parts of these prior layers each role receives:
    - `Microsoft-exFAT-spec.md`
    - `linux-exFAT-implementation-summary.md`
    - `ASTERINAS_ARCHITECT_PRIORS.md`
    - `ASTERINAS_CODE_QUALITY_PRIORS.md`
21. Prior precedence is fixed unless a packet records a justified exception:
    - Microsoft spec for normative on-disk semantics
    - Linux summary for preferred implementation guidance when the spec leaves room
    - Asterinas architect priors for local integration constraints
    - Asterinas code-quality priors for engineering-quality constraints
22. Role packets should be sliced aggressively:
    - architect gets semantic priors plus boundary-level local and quality slices;
    - designer gets only the semantic and integration material needed to specify the component plus design-level quality slices;
    - creator gets the designer-derived constraints plus only the semantic or integration excerpts needed to avoid semantic drift, plus `Q-CREATE`;
    - checker gets the relevant semantic excerpts, designer test obligations, required integration facts, and `Q-CHECK`;
    - reviewer normally receives the broadest quality slice and semantic priors only when semantic drift is in review scope.
23. If a delegated role appears to require prior material that the packet did not supply, the subagent must stop and report the missing input instead of silently substituting memory or unrelated files.
24. Legacy Asterinas `exfat` code is an integration reference, not the semantic target of the refactor. If local Asterinas interfaces force a divergence from Microsoft- or Linux-derived behavior, the architect or designer artifact must record that explicitly.
25. Temporary staging surfaces are allowed only when the packet or referenced artifact names them explicitly and gives an exit plan. The code comment and role artifact must both name the future owner, absorbing component, or removal condition.
26. Short helpers and field-exposing accessors need explicit justification. If the packet or referenced artifact cannot name the cross-module caller, trust boundary, or repeated error-prone pattern that requires the helper now, the helper should not be added.
27. The advisor role is optional. The main agent may send a checker finding list directly back into a creator repair batch when the scope is already narrow and obvious.
28. Main-agent handoff writing is continuous. The active handoff is the editable record of the current wave rather than a final append-only summary written at the end.
29. Every material wave action by the main agent must be reflected in the active handoff during that same wave. This includes at least:
    - component-planning and scheduling decisions,
    - implementation or repair waves the main agent drove or accepted,
    - checker or reviewer outcomes that change what happens next,
    - protocol, template, README, testing-guide, or packet-shaping changes.
30. The active handoff may be rewritten, condensed, or reorganized during the wave so the final note stays readable, but it must not drop decisions or state that a future main agent would need to resume safely.
31. Before a wave is considered complete or committed, the main agent must ensure the active handoff already reflects the final shape of that wave, and the finalized note must end with explicit next-main-agent tasks.

## 2. Role Ownership

This section only names scheduler-level ownership.
Ordinary subagents should follow the corresponding files under `protocol/` for role detail.

- Main agent:
  owns scheduling, acceptance, packet curation, continuity, task-board updates, and lock-stale decisions.
- Architect:
  splits work into dependency-safe components and must expose parallel-ready waves instead of only a single linear chain.
- Designer:
  produces the bounded spec set for one component: `01_designer_core.md`, optional `02_designer_async.md`, and `03_designer_ktest.md`.
- Creator:
  implements exactly one specified pass or one bounded repair batch; default mode is command-free.
- Checker:
  validates behavior, owns targeted test writing, and owns lock-guarded execution of build and runtime verification commands.
- Advisor:
  condenses checker findings into a bounded repair batch when the main agent decides the extra pass is worth it.
- Reviewer:
  performs static code-quality review and may directly edit in-scope code, but does not own runtime verification.

## 3. Workflow Gates

The normal component path is:

```text
Planned -> Architected -> Specified
  -> Serial creator/checker loop
  -> Concurrency creator/checker loop (only when the designer requires it)
  -> Reviewer
  -> Final checker
  -> Accepted
```

Gate rules:

1. `Architected` means the component has a valid architect artifact and explicit dependency-safe placement.
2. `Specified` means the required designer artifact set exists. For new components, this is always `01_designer_core.md` plus `03_designer_ktest.md`, and `02_designer_async.md` only when concurrency or serialization obligations need a dedicated artifact.
3. Serial and concurrency repair loops may be either direct `creator -> checker` loops or advisor-mediated `advisor -> creator -> checker` loops.
4. The reviewer runs after implementation and checker loops are complete enough for code-quality review.
5. A component is not `Accepted` until the final post-review checker pass reports no blocking findings.

Artifact content requirements live in `templates/`.
This protocol only defines which artifacts must exist before the next state transition is allowed.

## 4. Parallel Scheduling Model

The scheduler should think in terms of one serialized command lane plus as many safe command-free lanes as the current dependency graph allows.

Practical rules:

1. Do not wait for a current component to reach creator before planning the next wave. Once a prerequisite component's architect result is accepted and its boundary is stable enough, the main agent should start architecting or designing dependency-safe successors.
2. Keep the command lane narrow. Only the part of checker work that actually runs build, test, or QEMU commands belongs there.
3. Keep everyone else moving. While one checker is waiting for or holding the execution lock, other lanes should continue with architect, designer, creator, reviewer, packet preparation, and checker preparation work whenever write sets do not conflict.
4. Treat compile-only creator exceptions as rare. They consume the same shared command environment and should only be authorized when they provide clear signal that is worth delaying checker execution.

### Conceptual Best-Effort Wave Example

Suppose component `A` is the current critical path and the accepted architect output for `A` shows that later components `B` and `C` will depend on `A` but not on each other.

The preferred schedule is:

1. Finish and accept `A`'s architect artifact.
2. Immediately run `A`'s designer while also starting architect work for `B` and `C`.
3. Once `A`'s designer core boundary is stable enough for downstream planning, let `A` move into creator work while `B` and `C` move into designer work.
4. When `A` reaches checker, let that checker write ktests and prepare evidence first. Only the actual execution stage needs the lock.
5. While `A`'s checker waits for or holds `.agents/locks/checker-execution.lock/`, keep `B` and `C` moving with command-free work such as creator passes, reviewer work on earlier components, or more architect or designer work for the next wave.
6. Once `A` is accepted, `B` and `C` should ideally already be specified or partially implemented, so the next critical path does not restart from zero.

The anti-pattern is a full-chain schedule like:

```text
A architect -> A designer -> A creator -> A checker -> A accepted -> B architect -> ...
```

The target is instead:

```text
command lane:
  A checker execution

command-free lanes:
  B creator
  C designer
  next-wave architect or packet preparation
```

The workflow still has one serialized execution lane, but it should not behave like a one-component-at-a-time pipeline.

## 5. Artifacts And Templates

Use the naming and directory scheme from [`README.md`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/README.md).
Use the role-specific artifact templates under `templates/`.

Scheduler-level artifact rules:

1. The main agent creates component directories and owns `COMPONENT_INDEX.md`.
2. Each workflow step writes its own artifact; do not append new step results into an older role's file.
3. Repaired or repeated passes create new numbered artifacts rather than reopening closed ones.
4. Main-agent continuity notes live under `.agents/main-agent/` and should use the fancy-nickname filename pattern described in `README.md`.
5. Packet archives live under `.agents/subagent-tasks/<component-id>/` and should map cleanly back to the delegated step they authorized.

## 6. Acceptance Policy

A component may become `Accepted` only when:

1. the required artifacts exist and are internally consistent;
2. the implementation matches the latest accepted designer spec or approved repair batch;
3. blocking checker and reviewer findings are resolved or explicitly deferred by the main agent;
4. the final post-review checker pass reports no blocking findings;
5. the component is stable enough to become a dependency for later work without reopening its core contract by default.

Acceptance does not mean the whole filesystem is done.
It means later components may safely build on that component's current contract.
