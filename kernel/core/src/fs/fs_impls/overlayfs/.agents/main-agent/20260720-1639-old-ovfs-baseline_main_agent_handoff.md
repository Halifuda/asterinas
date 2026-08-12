<!-- SPDX-License-Identifier: MPL-2.0 -->

# Main-Agent Handoff — 2026-07-20 16:39 CST

**Status:** `CLOSED / HANDED OVER`

## 1. Global State Pointer

  - **Current Active Wave / Pass:** `pass_00_old_ovfs_baseline_test` (complete / baseline evidence only)
- **Blueprint Updates Made:** Yes — registered the pre-design legacy baseline Checker pass; Phase 2 Architect and Phase 3 Designer remain planned.

## 2. Pass Slicing Decisions

- `pass_00_old_ovfs_baseline_test` under `legacy_overlayfs_baseline_validation` covers the xfstests-mapped micro-feature set and all 80 cases in `overlay/run_list/full.list`.
- This is a pre-design baseline exception with no Creator-synchronized companion; it does not accept refactor implementation work.

## 3. Thread Activity Log

- **Dispatches Sent:**
- Agent `019f7ebb-09f6-7c22-b31a-1157328a91ea` (`Schrodinger`) dispatched for `old-ovfs-baseline-test` using `.agents/subagent-tasks/old-ovfs-baseline-test/pass_00_old_ovfs_baseline_test_checker_dispatch.md`.
- Follow-up agent `019f7f02-e0a0-7c90-9b4f-9083691caa6b` (`Plato`) dispatched for the controlled rerun after the original Checker closed blocked on image ownership; its continuation completed the final four targets after the one-hour controller ceiling.
- **Acceptance Outcomes:**
  - The controlled baseline receipt is accepted as evidence-only: 80 rows, `9 PASS`, `15 FAIL`, `49 NOTRUN`, `7 HANG/TIMEOUT`, and no `CORRUPT`.
- **Escalations / Deadlocks:**
  - None.

## 4. Explicit Agent-Level Decisions

- Treat the closed priors/scaffold handoff as historical input; do not modify it.
- Complete the legacy case matrix before dispatching the Architect design wave.
- Keep runtime execution in the serialized `codex-asterinas-dev` Checker lane; use one fresh QEMU per case, a 300-second cutoff, and one compact status table instead of per-case logs.
- Do not repeat the historical smoke test or startup investigation; it already established the usable overlay xfstests launch flow.
- The initial provisional `CORRUPT` rows and the non-attributable rerun results were cleared per user instruction; the historical smoke/full-list logs, including `pre-rerun-20260720-181525/`, were preserved.
- The replacement rerun used one fresh QEMU and freshly recreated TEST/SCRATCH images per case. Classification used only the current case's `qemu.log`: explicit pass/failure/not-run output was preserved, and only the 300-second cutoff produced `HANG/TIMEOUT`.
- The final report is `.agents/subagent-tasks/old-ovfs-baseline-test/evidence/pass_00_old_ovfs_baseline_test_checker.md`; the durable matrix is `.agents/subagent-tasks/old-ovfs-baseline-test/evidence/baseline_case_matrix_rerun.tsv`. No `CORRUPT` row was recorded.
- The controller reached its one-hour tool ceiling after launching `overlay/076`; that case's uniquely attributable `qemu.log` was reconciled as `NOTRUN`, after which `077`, `078`, `100`, and `101` completed. No QEMU, runlist, or generated image remains.

## 5. Next Actions for the Next Thread

1. Treat `.agents/subagent-tasks/old-ovfs-baseline-test/evidence/baseline_case_matrix_rerun.tsv` and its receipt as the completed legacy baseline evidence; do not treat it as refactor implementation acceptance.
2. Keep the Architect wave separate and dispatch it only after explicit scheduling direction; the baseline gate itself is satisfied.
3. Do not rerun the baseline unless new user direction changes the case list or classification policy.

## 6. Handoff Closure

- **This file was the live handoff for:** old-overlayfs baseline validation wave.
- **Closure:** The baseline wave is complete and ownership is handed over to the next main-agent tenure for an explicit Architect scheduling decision.
- **Supersedes / Replaces:** `20260720-overlayfs-priors-complete_main_agent_handoff.md` (closed historical handoff).

This handoff is closed. No further baseline execution is pending.
