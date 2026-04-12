<!-- SPDX-License-Identifier: MPL-2.0 -->

# Creator Packet Rules

Read this file together with `COMMON_SUBAGENT.md` and the task packet.

## Purpose

The creator implements exactly one specified component pass or one advisor-defined repair batch.

## Required behavior

1. Follow the designer spec strictly.
2. Treat the packet's architectural-unit context as authoritative for this pass. If the packet says the work lands as owner methods, owner-private helpers, or owner-internal state, do not silently turn it into a standalone public module surface.
3. Keep comments selective and explain intent or invariants, not obvious mechanics.
4. Keep the implementation inside the assigned files and inside the assigned pass.
5. Ask to split the work further if the assigned spec still looks too large.
6. Creator passes are command-free by default. If compile-only verification is explicitly authorized as an exception and actually used, run it only in the environment named in the task packet, which should normally spell out the `docker exec ...` prefix and in-container repository path explicitly.
7. Implement against the supplied designer artifacts plus the packet's explicit prior excerpts. Do not assume unstated exFAT on-disk rules from memory when the packet did not provide them.
8. When the packet authorizes a temporary staging surface, leave the required short code comment in place and record the same future owner or removal condition in the creator artifact.
9. Do not add a short helper or field-exposing accessor unless the packet or designer artifact already proves why another component needs that helper now.
10. If the final owner is a concrete structure owner such as `ExfatFs`, prefer owner methods, owner-private associated helpers, or owner-internal state over module-scope free functions. Do not silently let a helper land as a floating module-level convenience seam when the intended final owner is already known.
11. Do not introduce a standalone struct, enum, or record-shape type unless the packet or designer artifact already justifies it as a validated value type, a durable record type, or an explicitly temporary seam with an exit plan.
12. When you add a new private helper, local type, accessor, or temporary seam, record it explicitly in the creator artifact together with its final owner or removal condition. Do not leave reviewer to infer why that surface exists.
13. Use the packet's implementation-quality slice as the main reusable quality checklist for the pass. Do not ask architect or designer artifacts to solve creator-local implementation choices unless the spec is genuinely incomplete.
14. Do not opportunistically add compile commands on your own. Parallel safety is part of the packet contract.
15. A creator pass is not required to produce a compile result unless the task packet explicitly makes one part of the pass contract.

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
