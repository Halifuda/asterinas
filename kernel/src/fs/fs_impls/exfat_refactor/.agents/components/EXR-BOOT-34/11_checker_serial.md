<!-- SPDX-License-Identifier: MPL-2.0 -->

# Checker Report

## Metadata

- Component ID: `EXR-BOOT-34`
- Title: `ExfatFs` Boot-Region Fallback And Persistent Boot-Flag Policy
- Status: `SerialChecked`
- Author: checker
- Date: `2026-04-13`
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-BOOT-34/20260413-1938-checker-serial-packet.md`
- Checked implementation: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- Pass kind: `serial`

## Scope of Review

Checked the landed `EXR-BOOT-34` policy slice in `fs.rs`, focusing on the owner-private boot-policy snapshot, primary-default source selection, persistent dirty-boot intent, and observational `percent_in_use` handling. I also verified the local mount/open publication path that consumes the policy snapshot before the ready root inode becomes visible.

## Test Changes

- Added three local `#[ktest]` regressions in `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`:
  - `boot_policy_publishes_before_root_open_and_stays_stable`
  - `boot_policy_dirty_intent_stays_separate_from_trusted_source`
  - `boot_policy_percent_in_use_is_observational_only`
- These tests are local to `fs.rs` and use owner-private helpers directly, which is permitted by the packet for this row.
- The checker run used source-backed suffix proof for the exact filters, since the runner output did not print the executed test names explicitly.
- The guest run used TCG, not KVM.

## Findings

No findings.

## Verified Properties

- `ExfatFs::open_root_inode()` publishes the boot-policy snapshot before root publication becomes visible, and repeated publication stays stable once the snapshot exists.
- The trusted boot source remains `Primary` on the production path when no fallback candidate is provided.
- A fallback candidate can be selected explicitly and remains owner-private to `ExfatFs`.
- Persistent dirty-boot intent is retained separately from the trusted-source decision and is exposed through `published_boot_dirty_intent()`.
- `ClearToZero` remains represented as persistent boot-region intent in the snapshot carrier.
- Changing only `percent_in_use` does not perturb the trusted boot source or the dirty-intent publication.

## Unverified Properties

- I did not validate later filesystem-wide sync ordering, because `EXR-SYNC-31` is still intentionally out of scope for this checker packet.
- The ktest run proved the local policy slice and mount/open handoff, but not any future backup parsing path.

## Recommendation

- Next owner: reviewer
- Reason: the serial checker slice is validated locally and the remaining work is cross-cutting review, not more checker-only repair.
- Blocking or non-blocking: non-blocking
- If this is the last checker pass before acceptance, state whether it was a required final checker or a previously recorded skip case: required serial checker pass, not a skip case.
