<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-UPCASE-07B-CHECK-RETRY-20260404-1605`
- Packet file: `kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-UPCASE-07B/20260404-1605-checker-retry-packet.md`
- Supersedes:
  - `EXR-UPCASE-07B-CHECK-20260404-1517`
- Role: checker
- Component: `EXR-UPCASE-07B`
- Phase: serial checker retry
- Authorizing main agent: main-agent
- Date: 2026-04-04 16:05 CST

## Goal

- Re-check `EXR-UPCASE-07B` after the narrow `fileset.rs` repair. Confirm that the canonical consumer path now validates `NameHash` through `ExfatUpcaseTable` instead of a raw UTF-16 checksum, and record the retry evidence in `13_checker_serial_retry.md`. Keep the pass tightly scoped to `fileset.rs`.

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
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07B/30_reviewer_report.md`
- `kernel/src/fs/fs_impls/exfat_refactor/fileset.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/upcase_table.rs`

## Write Set

- `kernel/src/fs/fs_impls/exfat_refactor/fileset.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-07B/13_checker_serial_retry.md`

## Forbidden Files

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/*`
- existing architect, designer, creator, reviewer, and prior checker artifacts for `EXR-UPCASE-07B`
- all files outside the write set unless a test-local edit in `fileset.rs` is truly required by this packet

## Required Inputs

- Role protocol files accompanying this packet:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/CHECKER.md`
- Required artifact inputs:
  - architect, designer, creator, reviewer, and prior checker artifacts listed in the read set
- Required environment input:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/TESTING_GUIDE.md`

## Semantic Prior Inputs

- Use only prior-derived semantic constraints from the architect and designer artifacts.
- Semantic focus:
  - `fileset.rs` must validate `stream_dentry.name_hash` through the canonical table-backed service,
  - raw UTF-16 checksum behavior is not the canonical `07B` contract,
  - structure-only local tests may remain separate from the canonical validation boundary if that split is explicit and test-only.

## Local Architectural Prior Inputs

- Use integration constraints derived by the architect, designer, repair creator, and reviewer artifacts.
- Local focus:
  - this retry exists to verify the repaired `fileset.rs` boundary, not to reopen `upcase_table.rs` design or mount work,
  - the reviewer report's sole blocking finding should now be either cleared or restated with concrete evidence.

## Quality Prior Inputs

- Use `Q-CHECK`
- In scope:
  - focused regression evidence on the repaired boundary,
  - surfacing any remaining in-scope blocking defect,
  - ensuring temporary ktest-only helpers stay explicitly temporary.
- Out of scope:
  - broad quality cleanup,
  - the separately queued post-loop accessor/free-function review topics.

## Prior Delivery Notes

- Prefer no code changes if the repaired `fileset.rs` boundary already compiles and the existing local tests are sufficient.
- Add only a truly necessary local `#[ktest]` if the retry would otherwise miss the repaired boundary.

## Temporary Interfaces And Exit Plan

- The ktest-only `new_structure_only()` helper and raw metadata builder may remain only as explicitly temporary test support until a later write-side owner absorbs or removes them.
- If the checker touches those helpers, the retry artifact must restate that temporary role.

## Helper Justification

- No new production helper is authorized.
- Test-local edits are allowed only if the retry exposes a missing regression in `fileset.rs`.

## Allowed Commands

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/tools/checker_lock.sh acquire ...`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/tools/checker_lock.sh release`
- `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test fileset::tests'`

## Parallelism Classification

- Lane class:
  - runtime/test-producing
- May overlap with:
  - no other command-producing delegated work
- Known conflicts:
  - all other command-producing delegated work in the shared container
  - any lane writing `fileset.rs` or the retry checker artifact

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
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/tools/checker_lock.sh acquire --component EXR-UPCASE-07B --phase checker-serial-retry --command "docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test fileset::tests'" --retry-seconds 60 --wait-budget-seconds 1800`
- Quiet-wait retry interval:
  - `60` seconds
- Maximum wait budget before reporting back:
  - `1800` seconds
- Stale-lock review remains reserved to the main agent.

## Stop Condition

- Stop after:
  - rerunning the focused checker command under the lock,
  - applying only any truly necessary local test edit in `fileset.rs`,
  - and writing `13_checker_serial_retry.md`.
- Do not write reviewer or final-checker artifacts.

## Escalation Rule

- If a remaining blocker now lies outside the write set, report it in `13_checker_serial_retry.md` instead of editing around it.
