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

- **`pass_00_old_ovfs_baseline_test`** — **Complete / Baseline Evidence Only**
  - **Kind**: Pre-design legacy baseline Checker pass; not an implementation pass
  - **Parent**: `legacy_overlayfs_baseline_validation` (temporary validation lane, not an Architect meso-component)
  - **Scope**: `overlay/001`–`overlay/078`, `overlay/100`, and `overlay/101`; classify every case as `PASS`, `FAIL`, `CORRUPT`, `NOTRUN`, or `HANG/TIMEOUT`
  - **Dispatch**: Follow-up agent `019f7f02-e0a0-7c90-9b4f-9083691caa6b` completed the attributable rerun; prior diagnostic agent `019f7ebb-09f6-7c22-b31a-1157328a91ea` is also closed
  - **Execution shape**: One fresh QEMU and freshly recreated TEST/SCRATCH image pair per case, a 300-second hang cutoff, and immediate append to the rerun status table
  - **Result**: 80 attributable rows: 9 `PASS`, 15 `FAIL`, 49 `NOTRUN`, and 7 `HANG/TIMEOUT`; no `CORRUPT`. The Architect design wave is now unblocked as a separate user-authorized scheduling decision.

## 4. Open Escalations / Notes

- The legacy baseline is intentionally scheduled before the Architect wave, following the closed priors handoff. It does not advance Phase 2 or Phase 3 and cannot accept refactor implementation work.
- User-directed execution constraint satisfied: one QEMU and freshly recreated TEST/SCRATCH images were used per case; no shared batch was used.
- Diagnostic checkpoint: the initial 34 rows are provisional and invalid because image freshness was not proven and post-QEMU mount disappearance was misclassified as `CORRUPT`.
- Controlled rerun complete: classification used only each case's `qemu.log`; no `CORRUPT` was recorded. The one-hour controller ceiling was reconciled from the uniquely attributable `overlay/076` log, and the final 80-row matrix is preserved in `.agents/subagent-tasks/old-ovfs-baseline-test/evidence/baseline_case_matrix_rerun.tsv`.
