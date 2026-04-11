<!-- SPDX-License-Identifier: MPL-2.0 -->

# exFAT Refactor Multi-Agent Protocol

This file is the main-agent-owned scheduler protocol for the `exfat_refactor` workspace.
It defines what the main agent controls: delegation, gates, parallel scheduling, and acceptance.
It is intentionally narrower than the old all-in-one protocol.

Use the surrounding documents as follows:

- [`README.md`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/README.md):
  workspace map and project framing
- `$exfat-main-agent`:
  preferred Codex entry point for ordinary main-agent resume, scheduling, packet-shaping, and handoff work
- `$exfat-subagent-workflow`:
  preferred Codex entry point for ordinary delegated architect, designer, creator, checker, reviewer, and advisor work
- `protocol/`:
  source-text role rules mirrored by `$exfat-subagent-workflow`
- `templates/`:
  required artifact content and handoff formats
- `subagent-tasks/`:
  archived task packets that were actually sent

Ordinary subagents should normally receive only the relevant `protocol/` files plus a task packet, not this full scheduler document.
Because this file no longer repeats role-level detail, ordinary subagent packets should not forward `PROTOCOL.md` at all unless the delegated task is itself main-agent continuity or protocol-maintenance work.
In Codex sessions, prefer invoking `$exfat-main-agent` for scheduler work and `$exfat-subagent-workflow` for ordinary delegated work instead of replaying these stable rules in packet prose.

## 0. Core Terms

- Functional unit:
  the smallest functionally coherent implementation slice that has a stable final owner and a justified architectural boundary in the finished system.
- Architectural owner:
  the stable finished-system owner that ultimately carries the unit's behavior, state, and invariants.
  In this workspace, the canonical owner classes are:
  - VFS trait carrier,
  - structure owner (including owner-local runtime structures and on-disk-structure-derived internal structures),
  - daemon process,
  - record type.
- Work slice:
  a packet-sized implementation step used for delegation or parallelism. A work slice may cover only part of one functional unit and does not by itself justify a long-lived API, file, struct, or module boundary.

Dependency safety constrains scheduling, but it does not by itself justify a standalone tracked component.
Role-scoped protocol files must restate the minimum subset of these terms that their subagents need; ordinary subagents should not be expected to infer scheduler terminology they never received.

## 1. Scheduler Rules

1. Every agent must obey the repository-level `AGENTS.md`.
2. `kernel/src/fs/fs_impls/exfat_refactor/` must remain safe Rust. No agent may introduce `unsafe` there.
3. The main agent is the only scheduler. Only the main agent may change official component state, ownership, or [`COMPONENT_INDEX.md`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md).
4. No component may enter implementation before its architect handoff and required designer artifacts exist. For new components, that normally means `01_designer_core.md` plus `03_designer_ktest.md`; `02_designer_async.md` is required only when the component has meaningful concurrency, serialization, atomicity, lock-ordering, or async-facing obligations that later roles cannot safely infer from the core spec alone.
5. Components in [`COMPONENT_INDEX.md`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md) are tracked functional units, not packet-convenience cuts. The architect artifact must name the unit's final owner and explain why the boundary is architecturally real rather than only useful for delegation.
   The named final owner should fit one of the canonical owner classes from section `0`, or the architect artifact must explicitly justify why a different class is unavoidable.
6. Creator work slices must stay narrow. A creator pass should normally land about `150-300` lines of initial implementation and stay comfortably below `400`; exceeding `500` lines requires an explicit main-agent decision that records why a smaller work slice would be worse.
7. A component may depend only on accepted components or stable pre-existing kernel interfaces.
8. The legacy `kernel/src/fs/fs_impls/exfat/` remains the active registered filesystem and regression baseline until the main agent explicitly schedules takeover. `exfat_refactor` may compile in tree, but it must not silently become the registered `exfat` type during exploratory refactor work.
9. All edits must stay inside `/home/halifuda/asterinas` and inside the task packet write scope. No agent may patch external tools, home-directory state, or other workspaces.
10. Test authoring is checker-owned by default. Designers must still state test obligations, but creators should ignore test-writing obligations unless the task packet explicitly overrides that default.
11. Role command authority is strict:
    - main agent, architect, designer, advisor, and reviewer must not run kernel build, test, or QEMU commands;
    - creator passes are command-free by default;
    - creator compile-only commands are rare packet-scoped exceptions, not a default step;
    - checker owns kernel test execution and other runtime verification commands.
