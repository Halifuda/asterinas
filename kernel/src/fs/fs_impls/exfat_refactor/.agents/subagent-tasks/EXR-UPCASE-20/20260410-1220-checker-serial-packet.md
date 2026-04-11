<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-UPCASE-20-20260410-1220-CHECK-SERIAL`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-UPCASE-20/20260410-1220-checker-serial-packet.md`
- Supersedes: None
- Role: `checker`
- Component: `EXR-UPCASE-20`
- Phase: `serial checker`
- Authorizing main agent: `main-agent`
- Date: `2026-04-10 12:20 CST`

## Goal

- Validate the new owner-local upcase-table state and canonicalization services in `fs.rs`, add the required local ktests, run filtered executable verification under the checker lock, and write the checker report.

## Architectural Unit Context

- Functional goal: `ExfatFs`-owned validated upcase table plus UTF-16 folding and exFAT name hashing
- Final architectural owner: `ExfatFs`
- Expected landing form: owner-private `UpcaseTable` state and owner methods in `fs.rs`
- Parent unit: `EXR-DIR-ENGINE-19`

## Required Resolution Questions

- Verify valid and invalid upcase-table publication are distinguished.
- Verify folding uses the installed volume table and remains deterministic.
- Verify name hashing is computed from folded UTF-16 bytes.
- Verify the same installed table is the sole source of canonicalization.
- If compile/test failure is strictly local to `fs.rs`, make a minimal in-scope fix and record it.

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/CHECKER_REPORT_TEMPLATE.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-20/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-20/02_designer_async.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-20/03_designer_ktest.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-20/10_creator_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/dentry.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/test_support.rs`

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-20/11_checker_serial.md`

## Forbidden Files

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/bitmap.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/`
- Sibling component artifacts.

## Required Inputs

- Skill to use: `$exfat-subagent-workflow`
- Required role reference: `references/checker.md`
- Also read: `references/testing-guide.md`

## Semantic Prior Inputs

- Use the accepted designer constraints only. Do not reopen Linux or Microsoft exFAT behavior beyond the designer-approved upcase-table and folded-name-hash semantics.

## Integration Prior Inputs

- Treat the `EXR-INODE-CACHE-18` owner-private cache boundary as already accepted background in the same `fs.rs` owner.
- Do not widen this checker into directory discovery, bitmap ownership, or mount/open sequencing.

## Workflow Prior Inputs

- Runtime/test-producing checker lane
- Command-producing verification must hold the checker execution lock
- Temporary ktest-local debug output is allowed only if needed to surface a failure and must be removed before stopping unless the final artifact records why it remains
- If the standard filtered run does not surface enough evidence, debug-oriented reruns of the same filtered tests are allowed

## Quality Prior Inputs

- Use the checker-role quality slice from `$exfat-subagent-workflow` and the local designer ktest obligations.

## Temporary Interfaces And Exit Plan

- Keep the upcase table immutable after publication.
- Do not add a generic text helper module, fallback locale table, or mount/open sequencing shell.

## Helper Justification

- Local helpers in `fs.rs` are justified only when they keep validation, folding, and hashing readable and owner-private.

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
  - `fs.rs`

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
  - `bash ./kernel/src/fs/fs_impls/exfat_refactor/.agents/tools/checker_lock.sh acquire --component EXR-UPCASE-20 --phase serial --command "<exact docker commands actually used>" --retry-seconds 60 --wait-budget-seconds 1800`
- Release with:
  - `bash ./kernel/src/fs/fs_impls/exfat_refactor/.agents/tools/checker_lock.sh release`
- Stale-lock review is main-agent-only.

## Stop Condition

- Stop after writing `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-20/11_checker_serial.md`

## Escalation Rule

- If the checker needs edits outside `fs.rs` or cannot get trustworthy evidence from the allowed filtered commands, report that and stop.
