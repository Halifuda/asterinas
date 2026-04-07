<!-- SPDX-License-Identifier: MPL-2.0 -->

# Main Agent Handoff

## Metadata

- Fancy nickname: `amber-ledger`
- Date: 2026-04-02 17:12 CST
- Author: main-agent
- Covered hours: approximately `5.5` hours during the 2026-04-02 protocol-and-prior maintenance wave
- Workspace: `/home/halifuda/asterinas`
- Container or environment: host workspace only for this wave; no new Docker or QEMU validation was performed
- Status: protocol and prior system tightened again; no production code changes; no component implementation active

## Environment Summary

- Host workspace: `/home/halifuda/asterinas`
- Container workspace: not revalidated in this wave
- Container name: latest known continuity still points to `codex-asterinas-dev`, but this wave did not re-check it
- KVM status: not revalidated in this wave
- Revalidated commands:
  - `git status --short`
  - `date '+%Y-%m-%d %H:%M %Z'`
  - host-side read-only inspection commands under the workspace
- Known environment blockers:
  - shared-worktree discipline still matters
  - unrelated worktree changes exist in production code outside this wave:
    - `kernel/src/fs/fs_impls/exfat_refactor/boot_sector.rs`
    - `kernel/src/fs/fs_impls/exfat_refactor/dentry.rs`
  - untracked `.codex/` also exists outside this wave

## Current Project State

- Current goal: make the `.agents` protocol/prior system more auditable, more role-appropriate, and cheaper for future main agents to resume
- Current phase: protocol and prior maintenance
- Active or next component: no component is active in this wave; next component planning remains open
- Latest accepted components: unchanged by this wave
- Components in progress:
  - none
- Blocked components:
  - none newly blocked by this wave

## Recent Decisions

- Converted `TASK-PACKET` from an implied mechanism into an explicit archived artifact requirement:
  - delegated packets must be stored under `.agents/subagent-tasks/`
  - delegated role artifacts must cite the packet they followed
  - packet templates now carry explicit packet metadata
- Reworked prior structure:
  - `ASTERINAS_ARCHITECT_PRIORS.md` remains the local architectural/integration prior
  - `ASTERINAS_CODE_QUALITY_PRIORS.md` was added as a distinct reusable quality prior
  - packet templates and protocol now separate semantic, architectural, and quality prior delivery
- Merged the former project brief content into `README.md` so README is now the single entry point
- Updated `PROJECT_BRIEF.md` into compatibility-only redirect content rather than a full second entry document
- Tightened role boundaries for quality priors:
  - architect and designer receive only boundary-level or design-level quality slices
  - creator, checker, and reviewer receive the heavier quality slices appropriate to their jobs
- Tightened workflow optionality and parallelism:
  - `02_designer_async.md` is no longer mandatory for every new component
  - `advisor` is now an optional repair-loop role rather than an unconditional phase
  - creator passes are now command-free by default
  - checker passes may do command-free preparation in the same pass before entering execution
  - checker execution is now modeled as one serialized command lane guarded by `.agents/locks/checker-execution.lock/`
  - the checker lock uses one `owner.toml` metadata file and quiet retries with a minimum `60` second interval
  - task packets now record lane classification, overlap expectations, execution-lock behavior, and the quiet-wait budget
- Confirmed empirically that host-side read-only access to `/home/halifuda/linux` works in the current sandbox, but did not encode that fact into protocol because the user preferred not to widen protocol around it
- Chose an indexing strategy for Linux prior growth instead of writing a much larger Linux prose prior:
  - `linux-exFAT-implementation-summary.md` now explains how it should be used
  - it now contains a `Topic-To-Code Index` pointing future main agents to the relevant Linux files and functions under `/home/halifuda/linux/fs/exfat/`
  - it now explicitly tells future main agents to drop to source instead of growing the summary blindly
- Slimmed `PROTOCOL.md` aggressively so it now acts as a scheduler document rather than a giant role-and-template dump:
  - detailed role behavior remains in `protocol/`
  - detailed artifact content remains in `templates/`
  - `PROTOCOL.md` now includes a conceptual best-effort parallel scheduling example instead of restating all role details