12. In the shared worktree and shared container, there is one serialized command lane by default. Unless the main agent prepared a genuinely isolated environment, do not run multiple command-producing verification lanes in parallel.
13. Checker execution is lock-guarded:
    - command-free checker work may happen before execution in the same pass;
    - before any build, `cargo osdk test`, `make ktest`, or QEMU-producing command, the checker must use `.agents/tools/checker_lock.sh acquire` to claim `.agents/locks/checker-execution.lock/`;
    - the script writes `.agents/locks/checker-execution.lock/owner.toml` with the component, checker phase, command, pid, and start time;
    - if the lock already exists, the checker waits quietly and retries through the script rather than open-coding a new lock procedure;
    - the retry interval passed to the script must be at least `60` seconds unless the task packet requires a longer interval;
    - after the command-producing stage completes, the checker must release the lock through `.agents/tools/checker_lock.sh release`;
    - only the main agent may decide a lock is stale and clear it.
14. Filtered verification commands must prove that they targeted the intended tests. A green exit status alone is insufficient evidence when `cargo osdk test <filter>` is used. The task packet and checker artifact must record either:
    - an exact or otherwise uniquely justified test-path suffix derived from source inspection, or
    - command output that explicitly names the executed tests.
    Broad module-like filters are not enough unless the checker also records why they cannot silently miss or over-match the intended coverage.
15. Command-free work should fill the parallel lanes whenever dependencies and write sets allow it. Architect, designer, reviewer, most advisor work, creator passes, checker preparation work, and packet preparation should not sit idle just because the command lane is busy.
16. When a delegated command-free lane stalls because the subagent misread scope, lacked packet clarity, or otherwise failed to start correctly, the main agent should repair and continue that delegated lane first by clarifying, re-packetizing, or re-dispatching it. The main thread should not absorb unfinished command-free delegated work just to preserve momentum unless the user explicitly asks for local takeover or delegation has become impossible and that exception is recorded in the active handoff.
17. Main-agent scheduling should be organized in loops. One loop may contain one creator round, and that round may include multiple creator packets in parallel when they belong to the same planned wave, have stable prerequisites, and keep disjoint write sets. After that creator round has been launched, the same loop should spend the remaining parallel budget on architect, designer, reviewer, packet-preparation, or checker-preparation work rather than opening a second creator round.
18. Task packets are mandatory for delegated ordinary-subagent work. Each packet must define read scope, write scope, forbidden files, prior inputs, lane classification, stop condition, and command environment if commands are allowed.
19. Every delegated ordinary-subagent packet must also name either:
    - the role-specific protocol file set under `protocol/`, or
    - the mirrored skill invocation `$exfat-subagent-workflow` plus the matching role reference inside that skill.
    In Codex sessions, prefer the skill path. A packet is incomplete if it provides only the packet body without any matching role rules.
20. `PROTOCOL.md` is main-agent-facing by default. Prefer `$exfat-main-agent` for ordinary main-agent resume or scheduling work, and do not forward `PROTOCOL.md` to ordinary subagents unless the delegated task is explicitly about main-agent continuity, protocol maintenance, or another scheduler-owned workflow task.
21. The actual packet sent to a subagent must be archived under `.agents/subagent-tasks/<component-id>/`. Reissued packets must be kept as new historical files rather than overwriting old ones.
22. Every delegated role artifact must cite the archived packet it followed. If the main agent performed the step locally, the artifact should say so explicitly.
23. Task packets must state whether the step is `command-free`, an `explicit compile-only exception`, or `runtime/test-producing`, and must name known conflicts that block overlap with sibling lanes.
24. Prior knowledge is also packet-scoped. The main agent must curate which parts of these prior layers each role receives:
    - `Microsoft-exFAT-spec.md`
    - `linux-exFAT-implementation-summary.md`
    - `ASTERINAS_ARCHITECT_PRIORS.md`
    - `ASTERINAS_CODE_QUALITY_PRIORS.md`
