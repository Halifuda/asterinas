<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet Template

Use this template when delegating ordinary subagent work.
The packet should be small enough that the subagent does not need the full scheduler protocol or full thread history.
For ordinary delegated work, do not attach `PROTOCOL.md`; attach the relevant files under `protocol/` instead.

## Metadata

- Packet ID:
- Packet file:
- Supersedes:
- Role:
- Component:
- Phase:
- Authorizing main agent:
- Date:

## Goal

- One short paragraph describing the exact assigned step.

## Read Set

- Files the subagent is allowed to read by default.

## Write Set

- Files the subagent is allowed to edit.

## Forbidden Files

- Scheduler-owned or out-of-scope files the subagent must not touch.

## Required Inputs

- Artifacts, code paths, or test outputs the subagent must rely on.
- When the component uses split designer artifacts, list exactly which designer files this role is allowed to see.
- List the role-scoped protocol files that accompany this packet.
- Do not treat `PROTOCOL.md` as a substitute for those role files.

## Semantic Prior Inputs

- State whether this role receives:
  - the full semantic prior set,
  - selected excerpts from `Microsoft-exFAT-spec.md`,
  - selected excerpts from `linux-exFAT-implementation-summary.md`,
  - or only prior-derived semantic constraints from an earlier artifact.
- If excerpts are used, list the exact files or sections.
- State the intended precedence among those semantic inputs. Unless the packet records a justified exception, use:
  - `Microsoft-exFAT-spec.md` for normative exFAT semantics,
  - `linux-exFAT-implementation-summary.md` for preferred implementation guidance when the spec leaves design room.
- If the assigned work must deliberately diverge from Microsoft- or Linux-derived behavior because of an Asterinas interface constraint, record that exception explicitly here instead of letting the subagent infer it from legacy code.

## Local Architectural Prior Inputs

- State whether this role receives:
  - the full `ASTERINAS_ARCHITECT_PRIORS.md`,
  - selected excerpts from it,
  - a profile reference such as `I-ARCH` or `I-CHECK`,
  - or only integration constraints derived by an earlier artifact.
- If excerpts are used, list the exact files or sections.
- Record which local architectural facts matter for this packet, for example:
  - VFS or page-cache surface constraints,
  - mount-owned state boundaries,
  - shared-container testing reality,
  - legacy source-map context.

## Quality Prior Inputs

- State whether this role receives:
  - the full `ASTERINAS_CODE_QUALITY_PRIORS.md`,
  - selected excerpts from it,
  - a profile reference such as `Q-ARCH`, `Q-DESIGN`, `Q-CREATE`, `Q-CHECK`, or `Q-REVIEW`,
  - or only quality constraints derived by an earlier artifact.
- If excerpts are used, list the exact files or sections.
- State which quality concerns are in scope for this role and which are intentionally out of scope so the packet does not push one role into another role's job.

## Prior Delivery Notes

- Explain how the packet was kept narrow.
- Prefer profile labels and section references over pasted long prose.
- State any relevant earlier artifact that the subagent may rely on instead of reopening a larger prior source.

## Temporary Interfaces And Exit Plan

- List any staging-only wrapper, placeholder owner, or temporary surface that this role may introduce or preserve.
- For each one, state:
  - why it exists in the current component,
  - which later component or owner should absorb or remove it,
  - what short code comment must mark it as temporary.
- If no temporary interface is authorized, say so explicitly.

## Helper Justification

- List the helper APIs this role is allowed to add or preserve.
- For every short helper or field-exposing accessor, name the expected cross-module caller, trust boundary, or repeated error-prone pattern that justifies the helper.
- If no such proof exists yet, say that the helper must not be added.

## Allowed Commands

- For example: read-only shell commands, compile-only commands, or sequential ktest commands.
- For creator packets, compile-only commands should appear here only when the main agent explicitly authorizes a command exception.

## Parallelism Classification

- Lane class:
  - command-free,
  - explicit compile-only exception,
  - or runtime/test-producing.
- May overlap with:
  - no other delegated lanes,
  - command-free lanes with disjoint write sets,
  - or only the specifically named sibling lanes.
- Known conflicts:
  - files, components, commands, or shared runtime state that make this step incompatible with other lanes.

## Execution Environment

- Host or Docker:
- Required command prefix:
- Required working directory:
- Isolation notes:
- State whether this task must run serially with respect to other command-producing work.
- If the task is command-free, state that the subagent must not add compile or runtime commands on its own.

## Execution Lock

- Lock path:
  - normally `.agents/locks/checker-execution.lock/` for checker execution stages
- Lock metadata file:
  - `owner.toml`
- If this task includes a command-producing checker stage, state:
  - whether the stage must hold the execution lock,
  - the quiet-wait retry interval, which must be at least `60` seconds,
  - the maximum wait budget before reporting back,
  - and whether stale-lock review is reserved to the main agent.

## Stop Condition

- State exactly when the subagent must stop.
- Example: "Stop after writing `10_creator_serial.md`; do not proceed into checker work."

## Escalation Rule

- State what the subagent should do if the assigned step is blocked, too large, or appears to require another role.
