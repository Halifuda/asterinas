<!-- SPDX-License-Identifier: MPL-2.0 -->

# Main Agent Handoff

## Metadata

- Fancy nickname: `iron-ridge`
- Date: 2026-04-01 10:52 CST
- Author: main-agent
- Workspace: `/home/halifuda/asterinas`
- Container or environment: Docker container `codex-asterinas-dev`
- Status: Ready for handoff to the next main agent

## Environment Summary

- Image or base environment: `asterinas/asterinas:0.17.1-20260317`
- Working path: `/root/asterinas` inside the container, `/home/halifuda/asterinas` on the host
- Container name, if any: `codex-asterinas-dev`
- KVM status: current exFAT checker evidence still says `no-kvm`, so observed ktest runs are TCG-backed.
- Validated commands:
  - `docker exec codex-asterinas-dev bash -lc 'test -e /dev/kvm && ls -l /dev/kvm || echo no-kvm'`
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test validated_boot_sector_is_required_for_superblock_normalization'`
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test boot_region_loads_super_block'`
- Known environment blockers:
  - No `/dev/kvm` inside the current container.
  - Shared-worktree and shared-container command execution is not parallel-safe; build, OSDK, and QEMU-producing work should be treated as serial unless explicit isolation is prepared first.

## Current Project State

- Current goal: Continue the refactor under the tightened subagent-dispatch protocol while starting the first post-bootstrap implementation wave.
- Current phase: Bootstrap slice stabilized and accepted; ready-now wave planning is active for `EXR-FATVAL-03A` and `EXR-DENTRY-04A`.
- Active or next component:
  - immediate implementation wave: `EXR-FATVAL-03A`, `EXR-DENTRY-04A`
  - immediate planning wave: pre-split only the future components that still look too large under the tighter architect rule
- Latest accepted components:
  - `EXR-BOOT-01`
  - `EXR-IO-02`
  - `EXR-BOOTTYPE-14`
- Components in progress:
  - none yet at the time of this handoff note; `EXR-FATVAL-03A` and `EXR-DENTRY-04A` have just been specified and are ready for creator dispatch
- Blocked components:
  - none

## Recent Decisions

- The workflow protocol has been split into a main-agent scheduler protocol plus role-scoped subagent packet rules under `.agents/protocol/`.
- Task packets must now declare read scope, write scope, forbidden files, stop condition, and explicit execution environment.
- Command-producing subagent work is treated as serial by default in the current shared checkout and shared container model.
- A narrow cleanup component, `EXR-BOOTTYPE-14`, was added and accepted to type the validated boot-sector boundary via `ValidatedBootSector`.
- The component graph was tightened:
  - `EXR-FATVAL-03A`
  - `EXR-CHAIN-03B`
  - `EXR-DENTRY-04A`
  - `EXR-FILESET-04B`
  - `EXR-INOKEY-05A`
  - `EXR-INODE-05B`
- `EXR-FATVAL-03A` and `EXR-DENTRY-04A` are now architected and specified as the next ready-now implementation wave.

## Open Risks And Assumptions

- The new protocol split only helps if the main agent actually avoids `fork_context: true` for ordinary subagents and sends minimal task packets.
- `EXR-FATVAL-03A` and `EXR-DENTRY-04A` are intentionally narrow. Future creators or reviewers must not let them absorb chain walking or file-record validation.
- Future planning still needs scrutiny around `EXR-CREATE-12` and `EXR-WRITE-13`, and possibly whichever component grows during detailed design.
- The old main-agent note `silver-compass-20260331-1841-evening-checkpoint.md` is still untracked in the working tree and was intentionally left out of recent commits.

## Recommended Next Actions

1. Dispatch serial creators for `EXR-FATVAL-03A` and `EXR-DENTRY-04A` with role-scoped protocol packets and explicit Docker or command rules.
2. Dispatch a separate architect task that inspects only the remaining still-large future components and proposes further splits where truly necessary.
3. Keep checker and any later runtime verification serial; do not let multiple command-producing subagents overlap in the current container.

## Resume Checklist

- Read `PROJECT_BRIEF.md`.
- Read `PROTOCOL.md`.
- Read `COMPONENT_INDEX.md`.
- Read the latest main-agent handoff note.
- Verify the environment summary above still matches reality.
