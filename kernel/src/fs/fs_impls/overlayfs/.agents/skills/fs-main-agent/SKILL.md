---
name: fs-main-agent
description: Scheduler and acceptance guide for the filesystem implementation/refactor workspace in `kernel/src/fs/fs_impls/overlayfs`. Use when resuming the board, updating `SYSTEM_BLUEPRINT.md`, curating dispatch stubs, validating artifacts against protocol templates, enforcing checker-lane rules, or writing the continuous main-agent handoff.
---

# Filesystem Main Agent

Use this skill only when you are the scheduler for the `overlayfs` workspace.
For ordinary delegated work, send the task to `$fs-subagent-workflow`.
For Architect packets, send the task to `$fs-architect-agent`.

## Quick start

1. Open the repository-local state first:
   - `kernel/src/fs/fs_impls/overlayfs/.agents/README.md`
   - `kernel/src/fs/fs_impls/overlayfs/.agents/PROTOCOL.md`
   - `kernel/src/fs/fs_impls/overlayfs/.agents/SYSTEM_BLUEPRINT.md`
   - the latest file under `kernel/src/fs/fs_impls/overlayfs/.agents/main-agent/`
2. Treat the repository protocol as the source of truth. Use this skill as the compact entry point, then load only the repo files or reference notes needed for the decision in front of you.
3. Keep the scheduler boundary hard:
   - only the main agent updates official state in `SYSTEM_BLUEPRINT.md`
   - when discussing durable filesystem structures, use the full term `On-disk Structure Owner`; do not shorten it to an ambiguous generic phrase
   - only the main agent accepts or rejects protocol artifacts
   - no component advances into implementation before required Architect and Designer artifacts exist
   - the main agent owns Creator/Checker pass slicing and must keep covered micro-features explicit
   - when a wave is about structural cleanup, explicitly decide whether surviving entities (not only newly introduced ones) are in scope for review; do not assume the narrower default census is enough
   - packet helper-family cleanup explicitly when needed; do not rely on general owner-placement wording to automatically catch clusters of naked helpers, thin `read_le_*` wrappers, or flat test-only support namespaces
4. Keep packets minimal:
   - archive every dispatch under `.agents/subagent-tasks/<component-id>/`
   - use the dispatch template in `.agents/protocol/templates/[level]_[XX]_[component]_[role]_dispatch_TEMPLATE.md`
   - packets are pointer routes, not replayed design summaries
   - remind agents to use the `ra-code-nav` skill (LSIF index + `jq`) when they need packet-scoped Rust symbol navigation
5. Keep execution serialized:
   - Checker owns build and test commands
   - Checker should use the repo-approved execution lane for compile/build receipts and upstream-approved filesystem validation; the expected validation route is NixOS xfstests unless upstream standardizes a different lane; for early `overlayfs` smoke, prefer the repo-local prebuilt-image lane documented at `kernel/src/fs/fs_impls/overlayfs/.agents/XFSTESTS_PREBUILT_IMAGE_GUIDE.md` and any workspace-local wrapper the packet explicitly names
   - Checker must not propose, create, or modify filesystem-local `#[ktest]`, `#[cfg(ktest)]`, `test_support/`, memory-disk fixture, or test-only helper code under `kernel/src/fs/fs_impls/`
   - Checker execution must wrap `.agents/tools/checker_lock.sh` and archive guest logs plus upstream-suite result files before they can be overwritten
   - Checker must use `.agents/tools/checker_lock.sh` directly only when not using the runner wrapper
   - filtered or partial upstream-suite runs need proof that the intended tests actually executed
   - QEMU-backed failures require preserved guest log and suite-result inspection before classification
6. Keep the repair loop bounded:
   - Checker condenses failures into Creator repair batches
   - the main agent routes those batches without reinterpretation
   - within the same pass and role, reuse an existing live subagent for repairs or follow-up work when possible instead of spawning a new duplicate agent
   - after 5 failed Creator/Checker loops, escalate back upward instead of continuing blindly
7. Keep integration validation distinct:
   - Creator-synced Checker passes must mirror the matching Creator Pass exactly
   - meso-level integration scenarios stay in separate Checker-owned integration passes
8. Keep the handoff live:
   - update the active main-agent note during the wave, not only at the end
   - conclude every session with explicit next-main-agent actions
   - record any wave-local tightening of structural quality rules (for example surviving-entity re-audits or dedicated test-support layout requirements) so later main agents do not silently revert to the narrower default interpretation

## Local priors

Before reaching for outside context, prefer the repository priors already staged in `.agents/priors/`:

- `FILESYSTEM_SPEC_SUMMARY.md`
- `FILESYSTEM_SPEC_INDEX.md`
- `REFERENCE_IMPLEMENTATION_SUMMARY.md`
- `ASTERINAS_INTEGRATION_PRIORS.md`
- `ASTERINAS_CODE_QUALITY_PRIORS.md`

Do not paraphrase large prior bodies into packets when direct file paths are enough.

## Local tools

- `.agents/tools/checker_run.sh`
  Existing Checker compile/build wrapper. It may be extended or wrapped for upstream-approved validation lanes such as NixOS xfstests; validation runs must preserve guest logs and suite result files before they can be overwritten.
- `.agents/tools/checker_lock.sh`
  Low-level checker execution lock used by the runner and by any rare manual Checker command sequence.
- `ra-code-nav` skill (repository-root `.agents/skills/ra-code-nav/`)
  Preferred read-only Rust code navigation helper for agents that need scoped Asterinas code lookup. It queries a pre-generated rust-analyzer LSIF index with shell + `jq` for symbol search, definition, references, hover, and document symbols. It is semantic navigation, not natural-language embedding search, and does not widen a packet's authorized file scope.

## Delegation rule

When spawning a subagent, do not fork the main-thread context; the archived packet is the authorized context boundary. If a live subagent already owns the same role for the same pass, send the repair or follow-up there instead of starting a duplicate agent.

Use prompts of this shape whenever possible:

```text
Use $fs-subagent-workflow to execute the archived packet at <packet-path> for role <role>.
```

For Architect work:

```text
Use $fs-architect-agent to execute the archived packet at <packet-path>.
```

## Reference map

- `references/scheduler-checklist.md`
  Main-agent-only invariants to keep in mind while resuming or reshaping a wave.
- `references/dispatch-and-funnel.md`
  How to keep dispatch stubs minimal and aligned with the strict information funnel.
- `references/acceptance-and-handoff.md`
  Acceptance gates, retry escalation, and continuous handoff discipline.
