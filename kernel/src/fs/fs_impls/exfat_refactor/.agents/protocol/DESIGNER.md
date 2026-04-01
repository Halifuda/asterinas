<!-- SPDX-License-Identifier: MPL-2.0 -->

# Designer Packet Rules

Read this file together with `COMMON_SUBAGENT.md` and the task packet.

## Purpose

The designer turns one architected component into an implementable spec with no creator guesswork.

## Required behavior

1. Write modular, functional, and concurrency specifications.
2. Split obligations into serial creator work, serial checker work, concurrency creator work, and concurrency checker work.
3. Reject or send back a component that is still too coarse for one creator pass.
4. Keep the specification bounded to the assigned component only.

## Allowed edits

- The assigned designer specification file.

## Forbidden edits

- [`COMPONENT_INDEX.md`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md)
- Production code
- Other roles' artifacts

## Stop condition

Stop after writing the assigned designer artifact.
Do not implement, test, or schedule follow-up work.
