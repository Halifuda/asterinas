<!-- SPDX-License-Identifier: MPL-2.0 -->

# Reviewer Report

## Metadata

- Component ID: EXR-INODE-CACHE-18
- Title: ExfatFs Opened-Inode Table And Validated InodeKey
- Status: `Reviewed`
- Author: reviewer
- Date: 2026-04-10
- Task packet: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-INODE-CACHE-18/20260410-1150-reviewer-packet.md`
- Reviewed implementation: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/fs.rs`
- Pass kind: `review`

## Scope of Review

Reviewed the landed opened-inode cache boundary in `fs.rs` against the designer boundary, the checker-owned regressions, and the retry checker evidence. The review focused on owner-private boundary discipline, maintainability, visibility hygiene, and residual local risks inside `fs.rs`.

## Findings

No findings.

## Verified Properties

- `InodeKey` remains an owner-private value type with a narrow constructor that accepts only location facts.
- The opened-inode state remains owned by `ExfatFs` and keeps the ordinary keyed table separate from the root slot.
- The temporary `root_inode()` seam is still preserved for `EXR-FS-OPEN-22` and has not been widened into a public helper surface.
- The checker-added tests remain local to `fs.rs` and are readable scenario-style regressions.
- The final shape stays within the packet boundary and does not introduce a sibling-file dependency or a synthetic root key.

## Unverified Properties

- I did not rerun executable verification in this review lane; the retry checker evidence is treated as authoritative.
- The later `EXR-FS-OPEN-22` wiring remains intentionally out of scope and therefore unverified here.

## Recommendation

- Next owner: main agent
- Reason: The reviewed shape is locally maintainable and consistent with the designer boundary, with no review-blocking defects found in `fs.rs`.
- Blocking or non-blocking: Non-blocking
- If this is the last checker pass before acceptance, state whether it was a required final checker or a previously recorded skip case: Not applicable; this is the reviewer pass after the required checker retry.
- Production code changed: No
