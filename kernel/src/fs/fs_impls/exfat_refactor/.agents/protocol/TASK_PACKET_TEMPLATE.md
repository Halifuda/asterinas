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
