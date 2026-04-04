<!-- SPDX-License-Identifier: MPL-2.0 -->

# Reviewer Report

## Metadata

- Component ID: `EXR-UPCASE-07B`
- Title: `Canonical Upcase-Backed Case Folding And Name Hashing`
- Status: `Reviewing`
- Author: `reviewer`
- Date: `2026-04-04`
- Task packet: `kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-UPCASE-07B/20260404-1616-reviewer-upcase-table-followup-packet.md`
- Reviewed implementation:
  - [`upcase_table.rs`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/upcase_table.rs)

## Review Scope

Reviewed the accessor-only surface in `upcase_table.rs` for boundary hygiene, visibility discipline, and whether any helper still existed without a named non-test caller after the 07A and 07B refactors.

## Findings

### Finding

- Severity: `medium`
- Location: `/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/upcase_table.rs:79-94`
- Description: `words()`, `byte_size()`, and `checksum()` were pure field-exposing accessors with no production caller. The only uses were local tests in the same file, so the helpers widened the public surface without adding a boundary or invariant. The tests can read the private fields directly from the descendant `mod tests`.
- Guideline or style principle involved: helper discipline, visibility hygiene, and avoiding field-exposing accessors without a proven caller need.
- Action taken: Removed the three accessors and updated the local tests to read `table.words`, `table.byte_size`, and `table.checksum` directly.

## Direct Edits

- Updated [`upcase_table.rs`](/home/halifuda/asterinas/kernel/src/fs/fs_impls/exfat_refactor/upcase_table.rs) to remove the accessor-only helpers and inline the test reads.

## Residual Concerns

- None in the owned surface after the accessor cleanup.

## Recommendation

- Next owner: `checker`
- Reason:
  - the follow-up issue was repaired locally, and the next pass should only need to confirm the narrower surface still satisfies the existing checker coverage.
