<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-UPCASE-07A-CHECK-RETRY-20260404-1438`
- Packet file: `kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-UPCASE-07A/20260404-1438-checker-retry-packet.md`
- Supersedes:
  - `EXR-UPCASE-07A-CHECK-20260404-1435`
- Role: checker
- Component: `EXR-UPCASE-07A`
- Phase: serial checker retry
- Authorizing main agent: main-agent
- Date: 2026-04-04 14:38 CST

## Goal

- Re-run the focused `EXR-UPCASE-07A` checker command now that the shared compile blocker in `bitmap.rs` has been cleared. Do not add new tests unless a new blocker inside the assigned write set appears. Record the retry evidence in `12_checker_serial_retry.md`.

## Read Set

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/CHECKER.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/TESTING_GUIDE.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07A/00_architect.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07A/01_designer_core.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07A/03_designer_ktest.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07A/10_creator_serial.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07A/11_checker_serial.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BITMAP-08A/11_checker_serial.md`
- `kernel/src/fs/fs_impls/exfat_refactor/upcase_table.rs`

## Write Set

- `kernel/src/fs/fs_impls/exfat_refactor/upcase_table.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07A/12_checker_serial_retry.md`

## Forbidden Files

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/*`
- existing architect, designer, creator, and prior checker artifacts for `EXR-UPCASE-07A`
- all files outside the write set unless a test-only edit is explicitly required by this packet

## Required Inputs

- Role protocol files accompanying this packet:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/CHECKER.md`
- Required artifact inputs:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07A/00_architect.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07A/01_designer_core.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07A/03_designer_ktest.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07A/10_creator_serial.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07A/11_checker_serial.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-BITMAP-08A/11_checker_serial.md`
- Required environment input:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/TESTING_GUIDE.md`

## Semantic Prior Inputs

- Use only prior-derived semantic constraints from the architect and designer artifacts.
- Semantic focus:
  - canonical on-disk upcase-table loading
  - size and checksum validation
  - rejecting malformed or truncated payloads
  - preserving the full loaded payload without widening into case folding or hashing

## Local Architectural Prior Inputs

- Use integration constraints derived by the architect and designer artifacts.
- Local focus:
  - treat this pass as execution-evidence repair only unless a new in-scope blocker appears
  - no mount bootstrap, fallback policy, name hashing, or case-folding behavior

## Quality Prior Inputs

- Use `Q-CHECK`
- In scope:
  - focused rerun evidence
  - surfacing any remaining in-scope blocking defect
- Out of scope:
  - broad style review beyond what blocks acceptance

## Prior Delivery Notes

- Prefer no code changes if the existing test block now compiles and runs.
- Keep the retry artifact explicit about the earlier external blocker and the new execution result.

## Temporary Interfaces And Exit Plan

- No temporary interface is authorized.

## Helper Justification

- No new production helper is authorized.
- Test-local edits are allowed only if the retry exposes a new defect inside `upcase_table.rs`.

## Allowed Commands

- `.agents/tools/checker_lock.sh acquire ...`
- `.agents/tools/checker_lock.sh release`
- `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test upcase_table::tests'`

## Parallelism Classification

- Lane class:
  - runtime/test-producing
- May overlap with:
  - command-free lanes with disjoint write sets only
- Known conflicts:
  - all other command-producing delegated work in the shared container
  - any lane writing `upcase_table.rs` or the retry checker artifact

## Execution Environment

- Host or Docker:
  - Docker container `codex-asterinas-dev`
- Required command prefix:
  - `docker exec codex-asterinas-dev bash -lc`
- Required working directory:
  - `/root/asterinas/kernel`
- Isolation notes:
  - shared container, `no-kvm`, sequential command lane only
- This task must run serially with respect to other command-producing work.

## Execution Lock

- Lock script:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/tools/checker_lock.sh`
- Lock path:
  - `.agents/locks/checker-execution.lock/`
- Lock metadata file:
  - `owner.toml`
- This checker stage must hold the execution lock.
- Acquire command shape:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/tools/checker_lock.sh acquire --component EXR-UPCASE-07A --phase checker-serial-retry --command "docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test upcase_table::tests'" --retry-seconds 60 --wait-budget-seconds 1800`
- Quiet-wait retry interval:
  - `60` seconds
- Maximum wait budget before reporting back:
  - `1800` seconds
- Stale-lock review remains reserved to the main agent.

## Stop Condition

- Stop after:
  - rerunning the focused checker command under the lock,
  - applying only any truly necessary in-scope test-local fix,
  - and writing `12_checker_serial_retry.md`.
- Do not write reviewer artifacts.

## Escalation Rule

- If a remaining blocker now lies outside the write set, report it in `12_checker_serial_retry.md` instead of editing around it.
