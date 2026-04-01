<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Packet Rules

Read this file together with `COMMON_SUBAGENT.md` and the task packet.

## Purpose

The designer turns one architected component into an implementable spec with no creator guesswork.

## Required behavior

1. Write three bounded designer artifacts:
   - `01_designer_core.md` for modular and functional specification,
   - `02_designer_async.md` for concurrency and async obligations,
   - `03_designer_ktest.md` for checker-owned serial and async test obligations.
2. Keep the three artifacts aligned but context-separated so later roles can be given only the spec slice they need.
3. Reject or send back a component that is still too coarse for one creator pass.
4. Keep the specification bounded to the assigned component only.
5. Read the curated prior packet supplied for this component and surface any prior-derived rules that later roles must preserve explicitly in the designer artifacts.

## Allowed edits

- The assigned designer artifact files.

## Forbidden edits

- [`COMPONENT_INDEX.md`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md)
- Production code
- Other roles' artifacts

## Stop condition

Stop after writing the assigned designer artifact set.
Do not implement, test, or schedule follow-up work.
