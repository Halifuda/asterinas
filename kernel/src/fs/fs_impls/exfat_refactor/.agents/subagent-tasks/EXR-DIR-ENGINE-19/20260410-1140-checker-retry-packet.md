<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-DIR-ENGINE-19-20260410-1140-CHECK-RETRY`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-DIR-ENGINE-19/20260410-1140-checker-retry-packet.md`
- Supersedes: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-DIR-ENGINE-19/20260410-1112-checker-serial-packet.md`
- Role: `checker`
- Component: `EXR-DIR-ENGINE-19`
- Phase: `serial checker retry`
- Authorizing main agent: `main-agent`
- Date: `2026-04-10 11:40 CST`

## Goal

- Re-run the `EXR-DIR-ENGINE-19` checker after the sibling `fs.rs` repair so the local directory-engine tests can reach executable evidence.

## Architectural Unit Context

- Functional goal: `ExfatFs`-owned internal directory record stream
- Final architectural owner: `ExfatFs`
- Expected landing form: owner-internal `DirectoryEngine` service in `directory.rs`
- Parent units: `EXR-IO-02`, `EXR-CHAIN-03B`, `EXR-FILESET-04B`

## Required Resolution Questions

- Verify the crate now compiles far enough for the `directory_engine_` checker tests to execute.
- Verify record emission preserves ordering and boundaries across read and cluster windows.
- Verify valid file records become validated `ExfatDentrySet` values.
- Verify singleton `Bitmap` and `Upcase` entries surface without policy interpretation.
- Verify tombstones are skipped and `Unused` terminates the scan.
- If another compile/test failure is strictly local to `directory.rs` or `mod.rs`, make the smallest in-scope fix and record it.

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/CHECKER_REPORT_TEMPLATE.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-ENGINE-19/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-ENGINE-19/03_designer_ktest.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-ENGINE-19/10_creator_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-ENGINE-19/11_checker_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/io.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fat.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fileset.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/dentry.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/super_block.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/test_support.rs`

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-ENGINE-19/12_checker_serial_retry.md`

## Forbidden Files

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/`
- Sibling component artifacts.

## Required Inputs

- Skill to use: `$exfat-subagent-workflow`
- Required role reference: `references/checker.md`
- Also read: `references/testing-guide.md`
- Checker report template and prior creator/checker artifacts listed above

## Semantic Prior Inputs

- Use the accepted designer constraints only. Do not reopen Linux or Microsoft exFAT behavior.

## Integration Prior Inputs

- Treat the local directory-engine checker tests as already landed. This retry is for validation after the sibling `fs.rs` repair, not for widening directory scope.

## Workflow Prior Inputs

- Runtime/test-producing checker lane
- Command-producing verification must hold the checker execution lock
- Temporary ktest-local debug output is allowed only if needed to surface a failure and must be removed before stopping unless the final artifact records why it remains

## Quality Prior Inputs

- Use the checker-role quality slice from `$exfat-subagent-workflow` and the local designer ktest obligations.

## Temporary Interfaces And Exit Plan

- Preserve `DirectoryEngine` as a read-only service.
- Do not widen this checker into name policy, bitmap policy, or write-side mutation.

## Helper Justification

- Helper methods are justified only when they keep record-stream parsing local to `DirectoryEngine`.

## Allowed Commands

- Read-only shell inspection commands under `/home/halifuda/asterinas`
- Lock-guarded Docker commands in `codex-asterinas-dev`:
  - `docker exec codex-asterinas-dev bash -lc 'test -e /dev/kvm && ls -l /dev/kvm || echo no-kvm'`
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test <exact-local-ktest-suffix>'`
- If the standard filtered run does not surface enough evidence, one or more additional debug-oriented reruns of the same filtered tests are allowed with extra verbosity or other non-scope-widening `cargo osdk test` flags. Record the exact command if used.

## Parallelism Classification

- Lane class: `runtime/test-producing`
- May overlap with command-free lanes only
- Known conflicts:
  - serialized checker command lane
  - `directory.rs`
  - `mod.rs`

## Execution Environment

- Host and Docker
- Required command prefix:
  - `docker exec codex-asterinas-dev bash -lc`
- Required working directory:
  - `/root/asterinas/kernel`

## Execution Lock

- Lock script:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/tools/checker_lock.sh`
- Lock path:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/locks/checker-execution.lock/`
- Lock metadata file:
  - `owner.toml`
- Acquire with:
  - `bash ./kernel/src/fs/fs_impls/exfat_refactor/.agents/tools/checker_lock.sh acquire --component EXR-DIR-ENGINE-19 --phase serial-retry --command "<exact docker commands actually used>" --retry-seconds 60 --wait-budget-seconds 1800`
- Release with:
  - `bash ./kernel/src/fs/fs_impls/exfat_refactor/.agents/tools/checker_lock.sh release`
- Stale-lock review is main-agent-only.

## Stop Condition

- Stop after writing `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-ENGINE-19/12_checker_serial_retry.md`

## Escalation Rule

- If the checker needs edits outside `directory.rs` or `mod.rs` or still cannot get trustworthy evidence from the allowed filtered commands, report that and stop.
