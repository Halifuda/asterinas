<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet

## Metadata

- Packet ID: `EXR-CHARSET-32-20260413-1744-CHECK-RETRY`
- Packet file: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-CHARSET-32/20260413-1744-checker-retry-packet.md`
- Supersedes:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-CHARSET-32/20260413-1409-checker-serial-packet.md`
- Role: `checker`
- Component: `EXR-CHARSET-32`
- Phase: `serial checker retry`
- Authorizing main agent: `main-agent`
- Date: `2026-04-13 17:44 CST`

## Goal

- Rerun the blocked `EXR-CHARSET-32` serial checker proof now that the unrelated `directory.rs` compile blocker has been fixed, using only complete ktest names rather than prefix fragments, and write the retry checker report.

## Architectural Unit Context

- Functional goal: `ExfatFs`-owned charset and visible-name conversion boundary
- Final architectural owner: `ExfatFs`
- Expected landing form: owner-private converted-name / converted-label value types and owner methods in `fs.rs`, with narrow read-side consumer migration in `inode.rs`

## Required Resolution Questions

- Prove each checker-owned charset regression executes successfully with an exact test-name filter rather than a broad prefix.
- Prove the accepted `lookup` and `readdir_at` read-side regressions still execute successfully after the `inode.rs` migration.
- If the rerun exposes a strictly local `fs.rs` or `inode.rs` issue, make the smallest in-scope fix and record it.
- If execution fails for reasons outside the packet write set, classify that clearly and stop.

## Read Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/templates/CHECKER_REPORT_TEMPLATE.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-CHARSET-32/01_designer_core.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-CHARSET-32/02_designer_async.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-CHARSET-32/03_designer_ktest.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-CHARSET-32/10_creator_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-CHARSET-32/11_checker_serial.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-CHARSET-32/30_reviewer_report.md`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/test_support.rs`
- `/home/halifuda/asterinas/kernel/src/fs/vfs/fs_apis/inode.rs`

## Write Set

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-CHARSET-32/12_checker_serial_retry.md`

## Forbidden Files

- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs`
- `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fileset.rs`
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

- `EXR-CHARSET-32` owns visible-name encode/decode policy; `EXR-UPCASE-20` remains the sole fold/hash owner.
- This is a proof rerun, not a redesign.
- Prefix filters such as `charset_` are not trustworthy proof for `cargo osdk test`; use only complete test names in execution commands.

## Integration Prior Inputs

- The exact charset regression names are:
  - `charset_convert_name_accepts_valid_external_name`
  - `charset_convert_label_accepts_valid_external_label`
  - `charset_visible_name_from_utf16_units_decodes_validated_units`
  - `charset_convert_name_and_label_reject_overlong_inputs`
  - `charset_repeated_conversion_returns_same_validated_output_shape`
- The exact accepted read-side regression names are:
  - `lookup_reuses_the_canonical_child_handle_for_case_equivalent_names`
  - `lookup_miss_does_not_publish_a_synthetic_child_handle`
  - `readdir_emits_visible_entries_in_stable_order`
  - `readdir_continuation_remains_stable_across_repeated_calls`
- Use those complete names directly in `cargo osdk test ...` commands. Do not substitute prefix fragments.

## Workflow Prior Inputs

- Runtime/test-producing checker lane.
- Command-producing verification must hold the checker execution lock.
- Record whether execution used KVM or fell back to TCG.
- If a nonzero exit does not clearly show the guest-side failure in terminal output, inspect `/home/halifuda/asterinas/qemu-serial.log` before classifying it.

## Quality Prior Inputs

- Use the checker-role quality slice from `$exfat-subagent-workflow`.
- Prefer report-only rerun unless a strictly local `fs.rs` or `inode.rs` fix is necessary.

## Allowed Commands

- Read-only shell inspection commands under `/home/halifuda/asterinas`
- Lock-guarded Docker commands in `codex-asterinas-dev`:
  - `docker exec codex-asterinas-dev bash -lc 'test -e /dev/kvm && ls -l /dev/kvm || echo no-kvm'`
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test charset_convert_name_accepts_valid_external_name'`
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test charset_convert_label_accepts_valid_external_label'`
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test charset_visible_name_from_utf16_units_decodes_validated_units'`
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test charset_convert_name_and_label_reject_overlong_inputs'`
  - `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test charset_repeated_conversion_returns_same_validated_output_shape'`
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
  - `bash ./kernel/src/fs/fs_impls/exfat_refactor/.agents/tools/checker_lock.sh acquire --component EXR-CHARSET-32 --phase serial-retry --command "<exact docker commands actually used>" --retry-seconds 60 --wait-budget-seconds 1800`
- Release with:
  - `bash ./kernel/src/fs/fs_impls/exfat_refactor/.agents/tools/checker_lock.sh release`
- Stale-lock review is main-agent-only.

## Stop Condition

- Stop after writing `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/components/EXR-CHARSET-32/12_checker_serial_retry.md`.

## Escalation Rule

- If the checker needs edits outside `fs.rs`, `inode.rs`, or the retry checker artifact, or cannot get trustworthy filtered-test evidence from the allowed commands, report that and stop.
