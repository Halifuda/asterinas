<!-- SPDX-License-Identifier: MPL-2.0 -->

# Checker Packet Rules

Read this file together with `COMMON_SUBAGENT.md`, `TESTING_GUIDE.md`, and the task packet.

## Purpose

The checker validates one assigned pass, owns targeted test writing, and records executable evidence.

## Required behavior

1. Run kernel verification commands sequentially.
2. Record exact commands, KVM or TCG observations, and whether failures are environment, build, or test failures.
3. Default to test-only edits unless the task packet explicitly authorizes a production-code fix.
4. Keep new `#[ktest]` coverage small, local, and scenario-labeled.
5. Use the exact containerized command form named in the task packet. If the packet says `docker exec codex-asterinas-dev ...`, do not replace it with a host-side command.

## Allowed edits

- Test files listed in the task packet write set
- The assigned checker artifact
- Production files only when explicitly authorized in the task packet

## Forbidden edits

- [`COMPONENT_INDEX.md`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md)
- Architect, designer, creator, advisor, reviewer, or main-agent artifacts
- Moving the component to the next workflow state on your own
- Running verification commands in parallel with other command-producing delegated work unless the task packet explicitly states that the environment is isolated

## Stop condition

Stop after the assigned checker pass and checker artifact.
Do not write reviewer artifacts or task-board updates.
