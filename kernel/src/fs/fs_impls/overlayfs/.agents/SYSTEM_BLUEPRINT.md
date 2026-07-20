<!-- SPDX-License-Identifier: MPL-2.0 -->

# System Blueprint & Dispatch Ledger

This file is the scheduler-owned live board for a filesystem implementation or refactor workspace.
Managers and subagents update artifacts elsewhere; only the main agent updates this board.

## 1. Macro Topology & Global Status

- [x] **Phase 0: Agent Workflow Bootstrap**
  - **Status**: Accepted
  - **Commit**: `b2d0df22a`
  - **Accepted Artifact**: `.agents/` directory layout, protocol, skills

- [x] **Phase 1: Priors Layer**
  - **Status**: Accepted
  - **Commit**: `f03ef716b` (priors) + `45362e19f` (ra-code-nav) + `ff1c00a2f` (xfstests mapping)
  - **Accepted Artifacts**: `priors/REFERENCE_IMPLEMENTATION_SUMMARY.md`,
    `priors/FILESYSTEM_SPEC_SUMMARY.md`, `priors/FILESYSTEM_SPEC_INDEX.md`,
    `priors/MICRO_FEATURE_INVENTORY.md`, `.agents/skills/ra-code-nav/`

- [ ] **Phase 2: Architect Handoff** (`macro_00_global_topology.md`)
  - **Status**: Planned
  - **Dispatch**: _fill when scheduled_
  - **Accepted Artifact**: _fill when accepted_

- [ ] **Phase 3: Designer Contracts** (per meso-component)
  - **Status**: Planned
  - **Prerequisite**: Phase 2

## 2. Meso-Component Pipeline Index

| Meso-Component | 1. Architect Map | 2. Designer Contract | 3. Creator Passes | 4. Checker Passes | 5. Integration Pass | 6. Reviewer | Overall Status |
| :------------- | :--------------: | :------------------: | :---------------: | :---------------: | :-----------------: | :---------: | :------------- |

_Meso-components will be registered here once the Architect establishes the Macro/Meso/Micro hierarchy from the staged priors._

## 3. Active Pass Tracking

Record only active or recently changed passes here. Durable slicing rationale belongs in `PASS_SLICING.md`.

_No active passes yet. The first wave will be scheduled after the Architect and Designer artifacts for the initial meso-component are accepted._

## 4. Open Escalations / Notes

- _Record blocked repair loops, stale-lock decisions, or user-directed protocol overrides here._
