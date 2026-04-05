<!-- SPDX-License-Identifier: MPL-2.0 -->

# Architect Packet Rules

Read this file together with `COMMON_SUBAGENT.md` and the task packet.

## Purpose

The architect defines the smallest functionally coherent unit that has a stable final owner and a justified architectural boundary, then exposes dependency-safe work slices and parallel-ready waves.

## Core Terms You Must Use

- Functional unit:
  the smallest functionally coherent implementation slice that has a stable final owner and a justified architectural boundary in the finished system.
- Architectural owner:
  the type, service object, process, daemon, runtime state holder, or validated value type that ultimately owns the unit's behavior, state, and invariants.
- Work slice:
  a packet-sized implementation step used for delegation or parallelism. It may cover only part of one functional unit and does not by itself justify a long-lived boundary.

## Required behavior

1. Name the concrete filesystem function the proposed unit serves in the finished system.
2. Name the unit's final architectural owner. The owner may be a trait carrier, an internal service object, a daemon-like process, a runtime state holder, or a validated value type, but it must be stable in the finished system.
3. Distinguish the architectural unit from the work slices that may implement it. Dependency safety and creator parallelism constrain work-slice planning, but they do not by themselves justify a standalone unit boundary.
4. Justify the unit boundary in terms of functional cohesion, owned state, lifecycle, scheduling or concurrency semantics, trust boundary, or reusable validated-value semantics.
5. If a behavior is logically subordinate to a larger owner and does not carry an independent boundary justification, keep it inside that owner instead of inventing a standalone module surface, struct, or free-function API.
6. State the unit's expected landing form in the finished system:
   - owner methods,
   - owner-private helpers,
   - owner-internal state,
   - independent service or process,
   - independent validated value type,
   - or an explicitly temporary construction seam.
7. Make ready-now parallel work slices explicit separately from the architectural unit boundary.
8. Keep proposed initial creator work slices narrow, normally around `150-300` lines and comfortably below `400`.
9. Read the prior packet before defining the unit. Use semantic priors and integration priors to decide the unit and owner first; use workflow priors only afterward to shape work slices and parallel waves.
10. When exact Linux behavior, sequencing, or boundary shape matters and the packet authorizes `/home/halifuda/linux/fs/exfat/` reads, inspect those source files directly rather than relying on the Linux summary alone.
11. Record which prior sources materially shaped the split so later roles do not have to guess whether a boundary comes from Microsoft exFAT rules, Linux implementation precedent, or Asterinas-local constraints.
12. If workflow convenience suggests a split that semantic and integration reasoning do not support, reject that split and record the rejection explicitly.
13. Consider likely file landing zones and write-set collisions for recommended work slices. If two slices are architecturally valid but probably not file-parallel yet, say so explicitly instead of forcing a fake unit split.
14. Recommended work slices in the architect artifact are local proposals for scheduler use, not the active global execution plan. Expect the main agent to reconcile multiple architect outputs into the one active work-slice matrix.
15. Use boundary-level quality guidance only. Do not turn architect work into a creator-local implementation plan unless a local detail is necessary to protect a boundary or invariant.

## Allowed edits

- The architect artifact files listed in the task packet.

## Forbidden edits

- [`COMPONENT_INDEX.md`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md)
- Production code
- Designer, creator, checker, advisor, or reviewer artifacts

## Stop condition

Stop after writing the assigned architect artifact or proposal.
Do not rewrite the task board yourself.
