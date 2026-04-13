<!-- SPDX-License-Identifier: MPL-2.0 -->

# Checker Report

## Metadata

- Component ID: `EXR-CHARSET-32`
- Title: `ExfatFs` Charset And External-Name Conversion Boundary
- Status: `SerialChecked`
- Author: Codex
- Date: `2026-04-13`
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-CHARSET-32/20260413-1744-checker-retry-packet.md`
- Checked implementation: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`, `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- Pass kind: `serial`

## Scope of Review

Reran the blocked serial checker proof for the owner-local charset conversion boundary in `fs.rs` and the read-side lookup/readdir consumers in `inode.rs`. Verified the exact regression names listed in the retry packet, using complete `cargo osdk test` filters rather than a prefix fragment.

## Test Changes

- No new tests were added in this retry pass.
- No tests were moved or renamed.
- The existing checker-owned charset regressions remain in `fs.rs`, and the existing read-side regressions remain in `inode.rs`.

## Findings

No findings.

## Verified Properties

- `/dev/kvm` is present in the host container, but each `cargo osdk test` run reported `qemu-system-x86_64` TCG warnings, so the executable proof used TCG rather than KVM.
- `charset_convert_name_accepts_valid_external_name` passed.
- `charset_convert_label_accepts_valid_external_label` passed.
- `charset_visible_name_from_utf16_units_decodes_validated_units` passed.
- `charset_convert_name_and_label_reject_overlong_inputs` passed.
- `charset_repeated_conversion_returns_same_validated_output_shape` passed.
- `lookup_reuses_the_canonical_child_handle_for_case_equivalent_names` passed.
- `lookup_miss_does_not_publish_a_synthetic_child_handle` passed.
- `readdir_emits_visible_entries_in_stable_order` passed.
- `readdir_continuation_remains_stable_across_repeated_calls` passed.
- No local `fs.rs` or `inode.rs` fix was required during this retry.

## Unverified Properties

- None from the retry packet. The requested proof set completed successfully.

## Recommendation

- Next owner: `main-agent`
- Reason: the serial retry proof is complete and no in-scope repair is pending.
- Blocking or non-blocking: non-blocking
- If this is the last checker pass before acceptance, state whether it was a required final checker or a previously recorded skip case: required serial checker retry pass, now satisfied.

## Command Evidence

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
