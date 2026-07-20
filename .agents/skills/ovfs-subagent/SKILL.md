---
name: ovfs-subagent
description: Use when executing a bounded Architect, Designer, Creator, Checker, or Reviewer packet in the overlayfs refactor workspace.
---

# Overlayfs Subagent

Execute one archived packet inside the overlayfs refactor protocol. The packet
is the task contract and the repository files are the source of truth. Do not
widen the scope when context is missing; report the gap to the main agent.

## Plugin boundary

Do not load, invoke, or follow any `superpowers:*` skill for this task. The
workspace configuration disables the superpowers plugin; use this skill and
the repository-local role protocol instead. Report any packet that requires
superpowers as a protocol conflict to the main agent.

## Protocol intake

Read, in this order:

1. The archived packet under `kernel/src/fs/fs_impls/overlayfs/.agents/subagent-tasks/`.
2. `.agents/protocol/<ROLE>.md` for the assigned role. The available role
   entries are `ARCHITECT.md`, `DESIGNER.md`, `CREATOR.md`, `CHECKER.md`, and
   `REVIEWER.md`.
3. `kernel/src/fs/fs_impls/overlayfs/.agents/PROTOCOL.md` for scheduler-wide
   invariants and artifact boundaries.
4. The matching role protocol is the detailed role rule. Use the top-level
   `ra-code-nav` skill for packet-scoped Rust navigation.

The main-agent protocol overrides this compact entry point. Use
`$ovfs-checker` for packets that authorize runtime validation.

## Role boundary

### Architect

Internalize the staged priors and produce the static topology and traceability
artifacts required by the Architect packet, following
`kernel/src/fs/fs_impls/overlayfs/.agents/protocol/ARCHITECT.md`.

### Designer

Produce exactly one meso-level dynamic specification and one validation
contract. Keep them semantic and meso-scoped: no new Rust signature design,
private-helper design, pass slicing, or filesystem-local tests.

### Creator

Implement only the main-agent-assigned Creator Pass. Name the parent
meso-component and exact covered micro-features in the report. Follow the
Designer contract, preserve the lock topology, census all production entities
in the write-set, and do not add test code under
`kernel/src/fs/fs_impls/overlayfs/`.

### Checker

Validate the matching Creator Pass through the upstream-approved external
system-level lane. A Creator-synchronized Checker mirrors the Creator scope
exactly; meso integration is a separate pass. Preserve guest logs and suite
results before reuse, inspect the actual execution proof, and issue an
actionable repair batch on failure. Use `$ovfs-checker` when the packet runs
the overlay xfstests flow described by the workspace.

### Reviewer

Review only after implementation and Checker evidence stabilize. Keep edits
line-level and non-functional; return topology or structural cleanup to the
Creator. Check helper placement, temporary seams, entity census, and the
validation-harness boundary.

## Common prohibitions

- Do not spawn work outside the packet's authorized files and role.
- Do not add `#[ktest]`, `#[cfg(ktest)]`, `test_support/`, memory-disk fixtures,
  or other filesystem-local validation under `kernel/src/fs/fs_impls/overlayfs/`.
- Do not redesign lock topology, macro/meso ownership, or pass boundaries.
- Do not silently broaden a Creator or Checker result to the whole component.
- Do not treat a partial test count as proof that the intended suite ran.
