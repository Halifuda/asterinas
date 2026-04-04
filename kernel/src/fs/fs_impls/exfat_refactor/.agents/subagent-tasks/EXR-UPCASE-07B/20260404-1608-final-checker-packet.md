<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-UPCASE-07B-FINAL-CHECK-20260404-1608`
- Packet file: `kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-UPCASE-07B/20260404-1608-final-checker-packet.md`
- Supersedes: none
- Role: checker
- Component: `EXR-UPCASE-07B`
- Phase: final checker
- Authorizing main agent: main-agent
- Date: 2026-04-04 16:08 CST

## Goal

- Run the post-review final checker for `EXR-UPCASE-07B` by rerunning the focused local `fileset` ktests under the checker lock and recording the evidence in `31_checker_final.md`. The reviewer report's only blocking finding should now be treated as already repaired by `12_creator_serial_retry.md` and cleared by `13_checker_serial_retry.md`.

## Read Set

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/CHECKER.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/TESTING_GUIDE.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07B/00_architect.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07B/01_designer_core.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07B/03_designer_ktest.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07B/10_creator_serial.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07B/11_checker_serial.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07B/12_creator_serial_retry.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07B/13_checker_serial_retry.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07B/30_reviewer_report.md`
- `kernel/src/fs/fs_impls/exfat_refactor/fileset.rs`

## Write Set

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07B/31_checker_final.md`

## Forbidden Files

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/*`
- all production files unless the review artifacts explicitly demanded a test-only correction

## Required Inputs

- Role protocol files accompanying this packet:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/CHECKER.md`
- Required artifact inputs:
  - architect, designer, creator, retry-creator, checker, retry-checker, and reviewer artifacts listed in the read set
- Required environment input:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/TESTING_GUIDE.md`

## Semantic Prior Inputs

- Use only prior-derived semantic constraints from the accepted component artifacts.

## Local Architectural Prior Inputs

- Use the accepted component artifacts as the final-check contract.
- Local focus:
  - confirm that the canonical `fileset.rs` consumer path remains table-backed,
  - confirm the reviewer report's only blocker is resolved by the current code and retry evidence.

## Quality Prior Inputs

- Use `Q-CHECK`
- In scope:
  - confirming the reviewed implementation still passes the focused local `fileset` ktests,
  - recording exact execution evidence,
  - noting that the reviewer report's sole finding has been cleared if the rerun passes.
- Out of scope:
  - broad new test planning,
  - redesign.

## Prior Delivery Notes

- This is a rerun-only final checker unless the prior reviewer or retry-checker artifacts imply a missing final-check obligation.
- Do not reopen the separately queued post-loop quality topics.

## Temporary Interfaces And Exit Plan

- The ktest-only fileset helpers remain temporary staging surfaces; no new temporary interface is authorized in this final pass.

## Helper Justification

- No new helper is authorized.

## Allowed Commands

- `.agents/tools/checker_lock.sh acquire ...`
- `.agents/tools/checker_lock.sh release`
- `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test fileset::tests'`

## Parallelism Classification

- Lane class:
  - runtime/test-producing
- May overlap with:
  - command-free lanes with disjoint write sets only
- Known conflicts:
  - all other command-producing delegated work in the shared container

## Execution Environment

- Host or Docker:
  - Docker container `codex-asterinas-dev`
- Required command prefix:
  - `docker exec codex-asterinas-dev bash -lc`
- Required working directory:
  - `/root/asterinas/kernel`
- Isolation notes:
  - shared container, `no-kvm`, sequential command lane only

## Execution Lock

- Lock script:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/tools/checker_lock.sh`
- Lock path:
  - `.agents/locks/checker-execution.lock/`
- Lock metadata file:
  - `owner.toml`
- This checker stage must hold the execution lock.
- Acquire command shape:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/tools/checker_lock.sh acquire --component EXR-UPCASE-07B --phase final-checker --command "docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test fileset::tests'" --retry-seconds 60 --wait-budget-seconds 1800`
- Quiet-wait retry interval:
  - `60` seconds
- Maximum wait budget before reporting back:
  - `1800` seconds
- Stale-lock review remains reserved to the main agent.

## Stop Condition

- Stop after rerunning the focused local `fileset` ktests and writing `31_checker_final.md`.
- Do not write reviewer notes, creator repairs, or board updates.

## Escalation Rule

- If the post-review rerun fails, record the concrete failing evidence in `31_checker_final.md` and stop.