25. Prior precedence is fixed unless a packet records a justified exception:
    - Microsoft spec for normative on-disk semantics
    - Linux summary for preferred implementation guidance when the spec leaves room
    - Asterinas architect priors for local integration constraints
    - Asterinas code-quality priors for engineering-quality constraints
26. Role packets should be sliced aggressively and should distinguish among:
    - semantic priors: exFAT rules and behavior,
    - integration priors: VFS, page-cache, runtime-owner, lifecycle, and local architectural constraints,
    - workflow priors: write-set, packet-size, creator-parallelism, and shared-environment scheduling constraints.
27. Prior priority for architect work is fixed unless a packet records a justified exception:
    - semantic priors first,
    - integration priors second,
    - workflow priors third.
    Workflow constraints may shape work slices and lane scheduling, but they must not be used to invent an architectural boundary that semantic and integration reasoning do not justify.
28. Role packets should be sliced aggressively:
    - architect gets the semantic and integration priors needed to define the unit, plus only the workflow priors needed to shape safe work slices afterward;
    - designer gets the architect-defined unit and owner context plus only the semantic and integration material needed to specify the component, plus design-level quality slices;
    - creator gets the designer-derived constraints plus only the semantic or integration excerpts needed to avoid semantic drift, plus `Q-CREATE`;
    - checker gets the relevant semantic excerpts, designer test obligations, required integration facts, and `Q-CHECK`;
    - reviewer normally receives the broadest quality slice and semantic priors only when semantic drift is in review scope.
29. If a delegated role appears to require prior material that the packet did not supply, the subagent must stop and report the missing input instead of silently substituting memory or unrelated files.
30. `linux-exFAT-implementation-summary.md` is an orientation aid, not a replacement for packet-authorized Linux source inspection. When exact Linux behavior, sequencing, or architectural boundary shape matters, the main agent should authorize the relevant `/home/halifuda/linux/fs/exfat/` paths directly in the packet read set.
31. Legacy Asterinas `exfat` code is an integration reference, not the semantic target of the refactor. If local Asterinas interfaces force a divergence from Microsoft- or Linux-derived behavior, the architect or designer artifact must record that explicitly.
32. Temporary staging surfaces are allowed only when the packet or referenced artifact names them explicitly and gives an exit plan. The code comment and role artifact must both name the future owner, absorbing component, or removal condition.
33. Short helpers and field-exposing accessors need explicit justification. If the packet or referenced artifact cannot name the cross-module caller, trust boundary, or repeated error-prone pattern that requires the helper now, the helper should not be added.
34. The advisor role is optional. The main agent may send a checker finding list directly back into a creator repair batch when the scope is already narrow and obvious.
35. Post-review final checker is conditional rather than mandatory. The main agent may skip it only when the reviewer artifact explicitly records that no production code changed or that the edits were non-functional only, and the main agent records the skip decision and reason in the active handoff or component notes.
36. Architect artifacts may recommend candidate work slices, write sets, and overlap opportunities for one unit, but they do not by themselves establish the current global execution plan.
37. File-level and write-set conflicts across slices inside one real functional unit are a shared design concern for the main agent, architect, and designer. They should prefer file organization and landing-zone planning that preserves future creator parallelism, but they must not invent fake architectural boundaries solely to obtain separate files.
38. When two candidate slices are logically separable but still collide on the same file or same region, the main agent may keep them serial for now or request a better file-organization plan. No hard numeric rule is imposed here; the requirement is to record the constraint instead of hiding it behind a fake unit split.
39. Main-agent handoff writing is continuous. The active handoff is the editable record of the current wave rather than a final append-only summary written at the end, and it is the scheduler-owned home of the active global work-slice matrix.
40. Every material wave action by the main agent must be reflected in the active handoff during that same wave. This includes at least:
    - component-planning and scheduling decisions,
    - adoption, rejection, or reshaping of architect-recommended work slices,
    - implementation or repair waves the main agent drove or accepted,
    - checker or reviewer outcomes that change what happens next,
    - protocol, template, README, testing-guide, or packet-shaping changes.
41. The active handoff may be rewritten, condensed, or reorganized during the wave so the final note stays readable, but it must not drop decisions or state that a future main agent would need to resume safely.
42. Before a wave is considered complete or committed, the main agent must ensure the active handoff already reflects the final shape of that wave, and the finalized note must end with explicit next-main-agent tasks.

