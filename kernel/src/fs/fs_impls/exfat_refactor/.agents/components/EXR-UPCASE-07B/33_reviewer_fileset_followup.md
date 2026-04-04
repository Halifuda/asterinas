<!-- SPDX-License-Identifier: MPL-2.0 -->

# Reviewer Follow-Up Report

## Metadata

- Component ID: `EXR-UPCASE-07B`
- Title: Canonical Upcase-Backed Case Folding And Name Hashing
- Status: `Reviewing`
- Author: `reviewer`
- Date: `2026-04-04`
- Task packet: `kernel/src/fs/fs_impls/exfat_refactor/.agents/subagent-tasks/EXR-UPCASE-07B/20260404-1616-reviewer-fileset-followup-packet.md`
- Reviewed implementation:
  - `kernel/src/fs/fs_impls/exfat_refactor/fileset.rs`

## Review Scope

Checked the current `fileset.rs` consumer boundary for the canonical `NameHash` validation path and the packet-scoped temporary ktest staging surfaces. Focused on boundary hygiene, helper discipline, and whether any retained test-only wrapper still lacked an explicit exit condition.

## Findings

None.

## Direct Edits

- Updated `kernel/src/fs/fs_impls/exfat_refactor/fileset.rs` to replace the temporary staging comments above `new_structure_only()`, `from_trusted_metadata()`, and `from_trusted_metadata_with_upcase()` with explicit `TODO(EXR-UPCASE-07B)` exit-condition comments.

## Residual Concerns

- The ktest-only staging helpers remain in production code under `#[cfg(ktest)]`, but they are now explicitly marked for removal or relocation to dedicated test support once production file-record synthesis no longer depends on local ktests.

## Recommendation

- Next owner: `checker`
- Reason: the follow-up review did not find a new boundary defect, and the remaining work is verification of the existing canonical path and staged helper comments.
