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

## Dispatch delivery facts (platform-verified 2026-08-08)

- Your task contract arrives through the **User Dispatch Turn** (the latest
  user turn forked at spawn) and the packet file it names. The spawn payload,
  the NEW_TASK header, and followup/send messages are NOT readable on this
  platform.
- Verify identity with `list_agents`: your path is the running non-root agent
  whose name matches the dispatched task_id. Report a gap instead of guessing
  when it cannot be confirmed.
- Parent-session user/assistant content is the main agent's context, not your
  task. Do not act on it.
- You hold the full tool set, but the protocol forbids subagents from
  spawning agents or sending followup/send messages; do not use them.
- UI/thread views of your context are not a substitute for reading the packet
  file; always read the packet directly and treat it as the sole contract.

## Protocol intake

Read, in this order:

1. The archived packet named by your dispatch turn:
   `kernel/src/fs/fs_impls/overlayfs/.agents/subagent-tasks/<component-id>/<task_id>_dispatch.md`.
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
contract. Map the authoritative design documents into a concrete meso-level
Rust code form (module layout, struct/enum/carrier/helper signatures) following
the coding guidelines (`priors/ASTERINAS_CODE_QUALITY_PRIORS.md` and
`book/src/to-contribute/coding-guidelines/`); see `PROTOCOL.md` §0.5. No pass
slicing and no filesystem-local tests.

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
- Do not spawn agents or send followup/send messages; you hold the tools but
  the protocol forbids their use by subagents (PROTOCOL.md §1.3).
- Do not add `#[ktest]`, `#[cfg(ktest)]`, `test_support/`, memory-disk fixtures,
  or other filesystem-local validation under `kernel/src/fs/fs_impls/overlayfs/`.
- Do not redesign lock topology, macro/meso ownership, or pass boundaries.
- Do not silently broaden a Creator or Checker result to the whole component.
- Do not treat a partial test count as proof that the intended suite ran.