- Clarified subagent dispatch rules:
  - ordinary subagents should not receive `PROTOCOL.md`
  - main-agent packets must explicitly include the accompanying role-scoped files under `protocol/`
  - `TASK_PACKET_TEMPLATE.md` now tells the main agent to list those role files explicitly
- Made creator handoff logging match the new creator policy:
  - `CREATOR_LOG_TEMPLATE.md` now treats self-checks as optional rather than expected
- Added an explicit reminder rule that protocol, template, or scheduler-doc edits in a wave must also be reflected in the active main-agent handoff before the wave is considered complete
- Strengthened the handoff model itself:
  - the active main-agent handoff is now treated as the editable record of each wave, not just a final summary
  - every material implementation-side or protocol-side wave action should be reflected in that handoff during the same wave
  - the handoff may be rewritten for clarity, but should not lose resume-critical facts
- This wave intentionally did not touch production Rust code and did not run builds or tests

## Wave Record

- Scheduling or planning changes made in this wave:
  - no new component wave was scheduled; this wave stayed focused on protocol and prior maintenance
- Components or passes advanced, accepted, repaired, blocked, or deferred in this wave:
  - no production component state changed in this wave
- Protocol, template, or packet-shaping changes made in this wave:
  - prior delivery was split into semantic, architectural, and quality layers
  - packet archival became mandatory going forward
  - ordinary subagents were restricted to role-scoped protocol files plus packets, not `PROTOCOL.md`
  - creator and checker command-lane rules were tightened around command-free creators and lock-guarded checker execution
  - `PROTOCOL.md` was slimmed into a scheduler document with an explicit parallel-wave example
  - main-agent handoff policy was strengthened so each wave must keep an updated editable record
- Important facts intentionally removed from earlier drafts because they are no longer relevant:
  - no obsolete per-role detail remains in `PROTOCOL.md`; those details now live under `protocol/` and `templates/`

## Open Risks And Assumptions

- The `.agents/subagent-tasks/` directory currently contains only the README; this wave established the archive location for future delegated work but did not rewrite older history.
- `COMPONENT_INDEX.md` still contains some stale historical notes, especially around `EXR-INOKEY-05A`; this wave did not normalize board prose.
- The Linux implementation summary is now in a better place, but it has reached the point where more additive prose would likely create repetition between source map, topic index, and algorithm summary.
- Because this wave did not revalidate Docker, KVM, or cargo-osdk behavior, future runtime work should still re-check the current execution environment before dispatching checker tasks.
- The checker lock protocol is documented, but this wave did not create or validate any actual `.agents/locks/` runtime implementation script. Future main agents still need to ensure packets and command forms use the same concrete lock procedure.
- Unrelated worktree edits in `boot_sector.rs`, `dentry.rs`, and `.codex/` were intentionally excluded from this wave and from the commit.

## Recommended Next Actions

1. Resume component planning from the updated README, protocol, and handoff, then decide the next architect wave without reopening the prior-structure or protocol-shape debate.
2. When a future task needs Linux behavior detail, use `linux-exFAT-implementation-summary.md` as an index into `/home/halifuda/linux/fs/exfat/` rather than extending the summary by default.
3. Apply the archived-task rule only going forward; do not retrofit imaginary packet history into older waves.
4. Preserve the rule that ordinary subagents should receive role-scoped protocol files plus a packet, not `PROTOCOL.md`.

## Next Main-Agent Tasks

1. Re-read `COMPONENT_INDEX.md` and choose the next executable component wave under the updated optional-role and parallel-lane rules.
2. When forming the next packet, include the role-scoped `protocol/` files explicitly, use the split prior fields, and cite only the minimal Linux or quality slices needed for that role.
3. Preserve the new rule that `02_designer_async.md` and `advisor` are conditional, not default ceremony.
4. Treat creator passes as command-free by default and reserve compile-only exceptions for rare packet-authorized cases.
5. Use the Linux topic index before adding any new Linux prior prose.
6. Continue excluding unrelated production-code edits from protocol-only commits unless the user explicitly merges those tracks.
