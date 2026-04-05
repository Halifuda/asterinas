<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-PGCACHE-11B-FINAL-20260405-1216`
- Packet file: `kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-PGCACHE-11B/20260405-1216-final-checker-packet.md`
- Supersedes: none
- Role: checker
- Component: `EXR-PGCACHE-11B`
- Phase: post-review final
- Authorizing main agent: main-agent
- Date: 2026-04-05 12:16 CST

## Goal

- Re-run the post-review final checker for `EXR-PGCACHE-11B` after the delegated reviewer pass. Use the same exact local ktest names, prove the filter hit coverage, and record the post-review result in `31_checker_final.md`.

## Read Set

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/README.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/TESTING_GUIDE.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/CHECKER_REPORT_TEMPLATE.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/CHECKER.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-11B/10_creator_serial.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-11B/30_reviewer_report.md`
- `kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/read.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- `osdk/deps/test-kernel/src/lib.rs`
- `ostd/libs/ostd-test/src/lib.rs`

## Write Set

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-11B/31_checker_final.md`

## Forbidden Files

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/*`
- all production and test code

## Required Inputs

- Role protocol files accompanying this packet:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/CHECKER.md`
- Required artifact/style inputs:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/CHECKER_REPORT_TEMPLATE.md`
  - delegated creator artifact `10_creator_serial.md`
  - delegated reviewer artifact `30_reviewer_report.md`
- Filter-hit proof requirements:
  - use these exact ktest names:
    - `backend_page_count_tracks_visible_length`
    - `contiguous_page_read_uses_mapping_boundary`
    - `fat_backed_page_read_uses_mapping_boundary`
    - `out_of_range_pages_stay_zero_backed`
    - `backend_contract_stays_out_of_buffered_read`
  - record in the artifact where these exact suffixes are defined in the checked sources, and note that `cargo osdk test` matches test-path suffixes

## Semantic Prior Inputs

- Use only the semantic constraints already captured in the architect and designer artifacts.

## Local Architectural Prior Inputs

- Use the local constraints already captured in the architect and designer artifacts.

## Quality Prior Inputs

- Use `Q-CHECK` from `kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_CODE_QUALITY_PRIORS.md`
- In scope:
  - post-review exact-hit rerun evidence
  - confirmation that the reviewed code still matches the narrow backend boundary
- Out of scope:
  - new review work

## Prior Delivery Notes

- No code edits are authorized in this final checker pass.

## Temporary Interfaces And Exit Plan

- No temporary interface is authorized.

## Helper Justification

- The final checker does not have authority to add helpers in this pass.

## Allowed Commands

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/tools/checker_lock.sh acquire --component EXR-PGCACHE-11B --phase final-checker --command "docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test backend_page_count_tracks_visible_length && cargo osdk test contiguous_page_read_uses_mapping_boundary && cargo osdk test fat_backed_page_read_uses_mapping_boundary && cargo osdk test out_of_range_pages_stay_zero_backed && cargo osdk test backend_contract_stays_out_of_buffered_read'" --retry-seconds 60 --wait-budget-seconds 1800`
- `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test backend_page_count_tracks_visible_length && cargo osdk test contiguous_page_read_uses_mapping_boundary && cargo osdk test fat_backed_page_read_uses_mapping_boundary && cargo osdk test out_of_range_pages_stay_zero_backed && cargo osdk test backend_contract_stays_out_of_buffered_read'`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/tools/checker_lock.sh release`

## Parallelism Classification

- Lane class:
  - runtime/test-producing
- May overlap with:
  - no other delegated lane for this component
- Known conflicts:
  - all command-producing work in the shared container

## Execution Environment

- Host or Docker:
  - host shell invoking Docker container `codex-asterinas-dev`
- Required command prefix:
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test ...'`
- Required working directory:
  - `/home/halifuda/asterinas`
- Isolation notes:
  - shared container and shared build state; commands must run serially
- If the task includes filtered tests, the checker must capture the exact suffix proof listed above.

## Execution Lock

- Lock script:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/tools/checker_lock.sh`
- Lock path:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/locks/checker-execution.lock/`
- Lock metadata file:
  - `owner.toml`
- This checker stage must hold the execution lock.
- Use the exact acquire command shape listed above.
- Retry interval:
  - `60` seconds
- Maximum wait budget:
  - `1800` seconds
- Stale-lock review remains reserved to the main agent.

## Stop Condition

- Stop after writing `31_checker_final.md`.
- Do not edit code or task-board state.

## Escalation Rule

- If the post-review rerun exposes a real failure, report it cleanly instead of attempting a silent repair in this pass.
