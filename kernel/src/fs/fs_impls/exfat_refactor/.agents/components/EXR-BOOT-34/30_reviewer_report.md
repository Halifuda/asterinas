<!-- SPDX-License-Identifier: MPL-2.0 -->

# Reviewer Report

## Metadata

- Component ID: `EXR-BOOT-34`
- Title: `ExfatFs` Boot-Region Fallback And Persistent Boot-Flag Policy
- Status: `Reviewed`
- Author: reviewer
- Date: `2026-04-13`
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-BOOT-34/20260413-1951-reviewer-packet.md`
- Reviewed implementation: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- Pass kind: `review`

## Scope Of Review

Reviewed the landed `EXR-BOOT-34` boot-policy boundary in `fs.rs`, focusing on owner-private helper shape, publication-state hygiene, mount/open handoff placement, and whether the checked tests support the intended owner boundaries.

## Findings

No findings.

## Review Notes

- `BootSource`, `BootDirtyIntent`, `BootPolicySnapshot`, and `BootPolicyState` remain owner-private to `ExfatFs`, which matches the packet's boundary expectations.
- `publish_boot_policy()` stays narrow and keeps the decision/state publication inside `ExfatFs` rather than turning into a public boot-policy API or a second parser.
- `published_boot_dirty_intent()` remains a sync-facing projection only, with no evidence of a separate sync manager being introduced here.
- `ExfatFs::open_root_inode()` publishes the boot-policy snapshot before the ready root inode becomes visible, which matches the intended mount/open handoff.
- The checker-added local tests exercise the intended stable-publish, dirty-intent separation, and observational `percent_in_use` behavior without widening the owner boundary.

## Residual Risks

- I did not review or validate later filesystem-wide sync ordering, because that remains intentionally owned by `EXR-SYNC-31`.
- The current review found no boundary drift, but future changes that add a second boot source or a public boot-policy surface would need a fresh review.

## Production Code

- No production code changed in this reviewer pass.
