<!-- SPDX-License-Identifier: MPL-2.0 -->

# Main Agent Handoff

## Metadata

- Fancy nickname: `clear-forge`
- Date: 2026-04-05 18:46 CST
- Covered hours: approximately `1.4` hours, from the rollback checkpoint at `2026-04-05 17:21 CST` to this board-reset checkpoint
- Author: main-agent
- Workspace: `/home/halifuda/asterinas`
- Container or environment: Docker container `codex-asterinas-dev`
- Status: Protocol reset and owner-first board rebuild checkpoint; no creator wave in flight

## Environment Summary

- Image or base environment: `asterinas/asterinas:0.17.1-20260317`
- Working path: `/root/asterinas` inside the container, `/home/halifuda/asterinas` on the host
- Container name, if any: `codex-asterinas-dev`
- KVM status: unchanged from the rollback checkpoint; current protocol work did not require runtime verification
- Validated commands:
  - read-only repository inspection commands
  - delegated architect planning pass only; no build, test, or QEMU commands run in this wave
- Known environment blockers:
  - no new blockers discovered in this wave

## Current Project State

- Current goal: keep the refactor paused at the code level while rebuilding the scheduler protocol and task board around real owners and real functional units
- Current phase: owner-first protocol reset and board reset completed; implementation remains paused
- Active or next component:
  - scheduler-owned architect proposal: [`WORKSPACE-ARCH-RESET/00_architect.md`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/WORKSPACE-ARCH-RESET/00_architect.md)
  - no creator/checker/reviewer component is active
- Latest accepted components:
  - `EXR-BOOT-01`
  - `EXR-IO-02`
  - `EXR-BOOTTYPE-14`
  - `EXR-SBGEOM-15`
  - `EXR-FATVAL-03A`
  - `EXR-CHAIN-03B`
  - `EXR-DENTRY-04A`
  - `EXR-FILESET-04B`
- Components in progress:
  - none
- Blocked components:
  - none formally blocked, but all implementation work is intentionally paused pending review of the rebuilt board

## Active Work Slice Matrix

This is the scheduler-owned global view of currently adopted work slices.
Architect artifacts may recommend local candidate slices, but this matrix is the authoritative active plan.

| Slice ID | Parent Unit | Goal | Write Set | Depends On | May Overlap With | Lane Class | Status | Source Artifact | Packet |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| None active | — | No creator/reviewer/checker wave is currently authorized | — | — | — | — | paused | [`WORKSPACE-ARCH-RESET/00_architect.md`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/WORKSPACE-ARCH-RESET/00_architect.md) | None |

## Recent Decisions

- The scheduler protocol now defines `functional unit`, `architectural owner`, and `work slice` explicitly and no longer lets packet-convenience splits masquerade as task-board units.
- Role-visible protocol files now restate the minimum owner-first terms that ordinary subagents need, instead of assuming they saw the scheduler-only glossary.
- Packet rules now support packet-scoped direct reads from `/home/halifuda/linux/fs/exfat/` when the Linux summary is insufficient for exact behavior or boundary questions.
- Post-review final checker is now conditional rather than mandatory, with explicit reviewer-report and main-agent-recording requirements for any skip.
- File-level and write-set conflicts inside one real functional unit are now explicitly a shared concern of main-agent, architect, and designer; they must not be “solved” by inventing fake architectural boundaries.
- Architect artifacts now recommend candidate work slices only; the active main-agent handoff owns the globally active work-slice matrix.
- A clean-context architect subagent was commissioned to redesign the whole board under the new rules, and its proposal was accepted as the basis for rebuilding `COMPONENT_INDEX.md`.
- The rebuilt board now exposes `Final Owner`, `Landing Form`, `Boundary Kind`, and `Scheduler Owner` explicitly instead of burying owner-first semantics inside `Notes`.

## Wave Record

- Scheduling or planning changes made in this wave:
  - commissioned the scheduler-owned architect pass `WORKSPACE-ARCH-RESET`
  - used a clean-context architect with a packet that authorized broad board redesign but forbade direct edits to `COMPONENT_INDEX.md`
  - rebuilt [`COMPONENT_INDEX.md`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md) from the resulting owner-first proposal
- Components or passes advanced, accepted, repaired, blocked, or deferred in this wave:
  - no implementation component advanced
  - old planned units from `EXR-INOKEY-05A` onward were replaced in the live board by the new owner-first plan:
    - `EXR-FS-CORE-16`
    - `EXR-INODE-CORE-17`
    - `EXR-INODE-CACHE-18`
    - `EXR-DIR-ENGINE-19`
    - `EXR-UPCASE-20`
    - `EXR-BITMAP-21`
    - `EXR-FS-OPEN-22`
    - `EXR-DIR-OPS-23`
    - `EXR-FILE-MAP-24`
    - `EXR-READ-OPS-25`
    - `EXR-PGCACHE-26`
    - `EXR-ALLOC-27`
    - `EXR-DENTRY-WRITE-28`
    - `EXR-NAMESPACE-29`
    - `EXR-WRITE-30`
    - `EXR-SYNC-31`
- Protocol, template, or packet-shaping changes made in this wave:
  - updated `README.md`, `PROTOCOL.md`, role protocol files, packet template, architect template, reviewer/checker templates, and main-agent handoff template
  - added the scheduler-owned architect packet archive:
    - [`WORKSPACE-ARCH-RESET/20260405-1800-architect-packet.md`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/WORKSPACE-ARCH-RESET/20260405-1800-architect-packet.md)
  - added the architect proposal artifact:
    - [`WORKSPACE-ARCH-RESET/00_architect.md`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/WORKSPACE-ARCH-RESET/00_architect.md)
  - rebuilt [`COMPONENT_INDEX.md`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md) with an explicit owner-first schema rather than relying on `Notes` to carry owner semantics
- Important facts intentionally removed from earlier drafts because they are no longer relevant:
  - the pre-`INOKEY` planning board is no longer the live plan; it remains only as rollback history

## Open Risks And Assumptions

- The new board is architecturally cleaner, but it has not yet been pressure-tested by a real designer or creator wave. The first implementation wave under the new board should be treated as protocol validation as well as code work.
- `EXR-IO-02` remains historically accepted as a standalone foundation/helper unit. This is acceptable for continuity, but future planners should avoid taking it as precedent for creating new helper-only tracked units without a stronger owner justification.
- The new owner-first board improves semantic and integration clarity, but real creator parallelism will still depend on file layout and write-set planning under `ExfatFs` and `ExfatInode`.

## Recommended Next Actions

1. Review the rebuilt [`COMPONENT_INDEX.md`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md) against [`WORKSPACE-ARCH-RESET/00_architect.md`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/WORKSPACE-ARCH-RESET/00_architect.md) and confirm the replacement graph is acceptable.
2. If the board is accepted, launch the first owner-first architect/designer follow-up on `EXR-FS-CORE-16` and `EXR-INODE-CORE-17`, with particular attention to realistic file landing zones and future creator write-set separation.
3. Keep the active work-slice matrix explicit in future handoffs; do not let architect-recommended slices silently become global scheduling truth without main-agent adoption.

## Resume Checklist

- Read `README.md`.
- Read `PROTOCOL.md`.
- Read `COMPONENT_INDEX.md`.
- Read the latest main-agent handoff note.
- Read the active work-slice matrix in that handoff before dispatching or reshaping any lanes.
- Verify the environment summary above still matches reality.
- Confirm this handoff already reflects the material implementation and protocol changes from this wave before committing or handing off.
