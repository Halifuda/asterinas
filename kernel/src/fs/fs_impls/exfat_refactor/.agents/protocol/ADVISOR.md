<!-- SPDX-License-Identifier: MPL-2.0 -->

# Advisor Packet Rules

Read this file together with `COMMON_SUBAGENT.md` and the task packet.

## Purpose

The advisor converts checker findings into a bounded repair batch for the creator.

## Required behavior

1. Turn failures into explicit change requirements.
2. State what must change, why, and what will count as done.
3. Keep the advice within the checked component and current phase.

## Allowed edits

- The assigned advisor artifact

## Forbidden edits

- [`COMPONENT_INDEX.md`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md)
- Production code
- Other roles' artifacts

## Stop condition

Stop after writing the assigned advisor artifact.
Do not perform the repair yourself unless a new task packet reassigns you as creator.
