<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-UPCASE-07A-CHECK-20260404-1435`
- Packet file: `kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-UPCASE-07A/20260404-1435-checker-packet.md`
- Supersedes: none
- Role: checker
- Component: `EXR-UPCASE-07A`
- Phase: serial checker
- Authorizing main agent: main-agent
- Date: 2026-04-04 14:35 CST

## Goal

- Validate the staged `EXR-UPCASE-07A` loader by adding the minimum local `#[ktest]` coverage in `upcase_table.rs`, then run the smallest relevant filtered test command under the checker execution lock. Record exact evidence in `11_checker_serial.md`.

## Read Set

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/CHECKER.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/TESTING_GUIDE.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07A/00_architect.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07A/01_designer_core.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07A/03_designer_ktest.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07A/10_creator_serial.md`
- `kernel/src/fs/fs_impls/exfat_refactor/upcase_table.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/sysroot.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/test_support.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/fat.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/io.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/super_block.rs`

## Write Set

- `kernel/src/fs/fs_impls/exfat_refactor/upcase_table.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07A/11_checker_serial.md`

## Forbidden Files

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/*`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07A/00_architect.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07A/01_designer_core.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07A/03_designer_ktest.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07A/10_creator_serial.md`
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
  - tests stay local to `upcase_table.rs`
  - no mount bootstrap, fallback policy, name hashing, or case-folding behavior

## Quality Prior Inputs

- Use `Q-CHECK`
- In scope:
  - targeted regression coverage
  - execution evidence
  - surfacing real boundary defects
- Out of scope:
  - broad style review beyond what blocks acceptance

## Prior Delivery Notes

- Keep checker edits local to `upcase_table.rs`.
- Prefer the smallest filtered ktest suffix that still cleanly targets this component.
- Add only the minimum test scaffolding needed to exercise the required scenarios.

## Temporary Interfaces And Exit Plan

- No temporary interface is authorized.

## Helper Justification

- No new production helper is authorized.
- Test-local helpers inside `#[cfg(ktest)] mod tests` are allowed when they stay local to `upcase_table.rs` and exist only to build the required loader fixtures.

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
  - any lane writing `upcase_table.rs` or the `EXR-UPCASE-07A` checker artifact

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
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/tools/checker_lock.sh acquire --component EXR-UPCASE-07A --phase checker-serial --command "docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test upcase_table::tests'" --retry-seconds 60 --wait-budget-seconds 1800`
- Quiet-wait retry interval:
  - `60` seconds
- Maximum wait budget before reporting back:
  - `1800` seconds
- Stale-lock review remains reserved to the main agent.

## Stop Condition

- Stop after:
  - adding any needed local `#[ktest]` coverage in `upcase_table.rs`,
  - running the focused checker command under the lock,
  - and writing `11_checker_serial.md`.
- Do not perform creator repairs beyond test-only edits and do not write reviewer artifacts.

## Escalation Rule

- If production code requires a fix beyond test-only edits, stop and report the blocking defect in `11_checker_serial.md` instead of silently repairing it.
