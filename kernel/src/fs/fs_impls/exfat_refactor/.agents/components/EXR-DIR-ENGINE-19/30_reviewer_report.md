<!-- SPDX-License-Identifier: MPL-2.0 -->

# Reviewer Report

## Metadata

- Component ID: `EXR-DIR-ENGINE-19`
- Title: `DirectoryEngine` Read-Only Record Stream
- Status: `Reviewed`
- Author: reviewer
- Date: `2026-04-10`
- Reviewed implementation:
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs`
  - `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/mod.rs`

## Review Scope

Reviewed the landed `DirectoryEngine` implementation against the designer boundary, the creator record, and the two checker reports.

The review focused on read-only boundary discipline, visibility hygiene, helper justification, and whether the top-level stream only exposes the designer-approved record shapes.

## Findings

### Finding

- Severity: 2
- Location: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/directory.rs:104-108`
- Description: The initial fallback path surfaced every non-file, non-`Bitmap`, non-`Upcase` dentry as a generic singleton candidate. That was broader than the designer contract, which only authorizes raw singleton candidates for `Bitmap` and `Upcase`.
- Violated spec clause or expected behavior: The directory stream should remain a read-only owner-internal service that emits validated file records and raw `Bitmap`/`Upcase` candidates only.
- Reproduction or reasoning: A top-level vendor, generic primary, or generic secondary dentry would have been accepted as if it were a meaningful candidate instead of being rejected as malformed top-level directory content. I corrected this in-place by returning `EINVAL` for unexpected top-level dentries.

## Direct Edits

- Production code changed: `yes`
- Functional or semantic edits introduced: `yes`
- Changed `directory.rs` only, by narrowing the top-level fallback to reject unexpected dentries instead of surfacing them as generic singleton candidates.

## Residual Concerns

- I did not run build, test, or QEMU commands in this review pass.
- Checker retry evidence already covers the intended directory-engine scenarios, but this review relied on static inspection only.

## Recommendation

- Next owner: `main-agent`
- Reason: The directory-engine review is complete and the bounded quality issue was fixed locally.
- Final-checker recommendation: `skippable`
- Basis for that recommendation: the checker retry already provides passing executable evidence, and this review did not introduce any additional verification risk beyond the local hardening edit.
