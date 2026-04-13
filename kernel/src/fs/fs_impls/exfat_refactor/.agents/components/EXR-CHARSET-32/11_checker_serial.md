<!-- SPDX-License-Identifier: MPL-2.0 -->

# Checker Report

## Metadata

- Component ID: `EXR-CHARSET-32`
- Title: `ExfatFs` Charset And External-Name Conversion Boundary
- Status: `SerialChecked`
- Author: Codex
- Date: `2026-04-13`
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-CHARSET-32/20260413-1409-checker-serial-packet.md`
- Checked implementation: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`, `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/inode.rs`
- Pass kind: `serial`

## Scope of Review

Checked the owner-local charset boundary in `fs.rs` and the migrated read-side consumers in `inode.rs`. Added checker-owned `#[ktest]` regressions for valid name conversion, valid label conversion, visible-name decode through `ExfatFs`, overlong rejection, and repeated-call determinism. Attempted the required filtered executable proof for the checker lane.

## Test Changes

- Added five checker-owned `#[ktest]` regressions in [`fs.rs`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs).
- The new checker-owned tests carry short scenario comments.
- No test relocation was needed.
- No `inode.rs` test edits were required for this pass.

## Findings

### Finding

- Severity: `P1`
- Location: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs:794`
- Description: `cargo osdk test charset_` is blocked by an unrelated compile error in `directory.rs` (`BlockDevice::write_bytes` is used without `VmIo` in scope). That prevents the checker from getting trustworthy filtered proof for the required charset and read-side regressions.
- Violated spec clause or expected behavior: The serial checker must run filtered executable verification and stop only after trustworthy proof for the assigned slice.
- Reproduction or reasoning: Running `docker exec codex-asterinas-dev bash -lc 'cd /root/asterinas/kernel && cargo osdk test charset_'` fails before any `charset_` test executes, so the required proof set cannot complete from the allowed checker commands.

## Verified Properties

- The checker-owned charset regressions are now present in `fs.rs` and cover the required boundary cases.
- `inode.rs` continues to route lookup and readdir through the `ExfatFs` owner helpers rather than local name conversion or decode policy.
- The local `fs.rs` fixes made for this pass stayed inside the allowed write set.

## Unverified Properties

- The required filtered `charset_` run did not complete because of the unrelated `directory.rs` compile failure.
- The required `lookup_reuses_the_canonical_child_handle_for_case_equivalent_names`, `lookup_miss_does_not_publish_a_synthetic_child_handle`, `readdir_emits_visible_entries_in_stable_order`, and `readdir_continuation_remains_stable_across_repeated_calls` proof commands were not trustworthy to run after the shared build failure.

## Recommendation

- Next owner: `directory.rs` owner / main agent
- Reason: unblock the kernel test build so the serial checker can complete the required filtered proof set.
- Blocking or non-blocking: blocking
- If this is the last checker pass before acceptance, state whether it was a required final checker or a previously recorded skip case: required serial checker pass, but the proof run was blocked by an unrelated build failure outside the allowed write set.
