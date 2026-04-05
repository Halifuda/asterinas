<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-PGCACHE-11B-CHECK-20260405-1244`
- Packet file: `kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-PGCACHE-11B/20260405-1244-checker-retry-packet.md`
- Supersedes:
  - `EXR-PGCACHE-11B-CHECK-20260405-1216`
- Role: checker
- Component: `EXR-PGCACHE-11B`
- Phase: serial checker retry
- Authorizing main agent: main-agent
- Date: 2026-04-05 12:44 CST

## Goal

- Re-run the serial checker for `EXR-PGCACHE-11B` after the bounded creator repair pass. Reuse the existing checker-owned local ktests, prove filtered-test hit coverage, and record the retry result in `13_checker_serial_retry.md`.

## Read Set

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/README.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/TESTING_GUIDE.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/CHECKER_REPORT_TEMPLATE.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/CHECKER.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-11B/00_architect.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-11B/01_designer_core.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-11B/03_designer_ktest.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-11B/11_checker_serial.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-11B/12_creator_serial_retry.md`
- `kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/read.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- `kernel/src/fs/vfs/page_cache.rs`
- `osdk/deps/test-kernel/src/lib.rs`
- `ostd/libs/ostd-test/src/lib.rs`

## Write Set

- `kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-PGCACHE-11B/13_checker_serial_retry.md`

## Forbidden Files

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/*`
- creator, reviewer, and final-checker artifacts other than the required read-only creator and prior checker artifacts
- production sections of `read.rs` and `mod.rs`

## Required Inputs

- Role protocol files accompanying this packet:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/COMMON_SUBAGENT.md`
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/protocol/CHECKER.md`
- Required artifact/style inputs:
  - `kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/CHECKER_REPORT_TEMPLATE.md`
  - repair creator artifact `12_creator_serial_retry.md`
  - prior checker report `11_checker_serial.md`
- Filter-hit proof requirements:
  - use these exact ktest names:
    - `backend_page_count_tracks_visible_length`
    - `contiguous_page_read_uses_mapping_boundary`
    - `fat_backed_page_read_uses_mapping_boundary`
    - `out_of_range_pages_stay_zero_backed`
    - `backend_contract_stays_out_of_buffered_read`
  - record in the artifact where these exact suffixes are defined in the checked sources, and note that `cargo osdk test` matches test-path suffixes

## Semantic Prior Inputs

- Use only the semantic constraints already captured in the architect, designer, and checker artifacts.

## Local Architectural Prior Inputs

- Use the local constraints already captured in the architect, designer, and checker artifacts.
- Local focus:
  - visible-length page count
  - `EXR-READ-11A` placement ownership
  - no buffered-read or write-growth drift

## Quality Prior Inputs

- Use `Q-CHECK` from `kernel/src/fs/fs_impls/exfat_refactor/.agents/ASTERINAS_CODE_QUALITY_PRIORS.md`
- In scope:
  - checker-owned local ktests
  - exact hit-proof evidence
  - rerun verification after the bounded creator repair
- Out of scope:
  - reviewer-level cleanup

## Prior Delivery Notes

- The checker-owned local ktests already exist in `fs.rs`. Reuse them unless the repair invalidated a fixture in a checker-owned way.
- Do not alter production behavior unless the packet is explicitly revised.

## Temporary Interfaces And Exit Plan

- No temporary interface is authorized.

## Helper Justification

- The checker does not have authority to add new production helpers in this pass.

## Allowed Commands

- `kernel/src/fs/fs_impls/exfat_refactor/.agents/tools/checker_lock.sh acquire --component EXR-PGCACHE-11B --phase checker-serial-retry --command "docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test backend_page_count_tracks_visible_length && cargo osdk test contiguous_page_read_uses_mapping_boundary && cargo osdk test fat_backed_page_read_uses_mapping_boundary && cargo osdk test out_of_range_pages_stay_zero_backed && cargo osdk test backend_contract_stays_out_of_buffered_read'" --retry-seconds 60 --wait-budget-seconds 1800`
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

- Stop after any authorized checker-owned ktest edits and after writing `13_checker_serial_retry.md`.
- Do not write reviewer, final-checker, or task-board artifacts.

## Escalation Rule

- If the repair still leaves a production-code failure, report it as a checker finding instead of patching around it.
