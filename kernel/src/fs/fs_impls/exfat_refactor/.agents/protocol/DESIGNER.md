<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Packet Rules

Read this file together with `COMMON_SUBAGENT.md` and the task packet.

## Purpose

The designer turns one architected component into an implementable spec with no creator guesswork.

## Required behavior

1. Write the bounded designer artifact set required by the component:
   - `01_designer_core.md` is always required for modular and functional specification,
   - `02_designer_async.md` is required only when the component has meaningful concurrency, serialization, atomicity, lock-ordering, or async-facing obligations that later roles cannot safely infer from the core spec alone,
   - `03_designer_ktest.md` is always required for checker-owned serial and any required async test obligations.
2. Keep the produced artifacts aligned but context-separated so later roles can be given only the spec slice they need.
3. Reject or send back a component that is still too coarse for one creator pass.
4. Keep the specification bounded to the assigned component only.
5. Read the curated prior packet supplied for this component and surface any prior-derived rules that later roles must preserve explicitly in the designer artifacts.
6. Do not specify multiple tiny helper APIs with overlapping semantics unless the packet records why each one is needed and which helper is the canonical surface for ordinary callers.
7. If the component needs a temporary staging surface, record why it exists, which later component should absorb or remove it, and what code comment must mark it as temporary.
8. Do not specify a short helper or field-exposing accessor unless the artifact also names the expected caller or boundary that proves the helper is needed now.
9. Use design-level quality guidance only. Do not micromanage creator-local naming, formatting, or file ordering unless that detail protects a boundary, invariant, or canonical helper contract.
10. If `02_designer_async.md` is omitted, say explicitly why no separate async artifact is needed and where any residual serialization assumptions are recorded.

## Allowed edits

- The assigned designer artifact files.

## Forbidden edits

- [`COMPONENT_INDEX.md`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md)
- Production code
- Other roles' artifacts

## Stop condition

Stop after writing the assigned designer artifact set.
Do not implement, test, or schedule follow-up work.
