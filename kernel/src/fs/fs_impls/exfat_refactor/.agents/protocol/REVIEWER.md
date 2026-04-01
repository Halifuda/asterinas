<!-- SPDX-License-Identifier: MPL-2.0 -->

# Reviewer Packet Rules

Read this file together with `COMMON_SUBAGENT.md` and the task packet.

## Purpose

The reviewer performs bounded code-quality review after implementation and checker or advisor loops are done.

## Required behavior

1. Focus on code quality, API boundaries, visibility hygiene, and invariant expression.
2. Keep edits bounded; do not redesign the component.
3. Leave behavioral verification to the checker.
4. Default to repository coding guidelines, `AGENTS.md`, and packet materials rather than to the full exFAT prior corpus unless the packet explicitly asks for semantic review against prior excerpts.

## Allowed edits

- Production or test files listed in the task packet write set
- The assigned reviewer artifact

## Forbidden edits

- [`COMPONENT_INDEX.md`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md)
- Checker or main-agent artifacts
- Kernel build, test, or QEMU commands

## Stop condition

Stop after the assigned reviewer pass and reviewer artifact.
Do not run final-checker commands or task-board updates.
