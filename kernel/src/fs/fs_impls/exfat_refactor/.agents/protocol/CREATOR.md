<!-- SPDX-License-Identifier: MPL-2.0 -->

# Creator Packet Rules

Read this file together with `COMMON_SUBAGENT.md` and the task packet.

## Purpose

The creator implements exactly one specified component pass or one advisor-defined repair batch.

## Required behavior

1. Follow the designer spec strictly.
2. Keep comments selective and explain intent or invariants, not obvious mechanics.
3. Keep the implementation inside the assigned files and inside the assigned pass.
4. Ask to split the work further if the assigned spec still looks too large.
5. If compile-only verification is authorized, run it only in the environment named in the task packet, which should normally spell out the `docker exec ...` prefix and in-container repository path explicitly.
6. Implement against the supplied designer artifacts plus the packet's explicit prior excerpts. Do not assume unstated exFAT on-disk rules from memory when the packet did not provide them.
7. When the packet authorizes a temporary staging surface, leave the required short code comment in place and record the same future owner or removal condition in the creator artifact.
8. Do not add a short helper or field-exposing accessor unless the packet or designer artifact already proves why another component needs that helper now.

## Allowed edits

- Production files listed in the task packet write set
- The assigned creator artifact

## Forbidden edits

- [`COMPONENT_INDEX.md`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md)
- Checker, advisor, reviewer, or main-agent artifacts
- Kernel runtime or ktest commands
- Guessing host-side command execution when the task packet did not authorize it

## Stop condition

Stop after the assigned creator pass and creator artifact.
Do not write checker tests, reviewer notes, or task-board updates.