## 2. Role Ownership

This section only names scheduler-level ownership.
Ordinary subagents should follow the corresponding files under `protocol/` for role detail.

- Main agent:
  owns scheduling, acceptance, packet curation, continuity, task-board updates, the active global work-slice matrix, and lock-stale decisions.
- Architect:
  defines functional units, names their final owners and boundary kinds, and then recommends dependency-safe work slices and parallel-ready waves.
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
  -> Optional final checker
  -> Accepted
```

Gate rules:

1. `Architected` means the component has a valid architect artifact that names its functional goal, final owner, architectural-boundary justification, and explicit dependency-safe placement.
2. `Specified` means the required designer artifact set exists. For new components, this is always `01_designer_core.md` plus `03_designer_ktest.md`, and `02_designer_async.md` only when concurrency or serialization obligations need a dedicated artifact.
3. Serial and concurrency repair loops may be either direct `creator -> checker` loops or advisor-mediated `advisor -> creator -> checker` loops.
4. The reviewer runs after implementation and checker loops are complete enough for code-quality review.
5. Post-review final checker is required by default, but the main agent may skip it when the reviewer artifact explicitly classifies the review edits as no production change or non-functional only and the skip decision is recorded.
6. A component is not `Accepted` until either:
   - the scheduled final post-review checker pass reports no blocking findings, or
   - the main agent records a valid final-checker skip under rule 5.

Artifact content requirements live in `templates/`.
This protocol only defines which artifacts must exist before the next state transition is allowed.

## 4. Parallel Scheduling Model

The scheduler should think in terms of one serialized command lane plus as many safe command-free lanes as the current dependency graph allows.

Practical rules:

1. Do not wait for a current component to reach creator before planning the next wave. Once a prerequisite component's architect result is accepted and its boundary is stable enough, the main agent should start architecting or designing dependency-safe successors.
2. Keep the command lane narrow. Only the part of checker work that actually runs build, test, or QEMU commands belongs there.
3. Keep everyone else moving. While one checker is waiting for or holding the execution lock, other lanes should continue with architect, designer, creator, reviewer, packet preparation, and checker preparation work whenever write sets do not conflict.
4. Inside one main-agent loop, creator work should appear as one bounded creator round. That round may contain multiple sibling creators in parallel, but the scheduler should not open a second creator round in the same loop after the first round is already in flight.
5. If a command-free delegated lane misfires, fix delegation rather than collapsing the lane back into the main thread. Parallelism comes from keeping those lanes alive, not from the scheduler doing their work itself.
6. Treat compile-only creator exceptions as rare. They consume the same shared command environment and should only be authorized when they provide clear signal that is worth delaying checker execution.

### Conceptual Best-Effort Wave Example

Suppose component `A` is the current critical path and the accepted architect output for `A` shows that later components `B` and `C` will depend on `A` but not on each other.

The preferred schedule is:

1. Finish and accept `A`'s architect artifact.
2. Immediately run `A`'s designer while also starting architect work for `B` and `C`.
3. Once `A`'s designer core boundary is stable enough for downstream planning, let `A` move into one creator round while `B` and `C` move into designer work.
4. When `A` reaches checker, let that checker write ktests and prepare evidence first. Only the actual execution stage needs the lock.
5. While `A`'s checker waits for or holds `.agents/locks/checker-execution.lock/`, keep `B` and `C` moving with command-free work such as reviewer work on earlier components, or more architect or designer work for the next wave.
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
  one creator round for the already-planned sibling implementations
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
6. Architect artifacts may record recommended work slices for their own unit, but the active main-agent handoff is the only scheduler-owned record of the currently active global work-slice matrix.

## 6. Acceptance Policy

A component may become `Accepted` only when:

1. the required artifacts exist and are internally consistent;
2. the implementation matches the latest accepted designer spec or approved repair batch;
3. blocking checker and reviewer findings are resolved or explicitly deferred by the main agent;
4. either the final post-review checker pass reports no blocking findings or the main agent has recorded a valid final-checker skip decision under the reviewer conditions above;
5. the component is stable enough to become a dependency for later work without reopening its core contract by default.

Acceptance does not mean the whole filesystem is done.
It means later components may safely build on that component's current contract.
