<!-- SPDX-License-Identifier: MPL-2.0 -->

# Reviewer Packet Rules

Read this file together with `COMMON_SUBAGENT.md` and the task packet.

## Purpose

The reviewer performs bounded code-quality review after implementation and checker or advisor loops are done.

## Required behavior

1. Focus on code quality, API boundaries, visibility hygiene, invariant expression, and owner-first landing form.
2. Check not only what a component does, but also how its surfaces land:
   - module-scope free helpers,
   - field-exposing accessors,
   - standalone structs or enums,
   - emitted record-shape types,
   - and temporary seams.
3. For each such surface in scope, ask explicitly:
   - does it have a clear final owner or justified record-type role,
   - is it an owner-private helper rather than a packet-convenience seam,
   - and if it is temporary, does it have an explicit exit plan?
4. Distinguish among:
   - justified owner-private helper or local record shape that is acceptable for now,
   - temporary seam that should be documented and deferred,
   - and ownerless or convenience-only surface that should be refactored now.
5. Keep edits bounded; do not redesign the component.
6. Leave behavioral verification to the checker.
7. Default to repository coding guidelines, `AGENTS.md`, and packet materials rather than to the full exFAT prior corpus unless the packet explicitly asks for semantic review against prior excerpts.
8. Remove or inline short helpers that lack a packet-backed reason to exist, especially field-exposing accessors with no proven cross-module caller.
9. When leaving a borderline helper or standalone type in place, say why the current landing form is acceptable for now and name the likely owner or removal condition instead of silently treating it as fine.
10. Ensure every temporary staging surface in scope is explicitly marked as temporary in code comments and echoed in the reviewer report with its future owner or removal condition.
11. Unless the packet explicitly puts test fixtures in scope, prioritize production owner boundaries over `#[cfg(ktest)]` harness-only convenience surfaces.
12. Use the packet's review-quality slice as the default bounded checklist instead of requiring the main agent to restate the same quality expectations manually each time.
13. Treat the packet's write-set and lane classification as a parallel-safety contract; do not widen into overlapping files or opportunistic command runs.
14. State explicitly in the reviewer report whether production code changed and whether any direct edits were functional or semantic rather than purely non-functional.

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
