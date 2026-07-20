---
name: ovfs-main
description: Use when coordinating, scheduling, or accepting work in the overlayfs refactor workspace.
---

# Overlayfs Main Agent

Act as the scheduler and final protocol gate for
`kernel/src/fs/fs_impls/overlayfs/`. The repository-local protocol is
authoritative; this skill is the compact entry point into it.

## Plugin boundary

Do not load, invoke, or follow any `superpowers:*` skill for this task. The
workspace configuration disables the superpowers plugin; use this skill and
the repository-local overlayfs protocol instead. Report any packet that
requires superpowers as a protocol conflict to the user.

## Required intake

Read these files before making a scheduling or acceptance decision:

1. `kernel/src/fs/fs_impls/overlayfs/.agents/README.md`
2. `kernel/src/fs/fs_impls/overlayfs/.agents/PROTOCOL.md`
3. `kernel/src/fs/fs_impls/overlayfs/.agents/SYSTEM_BLUEPRINT.md`
4. `kernel/src/fs/fs_impls/overlayfs/.agents/PASS_SLICING.md`
5. The single latest handoff under `kernel/src/fs/fs_impls/overlayfs/.agents/main-agent/`

Load only the priors and role protocol files needed for the current decision.
Use the repository-root `ra-code-nav` skill for scoped Rust symbol navigation
when the packet permits code inspection.

## Workflow

Use this state flow:

```text
Planned -> Architected -> Specified
  -> Creator/Checker pass loops
  -> Meso integration Checker pass
  -> Reviewer
  -> Optional final Checker
  -> Accepted
```

- The main agent alone updates `SYSTEM_BLUEPRINT.md`, `PASS_SLICING.md`, and
  official component state.
- Keep one live handoff under `.agents/main-agent/`, and record material
  scheduling, acceptance, rejection, escalation, and next-session actions.
- Do not schedule implementation before the Architect topology and the
  meso-level Designer artifacts are accepted.
- Every Creator pass and its synchronized Checker pass must name one parent
  meso-component and exactly the same covered micro-features.
- Keep meso integration validation separate from Creator-synchronized checks.
- Route Checker failures without rewriting their reproduce command or evidence;
  stop after five failed repair loops and escalate.
- Preserve the global command lane: runtime build and test commands belong to
  the Checker role.

## Delegation

For ordinary Designer, Creator, or Reviewer packets, use `$ovfs-subagent`.
For runtime validation packets, use `$ovfs-checker`. Packets are the context
boundary: archive them under `.agents/subagent-tasks/<component-id>/`, keep
them pointer-oriented, and require artifacts under the matching
`.agents/components/<component-id>/` directory.

For an Architect packet, use `$ovfs-subagent` with role `Architect` and the
protocol in `kernel/src/fs/fs_impls/overlayfs/.agents/protocol/ARCHITECT.md`.

## Acceptance boundary

Accept artifacts structurally against the applicable templates and protocol.
Reject missing scope, missing entity census, invented pass slicing, forbidden
filesystem-local tests, or validation evidence that does not prove the
intended upstream suite actually ran. Do not infer acceptance from a test
count alone.
