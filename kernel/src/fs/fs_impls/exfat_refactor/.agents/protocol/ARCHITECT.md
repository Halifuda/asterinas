<!-- SPDX-License-Identifier: MPL-2.0 -->

# Architect Packet Rules

Read this file together with `COMMON_SUBAGENT.md` and the task packet.

## Purpose

The architect identifies the smallest dependency-safe implementation units and exposes parallel-ready waves.

## Required behavior

1. Split by trust boundary, behavior family, and method count, not only by rough line count.
2. Treat names like `chain`, `dentry`, and `inode` as areas to decompose, not as automatically valid component boundaries.
3. Make ready-now parallel siblings explicit.
4. Distinguish command-lane blockers from command-free ready-now work so the main agent can overlap planning and review waves with checker-owned runtime work.
5. Keep proposed initial implementation budgets narrow, normally around `150-300` lines and comfortably below `400`.
6. If a unit still appears likely to add more than `3-4` non-trivial production methods, justify why it should remain one component.
7. Read the prior packet before splitting the component. If the packet includes the full semantic prior set, distill the relevant constraints for downstream roles inside the architect handoff.
8. Record which prior sources materially shaped the split so later roles do not have to guess whether a boundary comes from Microsoft exFAT rules, Linux implementation precedent, or Asterinas-local constraints.
9. Use boundary-level quality guidance only. Do not turn architect work into a creator-local implementation plan unless a local detail is necessary to protect a boundary or invariant.

## Allowed edits

- The architect artifact files listed in the task packet.

## Forbidden edits

- [`COMPONENT_INDEX.md`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md)
- Production code
- Designer, creator, checker, advisor, or reviewer artifacts

## Stop condition

Stop after writing the assigned architect artifact or proposal.
Do not rewrite the task board yourself.
