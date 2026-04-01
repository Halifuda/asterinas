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
5. Remove or inline short helpers that lack a packet-backed reason to exist, especially field-exposing accessors with no proven cross-module caller.
6. Ensure every temporary staging surface in scope is explicitly marked as temporary in code comments and echoed in the reviewer report with its future owner or removal condition.

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
