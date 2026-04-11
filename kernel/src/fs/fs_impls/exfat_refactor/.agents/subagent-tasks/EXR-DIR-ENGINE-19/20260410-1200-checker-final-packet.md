<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-DIR-ENGINE-19-20260410-1200-CHECK-FINAL`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-DIR-ENGINE-19/20260410-1200-checker-final-packet.md`
- Supersedes: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-DIR-ENGINE-19/20260410-1140-checker-retry-packet.md`
- Role: `checker`
- Component: `EXR-DIR-ENGINE-19`
- Phase: `final checker`
- Authorizing main agent: `main-agent`
- Date: `2026-04-10 12:00 CST`

## Goal

- Validate the reviewer hardening in `directory.rs`, add a local regression if needed, rerun the directory-engine filtered ktests under the checker lock, and write the final checker report.

## Architectural Unit Context

- Functional goal: `ExfatFs`-owned internal read-only directory record stream
- Final architectural owner: `ExfatFs`
- Expected landing form: owner-internal `DirectoryEngine` service in `directory.rs`
- Parent units: `EXR-IO-02`, `EXR-CHAIN-03B`, `EXR-FILESET-04B`

## Required Resolution Questions

- Verify unexpected top-level dentries now fail instead of surfacing as generic singleton candidates.
- Keep the owner-internal service read-only and within the designer boundary.
- Re-run filtered executable verification after any checker-owned local regression addition.
- If another compile/test failure is strictly local to `directory.rs` or `mod.rs`, make the smallest in-scope fix and record it.

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/CHECKER_REPORT_TEMPLATE.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-ENGINE-19/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-ENGINE-19/03_designer_ktest.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-ENGINE-19/10_creator_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-ENGINE-19/11_checker_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-ENGINE-19/12_checker_serial_retry.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-ENGINE-19/30_reviewer_report.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/dentry.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fileset.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/test_support.rs`

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-ENGINE-19/31_checker_final.md`

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

## Semantic Prior Inputs

- Use the accepted designer constraints only. Do not reopen Linux or Microsoft exFAT behavior.

## Integration Prior Inputs

- Treat the reviewer edit as a local hardening change only; do not widen the service back into generic singleton surfacing.

## Workflow Prior Inputs

- Runtime/test-producing checker lane
- Command-producing verification must hold the checker execution lock
- Add a local regression in `directory.rs` if needed to prove the reviewer hardening
- Temporary ktest-local debug output is allowed only if needed to surface a failure and must be removed before stopping unless the final artifact records why it remains

## Quality Prior Inputs

- Use the checker-role quality slice from `$exfat-subagent-workflow` and the local designer ktest obligations.

## Temporary Interfaces And Exit Plan

- Preserve `DirectoryEngine` as a read-only service.
- Do not widen this final check into name policy, bitmap policy, or write-side mutation.

## Helper Justification

- Helper changes are justified only when they keep malformed top-level dentry handling local to `DirectoryEngine`.

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
  - `bash ./kernel/src/fs/fs_impls/exfat_refactor/.agents/tools/checker_lock.sh acquire --component EXR-DIR-ENGINE-19 --phase final --command "<exact docker commands actually used>" --retry-seconds 60 --wait-budget-seconds 1800`
- Release with:
  - `bash ./kernel/src/fs/fs_impls/exfat_refactor/.agents/tools/checker_lock.sh release`
- Stale-lock review is main-agent-only.

## Stop Condition

- Stop after writing `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-ENGINE-19/31_checker_final.md`

## Escalation Rule

- If the checker needs edits outside `directory.rs` or `mod.rs` or cannot get trustworthy evidence from the allowed filtered commands, report that and stop.
