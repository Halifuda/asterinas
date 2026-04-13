<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-CHARSET-32-20260413-1409-CHECK-SERIAL`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-CHARSET-32/20260413-1409-checker-serial-packet.md`
- Supersedes: None
- Role: `checker`
- Component: `EXR-CHARSET-32`
- Phase: `serial checker`
- Authorizing main agent: `main-agent`
- Date: `2026-04-13 14:09 CST`

## Goal

- Validate the new `ExfatFs`-owned charset boundary in `fs.rs`, add the required local ktests, prove that accepted read-side `lookup` / `readdir_at` consumers still behave correctly after migrating off local conversion policy in `inode.rs`, run filtered executable verification under the checker lock, and write the checker report.

## Architectural Unit Context

- Functional goal: `ExfatFs`-owned charset and visible-name conversion boundary
- Final architectural owner: `ExfatFs`
- Expected landing form: owner-private converted-name / converted-label value types and owner methods in `fs.rs`, with narrow read-side consumer migration in `inode.rs`
- Parent units:
  - `EXR-FS-OPEN-22`
  - `EXR-UPCASE-20`
  - accepted `EXR-DIR-OPS-23` as a consumer surface

## Required Resolution Questions

- Verify valid external names convert to validated UTF-16 converted-name values.
- Verify valid external labels convert to validated UTF-16 converted-label values.
- Verify malformed or overlong input is rejected before publication.
- Verify repeated conversion is deterministic for the same mounted filesystem state.
- Verify visible-name decode for validated UTF-16 units is owned by `ExfatFs` and rejects malformed UTF-16.
- Verify accepted read-side `lookup` and `readdir_at` behavior still passes after the `inode.rs` consumer migration.
- If a compile/test failure is strictly local to `fs.rs` or `inode.rs`, make the smallest in-scope fix and record it.

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/CHECKER_REPORT_TEMPLATE.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-CHARSET-32/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-CHARSET-32/02_designer_async.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-CHARSET-32/03_designer_ktest.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-CHARSET-32/10_creator_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-DIR-OPS-23/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-UPCASE-20/11_checker_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/test_support.rs`
- `/home/halifuda/asterinas/kernel/src/fs/vfs/fs_apis/inode.rs`

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-CHARSET-32/11_checker_serial.md`

## Forbidden Files

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fileset.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/mod.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/COMPONENT_INDEX.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent/`
- sibling component artifacts
- reviewer, advisor, and handoff artifacts

## Required Inputs

- Skill to use: `$exfat-subagent-workflow`
- Required role reference: `references/checker.md`
- Also read: `references/testing-guide.md`

## Semantic Prior Inputs

- Use the accepted `EXR-CHARSET-32` designer constraints only.
- `EXR-CHARSET-32` owns visible-name encode and decode policy for exFAT; `EXR-UPCASE-20` remains the sole fold/hash owner.
- Accepted read-side `lookup` and `readdir_at` remain `EXR-DIR-OPS-23` behavior; this checker validates the consumer migration, not a read-side redesign.
- Low-level raw UTF-16 leaf seams remain out of scope unless a strictly local checker fix requires touching a call site already inside `fs.rs` or `inode.rs`.

## Integration Prior Inputs

- Add local ktests in `fs.rs` for the charset owner boundary. Prefer new names beginning with `charset_`.
- Reuse the existing `inode.rs` lookup and readdir regressions as executable proof that read-side consumers still behave correctly after migration:
  - `lookup_reuses_the_canonical_child_handle_for_case_equivalent_names`
  - `lookup_miss_does_not_publish_a_synthetic_child_handle`
  - `readdir_emits_visible_entries_in_stable_order`
  - `readdir_continuation_remains_stable_across_repeated_calls`
- If those existing tests need narrow updates to prove the new owner boundary or to keep filtered evidence readable, keep edits local to `inode.rs`.
- Do not widen this checker into namespace mutation, volume-label mutation, directory-entry writes, or sync ordering.

## Workflow Prior Inputs

- Runtime/test-producing checker lane.
- Command-producing verification must hold the checker execution lock.
- Prefer exact or uniquely filtered test names for final proof.
- If a nonzero exit does not clearly show the guest-side failure in terminal output, inspect `/home/halifuda/asterinas/qemu-serial.log` before classifying the result.

## Quality Prior Inputs

- Use the checker-role quality slice from `$exfat-subagent-workflow` and the local designer ktest obligations.
- Prefer test-only edits unless a strictly local production fix in `fs.rs` or `inode.rs` is necessary to satisfy the designer contract.

## Temporary Interfaces And Exit Plan

- Keep converted-name, converted-label, and visible-name decode helpers owner-private to `ExfatFs`.
- Do not add a generic Unicode helper module, second text subsystem, or public accessor surface.
- If checker keeps any helper or temporary seam, record why it is acceptable for now and what later owner or cleanup step should absorb it.

## Helper Justification

- New checker-local helpers are justified only when they keep charset boundary ktests readable in `fs.rs` or keep source-backed filtered evidence readable in `inode.rs`.

## Allowed Commands

- Read-only shell inspection commands under `/home/halifuda/asterinas`
- Lock-guarded Docker commands in `codex-asterinas-dev`:
  - `docker exec codex-asterinas-dev bash -lc 'test -e /dev/kvm && ls -l /dev/kvm || echo no-kvm'`
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test charset_'`
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test lookup_reuses_the_canonical_child_handle_for_case_equivalent_names'`
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test lookup_miss_does_not_publish_a_synthetic_child_handle'`
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test readdir_emits_visible_entries_in_stable_order'`
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test readdir_continuation_remains_stable_across_repeated_calls'`
- If the guest-side failure is unclear after a nonzero run, read-only inspection of `/home/halifuda/asterinas/qemu-serial.log` is allowed.

## Parallelism Classification

- Lane class: `runtime/test-producing`
- May overlap with command-free lanes only
- Known conflicts:
  - serialized checker command lane
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`

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
  - `bash ./kernel/src/fs/fs_impls/exfat_refactor/.agents/tools/checker_lock.sh acquire --component EXR-CHARSET-32 --phase serial --command "<exact docker commands actually used>" --retry-seconds 60 --wait-budget-seconds 1800`
- Release with:
  - `bash ./kernel/src/fs/fs_impls/exfat_refactor/.agents/tools/checker_lock.sh release`
- Stale-lock review is main-agent-only.

## Stop Condition

- Stop after writing `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-CHARSET-32/11_checker_serial.md`.

## Escalation Rule

- If the checker needs edits outside `fs.rs`, `inode.rs`, or the checker artifact, or cannot get trustworthy filtered-test evidence from the allowed commands, report that and stop.
