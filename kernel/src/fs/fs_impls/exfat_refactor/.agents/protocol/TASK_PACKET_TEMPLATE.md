<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet Template

Use this template when delegating ordinary subagent work.
The packet should be small enough that the subagent does not need the full scheduler protocol or full thread history.

## Metadata

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

## Prior Inputs

- State whether this role receives:
  - the full exFAT prior set,
  - selected excerpts from `Microsoft-exFAT-spec.md`,
  - selected excerpts from `linux-exFAT-implementation-summary.md`,
  - selected excerpts from `ASTERINAS_ARCHITECT_PRIORS.md`,
  - or only prior-derived constraints from an earlier artifact.
- If excerpts are used, list the exact files or sections.
- State the intended precedence among those prior inputs. Unless the packet records a justified exception, use:
  - `Microsoft-exFAT-spec.md` for normative exFAT semantics,
  - `linux-exFAT-implementation-summary.md` for preferred implementation guidance when the spec leaves design room,
  - `ASTERINAS_ARCHITECT_PRIORS.md` for local interface, style, and testing constraints only.
- If the assigned work must deliberately diverge from Microsoft- or Linux-derived behavior because of an Asterinas interface constraint, record that exception explicitly here instead of letting the subagent infer it from legacy code.

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

## Execution Environment

- Host or Docker:
- Required command prefix:
- Required working directory:
- Isolation notes:
- State whether this task must run serially with respect to other command-producing work.

## Stop Condition

- State exactly when the subagent must stop.
- Example: "Stop after writing `10_creator_serial.md`; do not proceed into checker work."

## Escalation Rule

- State what the subagent should do if the assigned step is blocked, too large, or appears to require another role.
