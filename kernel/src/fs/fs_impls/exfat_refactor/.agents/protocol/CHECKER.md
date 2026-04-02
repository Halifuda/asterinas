<!-- SPDX-License-Identifier: MPL-2.0 -->

# Checker Packet Rules

Read this file together with `COMMON_SUBAGENT.md`, `TESTING_GUIDE.md`, and the task packet.

## Purpose

The checker validates one assigned pass, owns targeted test writing, and records executable evidence.
The checker may do command-free preparation in the same pass before entering lock-guarded command execution.

## Required behavior

1. Run kernel verification commands sequentially when the assigned checker pass includes command execution.
2. Record exact commands, KVM or TCG observations, and whether failures are environment, build, or test failures.
3. Default to test-only edits unless the task packet explicitly authorizes a production-code fix.
4. Keep new `#[ktest]` coverage small, local, and scenario-labeled.
5. Use the exact containerized command form named in the task packet. If the packet says `docker exec codex-asterinas-dev ...`, do not replace it with a host-side command.
6. Validate behavior against the supplied prior excerpts and designer obligations, not merely against local implementation precedent.
7. Fail the pass when a temporary staging surface lacks the required temporary comment or artifact trace inside the assigned scope.
8. Fail the pass when a short production helper or field-exposing accessor has no packet-backed justification.
9. Use the packet's verification-quality slice when classifying code-quality defects inside scope instead of improvising a broader review standard.
10. Write findings so the main agent can decide cleanly whether a direct `creator -> checker` repair loop is enough or whether an advisor pass is needed to re-scope the repair.
11. Before running any build, ktest, or QEMU-producing command, acquire `.agents/locks/checker-execution.lock/` and write `owner.toml` inside it.
12. If the lock is busy, wait quietly and retry on the packet's schedule instead of immediately reporting back.
13. Do not retry more frequently than once per `60` seconds unless the packet explicitly requires a longer interval.
14. If the wait budget in the packet is exceeded, stop and report that the checker could not enter the execution stage.

## Allowed edits

- Test files listed in the task packet write set
- The assigned checker artifact
- Production files only when explicitly authorized in the task packet

## Forbidden edits

- [`COMPONENT_INDEX.md`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md)
- Architect, designer, creator, advisor, reviewer, or main-agent artifacts
- Moving the component to the next workflow state on your own
- Running verification commands in parallel with other command-producing delegated work unless the task packet explicitly states that the environment is isolated
- Clearing another checker's execution lock on your own

## Stop condition

Stop after the assigned checker pass and checker artifact.
Do not write reviewer artifacts or task-board updates.
