<!-- SPDX-License-Identifier: MPL-2.0 -->

# Main Agent Handoff

## Metadata

- Fancy nickname: `cedar-signal`
- Date: 2026-03-31 14:05 CST
- Author: Current main agent
- Covered hours: approximately `4.6` hours, inferred from the next main-agent handoff timestamp at `2026-03-31 18:41 CST`
- Workspace: `/home/halifuda/asterinas`
- Container or environment: Docker container `codex-asterinas-dev`
- Status: Ready for handoff to the next main agent

## Environment Summary

- Image or base environment: `asterinas/asterinas:0.17.1-20260317`
- Working path: `/root/asterinas` inside the container, `/home/halifuda/asterinas` on the host
- Container name, if any: `codex-asterinas-dev`
- KVM status: `/dev/kvm` is not visible inside the current container. A successful `cargo osdk test new_exfat` run was previously observed, but QEMU emitted TCG-related warnings. Future checker runs must continue to record whether KVM is actually available.
- Validated commands:
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas && cargo osdk --version'`
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas && make kernel'`
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test new_exfat'`
- Known environment blockers:
  - No `/dev/kvm` in the current container.
  - Test performance expectations must not assume hardware acceleration.

## Current Project State

- Current goal: Prepare the project to start architect-driven decomposition work for the parallel `exfat_refactor` implementation.
- Current phase: Pre-architect kickoff. Protocol, priors, testing guide, and environment path are in place.
- Active or next component: No concrete filesystem component has been scheduled yet. The next main-agent action should be to commission the architect pass and populate `COMPONENT_INDEX.md`.
- Latest accepted components:
  - Parallel module strategy for `exfat_refactor`
  - Moved `.agents` workspace under `kernel/src/fs/fs_impls/exfat_refactor/.agents/`
  - In-tree compilation hook for `exfat_refactor` without registering it as the active filesystem type
- Components in progress: None
- Blocked components: None yet, but no architect artifact exists and `COMPONENT_INDEX.md` is still empty

## Recent Decisions

- The project will keep the legacy `kernel/src/fs/fs_impls/exfat/` implementation intact as the active baseline.
- New refactor work will land in `kernel/src/fs/fs_impls/exfat_refactor/`.
- The new module is compiled in-tree but does not register itself as the `exfat` filesystem type yet.
- Validation for the refactor should prioritize targeted `#[ktest]` coverage and dedicated integration tests instead of switching the default mount path early.
- The checker role also owns test authoring by default and must record KVM versus TCG observations when test interpretation depends on runtime mode.
- The main agent must maintain continuity artifacts so work can survive thread changes or machine switches with minimal rediscovery.

## Open Risks And Assumptions

- The current container path is validated, but hardware acceleration is not available in the current session. A future machine or container may expose `/dev/kvm`, and checker notes must distinguish those cases.
- `exfat_refactor` is currently only a compiled module skeleton. No architected component plan exists yet.
- The project depends on process discipline: no creator work should start before architect and designer artifacts exist.
- The default system mount path still points at the legacy `exfat` implementation. This is intentional and should remain so until a deliberate takeover milestone is scheduled.

## Recommended Next Actions

1. Read the priors and produce the first architect artifact that decomposes `exfat_refactor` into dependency-safe components.
2. Populate `COMPONENT_INDEX.md` with the initial component graph, ownership plan, and code budgets.
3. Decide the first minimal component to specify and implement, likely somewhere on the mount/bootstrap path, while keeping the component scope comfortably below the protocol size budget.

## Resume Checklist

- Read `PROJECT_BRIEF.md`.
- Read `PROTOCOL.md`.
- Read `ASTERINAS_ARCHITECT_PRIORS.md`.
- Read `TESTING_GUIDE.md`.
- Read `COMPONENT_INDEX.md`.
- Read this handoff note before scheduling new work.
- Verify the container `codex-asterinas-dev` still exists and the validated commands above still work.
